# Special materials

`skip`, `clip` and `origin` are built into the editor and available in every map
without adding a texture folder. The browser shows them as `special/skip`,
`special/clip` and `special/origin`, but assigning one writes the short name to
the map. A project texture with the same short name takes precedence; removing
that project collection restores the built-in material.
