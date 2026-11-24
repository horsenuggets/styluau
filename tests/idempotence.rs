#![cfg(feature = "luau")]

use styluau_lib::{format_code, Config, IndentType, LuaVersion, OutputVerification};

fn config() -> Config {
    Config {
        syntax: LuaVersion::Luau,
        indent_type: IndentType::Spaces,
        ..Config::default()
    }
}

fn format(input: &str) -> String {
    format_code(input, config(), None, OutputVerification::Full)
        .unwrap_or_else(|error| panic!("Could not format {:?}. {}", input, error))
}

// Compact input hides how wide code becomes once spaces are added around operators and
// separators, so layout decisions must not depend on the input's own spacing.
const COMPACT_INPUTS: [&str; 6] = [
    // A table argument that only overflows once its fields are spaced.
    "local gradient = group(\"Image gradient\",\"Gradient\",{Color=ColorSequence.new(Color3.new(1,1,1)),Transparency=0,Rotation=0,Offset=Vector2.zero,Enabled=true})\n",
    // A final table argument that cannot fit on any line and should hug the parentheses.
    "for index = 1, 2 do\n    local swatch=W.Frame(scope,row.Button,{Name=\"GradientSwatch\",Position=UDim2.fromOffset(4,4),Size=UDim2.new(1,-8,1,-8),BackgroundTransparency=0,BackgroundColor3=Color3.new(1,1,1)})\nend\n",
    // A standalone table assignment that overflows only once spaced.
    "local defaults={Color=ColorSequence.new(Color3.new(1,1,1)),Transparency=0,Rotation=0,Offset=Vector2.zero,Enabled=true,Extra=1}\n",
    // A table followed by another argument, which must expand every argument.
    "call({Color=ColorSequence.new(Color3.new(1,1,1)),Transparency=0,Rotation=0,Offset=Vector2.zero,Enabled=true,Extra=1},value)\n",
    // Short compact code that fits on one line once spaced.
    "local small=W.Frame(scope,parent,{Name=\"Small\",Visible=false})\n",
    // A table that fits on its own argument line but not after the preceding arguments.
    "local panel = W.Frame(scope,row.Button,{Name=\"GradientSwatch\",Position=UDim2.fromOffset(4,4),Size=UDim2.new(1,-8,1,-8),Visible=true})\n",
];

#[test]
fn formatting_compact_input_is_idempotent() {
    for input in COMPACT_INPUTS {
        let once = format(input);
        let twice = format(&once);
        assert_eq!(once, twice, "Formatting was not stable for {input:?}");
    }
}

#[test]
fn compact_table_layouts_respect_formatting_options() {
    for trailing_comma in [false, true] {
        for fusion in [false, true] {
            for indent_type in [IndentType::Spaces, IndentType::Tabs] {
                for input in COMPACT_INPUTS {
                    let config = Config {
                        fusion,
                        indent_type,
                        table_trailing_comma: trailing_comma,
                        ..config()
                    };
                    let once =
                        format_code(input, config.clone(), None, OutputVerification::Full).unwrap();
                    let twice = format_code(&once, config, None, OutputVerification::Full).unwrap();
                    assert_eq!(once, twice, "Formatting was not stable for {input:?}");
                }
            }
        }
    }
}

#[test]
fn formatting_ignores_input_spacing() {
    for input in COMPACT_INPUTS {
        let formatted = format(input);
        // Reformatting from the already-spaced output, and from a version with doubled
        // spaces, must give the same layout as formatting the compact input.
        let spaced = formatted.replace(", ", ",  ").replace(" = ", "  =  ");
        assert_eq!(
            format(&spaced),
            formatted,
            "Spacing changed the layout of {input:?}"
        );
    }
}

#[test]
fn overflowing_compact_tables_expand_on_the_first_pass() {
    assert_eq!(
        format(COMPACT_INPUTS[0]),
        "local gradient = group(\"Image gradient\", \"Gradient\", {\n    Color = ColorSequence.new(Color3.new(1, 1, 1)),\n    Transparency = 0,\n    Rotation = 0,\n    Offset = Vector2.zero,\n    Enabled = true\n})\n"
    );
    assert_eq!(
        format(COMPACT_INPUTS[1]),
        "for index = 1, 2 do\n    local swatch = W.Frame(scope, row.Button, {\n        Name = \"GradientSwatch\",\n        Position = UDim2.fromOffset(4, 4),\n        Size = UDim2.new(1, -8, 1, -8),\n        BackgroundTransparency = 0,\n        BackgroundColor3 = Color3.new(1, 1, 1)\n    })\nend\n"
    );
}

#[test]
fn nested_tables_and_types_expand_consistently() {
    for input in [
        "local options={name=\"Bounce\",handles=false,kps={{0,0},{0.2,0.8},{0.35,0},{0.55,0.6},{0.7,0},{0.85,0.3},{1,0}}}\n",
        "local values={\n-- Quad\n{name=\"InQuad\",handles=true,kps={{0,0,hOutX=0.55,hOutY=0},{1,1,hInX=-0.2,hInY=-0.2}}},\n}\n",
        "type Flipbook={Columns:number,FPS:number?,FrameSize:Vector2,Frames:number,Loop:boolean?,RandomStart:boolean?}\n",
        "local function f()\n    for _, entry in entries do\n        local source={Path=instance:GetFullName(),Kind=instance.ClassName,Folders=if #folders>0 then folders else nil} :: Source\n    end\nend\n",
        "local function f()\n    for _, entry in entries do\n        W.Button(scope,parent,\"\",function() callback() end,{Name=entry[1],Position=UDim2.new(1,-(entry[3]::number),0,7),Size=UDim2.fromOffset(28,28)})\n    end\nend\n",
    ] {
        for trailing_comma in [false, true] {
            let config = Config { table_trailing_comma: trailing_comma, ..config() };
            let once = format_code(input, config.clone(), None, OutputVerification::Full).unwrap();
            let twice = format_code(&once, config, None, OutputVerification::Full).unwrap();
            assert_eq!(once, twice, "Formatting was not stable for {input:?}");
        }
    }
}

#[test]
fn typeof_documentation_stays_on_separate_lines() {
    for input in [
        "export type Connection = {\nDisconnect: typeof(\n--[[\nDisconnects the connection.\n]]\nfunction(self: Connection)end\n),\nConnected: boolean,\n}\n",
        "type Callback = typeof(\n-- Documents the callback.\nfunction(value: number): number return value end\n)\n",
        "type Callback = typeof(\n--[[Documents the callback.]]\nfunction() end\n)\n",
    ] {
        let once = format(input);
        assert_eq!(once, format(&once), "Formatting was not stable for {input:?}");
        assert!(!once.contains("]]function"));
        assert!(once.contains("typeof(\n"));
    }
}

#[test]
fn typeof_preserves_trailing_comments() {
    for input in [
        "type T = typeof(foo -- Keep this comment.\n)\n",
        "type T = typeof(foo --[[Keep this comment.]]\n)\n",
        "type T = typeof((foo -- Keep this comment.\n))\n",
        "type T = typeof(((foo -- Keep this comment.\n)))\n",
        "type T = typeof((\n-- Keep this comment.\nfoo))\n",
        "type T = typeof(\n-- Leading comment.\nfoo -- Keep this comment.\n)\n",
    ] {
        let once = format(input);
        assert!(once.contains("Keep this comment."));
        assert_eq!(once, format(&once));
    }
}

#[test]
fn nested_multiline_tables_finish_without_exponential_work() {
    for (opening, depth) in [("{", 20), ("{1+", 12)] {
        let input = format!(
            "local t={}[[\nx\n]]{}\n",
            opening.repeat(depth),
            "}".repeat(depth)
        );
        assert_cmd::Command::cargo_bin("styluau")
            .unwrap()
            .timeout(std::time::Duration::from_secs(10))
            .args(["--syntax", "Luau", "--verify", "-"])
            .write_stdin(input)
            .assert()
            .success();
    }
}

#[test]
fn chained_calls_use_their_actual_argument_layout() {
    let input = "do\n    Service:Create(a,b,{Foo=1111111111,Bar=2222222222}):Play()\nend\n";
    let config = Config {
        column_width: 46,
        ..config()
    };
    let once = format_code(input, config.clone(), None, OutputVerification::Full).unwrap();
    assert!(once.contains("Service:Create("));
    assert_eq!(
        once,
        format_code(&once, config, None, OutputVerification::Full).unwrap()
    );
}

#[test]
fn expression_whitespace_does_not_change_layout() {
    for (width, input) in [
        (120, "local x = a\n\n +\n\n b\n"),
        (120, "if a and b\n\n and c\n\n and d then end\n"),
        (
            40,
            "x = aaaaaaaaaa + (bbbbbbbbbb +                         cccccccccc) * d\n",
        ),
        (34, "local x = a and (b..c..d..e..f..g) or h\n"),
    ] {
        let config = Config {
            column_width: width,
            ..config()
        };
        let once = format_code(input, config.clone(), None, OutputVerification::Full).unwrap();
        let twice = format_code(&once, config, None, OutputVerification::Full).unwrap();
        assert_eq!(once, twice, "Formatting was not stable for {input:?}");
    }
}

#[test]
fn assignment_and_call_boundaries_are_stable() {
    for input in [
        include_str!("inputs/single-arg-function-calls.lua"),
        include_str!("inputs/hang-call-chain-small-prefix.lua"),
        include_str!("inputs/long-assignment-6.lua"),
        include_str!("inputs/table-2.lua"),
        include_str!("inputs/table-4.lua"),
        "do\n    do\n        local cancel=W.Button(inner,card,\"Cancel\",close,{Name=\"CancelRemoveTexture\",Position=UDim2.new(1,-260,1,-36),Size=UDim2.fromOffset(112,36)})\n    end\nend\n",
        "do\n    do\n        local frame = base * CFrame.fromEulerAnglesYXZ(math.rad(math.random(-5,5)),math.rad(math.random(-125,55)),0\n\n)\n    end\nend\n",
    ] {
        let config = Config { table_trailing_comma: true, ..config() };
        let once = format_code(input, config.clone(), None, OutputVerification::Full).unwrap();
        let twice = format_code(&once, config, None, OutputVerification::Full).unwrap();
        assert_eq!(once, twice, "Formatting was not stable for {input:?}");
    }
}

#[test]
fn chained_table_arguments_use_the_prefix_width() {
    for input in [
        "do\n    do\n        TweenService:Create(target, TweenInfo.new(0.5, Enum.EasingStyle.Linear, Enum.EasingDirection.Out), {CFrame=destination}):Play()\n    end\nend\n",
        "do\n    TweenService:Create(Beam.part.SpotLight, TweenInfo.new(0.3, Enum.EasingStyle.Linear, Enum.EasingDirection.Out), {Range=18}):Play()\nend\n",
    ] {
        let once = format(input);
        assert_eq!(once, format(&once), "Formatting was not stable for {input:?}");
    }
}

#[test]
fn table_assignments_do_not_hang_for_descendant_line_counts() {
    for prefix in ["", "button:set(scope:ButtonBase(Utils.merge(props, "] {
        let suffix = if prefix.is_empty() { "" } else { ")))" };
        let input = format!(
            "do\n    local component = {prefix}{{Child = scope:TKeyOf(parameters, {{\"{}\"}}, Colors.Gradients.White)}}{suffix}\nend\n",
            "x".repeat(70),
        );
        let once = format(&input);
        assert_eq!(
            once,
            format(&once),
            "Formatting was not stable for {input:?}"
        );
    }
}

#[test]
fn fitting_calls_stay_compact_and_long_tables_expand() {
    assert_eq!(
        format(COMPACT_INPUTS[4]),
        "local small = W.Frame(scope, parent, { Name = \"Small\", Visible = false })\n"
    );
    assert_eq!(
        format(COMPACT_INPUTS[5]),
        "local panel = W.Frame(scope, row.Button, {\n    Name = \"GradientSwatch\",\n    Position = UDim2.fromOffset(4, 4),\n    Size = UDim2.new(1, -8, 1, -8),\n    Visible = true\n})\n"
    );
}
