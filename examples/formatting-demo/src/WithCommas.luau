--[[

WithCommas

The override for this exact file enables final commas, for values and table types. The
base configuration omits them everywhere else. The formatter adds missing commas here.

--]]

type CardStyle = {
    Title: string,
    Size: {
        Width: number,
        Height: number,
    },
}

const style: CardStyle = {
    Title = "Inventory",
    Size = {
        Width = 320,
        Height = 180,
    },
}

return style
