use styluau_lib::{format_code, Config, Error, OutputVerification, Range};

fn check(source: &str, range: Option<Range>) -> Result<String, Error> {
    format_code(source, Config::default(), range, OutputVerification::None)
}

#[test]
fn valid_nested_regions_and_commands() {
    let source = "-- [styluau] fusion on start\nlocal a = 1\n-- [styluau] fusion off start\nlocal b = 2\n-- [styluau] fusion end\n-- [styluau] fusion scope my_scope\n-- [styluau] fusion not-scope my_scope\n-- [styluau] fusion on\nlocal c = 3\n-- [styluau] fusion off\nlocal d = 4\n-- [styluau] fusion end\nlocal e = 5\n";
    assert!(check(source, None).is_ok());
}

#[test]
fn valid_regions_inside_blocks_and_table_fields() {
    let source = "if true then\n-- [styluau] fusion on start\nlocal a = 1\n-- [styluau] fusion end\nlocal b = 2\nend\nlocal t = {\n-- [styluau] fusion off start\na = 1,\n-- [styluau] fusion end\nb = 2,\n}\n";
    assert!(check(source, None).is_ok());
}

#[test]
fn multiline_comment_directives_are_checked() {
    let source = "--[[\n[styluau] fusion on start\n[styluau] fusion end\n]]\nlocal a = 1\n";
    assert!(check(source, None).is_ok());
    assert!(matches!(
        check("--[[\n[styluau] fusion end\n]]", None),
        Err(Error::FusionDirectiveError(_))
    ));
}

#[test]
fn invalid_directives_are_rejected() {
    for (directive, expected) in [
        ("on maybe", "malformed"),
        ("", "malformed"),
        ("scope 1name", "identifier"),
        ("scope end", "identifier"),
        ("not-scope a.b", "identifier"),
        ("end", "unmatched"),
        ("on start", "unclosed"),
    ] {
        let source = format!("-- [styluau] fusion {directive}\nlocal a = 1\n");
        let err = check(&source, None).unwrap_err();
        assert!(matches!(err, Error::FusionDirectiveError(_)), "{:?}", err);
        assert!(err.to_string().contains(expected), "{}", err);
    }
}

#[test]
fn strings_and_unrelated_comments_are_not_directives() {
    let source = "local a = '-- [styluau] fusion end'\nlocal b = [[-- [styluau] fusion on start]]\n-- [styluau] fusional end\n-- unrelated [styluau] fusion end\n-- [styluau-extra] fusion end\n-- [styluau fusion end\n-- styluau] fusion end\n-- [styluau]fusion end\n-- styluau: fusion end\n";
    assert!(check(source, None).is_ok());
}

#[test]
fn old_colon_fusion_commands_are_plain_comments() {
    for (fusion, receiver, command, omitted) in [
        (false, "scope", "on", false),
        (false, "scope", "on start", false),
        (true, "scope", "off", true),
        (true, "scope", "off start", true),
        (true, "scope", "end", true),
        (true, "scope", "not-scope scope", true),
        (true, "custom", "scope custom", false),
    ] {
        let input = format!(
            "-- styluau: fusion {}\nlocal x = {}:New(\"Frame\")({{}})\n",
            command, receiver
        );
        let config = Config {
            fusion,
            ..Config::default()
        };
        let output = format_code(&input, config.clone(), None, OutputVerification::Full).unwrap();
        assert_eq!(
            output.contains(&format!("{}:New \"Frame\" {{}}", receiver)),
            omitted,
            "{}",
            output
        );
        assert_eq!(
            output,
            format_code(&output, config, None, OutputVerification::Full).unwrap()
        );
    }
}

#[test]
fn entire_ast_is_checked_for_range_formatting() {
    let source = "local a = 1\n-- [styluau] fusion end\nlocal b = 2\n";
    assert!(matches!(
        check(source, Some(Range::from_values(Some(0), Some(5)))),
        Err(Error::FusionDirectiveError(_))
    ));
}

#[test]
fn regions_end_before_block_delimiters() {
    for source in [
        "do\n-- [styluau] fusion on start\nlocal x = 1\n-- [styluau] fusion end\nend\n",
        "if true then\n-- [styluau] fusion on start\nlocal x = 1\n-- [styluau] fusion end\nelse\nlocal y = 2\nend\n",
        "repeat\n-- [styluau] fusion on start\nlocal x = 1\n-- [styluau] fusion end\nuntil true\n",
    ] {
        assert!(check(source, None).is_ok(), "{}", source);
    }
    let source =
        "do\n-- [styluau] fusion on start\nlocal x = 1\nend\n-- [styluau] fusion end\nlocal y = 2\n";
    assert!(matches!(
        check(source, None),
        Err(Error::FusionDirectiveError(_))
    ));
}

#[test]
fn regions_before_positional_tables_belong_to_the_parent() {
    let valid =
        "local t = {\n-- [styluau] fusion on start\n{},\n-- [styluau] fusion end\nvalue,\n}\n";
    assert!(check(valid, None).is_ok());
    let invalid = "local t = {\n-- [styluau] fusion on start\n{\n-- [styluau] fusion end\nvalue,\n},\nother,\n}\n";
    assert!(matches!(
        check(invalid, None),
        Err(Error::FusionDirectiveError(_))
    ));
}

#[test]
#[cfg(feature = "luau")]
fn regions_cannot_cross_table_type_boundaries() {
    let invalid =
        "-- [styluau] fusion on start\ntype T = {\n-- [styluau] fusion end\na: number,\n}\n";
    assert!(matches!(
        check(invalid, None),
        Err(Error::FusionDirectiveError(_))
    ));
    let valid =
        "type T = {\n-- [styluau] fusion on start\na: () -> (),\n-- [styluau] fusion end\nb: number,\n}\n";
    assert!(check(valid, None).is_ok());
}

#[test]
fn regions_cannot_cross_nested_table_or_block_boundaries() {
    let table = "-- [styluau] fusion on start\nlocal x = {\n-- [styluau] fusion end\na = 1\n}\n";
    let block =
        "-- [styluau] fusion on start\nif true then\n-- [styluau] fusion end\nlocal x = 1\nend\n";
    for source in [table, block] {
        assert!(matches!(
            check(source, None),
            Err(Error::FusionDirectiveError(_))
        ));
    }
}
