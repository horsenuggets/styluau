#![cfg(feature = "luau")]

use styluau_lib::{format_code, CallParenType, Config, LuaVersion, OutputVerification};

#[test]
fn imported_bindings_are_conservative() {
    for (setup, expected) in [
        ("local F = require('@packages/Fusion')\nlocal ui = F:scoped()\n", true),
        ("const F = require('@packages/Fusion')\nconst ui = F.scoped(F)\n", true),
        ("local F = require('@packages/Fusion')\nlocal initial = F:scoped()\nlocal ui = initial:innerScope()\n", true),
        ("local F = require('@packages/Fusion')\nlocal initial = F:scoped()\nlocal ui = initial:deriveScope()\n", true),
        ("local F = require('@packages/Fusion')\nlocal alias = F\nlocal ui = alias:scoped()\n", true),
        ("local F = require('@packages/Other')\nlocal ui = F:scoped()\n", false),
        ("local F = require('@packages/Fusion')\nlocal ui = F:scoped()\nui = somethingElse\n", false),
        ("local F = require('@packages/Fusion')\nlocal ui = F:scoped()\nlocal ui = somethingElse\n", false),
        ("local F = require('@packages/Fusion')\nlocal ui = F:scoped()\nlocal function f(ui) end\n", false),
        ("local F = require('@packages/Fusion')\nlocal ui = F:scoped()\nfor ui in pairs(values) do end\n", false),
        ("local F = require('@packages/Fusion')\nlocal ui = F:scoped()\nui.New = somethingElse\n", false),
        ("local F = require('@packages/Fusion')\nlocal ui = F:scoped()\nlocal alias = ui\nfunction alias:New() end\n", false),
        ("local F = require('@packages/Fusion')\nlocal self = F:scoped()\nfunction obj:render() return self:New('Frame')({}) end\nlocal ui = self\n", false),
        ("local require = otherLoader\nlocal F = require('@packages/Fusion')\nlocal ui = F:scoped()\n", false),
    ] {
        let source = format!("{}local panel = ui:New(\"Frame\")({{}})\n", setup);
        let config = Config { syntax: LuaVersion::Luau, fusion: true, fusion_scope_names: vec![], ..Config::default() };
        let output = format_code(&source, config.clone(), None, OutputVerification::Full).unwrap();
        assert_eq!(output.contains("ui:New \"Frame\" {}"), expected, "{}", output);
        assert_eq!(output, format_code(&output, config, None, OutputVerification::Full).unwrap());
    }
}

#[test]
fn comment_exclusion_overrides_import_detection() {
    let source = "local F = require('@packages/Fusion')\nlocal ui = F:scoped()\n-- [styluau] fusion not-scope ui\nlocal first = ui:New(\"Frame\")({})\n-- [styluau] fusion scope ui\nlocal second = ui:New(\"Frame\")({})\n";
    let config = Config {
        syntax: LuaVersion::Luau,
        fusion: true,
        fusion_scope_names: vec![],
        ..Config::default()
    };
    let output = format_code(source, config.clone(), None, OutputVerification::Full).unwrap();
    assert!(
        output.contains("local first = ui:New(\"Frame\")({})"),
        "{}",
        output
    );
    assert!(
        output.contains("local second = ui:New \"Frame\" {}"),
        "{}",
        output
    );
    assert_eq!(
        output,
        format_code(&output, config, None, OutputVerification::Full).unwrap()
    );
}

#[test]
fn detects_imported_fusion_scopes_without_affecting_other_calls() {
    let source = r#"local Fusion = require("@packages/Fusion")
local scoped = Fusion.scoped
local ui = scoped(Fusion)
local panel = ui:New("Frame")({ child = render("hello") })
local other = unrelated:New("Frame")({})
"#;
    let config = Config {
        syntax: LuaVersion::Luau,
        call_parentheses: CallParenType::Always,
        fusion: true,
        ..Config::default()
    };
    let output = format_code(source, config.clone(), None, OutputVerification::Full).unwrap();
    assert!(output.contains("ui:New \"Frame\" {"), "{}", output);
    assert!(output.contains("render(\"hello\")"), "{}", output);
    assert!(
        output.contains("unrelated:New(\"Frame\")({})"),
        "{}",
        output
    );
    assert_eq!(
        output,
        format_code(&output, config, None, OutputVerification::Full).unwrap()
    );
}
