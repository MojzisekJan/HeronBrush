# Engine logo sources

The projects' own marks, kept here so the tab icons in the export window can be
regenerated. `icon_color.*` is Godot's, `bevy_logo_dark.svg` Bevy's (only the
bird is used); `three_js_logo.svg`, `babylon_logo.png` (the full-colour mark from
Babylon.js's Brand-Toolkit, CC BY-ND 4.0, so it is scaled but never recoloured), `raylib_logo.png`, `defold_logo.svg`,
`unity_logo.svg`, `unreal_logo_black.png`, `flax_logo.png`, `gdevelop_icon.png` (from the macOS app) and
`cocos_logo.png` are the rest. Unreal ships a black
and a white mark; the card is light, so the black one is the one kept.

`generate.py` reads these, trims and unifies each to a 96px rounded card in
`../`, and draws a lettered tile for `.obj`, which has none.
Run it from this directory with rsvg-convert (librsvg), Pillow and NumPy available.
