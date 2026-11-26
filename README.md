# styluau

StyLuau is a deterministic code formatter for Luau, based on [StyLua](https://github.com/JohnnyMorganz/StyLua). It supports the syntax variants provided by its upstream formatter.

## Installation

Build the executable from the repository with the Rust toolchain...

```sh
cargo build --release --features luau
```

The resulting binary is `target/release/styluau`.

## Usage

Once installed, pass the files to format to the CLI:

```sh
styluau src/ foo.lua bar.lua
```

This command will format the `foo.lua` and `bar.lua` file, and search down the `src` directory to format any files within it.
StyLuau can also read from stdin, by using `-` as the file name.

### Glob Filtering

By default, when searching through a directory, StyLuau looks for all files matching the glob `**/*.lua` (or `**/*.luau` when `luau` is enabled) to format.
You can also specify an explicit glob pattern to match against when searching:

```sh
styluau --glob '**/*.luau' -- src # format all files in src matching **/*.luau
styluau -g '*.lua' -g '!*.spec.lua' -- . # format all Lua files except test files ending with `.spec.lua`
```

Note that the `-g/--glob` argument can take multiple strings at once, so `--` is required to separate between the glob patterns and the files to format.

By default, glob filtering (and `.styluauignore` files) are only applied during directory traversal and searching.
Files passed directly (e.g. `styluau foo.txt`) will override the glob / ignore and always be formatted.
To disable this behaviour, pass the `--respect-ignores` flag (`styluau --respect-ignores foo.txt`).

### Filtering using `.styluauignore`

You can create a `.styluauignore` file, with a format similar to `.gitignore`. Legacy
`.styluaignore` files remain supported.
Any files matching the globs in the ignore file are ignored by StyLuau.
For example, for a `.styluauignore` file with the following contents:

```
vendor/
```

running `styluau .` will ignore the `vendor/` directory.

### Filtering when using stdin

If you are formatting stdin by specifying `-` as the filename (usually as part of an editor integration)
you can optionally provide the filename via `--stdin-filepath`. To respect glob or `.styluauignore` filtering, pass `--respect-ignores`.

```sh
styluau --respect-ignores --stdin-filepath src/foo.lua -
```

### `--check`: Checking files for formatting

To check whether files require formatting (but not write directly to them), use the `--check` flag.
It will take files as input, and output a diff to stdout instead of rewriting the file contents.
If there are any files that require formatting, StyLuau will exit with status code 1.

There are different styles of output available:

- `--output-format=standard`: output a custom diff (default)
- `--output-format=unified`: output a unified diff, consumable by tools like `patch` or `delta`
- `--output-format=json`: output JSON representing the changes, useful for machine-readable output
- `--output-format=summary`: output a summary list of file paths that are incorrectly formatted

### `--verify`: Verifying formatting output

As a safety measure, you can use the `--verify` flag to verify the output of all formatting before saving the file.

If enabled, the tool will re-parse the formatted output to verify if the AST is still valid (no syntax errors) and is similar to the input (possible semantic changes).

This is useful when adopting StyLuau in a large codebase, where it is difficult to manually check all formatting is correct.
Note that this may produce false positives and negatives - we recommend manual verification as well as running tests to confirm.

### Ignoring parts of a file

To skip formatting a particular part of a file, add `-- [styluau] ignore` before it.
This is useful for preserving a particular layout for readability...

```luau
-- [styluau] ignore
local matrix = {
    { 0, 0, 0 },
    { 0, 0, 0 },
    { 0, 0, 0 },
}
```

To skip a block of code, use `-- [styluau] ignore start` and
`-- [styluau] ignore end`...

```luau
local foo = true
-- [styluau] ignore start
local   bar   =   false
local  baz      = 0
-- [styluau] ignore end
local foobar = false
```

Ignoring cannot cross scope boundaries. Once a block is exited, formatting is re-enabled.

All current StyLuau controls use the `[styluau]` prefix, including Fusion controls. The
previous `styluau:` spelling is not recognized. Existing upstream `stylua: ignore`,
`stylua: ignore start` and `stylua: ignore end` aliases remain supported. These commands
also work on trimmed lines inside multiline comments.

### Formatting Ranges

To format a specific range within a file, use `--range-start <num>` and/or `--range-end <num>`.
Both arguments are inclusive and optional - if an argument is not provided, the start/end of the file is used respectively.

Only whole statements lying within the range are formatted.
If part of a statement falls outside the range, the statement is ignored.

In editors, `Format Selection` is supported.

### Requires Sorting

StyLuau has built-in support for sorting require statements. We group consecutive require statements into a single "block",
and then requires are sorted only within that block. Blocks of requires do not move around the file.

StyLuau only considers requires of the form `local NAME = require(EXPR)`, or `const NAME = require(EXPR)` in Luau,
and sorts lexicographically based on `NAME`.
(StyLuau can also sort Roblox services of the form `local NAME = game:GetService(EXPR)`, or `const NAME = game:GetService(EXPR)` in Luau)

Requires sorting is off by default. To enable it, add the following to `styluau.toml`:

```toml
[sort_requires]
enabled = true
```

### Language Server Mode

StyLuau can run as a language server, connecting with language clients that follow the [Language Server Protocol](https://microsoft.github.io/language-server-protocol/).
It will then respond to `textDocument/formatting` and `textDocument/rangeFormatting` requests.
Formatting is only performed on files with a `lua` or `luau` language ID.

If the initialization option `respect_editor_formatting_options` is set to `true`, the formatting handler will override the configurations `indent-width` and `indent-type` with values from [FormattingOptions](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#formattingOptions).

You can start the language server by running:

```sh
styluau --lsp
```

StyLuau will listen to LSP messages on stdin and respond on stdout.

## Configuration

StyLuau has opinionated defaults, but also provides a few options that can be set per project.

### Finding the configuration

The CLI looks for `styluau.toml` or `.styluau.toml` before the upstream names
`stylua.toml` and `.stylua.toml`, starting from the directory of the file being formatted.
It will keep searching upwards until it reaches the current directory where the tool was executed.
If not found, we search for an `.editorconfig` file, otherwise fall back to the default configuration.
This feature can be disabled using `--no-editorconfig`.
See [EditorConfig](https://editorconfig.org/) for more details.

Use `--config-path <path>` to provide a custom path to the configuration.
If the file provided is not found/malformed, StyLuau will exit with an error.

By default, StyLuau does not search further than the current directory.
Use `--search-parent-directories` to recursively search parent directories.
This will keep searching ancestors and, if not found, will then look in `$XDG_CONFIG_HOME` / `$XDG_CONFIG_HOME/stylua` / `$HOME/.config` and `$HOME/.config/stylua`.

**Note: enabling searching outside of the current directory is NOT recommended due to possibilities of conflicting formatting:**

It is recommended to keep a `styluau.toml` file in your project root so that other developers can make use of the same configuration.

If a project uses the default configuration of StyLuau without a configuration file present, enabling external searching may cause conflicting formatting.

### File Glob Overrides

Use ordered `[[overrides]]` entries to apply settings to files relative to the directory
containing the selected configuration file...

```toml
indent_type = "Spaces"
indent_width = 4

[[overrides]]
files = ["src/ui/**/*.luau"]
fusion = true
fusion_scope_names = ["scope", "fusion", "uiScope"]
```

Each matching rule inherits the base settings. Later matching rules override earlier
ones, and explicit command-line options take precedence. Stdin without
`--stdin-filepath` uses the base settings because it has no path to match.

The `fusion` setting applies Fusion-style parentheses to supported constructor calls.
Configured `fusion_scope_names` replace the default names `scope` and `fusion`. Matching
includes these assumed receiver names, so an unrelated receiver with the same name can
also match. Static string imports whose final path component is `Fusion`, local module aliases,
`scoped` aliases, and derived `innerScope` or `deriveScope` scopes are also recognized.
Import tracking is deliberately conservative: reassigned or shadowed names are not
inferred, and member assignments disable import inference for the file. Dynamic imports
and scope parameters that cannot be traced use the configured names or scope comments.

Place a standalone comment before a statement or table field to enable or disable Fusion
formatting for that node. Region comments may be nested and must end in the same lexical
block...

```luau
-- [styluau] fusion on
local frame = scope:New("Frame")({})

-- [styluau] fusion on start
local button = scope:New("TextButton")({})
-- [styluau] fusion end
```

Use `-- [styluau] fusion off` or `-- [styluau] fusion off start` to disable formatting for
the next statement or a region. Within a block, `-- [styluau] fusion scope uiScope` marks
a receiver as a Fusion scope, while `-- [styluau] fusion not-scope scope` excludes it.
Invalid and unmatched directives cause a formatting error.

### Configuring Runtime Syntax Selection

By default, StyLuau releases comes with all flavours of Lua bundled into one binary, with a union of all syntax styles.
We do this to make it easier to get started with StyLuau on any codebase or project using Lua.

However, there are times where the union of syntaxes collide, causing issues. For example, Lua 5.2's goto label syntax
(`::label::`) conflicts with Luau's type assertion syntax (`x :: number`), and the latter ends up taking priority.

To disambiguate a particular syntax style for your codebase, set `syntax = "Style"` in your `styluau.toml` file, e.g.:

```toml
syntax = "Lua52"
```

Alternatively, you can specify it on the command line, with `styluau --syntax lua52 ...`

### Options

Multiline table values and Luau table types omit the final comma by default. Set
`table_trailing_comma = true` in `styluau.toml` to include it instead.

Set `space_inside_interpolation = true` in `styluau.toml` to add spaces inside Luau
backtick-string interpolation braces. The default is `false`, and the setting also works
in per-file `[[overrides]]`. Literal string contents and escaped braces are unchanged.

```luau
-- With space_inside_interpolation = false (the default).
const greeting = `Hello {name}, you have {count + 1} items.`

-- With space_inside_interpolation = true.
const greeting = `Hello { name }, you have { count + 1 } items.`
```

A space before an interpolated table constructor is always retained because Luau rejects
adjacent opening braces. This syntax requirement also applies when the option is `false`.

**Comment Reflow**

Enable Markdown-aware paragraph wrapping independently of code line length...

```toml
comment_reflow = true
comment_width = 90
```

The boolean defaults to `false`, which retains existing comment formatting. The width
must be positive and defaults to 90, including indentation and comment prefixes. Both
settings also work in ordered per-file `[[overrides]]`.

Standalone `--` groups and multiline comments join soft line breaks within paragraphs.
Blank lines separate paragraphs. Markdown list items beginning with markers such as `1.`,
`1)`, `-`, `+` or `*` remain separate, with hanging indentation when their prose wraps.
Headings, code fences, indented code, tables, hard line breaks and blockquote structure are
preserved. Inline code and links are not split. Indivisible words may exceed the width,
and preserving Markdown structure takes precedence over the target width. Paragraphs
with multiline code/link spans retain their deliberate layout. String contents, comments
trailing code, annotations and formatter directives are not reflowed. Wrapped block
comments put their delimiters on separate lines so subsequent formatting uses the same
width budget. Bracket depth and file-header closing markers are preserved.

End a comment line with a backslash to keep the next line from joining it. Whitespace
after the backslash is removed, and the lines on either side of the break still wrap at
the comment width. Two trailing spaces work the same way, as in Markdown...

```luau
-- This line stays \
-- separate from this one.
```

Use comment-only controls to preserve intentional layout without disabling code
formatting...

```luau
-- [styluau] comments off
-- Keep this comment group
-- on deliberately short lines.
const value = 1

-- [styluau] comments off start
-- A larger region can contain manually aligned comments.
const other = 2
-- [styluau] comments end
```

A one-shot `comments on` or `comments off` targets the next standalone comment group, not
its code statement. A multiline comment is one group; consecutive standalone `--` lines
form a group. Blank lines, code and directives separate groups.
Place a control before the target comment, not inside its prose. Nested
`comments on start` and `comments off start` regions restore the previous setting at
`comments end`.
Regions follow source order within the file. Malformed or unbalanced controls report an
error with the line number. Existing formatting-ignore controls take precedence.

StyLuau only offers the following options:

> [!NOTE]  
> When specifying the option on the command line, replace the underscore with a dash, e.g., `indent_type` becomes `--indent-type`.

| Option                       | Default            | Description                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| ---------------------------- | ------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `syntax`                     | `All`              | Specify a disambiguation for the style of Lua syntax being formatted. Possible options: `All` (default), `Lua51`, `Lua52`, `Lua53`, `Lua54`, `LuaJIT`, `Luau`, `CfxLua`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| `column_width`               | `120`              | Approximate line length for printing. Used as a guide for line wrapping - this is not a hard requirement: lines may fall under or over the limit.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `comment_reflow`             | `false`            | Reflow standalone comment paragraphs while preserving Markdown structure.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| `comment_width`              | `90`               | Positive comment line width, including indentation and comment prefixes. Independent of `column_width`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| `line_endings`               | `Unix`             | Line endings type. Possible options: `Unix` (LF) or `Windows` (CRLF)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| `indent_type`                | `Tabs`             | Indent type. Possible options: `Tabs` or `Spaces`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `indent_width`               | `4`                | Character size of single indentation. If `indent_type` is set to `Tabs`, this option is used as a heuristic to determine column width only.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| `quote_style`                | `AutoPreferDouble` | Quote style for string literals. Possible options: `AutoPreferDouble`, `AutoPreferSingle`, `ForceDouble`, `ForceSingle`. `AutoPrefer` styles will prefer the specified quote style, but fall back to the alternative if it has fewer string escapes. `Force` styles always use the specified style regardless of escapes.                                                                                                                                                                                                                                                                                                                                                                                                                             |
| `call_parentheses`           | `Always`           | Whether parentheses should be applied on function calls with a single string/table argument. Possible options: `Always`, `NoSingleString`, `NoSingleTable`, `None`, `Input`. `Always` applies parentheses in all cases. `NoSingleString` omits parentheses on calls with a single string argument. Similarly, `NoSingleTable` omits parentheses on calls with a single table argument. `None` omits parentheses in both cases. Note: parentheses are still kept in situations where removal can lead to obscurity (e.g. `foo "bar".setup -> foo("bar").setup`, since the index is on the call result, not the string). `Input` removes all automation and preserves parentheses only if they were present in input code: consistency is not enforced. |
| `space_after_function_names` | `Never`            | Specify whether to add a space between the function name and parentheses. Possible options: `Never`, `Definitions`, `Calls`, or `Always`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| `space_inside_interpolation` | `false`            | Add one space inside Luau backtick-string interpolation braces, changing `{value}` to `{ value }`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| `block_newline_gaps`         | `Never`            | Specify whether to preserve leading and trailing newline gaps for blocks. Possible options: `Never`, `Preserve`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| `collapse_simple_statement`  | `Never`            | Specify whether to collapse simple statements. Possible options: `Never`, `Input`, `FunctionOnly`, `ConditionalOnly`, or `Always`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |

Default `styluau.toml`, note you do not need to explicitly specify each option if you want to use the defaults:

```toml
syntax = "All"
column_width = 120
comment_reflow = false
comment_width = 90
line_endings = "Unix"
indent_type = "Tabs"
indent_width = 4
quote_style = "AutoPreferDouble"
call_parentheses = "Always"
collapse_simple_statement = "Never"
space_after_function_names = "Never"
space_inside_interpolation = false
block_newline_gaps = "Never"

[sort_requires]
enabled = false
```
