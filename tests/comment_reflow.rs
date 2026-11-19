use styluau_lib::{
    format_code, Config, Error, IndentType, LineEndings, LuaVersion, OutputVerification, Range,
    SortRequiresConfig,
};

fn config(width: usize) -> Config {
    Config {
        syntax: LuaVersion::Lua51,
        comment_reflow: true,
        comment_width: width,
        indent_type: IndentType::Spaces,
        ..Config::default()
    }
}

fn check(input: &str, expected: &str, config: Config) {
    let output = format_code(input, config.clone(), None, OutputVerification::Full).unwrap();
    assert_eq!(output, expected);
    assert_eq!(
        format_code(&output, config, None, OutputVerification::Full).unwrap(),
        output
    );
}

#[test]
fn paragraphs_and_blank_lines() {
    check(
        "-- one short line\n-- and another\n--\n-- Separate paragraph\n-- continues here.\nlocal x=1\n",
        "-- one short line and another\n--\n-- Separate paragraph continues here.\nlocal x = 1\n",
        config(90),
    );
    check(
        "-- first paragraph\n\n-- second paragraph\nlocal x=1\n",
        "-- first paragraph\n\n-- second paragraph\nlocal x = 1\n",
        config(90),
    );
}

#[test]
fn width_is_independent_of_code_width() {
    for column_width in [20, 120] {
        let config = Config {
            column_width,
            ..config(20)
        };
        check(
            "-- one two three four five six\nlocal x=1\n",
            "-- one two three\n-- four five six\nlocal x = 1\n",
            config,
        );
    }
}

#[test]
fn default_preserves_existing_line_breaks() {
    assert!(!Config::default().comment_reflow);
    assert_eq!(Config::default().comment_width, 90);
    check(
        "-- one short line\n-- and another\nlocal x=1\n",
        "-- one short line\n-- and another\nlocal x = 1\n",
        Config {
            comment_reflow: false,
            ..config(10)
        },
    );
}

#[test]
fn markdown_items_are_not_joined() {
    check(
        "-- 1. First item\n-- 2. Second item\n--\n-- - Third item\n-- - Fourth item\nlocal x=1\n",
        "-- 1. First item\n-- 2. Second item\n--\n-- - Third item\n-- - Fourth item\nlocal x = 1\n",
        config(90),
    );
}

#[test]
fn comment_only_and_block_end_comments() {
    check(
        "-- first line\n-- second line\n",
        "-- first line second line\n",
        config(90),
    );
    check(
        "do\n    -- first line\n    -- second line\nend\n",
        "do\n    -- first line second line\nend\n",
        config(90),
    );
}

#[test]
fn multiline_comments_keep_delimiters() {
    check(
        "--[=[\nfirst short line\nsecond short line\n]=]\nlocal x=1\n",
        "--[=[\nfirst short line second short line\n]=]\nlocal x = 1\n",
        config(90),
    );
    check(
        "--[[\n\nHeader\n\nfirst short line\nsecond short line\n\n--]]\nlocal x=1\n",
        "--[[\n\nHeader\n\nfirst short line second short line\n\n--]]\nlocal x = 1\n",
        config(90),
    );
}

#[test]
fn wrapped_block_comments_have_stable_delimiter_budgets() {
    let body = "one two three four five six seven eight nine ten ".repeat(4);
    for separator in [" ", "\n"] {
        for depth in [0, 1, 2] {
            let equals = "=".repeat(depth);
            let open = format!("--[{}[", equals);
            let close = format!("]{}]", equals);
            let input = format!("{} {}{}{}local x=1\n", open, body, close, separator);
            for width in [20, 45, 90] {
                let output =
                    format_code(&input, config(width), None, OutputVerification::Full).unwrap();
                check(&output, &output, config(width));
                assert!(output.starts_with(&format!("{}\n", open)), "{}", output);
                assert!(
                    output.lines().all(|line| line.chars().count() <= width),
                    "{}",
                    output
                );
                let actual = output
                    .strip_prefix(&open)
                    .unwrap()
                    .split(&close)
                    .next()
                    .unwrap();
                assert_eq!(
                    actual.split_whitespace().collect::<Vec<_>>(),
                    body.split_whitespace().collect::<Vec<_>>()
                );
            }
        }
    }
    check(
        "--[[one two\nthree four]] local x=1\n",
        "--[[\none two three four\n]]\nlocal x = 1\n",
        config(90),
    );
}

#[test]
fn indentation_and_crlf() {
    check(
        "do\n    -- one two three four five six\nend\n",
        "do\n    -- one two three\n    -- four five six\nend\n",
        config(20),
    );
    check(
        "-- first line\r\n-- second line\r\nlocal x=1\r\n",
        "-- first line second line\r\nlocal x = 1\r\n",
        Config {
            line_endings: LineEndings::Windows,
            ..config(90)
        },
    );
}

#[test]
fn nested_block_comments_and_tab_columns() {
    check(
        "do\n    --[[\n    first line\n    second line\n    ]]\nend\n",
        "do\n    --[[\n    first line second line\n    ]]\nend\n",
        config(90),
    );
    check(
        "do\n\t-- one two three four five six\nend\n",
        "do\n\t-- one two three\n\t-- four five six\nend\n",
        Config {
            indent_type: IndentType::Tabs,
            ..config(20)
        },
    );
}

#[test]
fn block_comment_body_indent_tracks_normalized_code_indent() {
    for original_indent in ["", "  ", "    ", "\t", "\t\t", " \t"] {
        for (indent_type, indent) in [(IndentType::Spaces, "    "), (IndentType::Tabs, "\t")] {
            let input = format!(
                "do\n{0}--[[\n{0}first line\n{0}second line\n{0}]]\nend\n",
                original_indent
            );
            let expected = format!(
                "do\n{0}--[[\n{0}first line second line\n{0}]]\nend\n",
                indent
            );
            check(
                &input,
                &expected,
                Config {
                    indent_type,
                    ..config(90)
                },
            );
        }
    }
}

#[test]
fn inline_comments_and_annotations_are_preserved() {
    for input in [
        "local x=1 -- one two three four five six seven eight\n",
        "local x=--[[one two three four five six seven eight]]1\n",
        "--!strict\n---@param value string\n-- [styluau] ignore\nlocal x=  1\n",
    ] {
        let expected = format_code(
            input,
            Config {
                comment_reflow: false,
                ..config(15)
            },
            None,
            OutputVerification::Full,
        )
        .unwrap();
        check(input, &expected, config(15));
    }
}

#[test]
fn inline_comment_layout_changes_are_idempotent() {
    for input in [
        "local t = { --[[one two three four five six seven eight]] a=1 }\n",
        "local x=call(--[[one two three four five six seven eight]]1)\n",
        "local x=call( -- one two three four five six seven eight\n1)\n",
        "local t = { -- one two three four five six seven eight\na=1 }\n",
    ] {
        let output = format_code(input, config(20), None, OutputVerification::Full).unwrap();
        check(&output, &output, config(20));
    }
}

#[test]
fn indentation_normalization_and_directive_mentions_are_idempotent() {
    check(
        "do\n-- first line\n  -- second line\nend\n",
        "do\n    -- first line second line\nend\n",
        config(90),
    );
    let input =
        "-- This paragraph mentions [styluau] comments end without activating it.\nlocal x = 1\n";
    check(input, input, config(20));
}

#[test]
fn markdown_hard_breaks_survive() {
    check(
        "-- first line  \n-- second line\nlocal x=1\n",
        "-- first line  \n-- second line\nlocal x = 1\n",
        config(90),
    );
}

#[test]
fn one_shot_controls_target_comments_not_statements() {
    check(
        "-- [styluau] comments off\nlocal x=1\n-- first line\n-- second line\nlocal y=2\n-- third line\n-- fourth line\nlocal z=3\n",
        "-- [styluau] comments off\nlocal x = 1\n-- first line\n-- second line\nlocal y = 2\n-- third line fourth line\nlocal z = 3\n",
        config(90),
    );
    check(
        "-- [styluau] comments on\n-- first line\n-- second line\nlocal x=1\n-- third line\n-- fourth line\nlocal y=2\n",
        "-- [styluau] comments on\n-- first line second line\nlocal x = 1\n-- third line\n-- fourth line\nlocal y = 2\n",
        Config { comment_reflow: false, ..config(90) },
    );
}

#[test]
fn nested_regions_restore_previous_state() {
    check(
        "-- [styluau] comments off start\n-- first line\n-- second line\nlocal x=1\n-- [styluau] comments on start\n-- third line\n-- fourth line\nlocal y=2\n-- [styluau] comments end\n-- fifth line\n-- sixth line\nlocal z=3\n-- [styluau] comments end\n-- seventh line\n-- eighth line\nlocal w=4\n",
        "-- [styluau] comments off start\n-- first line\n-- second line\nlocal x = 1\n-- [styluau] comments on start\n-- third line fourth line\nlocal y = 2\n-- [styluau] comments end\n-- fifth line\n-- sixth line\nlocal z = 3\n-- [styluau] comments end\n-- seventh line eighth line\nlocal w = 4\n",
        config(90),
    );
}

#[test]
fn invalid_controls_and_zero_width_are_errors() {
    for (command, reason) in [
        ("", "malformed"),
        ("off maybe", "malformed"),
        ("end", "unmatched"),
        ("on start", "unclosed"),
    ] {
        let input = format!("-- [styluau] comments {}\nlocal x=1\n", command);
        let error = format_code(&input, config(90), None, OutputVerification::Full).unwrap_err();
        assert!(matches!(error, Error::CommentDirectiveError(_)));
        assert!(error.to_string().contains(reason), "{}", error);
        assert!(error.to_string().contains("line 1"), "{}", error);
    }
    assert!(matches!(
        format_code("local x=1", config(0), None, OutputVerification::Full),
        Err(Error::ConfigError(_))
    ));
    check(
        "local s = '-- [styluau] comments end'\n-- [styluau] comments-extra end\nlocal x=1\n",
        "local s = \"-- [styluau] comments end\"\n-- [styluau] comments-extra end\nlocal x = 1\n",
        config(90),
    );
}

#[test]
fn ignore_regions_and_ranges_preserve_unselected_comments() {
    let input = "-- [styluau] ignore start\n-- first line\n-- second line\nlocal x=  1\n-- [styluau] ignore end\n-- third line\n-- fourth line\nlocal y=2\n";
    let output = format_code(input, config(90), None, OutputVerification::Full).unwrap();
    assert!(
        output.contains("-- first line\n-- second line\nlocal x=  1"),
        "{}",
        output
    );
    assert!(output.contains("-- third line fourth line"), "{}", output);

    let input =
        "-- outside one\n-- outside two\nlocal x=1\n-- inside one\n-- inside two\nlocal y=2\n";
    let start = input.find("local y").unwrap();
    let output = format_code(
        input,
        config(90),
        Some(Range::from_values(Some(start), None)),
        OutputVerification::Full,
    )
    .unwrap();
    assert!(
        output.starts_with("-- outside one\n-- outside two\nlocal x=1\n"),
        "{}",
        output
    );
}

#[test]
fn ignored_comment_only_regions_and_implicit_scope_ends() {
    check(
        "-- [styluau] ignore start\n-- first line\n-- second line\n-- [styluau] ignore end\n-- third line\n-- fourth line\nlocal x=1\n",
        "-- [styluau] ignore start\n-- first line\n-- second line\n-- [styluau] ignore end\n-- third line fourth line\nlocal x = 1\n",
        config(90),
    );
    check(
        "-- [styluau] ignore start\nlocal x=  1\n-- first line\n-- second line\n",
        "-- [styluau] ignore start\nlocal x=  1\n-- first line\n-- second line\n",
        config(90),
    );
    check(
        "do\n-- [styluau] ignore start\n-- first line\n-- second line\nlocal x=  1\nend\n-- third line\n-- fourth line\nlocal y=2\n",
        "do\n-- [styluau] ignore start\n-- first line\n-- second line\nlocal x=  1\nend\n-- third line fourth line\nlocal y = 2\n",
        config(90),
    );
}

#[test]
fn require_sorting_preserves_control_boundaries() {
    let input = "-- [styluau] comments off start\n-- first line\n-- second line\nlocal z=require('z')\n-- [styluau] comments end\n-- third line\n-- fourth line\nlocal a=require('a')\n";
    check(
        input,
        "-- [styluau] comments off start\n-- first line\n-- second line\nlocal z = require(\"z\")\n-- [styluau] comments end\n-- third line fourth line\nlocal a = require(\"a\")\n",
        Config { sort_requires: SortRequiresConfig { enabled: true }, ..config(90) },
    );
}
