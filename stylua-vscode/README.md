# StyLuau VSCode Extension

StyLuau is a fork of [StyLua](https://github.com/JohnnyMorganz/StyLua), a deterministic code formatter for Lua 5.1, 5.2, 5.3, 5.4, LuaJIT and [Luau](https://luau.org/), built using [full-moon](https://github.com/Kampfkarren/full-moon).
StyLuau is inspired by the likes of [prettier](https://github.com/prettier/prettier), it parses your Lua codebase, and prints it back out from scratch,
enforcing a consistent code style.

## Information

For more information, see the main project README.

## Usage

Set StyLuau as your formatter when prompted, or add the following configuration to your `settings.json` file:

```json
"[lua]": {
    "editor.defaultFormatter": "<publisher>.styluau"
},
```

If you are working with Luau code, you may need to also configure under the `luau` namespace:

```json
"[luau]": {
    "editor.defaultFormatter": "<publisher>.styluau"
},
```

You can then use StyLuau to format your code by running the `Format Document` command (In `CMD/CTRL + Shift + P`).
The `Format Selection` command is also supported, firstly highlight the code you wish to format, and select `Format Selection`.

You can also enable `editor.formatOnSave` to format your code automatically on save.

## Finding a StyLuau binary

You can specify the path of the StyLuau binary using the `styluau.styluauPath` setting.

If no configuration is specified, then we lookup the version through the following steps:

1. If `styluau.searchBinaryInPATH` is enabled, then lookup a "styluau" binary on the PATH.

- If found, run `styluau --version` to ensure it executes appropriately

2. If binary not found on PATH, or does not execute, then fall back to a bundled version.

The bundled version of StyLuau is downloaded from GitHub releases, and stored in your local storage.

By default, the extension downloads the latest version. To configure the version to use, set `styluau.targetReleaseVersion`.
Or alternatively, use the `StyLuau: Select Version` command.

When a new StyLuau version is available, you will be notified on the VSCode status bar. You can disable these notifications
via `styluau.disableVersionCheck`.
