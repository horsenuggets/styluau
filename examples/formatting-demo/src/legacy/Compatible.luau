--[[

Compatible

The nearest stylua.toml is still recognized. This folder uses two spaces, single quotes,
and final commas instead of the root style. A nested config replaces the parent config;
it does not automatically inherit root overrides.

--]]

const settings = {
  Title = 'LegacyConfiguration',
  Colors = {
    'Blue',
    'Green',
  },
}

return settings
