--[[

Controls

This file does NOT match the UI glob. Fusion is off until a comment enables it. Compare
one-shot controls, nested regions, field controls, and receiver declarations below.

--]]

-- This paragraph starts on several short lines, which are joined and wrapped
-- independently of the code column width. Markdown list markers below keep their items
-- separate even without blank lines between them.
--
-- 1. Enable Fusion for a single statement without changing the surrounding ordinary
--    calls.
-- 2. Nest enabled and disabled regions to control constructor formatting locally.
-- - Mark a custom receiver explicitly when it cannot be traced back to a Fusion import.
-- - Preserve ordinary receiver calls even when they appear inside a Fusion component.

-- [styluau] comments off
-- Keep this group
-- deliberately narrow.

const Fusion = require("@packages/Fusion")
const scope = Fusion.scoped(Fusion)

-- [styluau] comments off start
-- Receiver       Mode
-- scope          Ordinary until enabled below.
const ordinary = scope:New("Frame")({ Name = "OutsideFusion" })
-- [styluau] comments end

-- This enables only the next statement.
-- [styluau] fusion on
const oneShot = scope:New "Frame" { Name = "OneShot" }
const backToNormal = scope:New("Frame")({ Name = "OrdinaryAgain" })

-- An enabled region can contain a nested disabled region.
-- [styluau] fusion on start
const regional = scope:New "Frame" { Name = "EnabledRegion" }

-- [styluau] fusion off start
const disabled = scope:New("Frame")({ Name = "DisabledRegion" })
-- [styluau] fusion end

const enabledAgain = scope:New "Frame" { Name = "EnabledAgain" }

-- A one-shot off comment affects only this table field, including its callback body.
const callbacks = {
    -- [styluau] fusion off
    Ordinary = function()
        return scope:New("Frame")({ Name = "OrdinaryCallback" })
    end,
    Fusion = function()
        return scope:New "Frame" { Name = "FusionCallback" }
    end
}
-- [styluau] fusion end

const outsideRegion = scope:New("Frame")({ Name = "OutsideRegion" })

-- A scope parameter cannot be traced to an import, so mark it explicitly.
const function build(scopedParameter: any)
    assert(type(scopedParameter) == "table")
    -- [styluau] fusion on start
    -- [styluau] fusion scope scopedParameter
    const marked = scopedParameter:New "Frame" { Name = "ExplicitScope" }

    do
        -- Exclusion lasts only within this lexical block.
        -- [styluau] fusion not-scope scopedParameter
        const excluded = scopedParameter:New("Frame")({ Name = "ExcludedScope" })
        print(excluded)
    end

    const restored = scopedParameter:New "Frame" { Name = "RestoredScope" }

    -- Exclusion also wins over a configured name or an inferred import.
    -- [styluau] fusion not-scope scope
    const excludedImport = scope:New("Frame")({ Name = "ExcludedImport" })
    -- [styluau] fusion scope scope
    const includedImport = scope:New "Frame" { Name = "IncludedAgain" }
    -- [styluau] fusion end

    return {
        Marked = marked,
        Restored = restored,
        Excluded = excludedImport,
        Included = includedImport
    }
end

return {
    OneShot = oneShot,
    Regional = regional,
    Disabled = disabled,
    Enabled = enabledAgain,
    Callbacks = callbacks,
    Ordinary = ordinary,
    BackToNormal = backToNormal,
    Outside = outsideRegion,
    Build = build
}
