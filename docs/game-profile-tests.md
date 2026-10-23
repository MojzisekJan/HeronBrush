# Game profile tests

Heron Brush ships game profiles in `gamedata/`. Each one is tried on the real game:
install it, build a playable map in Heron following that game's own mapping guides,
compile it with the game's tools, run it in the game and fix what Heron gets wrong.
Engine export targets are checked separately, see [feature matrix](feature-matrix.md).

## Order

Games that are free to download come first, one at a time.

| Profile | Free game or data | Status |
| --- | --- | --- |
| Quake | LibreQuake data, a source port, ericw-tools | in progress |
| Neverball | Neverball and its `mapc` | planned |
| Quake3 | OpenArena, q3map2 | planned |
| Quetoo | Quetoo and its tools | planned |
| DigitalPaintball2 | Digital Paint: Paintball 2 | planned |
| Quake2 | the freely distributed demo, a source port | planned |
| DDayNormandy | the free mod on the Quake 2 demo | planned |
| Hexen2 | the freely distributed demo | planned |

Not tried, because the game has to be bought: Halflife, Kingpin, SoF, Heretic2,
Daikatana and Wrath. `Generic` and `heron_brush` are Heron's own profiles.
