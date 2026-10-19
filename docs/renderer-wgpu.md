# The wgpu renderer (`crates/render_wgpu`)

The 3D view of the egui app, on **wgpu** (Metal, Vulkan, DX12), structured like Bevy's:
a retained scene fed by changes from `editor_core`, cascaded sun shadows, image-based
lighting, GTAO, bloom, four tone maps, normal maps, sprites, point and spot lights from
light entities with clustered shading and cached cube shadows, contact shadows,
screen-space reflections, and temporal antialiasing that converges to a supersampled
still picture. The PNG render page uses it too, with accumulation and depth of field.

It is the `wgpu` cargo feature of `heron-brush`, on by default. `--no-default-features`
builds the older glow / GL 4.1 renderer in `crates/render`, kept as a fallback until wgpu
has been tested on every platform; `docs/renderer-plan.md` holds the audit that led
here.

## Architecture

### Data flow: a retained render world fed by deltas

```
editor_core::Document ──preview mesh + owners──▶ SceneTracker ──SceneSink deltas──▶ RenderScene
        (CPU, per node)                   (per-object hash,           (clear / upsert / remove /
                                           selection fast path)        set_state / finish)
Scene3d::build_overlay ──points, guides──▶ Renderer::render(ViewParams) ──▶ output texture ──▶ host
```

- **`editor_core::render_scene`** is the backend-neutral side. `Document::with_preview_surfaces`
  borrows the document's per-node surfaces (`document::SurfaceCache`): each drawn brush,
  patch and point entity keeps its triangles and a *stamp* that changes with them, and
  every rebuild appends the nodes it changed (or whose selection changed, or that left or
  joined the drawn set) to a change log. `SceneTracker::sync` reads the log since its last
  sync and visits only those nodes: a new stamp upserts the object, a changed selection
  sends its *state* (selected / locked / hidden), a node no longer drawn is removed. A
  tracker that fell behind the log (or a new one) compares stamps over the drawn nodes --
  still no hashing of geometry. A face selection is carried by per-vertex flags
  (`triangle_flags`), so it re-sends just that object; painted faces carry `NO_TINT` per
  vertex whatever the selection, so selecting a painted brush is a state change too.
- What a rebuild retriangulates comes from the edit path (`touched_ids` of each applied
  edit, the transaction's rollback and owned apply); a selection change patches flags;
  hiding, isolating and layer visibility re-walk the tree without triangulating; anything
  else -- including a revision bump nobody explained -- rebuilds every node. Context the
  surfaces depend on (generation, materials, models, geometry filters, colour preferences)
  rebuilds everything when it changes. A debug assertion compares every incremental
  rebuild of a map under 4,096 nodes with a rebuild from nothing (paint, gradients, face
  selection, layers, hiding, filters and the clip preview are covered by tests).
- **`Scene3d::prepare_frame`** does what `render_kept` does for GL minus the drawing: render
  settings and FOV (`begin_frame`), animation pose, the sync, and the tools' overlay and
  HUD (`build_overlay`, shared with GL). The GL path calls the same `begin_frame` and
  `build_overlay`, so the overlay logic exists once.
- **`render_wgpu::RenderScene`** (the render world) implements `SceneSink`. It never reads
  the document.

### Geometry: chunked slabs with sparse updates

- Objects are packed into **compact indexed vertices, 32 bytes** (f32 position, snorm8
  normal, f32 texel UV, unorm8 sRGB colour, u32 object slot + 8 flag bits), de-duplicated
  per object. A quad costs 4 × 32 + 6 × 4 = 152 bytes instead of GL's 288.
- Objects live in **spatial chunks** (1024-unit grid by object centre). A chunk owns a
  vertex buffer, an index buffer sorted by (phase, material instance), and an edge index
  buffer for wireframe; buffers grow by half and are rewritten in place. A changed object
  dirties only its chunk; `finish()` rebuilds dirty chunks from the per-object CPU copies.
- Planned: one shared vertex/index slab per frame-graph "mesh allocator" with free-list
  sub-allocation (Bevy 0.15+), so a chunk rebuild becomes a sub-range write and multi-draw
  indirect can address every chunk from one buffer; chunk splitting above ~512 objects.

### Object and material tables

- **Object table**: storage buffer, 16 bytes per object (flags: selected, locked, hidden;
  tint). Slots come from a free list; only dirty slots are written, coalesced into runs.
  A selection click on a big map is one 16-byte write — no geometry moves.
- **Material instance table**: storage buffer, 32 bytes per (texture, per-face PBR factors)
  instance: texel size, alpha mode (opaque / masked / blended), flags, roughness, metallic,
  emission. Draws pass the instance slot as `first_instance`, so the shader reads
  `materials[instance_index]` — the Bevy pattern that becomes `INDIRECT_FIRST_INSTANCE`
  in indirect draws later.
- The flags say whether the material has a normal map and whether its green channel
  points down (DirectX maps), from `Material::normal_map`.
- Motion: an object re-sent with the same vertices and triangles in other places (the
  animation preview, a drag) keeps its previous positions; its chunk then carries a
  second vertex buffer, a copy of its vertices at last frame's positions, which the
  prepass reads for motion vectors (a still chunk binds its own buffer there). The
  motion is dropped, and the chunk rebuilt without the copy, at the first sync that does
  not move the object again or the first frame drawn without a sync.
- Later: transforms per object (animation preview as a per-object matrix instead of
  re-sending triangles), ORM / emissive maps as further flags and textures once the
  material collection loads them.

### Textures

Still **one bind group per material**: albedo (`Rgba8UnormSrgb`, 16× anisotropic) and
its normal map (`Rgba8Unorm`; a shared 1×1 flat normal when it has none). The textures
are **streamed** (`textures.rs`):

- `map_model::material` reads a folder from the image headers only (name, size, whether
  there is an alpha channel); `Material::pixels` decodes on demand and keeps the decode
  only while someone holds it (plus a 64 MB ring of recent decodes). A decode settles the
  alpha class and the average colour, which `material-info.txt` in the configuration
  directory remembers for the next opening of those files.
- Each frame the renderer notes which base materials the visible draws (and sprites) used,
  queues the missing ones to 1–4 worker threads that decode and mip them (box filter in
  linear light for albedo, plain for normal maps), and uploads finished ones within
  **48 MB per frame**. Until then the material table marks the instance *pending* and the
  forward shader draws the average colour (grey before it is known); a pending texture
  cannot cut or blend, so its draws are opaque.
- A texture whose decode shows a different alpha class than assumed (an alpha channel
  that turns out to cut or blend) marks the chunks drawing it dirty; chunks keep each
  object's batches by (see-through paint, material slot) and work out the pass when they
  are built, so no triangles are re-sent.
- Resident textures are evicted least recently drawn first (never one drawn this frame)
  while they exceed the *Texture memory* preference (`render/Texture memory`, MiB, 1024
  by default; wgpu builds only). Evicted textures stream in again when seen.
- `Renderer::streaming` says frames are still wanted (the egui view keeps drawing, and the
  temporal filter does not count as converged, until the textures in view are in); the PNG
  render page calls `RenderScene::load_textures_now` (all cores, no budget) so its first
  sample is final.

**Arrays and bindless were not done**, by measurement: on a 50k-brush map with 2,000
textures the binding of per-material bind groups is about 30 % of the frame's CPU
encoding (`set_bind_group` → Metal `setFragmentTexture` and resource tracking), about 3 ms
at 8,000 draws, roughly 0.35 µs per bind. Texture arrays by size class would save that,
but force a page's textures to one size and format and make eviction a page at a time; the
bigger lever is fewer draws (below). Next: a bindless array where
`TEXTURE_BINDING_ARRAY` + non-uniform indexing exist, which together with indirect draws
makes the forward pass one call per chunk; BC/ASTC compression with a KTX2 disk cache.

### Draw submission and culling

- CPU frustum culling of chunk AABBs, then per visible chunk one draw per
  (phase, material instance).
- Depth-only passes -- shadow cascades, light cube faces, and the prepass for
  materials without a normal map -- draw all of a chunk's opaque surfaces in one call
  (a chunk's indices are sorted by pass, then normal-mapped or not, then material, so
  those are contiguous ranges); masked surfaces keep a draw per material for their holes.
- Planned: **GPU-driven**: a compute pass culls chunks (and later meshlets/objects) against
  the frustum and a hierarchical-Z pyramid from the previous frame's depth, writes
  `DrawIndexedIndirectArgs` with `first_instance` = material slot, and the passes issue
  `multi_draw_indexed_indirect(_count)` where `Features::MULTI_DRAW_INDIRECT(_COUNT)` and
  `INDIRECT_FIRST_INSTANCE` exist. Not done because profiles show the CPU culling is
  negligible (about 1 % of the frame's CPU time for ~800 chunks); what costs
  is the number of draws and bind groups, which only bindless textures plus indirect
  draws remove.

### Passes

Every pass is a fragment pass (no compute), so a GLES path (see *Not done yet*) can keep
them. Solid mode skips 1, 2, 3, 5, 7 and 9; wireframe draws the chunk edge
buffers straight into the output, then the overlay.

| # | pass | target | when | notes |
|---|------|--------|------|-------|
| 1 | IBL | 128² sky cube (8 mips), 128² specular cube (6 mips), 9×1 SH, all `Rgba16Float`/`Rgba32Float` | sky colours or sun changed | sky drawn into the cube without the sun's disc (the direct light has it), box mips, GGX prefilter with filtered importance sampling (64 samples), SH9 projection of the 16² mip; ~1.5 MB |
| 2 | shadow cascades | 4 × `Depth32Float` array layers, half the *Shadow map size* preference each (1024² by default: GL's texel count and memory, 16 MB) | per cascade, when its key changed | see *Shadows* below |
| 3 | light shadows | `Depth32Float` cube-map array, a quarter of a cascade's side (256² by default), as many cubes as shadowed lights in view (up to 8: 12.6 MB) | per cube face, when its key changed | see *Light entities* below |
| 4 | prepass | `Depth32Float` (reverse-Z), `R32Uint` id, `Rg16Float` normal (octahedral, normal-mapped), `Rgba16Float` motion (world units since last frame) | unless only shading changed | opaque and masked; kept (with the occlusion) when the jittered view-projection, the chunks drawn, hidden objects and materials are as last frame — a selection click. The motion is each surface's own (zero unless its object moved), so a kept prepass stays right whatever the camera did |
| 5 | GTAO | `Rg16Float` (occlusion, view depth), full resolution up to 2.5 MP, else half | AO on, with the prepass | XeGTAO-style: 6 steps × 2 sides per slice, 1 slice per frame under the temporal filter (its noise turns every frame, so the history averages many) or 3 without; cosine-weighted visibility with distance falloff over the *Radius* preference; then a 4×4 depth-aware denoise |
| 6 | sky, forward, blended | `Rgba16Float` HDR + `Rgba8Unorm` reflectivity | always | Cook-Torrance GGX sun with cascaded shadows and contact shadows; light entities from the clusters; IBL: SH irradiance, prefiltered specular by roughness, split-sum DFG LUT (64², made once), multi-scattering energy compensation (Fdez-Agüera) for IBL and every light's specular; GTAO for diffuse (Jimenez multi-bounce) and specular occlusion (Lagarde); normal maps with GL's derivative tangent frame. `GreaterEqual` against the prepass, depth read-only (contact shadows sample it) → every pixel shaded once |
| 7 | SSR | `Rgba16Float`, half resolution | *Reflections* on | see *Reflections* below |
| 8 | TAA | 2 × `Rgba16Float` history, 2 × `Rg16Float` per-pixel depth and sample count | *Antialiasing* or *Accumulate samples* on | see *Temporal antialiasing* below |
| 9 | bloom | `Rgba16Float` mip chain from half resolution, 5–6 levels (the last 8–16 texels across) | bloom on | COD/Bevy: 13-tap downsample, the first with GL's soft threshold and Karis averaging, then 3×3 tent upsample added back up; intensity as GL (normalised by the level count) |
| 10 | resolve | `Rgba8Unorm` (the output itself unless FXAA or an offline sample follows) | always | reflections (unless TAA has them), bloom, depth of field (offline renders), exposure, tone map (AgX default, Khronos PBR Neutral, TonyMcMapface LUT, ACES), the exact sRGB curve, background colour, selection outline from the id target (depth-aware), triangular dither |
| 11 | sprites, overlay | same | always | point-entity `iconsprite`s as camera-facing quads against the depth; axes, guides and handles against the depth, then at 35 % through it; both with the unjittered projection (the overlay with half a percent of depth slack against the jittered depth) |
| 12 | FXAA | output `Rgba8Unorm` | without TAA only | GL's FXAA (kept for the GLES path), or a copy for an accumulation sample |

**Shadows.** Splits are 90 % logarithmic between where the scene starts (the nearest
point of its bounds, or the near plane inside it) and its farthest corner, both quantised
to quarter octaves so they stay put while the camera moves about. Each cascade is the
bounding sphere of its frustum slice, whose radius depends only on the split and the
field of view (turning the camera does not resize it), snapped to whole texels in the
light's frame (moving does not make edges crawl); its depth range is the whole scene's,
so casters outside the slice still cast and the range does not follow the camera, widened
to multiples of 256 units so an animated object that moves the scene's bounds does not
change every cascade's depth mapping and bias each frame (only its casters' cascades are
redrawn, with the same matrix).
Lookups: a normal offset of about a texel (scaled by the *Bias* preference, larger at
grazing light), a slope-scaled raster bias, and a 16-tap Poisson disc of hardware 2×2 PCF
taps (`sampler_comparison`) with the GL renderer's radius (*Softness* × 1.6 texels); the
last tenth of each cascade blends into the next, the last into lit. A cascade is redrawn
only when the hash of (its matrix, the generation of every chunk in its box, the
hidden-object and material generations) changes: a selection click, a hover over a
handle, a settings change that is not the sun, or switching to solid and back draws no
shadow pass.

**Light entities.** `Document::scene_lights` reads point and spot lights into
backend-neutral `render_scene::SceneLight`s, sent through `SceneSink::set_lights` with the
sprites (so a selection click sends none). It understands the HERON_BRUSH/Godot keys of the bundled
definitions — `light_color` (0–255), `light_energy`, `omni_range` / `spot_range` in engine
units (× 32 map units per metre, the export's default scale), `omni_attenuation` /
`spot_attenuation` (Godot's distance exponent, 1 by default), `spot_angle` and
`spot_angle_attenuation`, `shadow_enabled` — and Quake / Half-Life ones: `light` as
brightness and reach in map units, `_color` (0–1 or 0–255), `_cone` / `_cone2`, `mangle`,
`angles`, `angle`, `pitch`. Missing keys take the HERON_BRUSH definition's defaults (energy 1,
5 m, shadows on); suns are left to the view's sun. Energy is scaled by π as engines do, so
energy 1 at a metre lights a white wall like the view's default sun does, and the falloff is
Godot's: `(1 − (d / range)⁴)² · d_metres^−decay`, with Godot's cone rim
`1 − ((1 − cos θ) / (1 − cos cutoff))^exponent`. Shading is the sun's Cook-Torrance with
the same energy compensation.

*Clusters* (Doom 2016 / Bevy, filled on the CPU): a 16 × 9 tile grid × 24 depth slices
spaced logarithmically from 16 units to 64 × the largest reach; each visible light's
view-space box gives a conservative range of tiles and slices, and two passes (count, then
fill after a prefix sum) build per-cluster lists (≤ 64 lights each) in vectors kept across
frames, uploaded as an (offset, count) table and an index list. The table holds up to
1,024 lights; the fragment shader loops over its cluster's list only, and skips the loop
entirely when no light is in view.

*Shadows*: the eight most important shadowed lights in view (luminance × falloff at the
camera's distance from the light's sphere) get a cube in one depth cube-map array,
allocated for as many as are needed (rounded up to a power of two); lights keep their slot
while they stay in the set. A face is drawn again only when the hash of (the light's
position and reach, the generation of each chunk its frustum covers, hidden objects,
materials) changes; spots skip faces their cone cannot reach. Faces use the cube-map
convention the hardware samples by (`lights::face_matrix`, tested against the sampling
rule), a near plane at 8 units so the light's own marker (inside its 16-unit entity box)
is not drawn into its cube, and no pancaking. Lookups: a normal offset of 1.5 texels at the
point's distance (scaled by the *Bias* preference) and eight hardware-PCF taps across the
direction. The *Entity lights* preference (`render/Entity lights`, wgpu builds only)
turns them off in the view without re-syncing anything.

**Contact shadows** (the GL settings: *Contact shadows*, *Contact distance*, *Contact
samples*; only with the sun's shadows, as in GL): in the forward pass, a ray from each
sun-lit surface towards the sun, marched in view space through the prepass depth; a sample
counts as blocked when the depth buffer holds a surface in front of it by more than the
thickness but less than 40 × it (GL's rule, against shadowing through walls). The thickness
grows with distance (0.25 units, or 0.2 % of the depth), and the start is jittered per
pixel for the temporal filter to soften.

**Reflections** (the GL settings: *Reflections*, *Distance*, *Samples*, *Strength*), GL's
scheme: the forward pass writes the weight the environment's specular was added at and the
roughness to a second target; a half-resolution pass marches the reflection through the
depth (the GL step count over the GL distance, then four halvings to the crossing) and
outputs what the geometry found minus the environment already reflected there, faded at
the window's edges and above roughness 0.55–0.9; the resolve (or the temporal filter)
adds it. Under the temporal filter rough surfaces scatter their rays by a GGX sample each
frame, so the history averages them into glossy reflections.

**Temporal antialiasing** (`taa.rs`, `shaders/taa.wgsl`; on when *Antialiasing* or
*Accumulate samples* is): every view frame is drawn with a Halton (2, 3) jitter (64-frame
period) and blended into a history reprojected by the camera's motion — unprojected with
this frame's jittered matrices at the nearest depth of the 3×3 neighbourhood, projected
with last frame's and this frame's unjittered ones — sampled Catmull-Rom (five bilinear
taps) and clipped towards the centre of the neighbourhood's variance box in a reversible
tone-mapped YCoCg space (Karis, Salvi). The current frame's weight is 1 / (samples + 1):
a moving camera keeps nine samples (0.1); a still one keeps adding, so the history is the
exact running average of jittered frames — GL's accumulation with every effect in it —
and after *Accumulation samples* (8 without accumulation) `Renderer::converged` is true
and the egui view stops drawing until the picture key changes. A change drawn under a
still camera keeps four samples, and its first frame repeats the last jitter, so when only
shading changed the prepass and GTAO are kept. With *Antialiasing* off a moving camera
draws plain frames. The overlay is drawn after the filter, unjittered.

*Moving objects* (the animation preview, a drag): the prepass writes each surface's motion
since last frame in world units, from the chunk's previous-position vertex buffer (see
*Object and material tables*), and the filter moves the reconstructed world position back
by it before projecting with last frame's matrix, so camera and object motion are both
exact. Beside the picture the history keeps, per pixel, the view depth of the surface it
holds and its sample count (`Rg16Float`, written by the filter as a second target). The
history counts only where it held this surface: of the four texels round the reprojected
spot the closest in depth must match the depth the surface had last frame (from the same
reprojection) within 0.5 % plus three quarters of the neighbourhood's depth range (a
pixel's worth of slope, so jitter and grazing floors pass). Where it does not — the floor
an object has just uncovered — the pixel starts afresh, and until it has three samples the
current frame is mixed towards its 3×3 tent-filtered neighbourhood, so a young history
shows neither the trail nor the raw one-slice occlusion and contact-shadow noise. Each
pixel blends at 1 / (its samples + 1), capped by the frame's count. Frames whose objects
moved (`RenderScene::moving`) keep sixteen samples and move on through the jitter
sequence; a change without motion vectors (an edit that changes the triangles) keeps four
and does not repeat the jitter; only a change of shading alone repeats it. The egui view
says `continuing` whenever its picture key did not change, animation frames included; the
renderer tells moving and changed geometry apart itself.

**Tone maps.** The *Tone mapping* preference (egui's Renderer page, wgpu builds only;
`render/Tone mapper` in the store) picks AgX (Blender 4's, as Benjamin Wrensch's polynomial
fit; the default), Khronos PBR Neutral, TonyMcMapface (Tomasz Stachowiak's 48³ LUT,
MIT/Apache-2.0, bundled as raw RGB9E5 in `crates/render_wgpu/assets`, 432 KB) or ACES
(the GL renderer's curve). The GL renderer always uses ACES; GL builds do not offer
the choice. Output is encoded with the sRGB curve (GL used a 2.2
power) and the blit decodes it exactly for sRGB surfaces.

**Offline renders** (the PNG render page): each egui frame draws one full frame at the
output size with the projection jittered by Halton (2, 3) and FXAA off, and averages it in
linear light into an `Rgba16Float` target (blend constant 1/n); depth of field is the GL
renderer's 32-tap gather in the resolve pass with its disc turned per sample, so it
converges to a smooth blur. The page uploads the document's clean render triangles (no
editor helpers) as one object into a renderer of its own.

## How egui integrates

- In the default build, `eframe::NativeOptions::renderer = Renderer::Wgpu`; the window, egui
  and the 3D view share eframe's `wgpu::Device` (`CreationContext::wgpu_render_state`).
  The device descriptor callback (`WgpuSetupCreateNew::device_descriptor`, wrapped in
  `main.rs`) requests `TIMESTAMP_QUERY` when the adapter has it, for per-pass GPU times
  while profiling; without it the renderer simply has no GPU timer. Indirect draws and
  bindless textures would request `MULTI_DRAW_INDIRECT(_COUNT)`,
  `INDIRECT_FIRST_INSTANCE`, `TEXTURE_BINDING_ARRAY` and `TEXTURE_COMPRESSION_BC` the
  same way.
- The renderer draws **into its own textures** during `View3d::show` (own encoder, own
  submit — wgpu orders it before egui's submission), then the view adds an
  `egui_wgpu::Callback` whose `CallbackTrait::paint` **blits** the output into egui's
  render pass inside the viewport egui sets for the rect (`render_wgpu::present::Blitter`,
  converting to linear when the surface is sRGB). Render-to-texture rather than drawing in
  egui's pass because the renderer needs many passes, its own depth and HDR targets, and
  the output doubles as the source for PNG export and the detached window.
- HUD text and the compass stay egui shapes (`EguiCanvas`), as on GL.

With `Renderer::Wgpu` there is no GL context, so everything that draws 3D goes through
`render_wgpu`: the main 3D view (`view3d.rs`; `Gpu` / `Backend` enums select the path),
the detached 3D window (it moves the same `View3d`), and the PNG render page
(`render_page.rs`, a renderer of its own, see *Offline renders*). Where the adapter
cannot run it (`view3d_wgpu::unsupported`: no storage buffers in the vertex stage, as on
GLES 3.0), the view says so instead of drawing.

## Not done yet

- **Android / GLES.** Storage buffers in the vertex stage are missing on GLES 3.0 and
  WebGL2, so the object and material tables need a uniform-buffer or data-texture variant
  chosen from `DownlevelCapabilities`; no compute means CPU culling (already the case);
  ETC2/ASTC textures; power-aware quality presets.
- **Fewer draws**: a shared mesh slab allocator, compute frustum + hierarchical-Z culling,
  multi-draw indirect, and bindless textures, together making the forward pass about one
  call per chunk. This is the main remaining cost on very large maps (see *Textures* and
  *Draw submission and culling*).
- BC/ASTC texture compression with a KTX2 disk cache.
- Per-object transforms, so the animation preview moves a matrix instead of re-sending
  the animated objects' triangles each frame.
- ORM / roughness / metallic / emissive maps (the material collection loads only albedo
  and normal maps; the material table has room for them).
- GPU picking from the id target (the CPU ray test is exact and fast enough so far),
  contact shadows for light entities, more than eight shadowed lights, auto-exposure, and
  reflection probes or irradiance volumes for interiors.
- The GL fallback restarts its accumulation every animation frame (it has no motion
  vectors) and uploads the whole mesh after every change.

## Profiling and benchmarks

`HERON_BRUSH_PROFILE=1` works for both backends (see `docs/performance.md`, *Profiling
the app*); with this renderer it also prints, for frames that changed the scene, how many
shadow cascades and cube faces were drawn and whether the environment was rebuilt, and
per-pass GPU times from timestamp queries (on Apple GPUs those overlap between passes and
do not add up to the frame). `HERON_BRUSH_ORBIT=<frames>` turns the camera and prints a
summary. The egui script's `shot` also writes the 3D view's own picture as
`<name>-3d.png`, which keeps working when a locked screen stalls window screenshots.

Headless tests in `crates/render_wgpu/tests/headless.rs` (all `--ignored`, release
builds, `--nocapture`):

```sh
# Cost per effect on one map, every effect on, the camera turning:
cargo test --release -p render_wgpu --test headless bench -- --ignored --nocapture

# A generated big map (sync, texture streaming, draws, CPU encode and frame times):
cargo test --release -p render_wgpu --test headless bench_big_maps -- --ignored --nocapture

# A picture of any map, for looking at:
RENDER_WGPU_MAP=demos/x/x.map RENDER_WGPU_SHOTS=dir \
    cargo test -p render_wgpu --test headless gallery -- --ignored
```

| variable | meaning |
|---|---|
| `RENDER_WGPU_MAP` | map for `bench` and `gallery` (default `demos/lantern_lane/lantern_lane.map`) |
| `RENDER_WGPU_YAW`, `_PITCH`, `_ZOOM` | camera for `gallery` |
| `RENDER_WGPU_SHOTS` | directory to write the tests' pictures to |
| `RENDER_WGPU_BIG` | brush count for `bench_big_maps` (default 50,000) |
| `RENDER_WGPU_TEXTURES` | texture count for `bench_big_maps` (default 2,000) |
| `RENDER_WGPU_SIZE` | output size for `bench_big_maps`, `WxH` (default `1400x900`) |
| `RENDER_WGPU_FRAMES` | orbit frames for `bench_big_maps` (default 90, more than 10) |
| `RENDER_WGPU_PROFILE` | `1`: per-pass GPU times in `bench_big_maps` (needs timestamp queries) |

## Representative numbers

Release builds on an Apple M-series laptop; the GPU clocks move absolute numbers by up to
2x between sessions, so read them as orders of magnitude.

On a ~1,000-object map at 1058×808 with everything on, a turning-camera frame costs about
2.6 ms including the GPU wait, against about 1.1 ms with no effects; shadows, ambient
occlusion and bloom are the largest parts (0.2–0.5 ms each), contact shadows,
reflections and the temporal filter under 0.15 ms each. A selection click redraws only
the shading (the prepass and GTAO are kept) and uploads one or two 16-byte object-table
slots; a converged still view draws nothing.

On a generated 50k-brush map (`bench_big_maps`, ~215 of ~800 chunks in view), a frame
issues about 8,000 draws with 64 materials and 9,700 over 2,000 textures; CPU encoding
takes about 10 / 15 ms and the frame including the GPU wait about 20 / 28 ms. About 70 %
of that CPU time is wgpu-core/Metal command encoding and bind-group changes, frustum
culling about 1 %. The first sync takes about 0.2 s; streaming 2,000 textures (752 MB with
mips) into view takes about 1.2 s over some 18 frames.

GPU memory at the default settings: shadow cascades 16 MB, environment about 1.5 MB,
light shadows 1.5 MB per shadowed light in view (at most 12.6 MB), and the effects' targets
about 52 bytes per pixel per view on top of the HDR, depth and id targets (normal, motion,
occlusion, reflectivity, half-resolution reflections, bloom chain and the temporal
history); textures are bounded by the *Texture memory*
preference.
