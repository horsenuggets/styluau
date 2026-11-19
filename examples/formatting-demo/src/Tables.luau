--[[

Tables

The root configuration omits the last comma in values and Luau table types. Ignore
comments preserve the original layout of the marked examples.

--]]

type Theme = {
    Accent: Color3,
    Padding: number,
    Fonts: {
        Heading: string,
        Body: string
    }
}

const theme: Theme = {
    Accent = Color3.fromRGB(80, 170, 255),
    Padding = 12,
    Fonts = {
        Heading = "GothamBold",
        Body = "Gotham"
    } -- This comment survives removal of the final comma.
}

const palette = {
    Color3.fromRGB(32, 32, 40),
    Color3.fromRGB(240, 240, 250)
}

-- A single ignored statement keeps its original spacing and trailing comma.
-- [styluau] ignore
const matrix = {{1, 0, 0}, {0, 1, 0}, {0, 0, 1},}

-- An ignored region also retains its original style.
-- [styluau] ignore start
const untouched = { 1,  2,   3, }
const alsoUntouched={ 4,5,6, }
-- [styluau] ignore end

-- The legacy spelling is still supported.
-- stylua: ignore
const legacyMatrix = {{1, 0}, {0, 1},}

const formattedAgain = {
    1,
    2,
    3
}

-- The root configuration keeps compact interpolation, unlike the UI override.
const summary = `Palette has {#palette} colors and {#formattedAgain} values.`

return {
    Theme = theme,
    Palette = palette,
    Matrix = matrix,
    Values = formattedAgain,
    Summary = summary
}
