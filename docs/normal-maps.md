# Normal maps

The 3D view and image renders support tangent-space normal maps on textured
materials. A normal map changes the normal used for lighting and reflections; it
does not change geometry, silhouette or collision. It is also used in the normal
pass for SSAO and screen-space reflections. The Flat diagnostic mode keeps simple
geometric shading.

## Loading

Put the normal map in the same folder as the colour texture, then load or reload
the material folder. For example:

```text
walls/brick.png
walls/brick_normal.png
```

Supported name suffixes, in order of precedence, are `_normal_gl`, `_normal`,
`_n` and `_normal_dx`. The first three use the OpenGL convention; `_normal_dx`
flips the green channel of a DirectX map. Matching ignores case. A colour texture
named `brick_albedo.png`, `brick_basecolor.png` or `brick_diffuse.png` also pairs
with `brick_normal.png`. The image format may differ from the colour texture.

Normal maps load as linear RGB and their alpha is ignored. Resolution may differ
from the colour texture. UV offset, rotation, scale and mirroring follow the face
mapping, and mipmapping reduces shimmer in the distance.

Paired normal maps are not listed in the browser as separate colour textures;
unpaired images remain available. A broken normal map does not hide a valid
colour texture, and the error appears among the material loading warnings.

## Limits

Automatic pairing applies to materials loaded from image folders. Plain colours
and gradients have no normal map input.

Exports that write GLB meshes embed the normal map in the OpenGL convention,
with tangents, as the material's `normalTexture`. Godot's StandardMaterial3D takes
a `<texture>_normal` image from the project's texture folder; the other suffixes
are not looked for there. Unity gets the map as `<material>_normal.png` beside the
albedo, also in the OpenGL convention, named on the MTL's `norm` line. Build Level
imports it as a normal map and sets it as the material's `_BumpMap`. The plain
OBJ writer, Defold's `.model` (one colour sampler) and Stride's materials do not
carry it. See [the feature matrix](feature-matrix.md).
