# Project material paths

Heron Brush 0.1.0-alpha.2 saves loaded material directories and WAD references
inside the map as editor metadata. Paths are relative to the saved map where
possible, including when using Save As or saving a backup in a different folder.
For a map in `maps/` with images in a sibling `textures/` folder, the header is:

```text
// heron material-sources [["folder","../textures"]]
```

The normal map beside an image is loaded with the material. Move the map and its
texture directories together to preserve the relative relationship. An explicitly
supplied material directory takes precedence over restored sources.

The header is a comment, so other map readers can ignore it. Engine exports do not
include these editor resource references. Material collections supplied by the
installed asset library are defaults and do not store installation paths in maps.
Missing source references are retained when saving and produce a diagnostic.

Older maps without this metadata discover a `textures/` directory beside the map,
or beside the enclosing `maps/` directory. An explicitly empty saved source list
means no project sources; it does not trigger this fallback.
