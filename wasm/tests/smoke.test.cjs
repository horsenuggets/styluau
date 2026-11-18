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
    it("should keep positional-table Fusion regions in the parent scope", () => {
      const config = stylua.Config.new();
      const code = 'local t = {\n-- styluau: fusion on start\n{},\n-- styluau: fusion end\nvalue,\n}\n';
      const result = stylua.formatCode(code, config, undefined, stylua.OutputVerification.Full);
      assert.ok(result.includes('styluau: fusion end'));
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
