# Face paint in engine exports

One brush can have several face materials. Exporters group its triangles into mesh
surfaces or primitives by material. The brush does not need to become several scene
objects to retain those assignments.

`paint_export.rs` rasterizes face paint for direct engine exports and OBJ:

- Opaque solid paint becomes a 1×1 RGBA PNG, shared by faces with identical pixels.
- Gradients become PNGs with 64–1024 pixels on each axis. The face's UV mapping,
  gradient angle and polygon bounds determine the colour progression.
- Translucent paint is composited over the original texture. Paint opacity is not
  interpreted as glass transparency.
- UVs are remapped to texel centres. The temporary map uses explicit Valve UV axes
  so sloped faces retain the gradient direction. The document is unchanged.
- Tool materials such as skip, clip and origin are not replaced by generated paint.

Godot receives StandardMaterial3D resources and external PNGs. Unity receives OBJ,
MTL and PNG files; material references are relative to the mesh folder. Its Build
Level importer sets the paint images to clamp, as the GLB samplers do. Engine
exports using GLB embed the paint images and PBR base-colour material assignments
inside each mesh. External PNGs are also available to loaders such as Defold's.
Standalone glTF/GLB retains its opaque-solid `baseColorFactor` optimisation and uses
the same rasterizer for gradients and translucent paint.

Godot and Unity live links prepare paints on the worker thread. Textures are
published and imported before the meshes. Content-derived resource names ensure a
paint-only edit also invalidates Unity's immutable mesh snapshot.

Run `cargo test -p editor_core --lib export::`, `cargo test -p editor_core --lib live_paint_images`
and `cargo test -p scene_export`. Tests inspect PNG pixels, UVs on rotated faces, mixed
materials on one brush, embedded GLB images, MTL paths, deduplication, skip handling,
read-only checks, and paint-only live updates.

Native engine editor imports, lighting and render-pipeline-specific shader defaults
still require testing in the respective engine projects.
