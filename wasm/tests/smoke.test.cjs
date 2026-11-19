/**
 * Smoke tests for StyLuau WASM module (CommonJS)
 * These tests verify that the CJS WASM bindings work correctly after build
 */

const assert = require("node:assert");
const { describe, it } = require("node:test");

// Import the Node.js CJS version
const stylua = require("../styluau_lib_node.cjs");

describe("StyLuau WASM Smoke Tests (CJS)", () => {
  describe("Module exports", () => {
    it("should export formatCode function", () => {
      assert.strictEqual(typeof stylua.formatCode, "function");
    });

    it("should export Config class", () => {
      assert.strictEqual(typeof stylua.Config, "function");
    });

    it("should export Range class", () => {
      assert.strictEqual(typeof stylua.Range, "function");
    });

    it("should export SortRequiresConfig class", () => {
      assert.strictEqual(typeof stylua.SortRequiresConfig, "function");
    });

    it("should export OutputVerification enum", () => {
      assert.ok(stylua.OutputVerification !== undefined);
    });

    it("should export LuaVersion enum", () => {
      assert.ok(stylua.LuaVersion !== undefined);
    });

    it("should export IndentType enum", () => {
      assert.ok(stylua.IndentType !== undefined);
    });
  });

  describe("Config", () => {
    it("should create a default config", () => {
      const config = stylua.Config.new();
      assert.ok(config !== undefined);
    });

    it("should expose configurable Markdown comment reflow", () => {
      for (const enabled of [false, true]) {
        const expected = enabled
          ? "-- first line second line\nlocal x = 1\n"
          : "-- first line\n-- second line\nlocal x = 1\n";
        for (const code of ["-- first line\n-- second line\nlocal x=1\n", expected]) {
          const config = stylua.Config.new();
          assert.strictEqual(config.comment_reflow, false);
          assert.strictEqual(config.comment_width, 90);
          config.comment_reflow = enabled;
          config.comment_width = 40;
          assert.strictEqual(config.comment_reflow, enabled);
          assert.strictEqual(config.comment_width, 40);
          assert.strictEqual(stylua.formatCode(code, config, undefined, stylua.OutputVerification.Full), expected);
        }
      }
    });

    it("should keep wrapped block comments stable before same-line code", () => {
      let result = "--[[ " + "one two three four five six seven eight nine ten ".repeat(4) + "]] local x=1\n";
      for (let pass = 0; pass < 3; pass++) {
        const config = stylua.Config.new();
        config.comment_reflow = true;
        config.comment_width = 90;
        const output = stylua.formatCode(result, config, undefined, stylua.OutputVerification.Full);
        assert.ok(output.startsWith("--[[\n"));
        assert.ok(output.split("\n").every(line => line.length <= 90));
        if (pass > 0) assert.strictEqual(output, result);
        result = output;
      }
    });

    it("should honor comment opt-outs and Markdown list boundaries", () => {
      const config = stylua.Config.new();
      config.comment_reflow = true;
      const code = "-- [styluau] comments off\n-- first line\n-- second line\nlocal x=1\n-- 1. First item\n-- 2. Second item\nlocal y=2\n";
      assert.strictEqual(
        stylua.formatCode(code, config, undefined, stylua.OutputVerification.Full),
        "-- [styluau] comments off\n-- first line\n-- second line\nlocal x = 1\n-- 1. First item\n-- 2. Second item\nlocal y = 2\n"
      );
    });

    it("should reject invalid comment width and unmatched comment regions", () => {
      const invalidWidth = stylua.Config.new();
      invalidWidth.comment_width = 0;
      assert.throws(() => stylua.formatCode("local x=1", invalidWidth, undefined, stylua.OutputVerification.Full), /comment_width/);
      assert.throws(() => stylua.formatCode("-- [styluau] comments end\n", stylua.Config.new(), undefined, stylua.OutputVerification.Full), /unmatched end/);
    });

    it("should expose configurable interpolation spacing", () => {
      for (const enabled of [true, false]) {
        const expected = enabled
          ? "local s = `Hello { name } { other }`\n"
          : "local s = `Hello {name} {other}`\n";
        for (const code of ["local s= `Hello {name} {other}`", expected]) {
          const config = stylua.Config.new();
          config.syntax = stylua.LuaVersion.Luau;
          assert.strictEqual(config.space_inside_interpolation, false);
          config.space_inside_interpolation = enabled;
          assert.strictEqual(config.space_inside_interpolation, enabled);
          const result = stylua.formatCode(
            code,
            config,
            undefined,
            stylua.OutputVerification.Full
          );
          assert.strictEqual(result, expected);
        }
      }
    });

    it("should expose configurable Fusion scope names", () => {
      const config = stylua.Config.new();
      assert.deepStrictEqual(config.fusion_scope_names, ["scope", "fusion"]);
      config.fusion_scope_names = ["uiScope"];
      assert.deepStrictEqual(config.fusion_scope_names, ["uiScope"]);
      config.fusion = true;
      config.syntax = stylua.LuaVersion.Luau;
      const result = stylua.formatCode(
        'local x = uiScope:New("Frame")({})',
        config,
        undefined,
        stylua.OutputVerification.Full
      );
      assert.ok(result.includes('uiScope:New "Frame" {}'));
    });

    it("should have default column_width of 120", () => {
      const config = stylua.Config.new();
      assert.strictEqual(config.column_width, 120);
    });

    it("should have default indent_width of 4", () => {
      const config = stylua.Config.new();
      assert.strictEqual(config.indent_width, 4);
    });
  });

  describe("formatCode", () => {
    it("should use bracket directives to enable Fusion for one statement", () => {
      const config = stylua.Config.new();
      config.syntax = stylua.LuaVersion.Luau;
      const code = [
        "-- [styluau] fusion on",
        'local first = scope:New("Frame")({})',
        'local second = scope:New("Frame")({})',
        "",
      ].join("\n");
      const result = stylua.formatCode(
        code,
        config,
        undefined,
        stylua.OutputVerification.Full
      );
      assert.ok(result.includes('local first = scope:New "Frame" {}'));
      assert.ok(result.includes('local second = scope:New("Frame")({})'));
    });

    it("should honor bracket ignore controls in block comments", () => {
      const config = stylua.Config.new();
      const code = [
        "--[[[styluau] ignore start]]",
        "local first=  1",
        "--[[[styluau] ignore end]]",
        "local second=  2",
        "",
      ].join("\n");
      const result = stylua.formatCode(
        code,
        config,
        undefined,
        stylua.OutputVerification.Full
      );
      assert.ok(result.includes("local first=  1"));
      assert.ok(result.includes("local second = 2"));
    });

    it("should report unmatched bracket Fusion regions", () => {
      const config = stylua.Config.new();
      assert.throws(() => stylua.formatCode(
        "-- [styluau] fusion end\nlocal x = 1\n",
        config,
        undefined,
        stylua.OutputVerification.Full
      ), /unmatched end/);
    });

    it("should keep positional-table Fusion regions in the parent scope", () => {
      const config = stylua.Config.new();
      const code = 'local t = {\n-- [styluau] fusion on start\n{},\n-- [styluau] fusion end\nvalue,\n}\n';
      const result = stylua.formatCode(code, config, undefined, stylua.OutputVerification.Full);
      assert.ok(result.includes('[styluau] fusion end'));
    });

    it("should format simple Lua code", () => {
      const config = stylua.Config.new();
      const result = stylua.formatCode(
        "local   x   =    1",
        config,
        undefined,
        stylua.OutputVerification.None
      );
      assert.strictEqual(result, "local x = 1\n");
    });

    it("should format code with custom config", () => {
      const config = stylua.Config.new();
      config.indent_type = stylua.IndentType.Spaces;
      config.indent_width = 2;
      const code = "if true then\nreturn 1\nend";
      const result = stylua.formatCode(
        code,
        config,
        undefined,
        stylua.OutputVerification.None
      );
      assert.ok(result.includes("  return"));
    });

    it("should throw on invalid Lua code", () => {
      const config = stylua.Config.new();
      assert.throws(() => {
        stylua.formatCode(
          "local x = ",
          config,
          undefined,
          stylua.OutputVerification.None
        );
      });
    });

    it("should format with OutputVerification.Full", () => {
      const config = stylua.Config.new();
      const result = stylua.formatCode(
        "local x = 1",
        config,
        undefined,
        stylua.OutputVerification.Full
      );
      assert.strictEqual(result, "local x = 1\n");
    });
  });

  describe("Range", () => {
    it("should create a range from values", () => {
      const range = stylua.Range.from_values(0, 100);
      assert.ok(range !== undefined);
      assert.strictEqual(range.start, 0);
      assert.strictEqual(range.end, 100);
    });
  });
});
