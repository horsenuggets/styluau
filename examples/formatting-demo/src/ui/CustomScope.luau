--[[

CustomScope

This file matches the general UI glob, then a narrower rule replaces the receiver list
with uiScope and switches back to four spaces. The unrelated scope parameter is NOT an
assumed Fusion receiver in this file. Both values are parameters, not inferred imports.

--]]

const function build(uiScope: any, scope: any)
    assert(type(uiScope) == "table")
    assert(type(scope) == "table")

    const fusionPanel = uiScope:New "Frame" {
        Name = "CustomReceiver",
        Visible = true
    }

    const ordinaryPanel = scope:New("Frame")({
        Name = "OrdinaryReceiver",
        Visible = true
    })

    return { Fusion = fusionPanel, Ordinary = ordinaryPanel }
end

return build
