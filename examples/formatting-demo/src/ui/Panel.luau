--[[

Panel

The UI glob enables Fusion and two-space indentation. New and Hydrate use Fusion-style
calls, while ordinary helpers nested inside property tables keep their parentheses.

Imports are only parsed here. This showcase does not download Fusion or run a Roblox app.

--]]

const Fusion = require("@packages/Fusion")
const scope = Fusion.scoped(Fusion)
const existingFrame = Instance.new("Frame")

-- Fusion methods other than New/Hydrate still use ordinary call parentheses.
const title = scope:Value("inventory")
const uppercaseTitle = scope:Computed(function(use)
  assert(type(use) == "function")
  return `{ string.upper(use(title)) } ({ 3 + 4 } items)`
end)

const panel = scope:New "Frame" {
  Name = "Inventory",
  Size = UDim2.fromOffset(320, 180),
  BackgroundColor3 = Color3.fromRGB(24, 28, 36),
  [Fusion.Children] = {
    scope:New "UICorner" { CornerRadius = UDim.new(0, 12) },
    scope:New "TextLabel" {
      Text = uppercaseTitle,
      TextColor3 = Color3.fromRGB(240, 244, 250),
      Size = UDim2.fromScale(1, 0.2)
    }
  }
}

const hydrated = scope:Hydrate(existingFrame) {
  Name = string.format("%sPanel", "Existing"),
  Visible = true
}

-- Block comments inside parentheses must never disappear.
const annotated = scope:New(--[[Keep this explanation.]] "TextLabel") {
  Text = "The parentheses around the class name must remain."
}

-- A suffix that would look like an index into the argument keeps its parentheses.
const safeChain = scope:New "Frame"({}).Name

return { Panel = panel, Hydrated = hydrated, Annotated = annotated, Name = safeChain }
