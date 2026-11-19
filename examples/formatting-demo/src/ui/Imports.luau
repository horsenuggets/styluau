--[[

Imports

This file has fusion_scope_names = [], so no receiver is assumed to be a Fusion scope. The
formatter follows a static Fusion import, its module alias, its scoped alias, and
innerScope/deriveScope calls instead. An unrelated receiver still keeps ordinary calls.

--]]

const Fusion = require("@packages/Fusion")
const library = Fusion
const scoped = library.scoped
const rootScope = scoped(library)
const childScope = rootScope:innerScope()
const leafScope = childScope:deriveScope()

const importedPanel = leafScope:New "Frame" {
  Name = "DetectedFromImport",
  BackgroundColor3 = Color3.fromRGB(40, 60, 80)
}

const function ordinary(factory: any)
  assert(type(factory) == "table")
  return factory:New("Frame")({ Name = "NotFusion" })
end

return { Panel = importedPanel, Ordinary = ordinary }
