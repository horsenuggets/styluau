use styluau_lib::{
    format_code, BlockNewlineGaps, CollapseSimpleStatement, Config, LineEndings, LuaVersion,
    OutputVerification, SortRequiresConfig,
};

fn format(input: &str, syntax: LuaVersion) -> String {
    let config = Config {
        syntax,
        ..Config::default()
    };
    format_code(input, config.clone(), None, OutputVerification::None).unwrap()
}

#[test]
#[cfg(feature = "luau")]
fn test_table_trailing_comma_option() {
    let input = "local t = {\n  1,\n  2,\n}\ntype T = {\n  a: number,\n  b: string,\n}\n";
    for trailing in [false, true] {
        let config = Config {
            syntax: LuaVersion::Luau,
            table_trailing_comma: trailing,
            ..Config::default()
        };
        let output = format_code(input, config.clone(), None, OutputVerification::Full).unwrap();
        let comma = if trailing { "," } else { "" };
        assert!(output.contains(&format!("2{}\n}}", comma)), "{}", output);
        assert!(
            output.contains(&format!("b: string{}\n}}", comma)),
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
#[cfg(feature = "luau")]
fn test_table_trailing_comma_comments() {
    let inputs = [
        "local t = {\n  1,\n  2, -- last\n}\n",
        "local t = {\n  1,\n  2 -- last\n}\n",
        "local t = {\n  nested = {\n    1,\n  },\n}\n",
        "type T = {\n  a: number, -- last\n}\n",
    ];
    for trailing in [false, true] {
        let config = Config {
            syntax: LuaVersion::Luau,
            table_trailing_comma: trailing,
            ..Config::default()
        };
        for input in inputs {
            let output =
                format_code(input, config.clone(), None, OutputVerification::Full).unwrap();
            if input.contains("-- last") {
                assert!(output.contains("-- last"), "{}", output);
            }
            assert_eq!(
                output,
                format_code(&output, config.clone(), None, OutputVerification::Full).unwrap()
            );
        }
    }
}

#[test]
#[cfg(feature = "luau")]
fn test_curried_calls_preserve_unrelated_call_parentheses() {
    use styluau_lib::CallParenType;

    let input = "local a = scope:New(\"Frame\")({ child = other({ value = 1 }) })\nlocal b = factory(\"Frame\")({ child = other({ value = 1 }) })\n";
    let config = Config {
        syntax: LuaVersion::Luau,
        call_parentheses: CallParenType::Always,
        ..Config::default()
    };
    let output = format_code(input, config.clone(), None, OutputVerification::Full).unwrap();
    assert!(output.contains("scope:New(\"Frame\")({"), "{}", output);
    assert!(output.contains("factory(\"Frame\")({"), "{}", output);
    assert_eq!(output.matches("other({").count(), 2, "{}", output);
    assert_eq!(
        output,
        format_code(&output, config, None, OutputVerification::Full).unwrap()
    );
}

#[test]
#[cfg(feature = "luau")]
fn test_fusion_call_parentheses() {
    use styluau_lib::CallParenType;

    let input = "local a = scope:New(\"Frame\")({ child = other({ value = 1 }) })\nlocal b = fusion:Hydrate(instance)({ child = other(\"text\") })\nlocal c = factory:New(\"Frame\")({})\nlocal d = scope:Other(\"Frame\")({})\nlocal e = wrapper.scope:New(\"Frame\")({})\n";
    for fusion in [false, true] {
        let config = Config {
            syntax: LuaVersion::Luau,
            call_parentheses: CallParenType::Always,
            fusion,
            ..Config::default()
        };
        let output = format_code(input, config.clone(), None, OutputVerification::Full).unwrap();
        if fusion {
            assert!(output.contains("scope:New \"Frame\" {"), "{}", output);
            assert!(output.contains("fusion:Hydrate(instance) {"), "{}", output);
        } else {
            assert!(output.contains("scope:New(\"Frame\")({"), "{}", output);
            assert!(output.contains("fusion:Hydrate(instance)({"), "{}", output);
        }
        assert!(output.contains("other({"), "{}", output);
        assert!(output.contains("other(\"text\")"), "{}", output);
        assert!(output.contains("factory:New(\"Frame\")({})"), "{}", output);
        assert!(output.contains("scope:Other(\"Frame\")({})"), "{}", output);
        assert!(
            output.contains("wrapper.scope:New(\"Frame\")({})"),
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
#[cfg(feature = "luau")]
fn test_fusion_custom_receiver_names() {
    use styluau_lib::CallParenType;

    let input = "local a = customScope:New(\"Frame\")({})\nlocal b = another:Hydrate(instance)({})\nlocal c = scope:New(\"Frame\")({})\nlocal d = customScopeExtra:New(\"Frame\")({})\n";
    for names in [vec!["customScope", "another"], vec![]] {
        let config = Config {
            syntax: LuaVersion::Luau,
            call_parentheses: CallParenType::Always,
            fusion: true,
            fusion_scope_names: names.iter().map(|name| (*name).to_owned()).collect(),
            ..Config::default()
        };
        let output = format_code(input, config.clone(), None, OutputVerification::Full).unwrap();
        if names.is_empty() {
            assert!(
                output.contains("customScope:New(\"Frame\")({})"),
                "{}",
                output
            );
        } else {
            assert!(
                output.contains("customScope:New \"Frame\" {}"),
                "{}",
                output
            );
            assert!(
                output.contains("another:Hydrate(instance) {}"),
                "{}",
                output
            );
        }
        assert!(output.contains("scope:New(\"Frame\")({})"), "{}", output);
        assert!(
            output.contains("customScopeExtra:New(\"Frame\")({})"),
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
#[cfg(feature = "luau")]
fn test_fusion_call_global_parentheses_modes() {
    use styluau_lib::CallParenType;

    let input = "local x = fusion:New(\"Frame\")({ nested = other({}), child = scope:Hydrate(\"TextLabel\")({}) })\n";
    for mode in [
        CallParenType::Always,
        CallParenType::Input,
        CallParenType::None,
    ] {
        let config = Config {
            syntax: LuaVersion::Luau,
            call_parentheses: mode,
            fusion: true,
            ..Config::default()
        };
        let output = format_code(input, config.clone(), None, OutputVerification::Full).unwrap();
        assert!(output.contains("fusion:New \"Frame\" {"), "{}", output);
        assert!(
            output.contains("scope:Hydrate \"TextLabel\" {}"),
            "{}",
            output
        );
        if mode == CallParenType::None {
            assert!(output.contains("other {}"), "{}", output);
        } else {
            assert!(output.contains("other({})"), "{}", output);
        }
        assert_eq!(
            output,
            format_code(&output, config, None, OutputVerification::Full).unwrap()
        );
    }
}

#[test]
#[cfg(feature = "luau")]
fn test_fusion_call_safety() {
    use styluau_lib::CallParenType;

    let config = Config {
        syntax: LuaVersion::Luau,
        call_parentheses: CallParenType::Always,
        fusion: true,
        ..Config::default()
    };
    for input in [
        "local x = scope:New(\"Frame\").foo\n",
        "local x = scope:New(\"Frame\")({}):foo()\n",
        "local x = scope:New( -- comment\n\"Frame\")({ child = other({}) })\n",
        "local x = scope:New(\"Frame\")(\"not a table\")\n",
        "local x = scope:New(\"Frame\")({})({})\n",
    ] {
        let output = format_code(input, config.clone(), None, OutputVerification::Full).unwrap();
        if input.contains("-- comment") {
            assert!(output.contains("-- comment"), "{}", output);
            assert!(output.contains("other({})"), "{}", output);
        }
        if input.contains(".foo") {
            assert!(output.contains("New(\"Frame\").foo"), "{}", output);
        }
        if input.contains(":foo()") {
            assert!(output.contains("({}):foo()"), "{}", output);
        }
        if input.contains("not a table") {
            assert!(output.contains("(\"not a table\")"), "{}", output);
        }
        if input.contains("({})({})") {
            assert!(output.contains("{}({})"), "{}", output);
        }
        assert_eq!(
            output,
            format_code(&output, config.clone(), None, OutputVerification::Full).unwrap()
        );
    }
}

#[test]
#[cfg(feature = "luau")]
fn test_fusion_keeps_parentheses_containing_comments() {
    use styluau_lib::CallParenType;

    for call_parentheses in [CallParenType::Always, CallParenType::None] {
        let config = Config {
            syntax: LuaVersion::Luau,
            call_parentheses,
            fusion: true,
            ..Config::default()
        };
        for input in [
            "local x = scope:New(--[[important]] \"Frame\")({})\n",
            "local x = scope:New(\"Frame\" --[[important]])({})\n",
            "local y = other(--[[important]] \"text\")\n",
            "local y = other(--[[important]] { value = 1 })\n",
        ] {
            let output =
                format_code(input, config.clone(), None, OutputVerification::Full).unwrap();
            assert!(output.contains("--[[important]]"), "{}", output);
            assert_eq!(
                output,
                format_code(&output, config.clone(), None, OutputVerification::Full).unwrap()
            );
        }
    }
}

#[test]
#[cfg(feature = "luau")]
fn test_fusion_directives_survive_require_sorting() {
    use styluau_lib::CallParenType;

    let config = Config {
        syntax: LuaVersion::Luau,
        call_parentheses: CallParenType::Always,
        fusion: true,
        sort_requires: SortRequiresConfig { enabled: true },
        ..Config::default()
    };
    let input = "-- styluau: fusion on\nlocal z = require(scope:New(\"Frame\")({}))\nlocal a = require(\"a\")\nlocal m = require(\"m\")\n";
    let output = format_code(input, config.clone(), None, OutputVerification::Full).unwrap();
    assert_eq!(
        output,
        "-- styluau: fusion on\nlocal z = require(scope:New \"Frame\" {})\nlocal a = require(\"a\")\nlocal m = require(\"m\")\n"
    );
    assert_eq!(
        output,
        format_code(&output, config, None, OutputVerification::Full).unwrap()
    );
}

#[test]
#[cfg(feature = "luau")]
fn test_fusion_regions_survive_inline_require_directives() {
    use styluau_lib::CallParenType;

    let config = Config {
        syntax: LuaVersion::Luau,
        call_parentheses: CallParenType::Always,
        fusion: true,
        sort_requires: SortRequiresConfig { enabled: true },
        ..Config::default()
    };
    let input = "local z = require(scope:New(\"Frame\")({}))\n--[[styluau: fusion off start]] local a = require(scope:New(\"Frame\")({}))\n-- styluau: fusion end\nlocal b = require(scope:New(\"Frame\")({}))\n";
    let output = format_code(input, config.clone(), None, OutputVerification::Full).unwrap();
    assert!(
        output.contains("local z = require(scope:New \"Frame\" {})"),
        "{}",
        output
    );
    assert!(
        output.contains("local a = require(scope:New(\"Frame\")({}))"),
        "{}",
        output
    );
    assert!(
        output.contains("local b = require(scope:New \"Frame\" {})"),
        "{}",
        output
    );
    assert!(
        output.find("local z").unwrap() < output.find("local a").unwrap(),
        "{}",
        output
    );
    assert_eq!(
        output,
        format_code(&output, config, None, OutputVerification::Full).unwrap()
    );
}

#[test]
#[cfg(feature = "luau")]
fn test_fusion_comment_overrides() {
    use styluau_lib::CallParenType;
    let input = "-- styluau: fusion on\nlocal a = scope:New(\"Frame\")({})\nlocal b = scope:New(\"Frame\")({})\n-- styluau: fusion on start\nlocal c = scope:New(\"Frame\")({})\n-- styluau: fusion off start\nlocal d = scope:New(\"Frame\")({})\n-- styluau: fusion end\n-- styluau: fusion scope custom\nlocal e = custom:New(\"Frame\")({})\n-- styluau: fusion not-scope scope\nlocal f = scope:New(\"Frame\")({})\n-- styluau: fusion end\nlocal g = scope:New(\"Frame\")({})\n";
    let config = Config {
        syntax: LuaVersion::Luau,
        call_parentheses: CallParenType::Always,
        ..Config::default()
    };
    let output = format_code(input, config.clone(), None, OutputVerification::Full).unwrap();
    for name in ["a", "c", "e"] {
        assert!(
            output.contains(&format!(
                "local {} = {}:New \"Frame\" {{}}",
                name,
                if name == "e" { "custom" } else { "scope" }
            )),
            "{}",
            output
        );
    }
    for name in ["b", "d", "f", "g"] {
        assert!(
            output.contains(&format!("local {} = scope:New(\"Frame\")({{}})", name)),
            "{}",
            output
        );
    }
    assert_eq!(
        output,
        format_code(&output, config, None, OutputVerification::Full).unwrap()
    );
}

#[test]
#[cfg(feature = "luau")]
fn test_fusion_comment_field_and_block_scope() {
    use styluau_lib::CallParenType;
    let input = "local t = {\n-- styluau: fusion on\na = scope:New(\"Frame\")({}),\nb = scope:New(\"Frame\")({}),\n}\ndo\n-- styluau: fusion on start\nlocal c = scope:New(\"Frame\")({})\n-- styluau: fusion end\nend\nlocal d = scope:New(\"Frame\")({})\n";
    let config = Config {
        syntax: LuaVersion::Luau,
        call_parentheses: CallParenType::Always,
        ..Config::default()
    };
    let output = format_code(input, config.clone(), None, OutputVerification::Full).unwrap();
    assert!(output.contains("a = scope:New \"Frame\" {}"), "{}", output);
    assert!(
        output.contains("b = scope:New(\"Frame\")({})"),
        "{}",
        output
    );
    assert!(
        output.contains("local c = scope:New \"Frame\" {}"),
        "{}",
        output
    );
    assert!(
        output.contains("local d = scope:New(\"Frame\")({})"),
        "{}",
        output
    );
    assert_eq!(
        output,
        format_code(&output, config, None, OutputVerification::Full).unwrap()
    );
}

#[test]
#[cfg(feature = "luau")]
fn test_fusion_region_applies_inside_formatting_range() {
    let source = "-- styluau: fusion on start\nlocal a = scope:New(\"Frame\")({})\nlocal b = scope:New(\"Frame\")({})\n-- styluau: fusion end\nlocal c = scope:New(\"Frame\")({})\n";
    let start = source.find("local b").unwrap();
    let config = Config {
        syntax: LuaVersion::Luau,
        ..Config::default()
    };
    let end = start + source[start..].find('\n').unwrap();
    let range = Some(styluau_lib::Range::from_values(Some(start), Some(end)));
    let output = format_code(source, config, range, OutputVerification::Full).unwrap();
    assert!(
        output.contains("local a = scope:New(\"Frame\")({})"),
        "{}",
        output
    );
    assert!(
        output.contains("local b = scope:New \"Frame\" {}"),
        "{}",
        output
    );
    assert!(
        output.contains("local c = scope:New(\"Frame\")({})"),
        "{}",
        output
    );
}

#[test]
#[cfg(feature = "luau")]
fn test_fusion_field_directive_applies_to_range_body() {
    let source = "local t = {\n-- styluau: fusion off\ncallback = function()\nlocal x = scope:New(\"Frame\")({})\nend,\n}\n";
    let start = source.find("local x").unwrap();
    let end = start + source[start..].find('\n').unwrap();
    let config = Config {
        syntax: LuaVersion::Luau,
        fusion: true,
        ..Config::default()
    };
    let output = format_code(
        source,
        config,
        Some(styluau_lib::Range::from_values(Some(start), Some(end))),
        OutputVerification::Full,
    )
    .unwrap();
    assert!(
        output.contains("local x = scope:New(\"Frame\")({})"),
        "{}",
        output
    );
}

#[test]
fn test_ignore_directive_names() {
    for name in ["styluau", "stylua"] {
        let input = format!("-- {}: ignore\nlocal x=  1\nlocal y=2\n", name);
        let output = format(&input, LuaVersion::Lua51);
        assert!(output.contains("local x=  1"), "{}", output);
        assert!(output.contains("local y = 2"), "{}", output);
    }
}

#[test]
fn test_standard() {
    insta::glob!("inputs/*.lua", |path| {
        let contents = std::fs::read_to_string(path).unwrap();
        insta::assert_snapshot!(format(&contents, LuaVersion::Lua51));
    })
}

#[test]
fn test_full_moon_test_suite() {
    insta::glob!("inputs-full_moon/*.lua", |path| {
        let contents = std::fs::read_to_string(path).unwrap();
        insta::assert_snapshot!(format(&contents, LuaVersion::Lua51));
    })
}

#[test]
#[cfg(feature = "luau")]
fn test_luau() {
    insta::glob!("inputs-luau/*.lua", |path| {
        dbg!(path);
        let contents = std::fs::read_to_string(path).unwrap();
        insta::assert_snapshot!(format(&contents, LuaVersion::Luau));
    })
}

#[test]
#[cfg(feature = "luau")]
fn test_luau_full_moon() {
    insta::glob!("inputs-luau-full_moon/*.lua", |path| {
        let contents = std::fs::read_to_string(path).unwrap();
        insta::assert_snapshot!(format(&contents, LuaVersion::Luau));
    })
}

#[test]
#[cfg(feature = "lua52")]
fn test_lua52() {
    insta::glob!("inputs-lua52/*.lua", |path| {
        let contents = std::fs::read_to_string(path).unwrap();
        insta::assert_snapshot!(format(&contents, LuaVersion::Lua52));
    })
}

#[test]
#[cfg(feature = "lua53")]
fn test_lua53() {
    insta::glob!("inputs-lua53/*.lua", |path| {
        let contents = std::fs::read_to_string(path).unwrap();
        insta::assert_snapshot!(format(&contents, LuaVersion::Lua53));
    })
}

#[test]
#[cfg(feature = "lua54")]
fn test_lua54() {
    insta::glob!("inputs-lua54/*.lua", |path| {
        let contents = std::fs::read_to_string(path).unwrap();
        insta::assert_snapshot!(format(&contents, LuaVersion::Lua54));
    })
}

#[test]
#[cfg(feature = "luajit")]
fn test_luajit() {
    insta::glob!("inputs-luajit/*.lua", |path| {
        let contents = std::fs::read_to_string(path).unwrap();
        insta::assert_snapshot!(format(&contents, LuaVersion::LuaJIT));
    })
}

#[test]
#[cfg(feature = "cfxlua")]
fn test_cfxlua() {
    insta::glob!("inputs-cfxlua/*.lua", |path| {
        let contents = std::fs::read_to_string(path).unwrap();
        insta::assert_snapshot!(format(&contents, LuaVersion::CfxLua));
    })
}

#[test]
fn test_ignores() {
    insta::glob!("inputs-ignore/*.lua", |path| {
        let contents = std::fs::read_to_string(path).unwrap();
        insta::assert_snapshot!(format(&contents, LuaVersion::Lua51));
    })
}

#[test]
fn test_collapse_single_statement() {
    insta::glob!("inputs-collapse-single-statement/*.lua", |path| {
        let contents = std::fs::read_to_string(path).unwrap();
        insta::assert_snapshot!(format_code(
            &contents,
            Config {
                collapse_simple_statement: CollapseSimpleStatement::Always,
                ..Config::default()
            },
            None,
            OutputVerification::None
        )
        .unwrap());
    })
}

#[test]
fn test_collapse_simple_statement_input_preserves_singleline() {
    let config = Config {
        collapse_simple_statement: CollapseSimpleStatement::Input,
        ..Config::default()
    };

    insta::assert_snapshot!(
        format_code(
            r#"function foo() return  bar end
if x then return  y end
"#,
            config,
            None,
            OutputVerification::None
        )
        .unwrap(),
        @r###"
    function foo() return bar end
    if x then return y end
    "###
    );
}

#[test]
fn test_collapse_simple_statement_input_preserves_multiline() {
    let config = Config {
        collapse_simple_statement: CollapseSimpleStatement::Input,
        ..Config::default()
    };

    insta::assert_snapshot!(
        format_code(
            r#"function foo()
    return  bar
end
if x then
    return  y
end
"#,
            config,
            None,
            OutputVerification::None
        )
        .unwrap(),
        @r###"
    function foo()
    	return bar
    end
    if x then
    	return y
    end
    "###
    );
}

#[test]
fn test_collapse_simple_statement_input_preserves_empty_function_shape() {
    let config = Config {
        collapse_simple_statement: CollapseSimpleStatement::Input,
        ..Config::default()
    };

    insta::assert_snapshot!(
        format_code(
            r#"function singleline() end
function multiline()
end
"#,
            config,
            None,
            OutputVerification::None
        )
        .unwrap(),
        @r###"
    function singleline() end
    function multiline()
    end
    "###
    );
}

#[test]
fn test_collapse_simple_statement_input_respects_column_width() {
    let config = Config {
        collapse_simple_statement: CollapseSimpleStatement::Input,
        column_width: 40,
        ..Config::default()
    };

    insta::assert_snapshot!(
        format_code(
            r#"if foooooooooooooooooooooooooooooooooooooooo then return  bar end
local function foo(aaaaaaaaaaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbbb, cccccccccccccccc) local x = 1 end
"#,
            config,
            None,
            OutputVerification::None
        )
        .unwrap(),
        @r###"
    if
    	foooooooooooooooooooooooooooooooooooooooo
    then
    	return bar
    end
    local function foo(
    	aaaaaaaaaaaaaaaaaaaa,
    	bbbbbbbbbbbbbbbbbbbb,
    	cccccccccccccccc
    )
    	local x = 1
    end
    "###
    );
}

#[test]
fn test_collapse_simple_statement_input_preserves_function_argument_hug() {
    let config = Config {
        collapse_simple_statement: CollapseSimpleStatement::Input,
        ..Config::default()
    };

    insta::assert_snapshot!(
        format_code(
            r#"task.spawn(function()

end)
x.SomeMethod("something", function()

end)
"#,
            config,
            None,
            OutputVerification::None
        )
        .unwrap(),
        @r###"
    task.spawn(function()
    end)
    x.SomeMethod("something", function()
    end)
    "###
    );
}

#[test]
fn test_preserve_block_newline_gaps() {
    insta::glob!("inputs-preserve-block-newline-gaps/*.lua", |path| {
        let contents = std::fs::read_to_string(path).unwrap();
        insta::assert_snapshot!(format_code(
            &contents,
            Config {
                block_newline_gaps: BlockNewlineGaps::Preserve,
                ..Config::default()
            },
            None,
            OutputVerification::None
        )
        .unwrap());
    })
}

// Collapse simple statement for goto
#[test]
#[cfg(feature = "lua52")]
fn test_collapse_single_statement_lua_52() {
    insta::assert_snapshot!(
        format_code(
            r###"
            if key == "s" then
                goto continue
            end
            "###,
            Config {
                syntax: LuaVersion::Lua52,
                collapse_simple_statement: CollapseSimpleStatement::Always,
                ..Config::default()
            },
            None,
            OutputVerification::None
        )
        .unwrap(),
        @r###"
    if key == "s" then goto continue end
    "###
    );
}

#[test]
fn test_sort_requires() {
    insta::glob!("inputs-sort-requires/*.lua", |path| {
        let contents = std::fs::read_to_string(path).unwrap();
        insta::assert_snapshot!(format_code(
            &contents,
            Config {
                sort_requires: SortRequiresConfig { enabled: true },
                ..Config::default()
            },
            None,
            OutputVerification::None
        )
        .unwrap());
    })
}

#[test]
#[cfg(feature = "luau")]
fn test_sort_requires_luau_const() {
    let input = r#"const Zebra = require("Zebra") :: any
local Apple = require("Apple")
const Banana: any = require("Banana")
const Players = game:GetService("Players")
const Lighting = game:GetService("Lighting")
local Workspace = game:GetService("Workspace")

const SplitZ = require("SplitZ")
local value = 1
const SplitA = require("SplitA")
"#;

    let output = format_code(
        input,
        Config {
            syntax: LuaVersion::Luau,
            sort_requires: SortRequiresConfig { enabled: true },
            ..Config::default()
        },
        None,
        OutputVerification::None,
    )
    .unwrap();

    assert_eq!(
        output,
        r#"local Apple = require("Apple")
const Banana: any = require("Banana")
const Zebra = require("Zebra") :: any
const Lighting = game:GetService("Lighting")
const Players = game:GetService("Players")
local Workspace = game:GetService("Workspace")

const SplitZ = require("SplitZ")
local value = 1
const SplitA = require("SplitA")
"#
    );
}

#[test]
fn test_crlf_in_multiline_comments() {
    // We need to do this outside of insta since it normalises line endings to LF
    let code = r#"
local a = "testing"
--[[
    This comment
    is multiline
    and we want to ensure the line endings
    convert to CRLF
]]
local x = 1
"#;

    let code_crlf = code.lines().collect::<Vec<_>>().join("\r\n");
    let output = format(&code_crlf, LuaVersion::Lua51);
    assert_eq!(output.find("\r\n"), None);
}

#[test]
fn test_crlf_in_multiline_strings() {
    // We need to do this outside of insta since it normalises line endings to LF
    let code = r###"
local a = [[
    This string
    is multiline
    and we want to ensure the line endings
    convert to CRLF
]]
local x = 1
"###;

    let code_crlf = code.lines().collect::<Vec<_>>().join("\r\n");
    let output = format(&code_crlf, LuaVersion::Lua51);
    assert_eq!(output.find("\r\n"), None);
}

/// Comments are lifted out of trivia and re-emitted without being reformatted, so each way of
/// doing that is exercised: the source's `\r` must not survive into the output, otherwise the
/// configured line ending gets appended after it (`\r\r\n`) and formatting is not idempotent.
fn assert_crlf_normalised(input: &str) {
    let input = input.replace('\n', "\r\n");

    for line_endings in [LineEndings::Windows, LineEndings::Unix] {
        let config = Config {
            line_endings,
            ..Config::default()
        };
        let once = format_code(&input, config.clone(), None, OutputVerification::None).unwrap();
        let expected_eol = match line_endings {
            LineEndings::Windows => "\r\n",
            LineEndings::Unix => "\n",
        };
        assert_eq!(
            once.replace("\r\n", "\n").replace('\n', expected_eol),
            once,
            "unexpected line endings with {:?}: {:?}",
            line_endings,
            once
        );

        let twice = format_code(&once, config, None, OutputVerification::None).unwrap();
        assert_eq!(
            once, twice,
            "formatting is not idempotent for CRLF input with {:?}",
            line_endings
        );
    }
}

#[test]
fn test_crlf_comment_in_binop_chain() {
    assert_crlf_normalised(
        "local x = \"a\"\n\t.. \"b\"\n\t-- comment\n\t.. \"c\"\nreturn x -- trailing\n",
    );
}

#[test]
fn test_crlf_comments_around_semicolon() {
    assert_crlf_normalised("local x = 1 -- a\n; -- b\nlocal y = 2\n");
}

#[test]
fn test_crlf_comments_around_assignment_equals() {
    assert_crlf_normalised("local x -- a\n= -- b\n\t1\nx -- c\n= -- d\n\t2\n");
}

#[test]
fn test_crlf_comments_around_table_field_equals() {
    assert_crlf_normalised("local t = {\n\ta -- a\n\t= -- b\n\t1,\n\t-- c\n\tb = 2, -- d\n}\n");
}

#[test]
fn test_crlf_comments_around_parentheses() {
    assert_crlf_normalised("local x = ( -- a\n\ty -- b\n) -- c\n");
}

#[test]
fn test_crlf_comments_around_function_arguments() {
    assert_crlf_normalised("call(a, -- a\n\tb, -- b\n\tc -- c\n)\n");
}

#[test]
fn test_crlf_comments_in_block_statements() {
    assert_crlf_normalised(
        "if a then -- a\n\t-- b\nelseif b then -- c\nelse -- d\nend -- e\nfor i = 1, 2 do -- f\nend\n",
    );
}

#[cfg(feature = "luau")]
#[test]
fn test_crlf_comments_in_luau_types() {
    assert_crlf_normalised("type Foo<T> = -- a\n\tT\ntype Bar = A -- b\n\t-- c\n\t| B\n");
}
