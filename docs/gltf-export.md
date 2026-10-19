# glTF and GLB export

Choose **glTF .gltf** or **GLB .glb** in the Export window.

- glTF writes a JSON scene, a `.bin` geometry buffer and a `<name>_textures` directory of PNG images. Keep these together when moving the scene. Empty scenes do not need a binary buffer.
- GLB packages the scene, geometry and images into one `.glb` file.

Both targets use glTF 2.0, right-handed Y-up coordinates and the selected units-per-metre value (32 by default). They preserve individual brushes, tessellated Bézier patches and, when enabled, layers and groups. Layers marked “omit from export” are excluded. Hidden editor layers are still exported unless explicitly omitted.

Skip, clip and origin faces are omitted using the current game's tool-material names or the export overrides. Remaining brush faces are triangulated with outward winding. Reverse winding and vertical UV flip are available under More. Patches use double-sided materials.

Loaded editor textures, including decoded WAD images, are copied into the export as PNG. Opaque solid face paint uses a material colour; gradients and translucent paint overlays are baked into face textures. Baking uses 64–1024 pixels per UV dimension, so very large painted textured faces can lose texture detail. Identical image bytes are shared. Unloaded textures produce a warning and a neutral material; load the material source in the editor before exporting. Custom shader graphs and extra PBR maps are not converted.

Point entities become named empty nodes at their origins. Entity properties, including gameplay and orientation properties, are preserved in `extras.entity`. Brush geometry is already in world space; parent groups do not apply a second transform. Gameplay scripts, engine-specific entity classes, physics and lightmap unwrapping are not generated. Brush merging and hidden-face optimisation are not applied by these portable targets; the related controls are hidden, and explicit MCP requests for them produce an export warning. Existing skip textures are respected.

Each target remembers its own options and supports export on map save. MCP accepts `engine: "gltf"` and `engine: "glb"` in `export`, `check_export`, and the export settings tools.

Validation covers scale, outward winding, hierarchy and entity metadata, skip and omitted layers, patches, image embedding, texture/paint composition, non-ASCII resource URIs and empty scenes. Hydrant, gate_paifang and submarine were exported through MCP in both formats and imported into Blender 4.0.2 with matching geometry and dimensions; the hydrant was also rendered for visual inspection.
