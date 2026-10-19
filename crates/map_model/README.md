# map_model

The map document: editable brushes, patches and entities and their document tree, UVs,
and readers for the formats of the supported games. Maps are the plain-text `.map` files
a Quake-lineage compiler reads, in the Standard and Valve 220 dialects.

Geometry uses typed indices into owned arrays. Editing operations build a valid result
before replacing existing geometry and report failures as errors. Coordinates are
right-handed with Z up; engines' conventions are applied at export and import. Brush
faces wind clockwise seen from outside.

Part of [Heron Brush](../../README.md).
