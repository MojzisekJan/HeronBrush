# Credits

## TrenchBroom

Heron Brush is a derivative work of [TrenchBroom](https://github.com/TrenchBroom/TrenchBroom),
copyright (C) 2010 Kristian Duske and contributors, licensed under the
GNU General Public License v3.0. Heron Brush is distributed under the same licence; see
`LICENSE`.

The `geometry_math` crate also derives from TrenchBroom's VmLib, copyright Kristian Duske
and Eric Wasylishen.

The game profiles and entity definitions in `gamedata/` are adapted from TrenchBroom
2026.1; the definition files keep their own authors' notices. See `gamedata/README.md`
and `gamedata/TrenchBroom-LICENSE.txt`.

## tbx

The `scene_export` crate comes from tbx, copyright (c) 2026 the tbx authors, licensed
under the MIT licence (`crates/scene_export/LICENSE-MIT`).

## three.js

The web build's playtest bundles three.js r186 and some of its example modules
(`crates/heron-brush-web/vendor/three/`), copyright 2010-2026 three.js authors, MIT
licence (`crates/heron-brush-web/vendor/three/LICENSE`).

## Fonts and icons

- Inter, copyright The Inter Project Authors, SIL Open Font License 1.1
  (`crates/heron-brush/fonts/Inter-LICENSE.txt`).
- Noto Sans CJK (SC, TC, JP and KR, cut down to the characters the editor shows), in the
  web build only, copyright Adobe, SIL Open Font License 1.1
  (`crates/heron-brush-web/fonts/NotoSansCJK-LICENSE.txt`).
- Ubuntu Light, Ubuntu Font Licence 1.0.
- Hack, MIT and Bitstream Vera licences.
- Noto Emoji, SIL Open Font License 1.1.
- emoji-icon-font, MIT licence.
- Phosphor icons (regular weight), copyright Phosphor Icons, MIT licence.

## Engine logos

The export window shows the logos of the engines it exports to
(`crates/heron-brush/assets/engines/`). They are the marks of their projects, used only to
name the export target. Babylon.js's is from its Brand-Toolkit, CC BY-ND 4.0, scaled onto
the card and otherwise unchanged. PlayCanvas's is the black mark from its press pack
(developer.playcanvas.com/user-manual/press-pack), which PlayCanvas offers to people
naming and promoting it; it is scaled onto the card and not recoloured. The mark is
PlayCanvas's trademark and is not covered by the engine's MIT licence.

## Babylon.js and Havok

The Babylon.js export's playtest page loads Babylon.js 9.29.0 and its loaders (Apache
licence 2.0) and the Havok physics plugin 1.3.14 (MIT licence) from jsDelivr. Nothing of
them is bundled.

## PlayCanvas and ammo.js

The PlayCanvas export's playtest page loads the PlayCanvas engine 2.23.1 (MIT licence)
from jsDelivr, and the ammo.js build the engine's repository ships at tag `v2.23.1`
(ammo.js is Bullet compiled with Emscripten, zlib licence). Nothing of them is bundled.

## Demos

Textures, sounds and the FPS starter kit used in `demos/` and `examples/` are by Kenney
(CC0) and carry their licence files beside them.
