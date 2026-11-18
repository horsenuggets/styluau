use assert_cmd::Command;
use assert_fs::prelude::*;

macro_rules! construct_tree {
    ({ $($file_name:literal:$file_contents:literal,)* }) => {{
        let cwd = assert_fs::TempDir::new().unwrap();

        $(
            cwd.child($file_name).write_str($file_contents).unwrap();
        )*

        cwd
    }};
}

fn create_stylua() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("styluau"))
}

#[test]
fn test_no_files_provided() {
    let mut cmd = create_stylua();
    cmd.assert()
        .failure()
        .code(2)
        .stderr("error: no files provided\n");
}

#[test]
fn test_format_stdin() {
    let mut cmd = create_stylua();
    cmd.arg("-")
        .write_stdin("local   x   = 1")
        .assert()
        .success()
        .stdout("local x = 1\n");
}

#[test]
fn test_format_file() {
    let cwd = construct_tree!({
        "foo.lua": "local   x    =   1",
    });

    let mut cmd = create_stylua();
    cmd.current_dir(cwd.path()).arg(".").assert().success();

    cwd.child("foo.lua").assert("local x = 1\n");

    cwd.close().unwrap();
}

#[test]
fn test_styluau_config_trailing_comma() {
    for (setting, expected) in [
        ("false", "local x = {\n\t1\n}\n"),
        ("true", "local x = {\n\t1,\n}\n"),
    ] {
        let cwd = construct_tree!({
            "foo.lua": "local x = {\n1,\n}\n",
        });
        cwd.child("styluau.toml")
            .write_str(&format!("table_trailing_comma = {}", setting))
            .unwrap();
        create_stylua()
            .current_dir(cwd.path())
            .arg("foo.lua")
            .assert()
            .success();
        cwd.child("foo.lua").assert(expected);
    }
}

#[test]
fn test_ordered_glob_overrides_per_file() {
    let cwd = construct_tree!({
        "styluau.toml": "indent_type = 'Spaces'\nindent_width = 4\n[[overrides]]\nfiles = ['*.lua']\nindent_width = 2\n[[overrides]]\nfiles = ['special.lua']\nindent_width = 3\n",
        "special.lua": "if true then\nprint(1)\nend\n",
        "other.lua": "if true then\nprint(1)\nend\n",
        "nested/other.lua": "if true then\nprint(1)\nend\n",
    });
    create_stylua()
        .current_dir(cwd.path())
        .arg(".")
        .assert()
        .success();
    cwd.child("special.lua")
        .assert("if true then\n   print(1)\nend\n");
    cwd.child("other.lua")
        .assert("if true then\n  print(1)\nend\n");
    cwd.child("nested/other.lua")
        .assert("if true then\n    print(1)\nend\n");
}

#[test]
fn test_glob_overrides_explicit_config_and_stdin() {
    let cwd = construct_tree!({
        "config/styluau.toml": "indent_type = 'Spaces'\nindent_width = 4\n[[overrides]]\nfiles = ['*.lua']\nindent_width = 2\n",
    });
    for (filepath, width, cli) in [
        (Some("config/a.lua"), 2, false),
        (None, 4, false),
        (Some("config/a.lua"), 3, true),
        (Some("a.lua"), 4, false),
    ] {
        let mut cmd = create_stylua();
        cmd.current_dir(cwd.path())
            .args(["--config-path", "config/styluau.toml"]);
        if let Some(filepath) = filepath {
            cmd.args(["--stdin-filepath", filepath]);
        }
        if cli {
            cmd.args(["--indent-width", "3"]);
        }
        cmd.arg("-")
            .write_stdin("if true then\nprint(1)\nend\n")
            .assert()
            .success()
            .stdout(format!(
                "if true then\n{}print(1)\nend\n",
                " ".repeat(width)
            ));
    }
}

#[test]
#[cfg(feature = "luau")]
fn test_fusion_glob_keeps_unrelated_calls() {
    let cwd = construct_tree!({
        "styluau.toml": "syntax = 'Luau'\ncall_parentheses = 'Always'\n[[overrides]]\nfiles = ['ui/**/*.luau']\nfusion = true\nfusion_scope_names = ['panelScope']\n",
        "ui/component.luau": "local x = panelScope:New(\"Frame\")({ child = render({}) })\nlocal y = render(\"ordinary\")\n",
        "other.luau": "local x = panelScope:New(\"Frame\")({})\n",
    });
    create_stylua()
        .current_dir(cwd.path())
        .arg(".")
        .assert()
        .success();
    let formatted = std::fs::read_to_string(cwd.child("ui/component.luau").path()).unwrap();
    assert!(
        formatted.contains("panelScope:New \"Frame\" {"),
        "{}",
        formatted
    );
    assert!(formatted.contains("render({})"), "{}", formatted);
    assert!(formatted.contains("render(\"ordinary\")"), "{}", formatted);
    cwd.child("other.luau")
        .assert("local x = panelScope:New(\"Frame\")({})\n");
}

#[test]
fn test_glob_overrides_invalid_pattern() {
    let cwd = construct_tree!({
        "styluau.toml": "[[overrides]]\nfiles = ['[']\n",
        "test.lua": "print(1)\n",
    });
    let result = create_stylua()
        .current_dir(cwd.path())
        .arg("test.lua")
        .assert()
        .failure();
    assert!(String::from_utf8_lossy(&result.get_output().stderr).contains("Invalid glob"));
}

#[test]
fn test_styluau_identity_and_ignore() {
    create_stylua()
        .arg("--version")
        .assert()
        .success()
        .stdout(format!("styluau {}\n", env!("CARGO_PKG_VERSION")));
    let cwd = construct_tree!({
        ".styluauignore": "ignored/",
        "foo.lua": "local   x    =   1",
        "ignored/bar.lua": "local   x    =   1",
    });
    create_stylua()
        .current_dir(cwd.path())
        .arg(".")
        .assert()
        .success();
    cwd.child("foo.lua").assert("local x = 1\n");
    cwd.child("ignored/bar.lua").assert("local   x    =   1");
}

#[test]
fn test_styluau_ignore_for_explicit_file() {
    let cwd = construct_tree!({
        ".styluauignore": "foo.lua",
        "foo.lua": "local   x    =   1",
    });
    create_stylua()
        .current_dir(cwd.path())
        .args(["--respect-ignores", "foo.lua"])
        .assert()
        .success();
    cwd.child("foo.lua").assert("local   x    =   1");
}

#[test]
fn test_both_ignore_files_apply_to_explicit_file() {
    let cwd = construct_tree!({
        ".styluauignore": "new.lua",
        ".styluaignore": "legacy.lua",
        "new.lua": "local   x   =   1",
        "legacy.lua": "local   x   =   1",
        "other.lua": "local   x   =   1",
    });
    create_stylua()
        .current_dir(cwd.path())
        .args(["--respect-ignores", "new.lua", "legacy.lua", "other.lua"])
        .assert()
        .success();
    cwd.child("new.lua").assert("local   x   =   1");
    cwd.child("legacy.lua").assert("local   x   =   1");
    cwd.child("other.lua").assert("local x = 1\n");
}

#[test]
fn test_stylua_ignore() {
    let cwd = construct_tree!({
        ".styluaignore": "ignored/",
        "foo.lua": "local   x    =   1",
        "ignored/bar.lua": "local   x    =   1",
    });

    let mut cmd = create_stylua();
    cmd.current_dir(cwd.path()).arg(".").assert().success();

    cwd.child("foo.lua").assert("local x = 1\n");
    cwd.child("ignored/bar.lua").assert("local   x    =   1");

    cwd.close().unwrap();
}

#[test]
fn explicitly_provided_files_dont_check_ignores() {
    let cwd = construct_tree!({
        ".styluaignore": "foo.lua",
        "foo.lua": "local   x    =   1",
    });

    let mut cmd = create_stylua();
    cmd.current_dir(cwd.path())
        .arg("foo.lua")
        .assert()
        .success();

    cwd.child("foo.lua").assert("local x = 1\n");

    cwd.close().unwrap();
}

#[test]
fn explicitly_provided_files_dont_check_ignores_stdin() {
    let cwd = construct_tree!({
        ".styluaignore": "foo.lua",
    });

    let mut cmd = create_stylua();
    cmd.current_dir(cwd.path())
        .args(["--stdin-filepath", "foo.lua", "-"])
        .write_stdin("local   x    =   1")
        .assert()
        .success()
        .stdout("local x = 1\n");

    cwd.close().unwrap();
}

#[test]
fn explicitly_provided_files_not_in_cwd() {
    let cwd = construct_tree!({
        ".styluaignore": "foo.lua",
    });

    let another = construct_tree!({});

    let mut cmd = create_stylua();
    cmd.current_dir(cwd.path())
        .args([
            "--respect-ignores",
            "--stdin-filepath",
            another.child("foo.lua").to_str().unwrap(),
            "-",
        ])
        .write_stdin("local   x    =   1")
        .assert()
        .success()
        .stdout("local x = 1\n");

    cwd.close().unwrap();
}

#[test]
fn test_respect_ignores() {
    let cwd = construct_tree!({
        ".styluaignore": "foo.lua",
        "foo.lua": "local   x    =   1",
    });

    let mut cmd = create_stylua();
    cmd.current_dir(cwd.path())
        .args(["--respect-ignores", "foo.lua"])
        .assert()
        .success();

    cwd.child("foo.lua").assert("local   x    =   1");

    cwd.close().unwrap();
}

#[test]
fn test_respect_ignores_stdin() {
    let cwd = construct_tree!({
        ".styluaignore": "foo.lua",
    });

    let mut cmd = create_stylua();
    cmd.current_dir(cwd.path())
        .args(["--respect-ignores", "--stdin-filepath", "foo.lua", "-"])
        .write_stdin("local   x    =   1")
        .assert()
        .success()
        .stdout("local   x    =   1");

    cwd.close().unwrap();
}

#[test]
fn test_respect_ignores_directory_no_glob() {
    // https://github.com/JohnnyMorganz/StyLua/issues/845
    let cwd = construct_tree!({
        ".styluaignore": "build/",
        "build/foo.lua": "local   x    =   1",
    });

    let mut cmd = create_stylua();
    cmd.current_dir(cwd.path())
        .args(["--check", "--respect-ignores", "build/foo.lua"])
        .assert()
        .success();

    cwd.close().unwrap();
}

#[test]
fn test_formatting_respects_gitignore() {
    let cwd = construct_tree!({
        ".git/dummy.txt": "", // Need a .git folder for .gitignore lookup
        ".gitignore": "foo.lua",
        "foo.lua": "local   x    =   1",
    });

    let mut cmd = create_stylua();
    cmd.current_dir(cwd.path()).args(["."]).assert().success();

    cwd.child("foo.lua").assert("local   x    =   1");

    cwd.close().unwrap();
}

#[test]
fn test_formatting_still_formats_gitignore_files_if_requested() {
    let cwd = construct_tree!({
        ".git/dummy.txt": "", // Need a .git folder for .gitignore lookup
        ".gitignore": "foo.lua",
        "foo.lua": "local   x    =   1",
    });

    let mut cmd = create_stylua();
    cmd.current_dir(cwd.path())
        .args(["--no-ignore-vcs", "."])
        .assert()
        .success();

    cwd.child("foo.lua").assert("local x = 1\n");

    cwd.close().unwrap();
}

#[test]
fn test_stdin_filepath_respects_cwd_configuration_next_to_file() {
    let cwd = construct_tree!({
        "stylua.toml": "quote_style = 'AutoPreferSingle'",
    });

    let mut cmd = create_stylua();
    cmd.current_dir(cwd.path())
        .args(["--stdin-filepath", "foo.lua", "-"])
        .write_stdin("local x = \"hello\"")
        .assert()
        .success()
        .stdout("local x = 'hello'\n");

    cwd.close().unwrap();
}

#[test]
fn test_stdin_filepath_respects_cwd_configuration_for_nested_file() {
    let cwd = construct_tree!({
        "stylua.toml": "quote_style = 'AutoPreferSingle'",
    });

    let mut cmd = create_stylua();
    cmd.current_dir(cwd.path())
        .args(["--stdin-filepath", "build/foo.lua", "-"])
        .write_stdin("local x = \"hello\"")
        .assert()
        .success()
        .stdout("local x = 'hello'\n");

    cwd.close().unwrap();
}

#[test]
fn test_cwd_configuration_respected_when_formatting_from_stdin() {
    let cwd = construct_tree!({
        "stylua.toml": "quote_style = 'AutoPreferSingle'",
        "foo.lua": "local x = \"hello\"",
    });

    let mut cmd = create_stylua();
    cmd.current_dir(cwd.path())
        .arg("-")
        .write_stdin("local x = \"hello\"")
        .assert()
        .success()
        .stdout("local x = 'hello'\n");

    cwd.close().unwrap();
}

#[test]
fn test_cwd_configuration_respected_for_file_in_cwd() {
    let cwd = construct_tree!({
        "stylua.toml": "quote_style = 'AutoPreferSingle'",
        "foo.lua": "local x = \"hello\"",
    });

    let mut cmd = create_stylua();
    cmd.current_dir(cwd.path())
        .arg("foo.lua")
        .assert()
        .success();

    cwd.child("foo.lua").assert("local x = 'hello'\n");

    cwd.close().unwrap();
}

#[test]
fn test_cwd_configuration_respected_for_nested_file() {
    let cwd = construct_tree!({
        "stylua.toml": "quote_style = 'AutoPreferSingle'",
        "build/foo.lua": "local x = \"hello\"",
    });

    let mut cmd = create_stylua();
    cmd.current_dir(cwd.path())
        .arg("build/foo.lua")
        .assert()
        .success();

    cwd.child("build/foo.lua").assert("local x = 'hello'\n");

    cwd.close().unwrap();
}

#[test]
fn test_configuration_is_not_used_outside_of_cwd() {
    let cwd = construct_tree!({
        "stylua.toml": "quote_style = 'AutoPreferSingle'",
        "build/foo.lua": "local x = \"hello\"",
    });

    let mut cmd = create_stylua();
    cmd.current_dir(cwd.child("build").path())
        .arg("foo.lua")
        .assert()
        .success();

    cwd.child("build/foo.lua").assert("local x = \"hello\"\n");

    cwd.close().unwrap();
}

#[test]
fn test_configuration_used_outside_of_cwd_when_search_parent_directories_is_enabled() {
    let cwd = construct_tree!({
        "stylua.toml": "quote_style = 'AutoPreferSingle'",
        "build/foo.lua": "local x = \"hello\"",
    });

    let mut cmd = create_stylua();
    cmd.current_dir(cwd.child("build").path())
        .args(["--search-parent-directories", "foo.lua"])
        .assert()
        .success();

    cwd.child("build/foo.lua").assert("local x = 'hello'\n");

    cwd.close().unwrap();
}

#[test]
fn test_configuration_is_searched_next_to_file() {
    let cwd = construct_tree!({
        "build/stylua.toml": "quote_style = 'AutoPreferSingle'",
        "build/foo.lua": "local x = \"hello\"",
    });

    let mut cmd = create_stylua();
    cmd.current_dir(cwd.path())
        .arg("build/foo.lua")
        .assert()
        .success();

    cwd.child("build/foo.lua").assert("local x = 'hello'\n");

    cwd.close().unwrap();
}

#[test]
fn test_configuration_is_used_closest_to_the_file() {
    let cwd = construct_tree!({
        "stylua.toml": "quote_style = 'AutoPreferDouble'",
        "build/stylua.toml": "quote_style = 'AutoPreferSingle'",
        "build/foo.lua": "local x = \"hello\"",
    });

    let mut cmd = create_stylua();
    cmd.current_dir(cwd.path())
        .arg("build/foo.lua")
        .assert()
        .success();

    cwd.child("build/foo.lua").assert("local x = 'hello'\n");

    cwd.close().unwrap();
}

#[test]
fn test_respect_config_path_override() {
    let cwd = construct_tree!({
        "stylua.toml": "quote_style = 'AutoPreferDouble'",
        "build/stylua.toml": "quote_style = 'AutoPreferSingle'",
        "foo.lua": "local x = \"hello\"",
    });

    let mut cmd = create_stylua();
    cmd.current_dir(cwd.path())
        .args(["--config-path", "build/stylua.toml", "foo.lua"])
        .assert()
        .success();
}

#[test]
fn test_respect_config_path_override_for_stdin_filepath() {
    let cwd = construct_tree!({
        "stylua.toml": "quote_style = 'AutoPreferDouble'",
        "build/stylua.toml": "quote_style = 'AutoPreferSingle'",
        "foo.lua": "local x = \"hello\"",
    });

    let mut cmd = create_stylua();
    cmd.current_dir(cwd.path())
        .args(["--config-path", "build/stylua.toml", "-"])
        .write_stdin("local x = \"hello\"")
        .assert()
        .success()
        .stdout("local x = 'hello'\n");

    cwd.close().unwrap();
}

#[test]
fn test_uses_cli_overrides_instead_of_default_configuration() {
    let cwd = construct_tree!({
        "foo.lua": "local x = \"hello\"",
    });

    let mut cmd = create_stylua();
    cmd.current_dir(cwd.path())
        .args(["--quote-style", "AutoPreferSingle", "."])
        .assert()
        .success();

    cwd.child("foo.lua").assert("local x = 'hello'\n");

    cwd.close().unwrap();
}

#[test]
fn test_uses_cli_overrides_instead_of_default_configuration_stdin_filepath() {
    let cwd = construct_tree!({
        "foo.lua": "local x = \"hello\"",
    });

    let mut cmd = create_stylua();
    cmd.current_dir(cwd.path())
        .args(["--quote-style", "AutoPreferSingle", "-"])
        .write_stdin("local x = \"hello\"")
        .assert()
        .success()
        .stdout("local x = 'hello'\n");

    cwd.close().unwrap();
}

#[test]
fn test_uses_cli_overrides_instead_of_found_configuration() {
    let cwd = construct_tree!({
        "stylua.toml": "quote_style = 'AutoPreferDouble'",
        "foo.lua": "local x = \"hello\"",
    });

    let mut cmd = create_stylua();
    cmd.current_dir(cwd.path())
        .args(["--quote-style", "AutoPreferSingle", "."])
        .assert()
        .success();

    cwd.child("foo.lua").assert("local x = 'hello'\n");

    cwd.close().unwrap();
}

#[test]
fn test_uses_cli_overrides_instead_of_found_configuration_stdin_filepath() {
    let cwd = construct_tree!({
        "stylua.toml": "quote_style = 'AutoPreferDouble'",
        "foo.lua": "local x = \"hello\"",
    });

    let mut cmd = create_stylua();
    cmd.current_dir(cwd.path())
        .args(["--quote-style", "AutoPreferSingle", "-"])
        .write_stdin("local x = \"hello\"")
        .assert()
        .success()
        .stdout("local x = 'hello'\n");

    cwd.close().unwrap();
}

#[test]
#[cfg(feature = "editorconfig")]
fn test_editorconfig_child_without_root_merges_with_parent() {
    let cwd = construct_tree!({
        ".editorconfig": "root = true\n\n[*.lua]\nindent_style = space\nindent_size = 2\n",
        "child/.editorconfig": "[*.lua]\nquote_type = single\n",
        "child/foo.lua": "local foo = {\n\ta = 1,\n}\n\nlocal bar = \"\"\n",
    });

    let mut cmd = create_stylua();
    cmd.current_dir(cwd.path())
        .arg("child/foo.lua")
        .assert()
        .success();

    // indent from parent, quotes from child
    cwd.child("child/foo.lua")
        .assert("local foo = {\n  a = 1\n}\n\nlocal bar = ''\n");

    cwd.close().unwrap();
}

#[test]
#[cfg(feature = "editorconfig")]
fn test_editorconfig_root_true_stops_upward_search() {
    let cwd = construct_tree!({
        ".editorconfig": "root = true\n\n[*.lua]\nindent_style = space\nindent_size = 2\nquote_type = single\n",
        "child/.editorconfig": "root = true\n\n[*.lua]\nindent_style = space\nindent_size = 4\n",
        "child/foo.lua": "local foo = {\n\ta = 1,\n}\n\nlocal bar = \"\"\n",
    });

    let mut cmd = create_stylua();
    cmd.current_dir(cwd.path())
        .arg("child/foo.lua")
        .assert()
        .success();

    // indent_size=4 from child; parent's quote_type=single must NOT be inherited
    cwd.child("child/foo.lua")
        .assert("local foo = {\n    a = 1\n}\n\nlocal bar = \"\"\n");

    cwd.close().unwrap();
}

#[test]
#[cfg(feature = "editorconfig")]
fn test_editorconfig_closer_config_takes_precedence() {
    let cwd = construct_tree!({
        ".editorconfig": "root = true\n\n[*.lua]\nindent_style = space\nindent_size = 2\nquote_type = single\n",
        "child/.editorconfig": "[*.lua]\nindent_size = 4\n",
        "child/foo.lua": "local foo = {\n\ta = 1,\n}\n\nlocal bar = \"\"\n",
    });

    let mut cmd = create_stylua();
    cmd.current_dir(cwd.path())
        .arg("child/foo.lua")
        .assert()
        .success();

    // indent_size=4 from child overrides parent's 2; quote_type=single inherited from parent
    cwd.child("child/foo.lua")
        .assert("local foo = {\n    a = 1\n}\n\nlocal bar = ''\n");

    cwd.close().unwrap();
}

#[test]
#[cfg(feature = "editorconfig")]
fn test_editorconfig_merges_across_three_directory_levels() {
    let cwd = construct_tree!({
        ".editorconfig": "root = true\n\n[*.lua]\nindent_style = space\nindent_size = 2\n",
        "middle/.editorconfig": "[*.lua]\nquote_type = single\n",
        "middle/inner/.editorconfig": "[*.lua]\nmax_line_length = 80\n",
        "middle/inner/foo.lua": "local foo = {\n\ta = 1,\n}\n\nlocal bar = \"\"\n",
    });

    let mut cmd = create_stylua();
    cmd.current_dir(cwd.path())
        .arg("middle/inner/foo.lua")
        .assert()
        .success();

    // indent from root, quotes from middle, max_line_length from inner
    cwd.child("middle/inner/foo.lua")
        .assert("local foo = {\n  a = 1\n}\n\nlocal bar = ''\n");

    cwd.close().unwrap();
}
