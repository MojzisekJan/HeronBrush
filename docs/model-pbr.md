# PBR materials from GLB models

Static GLB imports retain the core glTF metallic-roughness material:

- Roughness and metallic factors and their shared G/B texture.
- Tangent-space normal map and normal scale.
- Occlusion R texture and occlusion strength.
- Emissive RGB texture and linear emissive factor.

Base colour and emission images use sRGB. Normal, metallic-roughness and occlusion
images are linear data. Base-colour factors are multiplied in linear light before
the editor stores the tinted skin. Each map keeps its own dimensions. Source
textures use TEXCOORD_0; texture transforms and alternate UV sets are rejected.
Sampler/filter settings currently use the editor's repeating defaults.
These channel and colour-space rules follow the
[glTF specification](https://github.com/KhronosGroup/glTF/blob/main/specification/2.0/Specification.adoc).

Both the OpenGL and wgpu renderers show these materials. The preview packs AO and
metallic-roughness into a linear ORM image, capped at 4096 pixels per dimension;
different source dimensions use nearest resampling during this packing. Emission
and normals remain separate. AO affects indirect light. The wgpu texture budget
includes the extra images and their mip levels. Source PBR images are shared by
transformed model copies, and material hashes distinguish different PBR data even
when the base-colour images match.

## Export behaviour

Portable glTF/GLB and the common engine GLB writer preserve original PBR map pixels,
dimensions and factors. They generate tangents for normal-mapped primitives. The
original maps are exported instead of the preview ORM packing. Repeated models
retain the shared-geometry export path.

Godot receives generated companion PNG files and StandardMaterial3D properties.
Roughness uses G, metallic uses B and AO uses R. Godot's emissive colour is converted
to sRGB at the resource boundary, matching its native GLB importer. Since this
material path has no independent glTF AO-strength scalar, the Godot AO companion
image bakes that strength into R. The source and other exports stay unchanged.

The standalone OBJ writer does not carry these PBR maps into runtime materials.
Unity's OBJ geometry export now has a companion material adapter. It converts
MR.B times metallic into Unity's metallic R channel and one minus MR.G times
roughness into smoothness A. AO.R and its strength become Unity occlusion G.
Normal-map type/scale and emissive RGB/factor are assigned separately. Data maps
are imported linearly without compression or NPOT resizing. The adapter preserves
material asset GUIDs on rebuild. Source PNGs remain unchanged.

The Unity adapter selects Standard for the built-in pipeline and URP Lit for URP.
Other pipelines require explicit project materials. Native Unity 6000.3.24f1
checks cover Standard material state, map pixels/dimensions, shader keywords,
rebuild and scene save/reopen in batch/nographics mode. They do not prove rendered
appearance or URP support in a running project. Godot and Three.js retain their
separate native/browser material checks. Other engines need their own checks.

OBJ MTL PBR extensions are not imported. Material extensions such as clearcoat,
transmission, anisotropy and emissive strength are not implemented. Required glTF
extensions remain rejected. Source tangent attributes are recomputed from geometry
and UVs. Alpha-mode and double-sided material fidelity still need separate work.

## Local checks

`examples/editor-checks/fixtures/pbr.glb` is a generated 2.5 kB asset. It contains
two materials with equal albedo and different roughness, independent map sizes and
distinct channel values. Regenerate it without downloads:

```sh
python3 examples/editor-checks/make_pbr_fixture.py examples/editor-checks/fixtures/pbr.glb
```

The `pbr_maps_survive_preview_and_all_glb_export_paths` core test verifies preview
materials, all 13 export targets and exact PBR round trips for generated GLBs. To
write the fixture exports to a temporary directory:

```sh
HERON_PBR_OUTPUT=/tmp/heron-pbr-check cargo test -p editor_core --lib --offline pbr_maps_survive
cp examples/editor-checks/verify_pbr.gd /tmp/heron-pbr-check/Godot/
godot --headless --editor --path /tmp/heron-pbr-check/Godot --import --quit
godot --headless --path /tmp/heron-pbr-check/Godot --script verify_pbr.gd
```

Use a fresh output directory to avoid stale assets. Isolate XDG config/data paths
when running native Godot checks alongside a working editor. The script compares
factors with Godot's original GLB import and checks map dimensions, channels and
the baked AO companion.

For Three.js, serve the same fixture using existing vendored libraries:

```sh
python3 examples/editor-checks/serve_pbr_check.py /tmp/heron-pbr-check
```

Open the printed localhost URL. The page checks portable and engine GLBs, map
colour spaces, factors, texture dimensions and generated tangents, then renders a
preview. It makes no external module requests. Stop the server after the check.

The wgpu headless test `imported_pbr_maps_change_the_rendered_material` turns off
normal influence, metallic-roughness, AO and emission separately and requires a
visible image difference for each. A GPU or software adapter is required; the test
does not silently skip this check when no adapter is available.

## Unity material check without package downloads

Use a new output directory for fixture exports and a new Unity project directory:

```sh
HERON_PBR_OUTPUT=/tmp/heron-pbr-check cargo test -p editor_core --lib --offline pbr_maps_survive
python3 examples/editor-checks/prepare_unity_pbr.py /tmp/heron-pbr-check/Unity /tmp/heron-unity-pbr
/path/to/Unity -batchmode -nographics -projectPath /tmp/heron-unity-pbr -executeMethod VerifyHeronPbr.Run -logFile /tmp/heron-unity-pbr.log
```

The project manifest uses only built-in physics, IMGUI, JSON and image-conversion
modules. A successful test logs HERON_PBR_PASS and exits Unity. Do not leave Unity
or Unity Hub running after native checks on this machine.

## Alpha and culling reference fixture

Generate the next material-fidelity fixture without downloads:

```sh
python3 examples/editor-checks/make_pbr_fixture.py --surface-modes /tmp/heron-material-modes.glb
```

It places six triangles side by side. They share a 2x2 albedo alpha gradient
with values 0, 96, 192 and 255 and a base-color alpha factor of 0.75. Each
OPAQUE, MASK and BLEND mode has single-sided and double-sided variants. MASK
uses an explicit cutoff of 0.42. View both front and back, and compare coverage
against the source glTF material. OPAQUE must ignore texture/factor alpha, MASK
must use the cutoff after the factor, and BLEND must retain continuous alpha.
Double-sided materials must remain visible from the back.

The existing default fixture remains byte-identical when this option is omitted.
The generated GLB structure and all six mode/culling combinations were checked
locally. ModelPbr retains source alpha mode, cutoff and double-sidedness,
including them in its material hash so equal texture pixels with different
rules remain distinct. GeneratedPbr now carries those rules to engine GLBs,
and the separate portable GLB writer also honors them. Explicit OPAQUE can
therefore retain alpha-bearing texture pixels without becoming BLEND.
Six GLB import regression tests pass, including invalid mode/flag/negative
cutoff rejection and valid cutoffs greater than one. The
`material_modes_survive_all_generated_glb_paths` test runs all export targets
and reimports every emitted GLB, requiring exact source PBR/rule equality and
unchanged albedo pixels for all six materials. It passes, as does the original
PBR map/factor regression. This validates emitted assets, not runtime rendering.
Editor preview pipelines and rendered fidelity remain pending. Unity
material-state verification is recorded below. The Godot scene adapter now writes
explicit transparency, mask cutoff and culling from the imported rules.

The source fixture also passed native Godot 4.7.1 import and
`verify_material_modes_source.gd` in headless mode. This checks all six material
names, transparency modes, culling, cutoff, alpha factor and decoded alpha
gradient. Godot's GLB importer maps BLEND to alpha with a depth prepass. This
is source-reference validation, not a Heron export or rendered-image check.

To reproduce, create a disposable Godot project with the generated GLB named
`source.glb`, copy `examples/editor-checks/verify_material_modes_source.gd` into
it, run `godot --headless --editor --path PROJECT --import --quit`, then
`godot --headless --path PROJECT --script verify_material_modes_source.gd`.
Use isolated XDG directories alongside a running editor. The local result is
recorded in `/tmp/heron-material-modes-source-check.log`.

## Godot material-mode export check

The generated Godot scene passed native Godot 4.7.1 headless import and
`verify_material_modes.gd`. Its six materials match the source GLB's native
transparency and culling modes, mask cutoff, texture dimensions and effective
per-pixel alpha. The check permits one 8-bit step for baking the source alpha
factor into the exported image. This is material-state validation, not an image
render or a test of alpha sorting under gameplay.

```sh
HERON_MATERIAL_MODES_OUTPUT=/tmp/heron-material-modes-export cargo test -p editor_core --lib --offline material_modes_survive_all_generated_glb_paths
cp examples/editor-checks/verify_material_modes.gd /tmp/heron-material-modes-export/Godot/
godot --headless --editor --path /tmp/heron-material-modes-export/Godot --import --quit
godot --headless --path /tmp/heron-material-modes-export/Godot --script verify_material_modes.gd
```

Use fresh output and isolated XDG directories. Local evidence is
`/tmp/heron-material-modes-godot-check.log`.

## Unity alpha-mode check

The Unity PBR adapter now distinguishes OPAQUE, MASK and BLEND, rather than
selecting blending from image pixels. It sets cutoff, shader keywords, depth
writes, render tags and queues explicitly, clearing incompatible keywords on
rebuild. The Standard material path passed native Unity 6000.3.24f1 checks
for all six fixture materials and a second level build. Expected per-material
rules come from the independently checked Bevy GLB, not the generated C# code.
The test uses the shader's effective default queue for opaque materials.

```sh
HERON_MATERIAL_MODES_OUTPUT=/tmp/heron-unity-material-modes cargo test -p editor_core --lib --offline material_modes_survive_all_generated_glb_paths
python3 examples/editor-checks/prepare_unity_material_modes.py /tmp/heron-unity-material-modes /tmp/heron-unity-modes-check
/path/to/Unity -batchmode -nographics -projectPath /tmp/heron-unity-modes-check -executeMethod VerifyHeronMaterialModes.Run -logFile /tmp/heron-unity-material-modes-native.log
```

Use a fresh project directory. Preparation uses only built-in Unity modules,
with no package downloads. Native evidence is
`/tmp/heron-unity-material-modes-native-final.log`. Unity exited after the check.
This verifies Standard material state, not a rendered image. URP is not natively
verified here. Standard does not expose a configurable `_Cull` property. Double-sided
built-in materials now select the exported `Heron Brush/Standard Two Sided`
shader, which uses Unity Standard surface lighting, exposes culling and flips
the shading normal on back faces. It retains MR/smoothness, normal, AO and
emission inputs and alpha modes. The shader file is emitted only when needed.
URP materials receive their supported `_Cull` setting directly.

The extended native check passed shader import/error status, selection of the
two-sided shader for all three double-sided fixture materials, `_Cull = 0`,
double-sided GI and a second build, in addition to the alpha checks above.
Evidence is `/tmp/heron-unity-sidedness-native.log`. It ran with `-nographics`,
so that run did not establish rendered coverage. The separate graphics check
below covers front/back alpha coverage. Back-face normal lighting and other GPU
shader variants still require validation. URP remains unverified.

## Unity rendered front/back coverage

`VerifyHeronMaterialModes.RunRendered` runs the same prepared project with a
graphics device, using Camera.Render into a linear 128x128 texture. It renders
each generated material on the front and back of a separate probe quad. White
emission isolates coverage from lighting; this does not validate PBR lighting
or back-face normal orientation. Black background exposes unwanted coverage.

On local Unity 6000.3.24f1, OpenGL Core, AMD Radeon 610M, the six cases passed.
Single-sided back faces had zero coverage. Double-sided front/back mean
coverage matched within 0.02. OPAQUE covered 0.635 of the image, MASK about
0.296 and BLEND about 0.270. Opaque coverage stayed solid despite alpha-bearing
source pixels. The test also rejects intermediate intensities for opaque and
masked output and requires them for blended output. PNG probes are saved in
the disposable project's Assets directory for inspection.

Run Unity with `-batchmode -force-glcore` and
`-executeMethod VerifyHeronMaterialModes.RunRendered`; omit `-nographics`.
The test explicitly fails on a Null graphics device and checks shader errors
after rendering. The initial geometric probe was corrected to use a two-unit
quad before these results were collected.

Final coverage evidence is `/tmp/heron-unity-material-render-coverage.log`.
All opaque/mask intermediate-pixel ratios were zero; Blend was 0.63055.
Unity exited after the test.

## Editor material classification

The shared material classifier now gives an imported ModelPbr alpha mode
precedence over cached pixel classification. An OPAQUE model therefore stays
in the opaque draw class even if its image contains alpha, and MASK stays
masked instead of being mistaken for BLEND. Explicit source rules are known
without decoding a lazy image. The regression covers all three modes with
partial-alpha pixels and an unloaded image. All ten material tests pass.
The subsequent cutoff work below also fixes shader propagation and the
asynchronous decode path. Front/back culling and normal propagation still need
implementation and rendered checks in the editor.

## Editor mask cutoff and material refresh

WGPU material entries now carry source alpha cutoff in an existing reserved
parameter. The forward, depth/normal prepass and masked shadow shader use that
value. Loose textures retain cutoff 0.5. OpenGL's color shader also receives a
per-material cutoff; its rendered behavior is not yet verified here.

The GPU regression imports the six-mode GLB, changes only its two MASK
materials between cutoffs 0 and 1, loads textures and requires a visible
readback difference. It passed on the local adapter. The initial failing test
also exposed two bugs fixed with this change: asynchronous pixel decoding
overrode explicit source alpha modes, and replacing materials reset slot IDs
still referenced by existing vertices. Replacement now preserves those slots
and refreshes entries and draw grouping. Test scene changes complete the scene
sync before loading textures.

The original PBR image-difference check and texture streaming/eviction check
both pass after this fix. Evidence: `/tmp/heron-preview-mask-cutoff-gpu.log`,
`/tmp/heron-preview-pbr-refresh-gpu.log` and
`/tmp/heron-preview-texture-residency-gpu.log`. The color comparison proves
cutoff affects visible coverage; it does not independently measure the shadow
silhouette. Preview culling and back-face normal orientation remain open.

## WGPU editor face rules

WGPU material entries distinguish ordinary editor surfaces from imported
single-sided and double-sided models. Color and depth/normal passes discard
imported single-sided back faces. Double-sided imported shading reverses the
complete perturbed normal on back faces; ordinary brush/patch behavior remains
unchanged. Masked and opaque sun/light shadow passes also receive face rules.
Imported single-sided opaque shadow casters use a minimal fragment stage for
this per-material decision. Ordinary/double-sided-only chunks retain a
vertex-only bulk depth pass; mixed-chunk cost still needs measurement.

`imported_face_rules_control_front_and_back_coverage` passed on the local GPU.
It clears selection overlays, uses emissive imported surfaces against black,
and renders from both sides. Single-sided front coverage must be present and
back coverage zero. Both double-sided views must be present and agree within
20 percent. This isolates visibility from lighting and does not establish
normal-map lighting fidelity. The prior cutoff coverage test and the light
entity/shadow-cache regression also pass. Evidence is
`/tmp/heron-preview-face-rules-gpu.log`,
`/tmp/heron-preview-sidedness-shader-check.log` and
`/tmp/heron-preview-face-shadow-regression.log`.

Remaining preview checks include back-face normal-map lighting, an independent
front/back shadow silhouette comparison, OpenGL face-rule propagation and
rendering, and measured shadow-pass cost on large maps.

## OpenGL editor face rules

The OpenGL color and normal passes now receive the imported face rule alongside
normal-map settings. Single-sided back faces are discarded. Double-sided
imported normals reverse after normal-map evaluation, so their tangent
perturbation reverses too; ordinary editor surfaces keep their prior behavior.

Run `python3 scripts/tests/check_gl_material_shaders.py`. It uses only Python's
standard library and installed libEGL/libGL to create a surfaceless core context,
compile and link the actual production scene/normal shader pairs, and render
normal-pass triangles. It requires single-sided front coverage, absent
single-sided back coverage, both double-sided faces and complementary encoded
front/back normal vectors, both with and without a normal map. Missing EGL/GL
support fails rather than silently skipping.

The check passed locally on llvmpipe, OpenGL 4.5 Core, Mesa 25.2.8. The render
crate also passed its offline cargo check. Evidence is
`/tmp/heron-gl-face-rules-native.log` and
`/tmp/heron-gl-face-rules-check.log`. This proves the normal-pass pixels and
scene shader linkage, not a full editor color render or shadow silhouette.
OpenGL color/shadow image comparisons and WGPU back-face lighting remain open.

The same EGL script now also renders the production color shader. With a
constant quarter-alpha albedo it requires OPAQUE to keep full coverage, MASK
to retain pixels at cutoff 0.2 and reject them at 0.3, and BLEND to preserve
continuous output alpha. It also enables actual framebuffer blending and
requires a partial color contribution against black. Single-sided backs
must remain clear and double-sided backs must retain coverage. These checks
passed on the same llvmpipe adapter; evidence is
`/tmp/heron-gl-material-color-native.log`. Flat shading isolates material
coverage from PBR lighting. OpenGL shadow silhouettes and full lighting
comparisons remain unverified.

## OpenGL shadow material rules

The GL renderer keeps its original single-draw depth shader when all batches
are ordinary opaque surfaces. Maps with masks or imported face rules use a
separate material-aware depth program. It samples alpha only for masked
materials, applies source cutoff and single-sided rejection, and omits blended
and skip-preview batches. Ordinary opaque brush batches retain their previous
back-face shadow bias strategy; imported single-sided batches cast from their
front and double-sided batches from both faces.

The EGL test now creates a real depth attachment and reads its values. It
requires opaque/retained-mask samples to write depth 0.5 and mask holes,
single-sided backs and blended samples to leave depth 1.0. A double-sided back
must write depth. The original fast depth shader is also compiled and rendered.
All these tests, the earlier color/normal tests and the render crate cargo check
pass. Evidence is `/tmp/heron-gl-mask-shadow-native.log` and
`/tmp/heron-gl-mask-shadow-check.log`. This verifies production shader depth
behavior on llvmpipe, not a complete editor screenshot with a shadow receiver.
The material-aware path's large-map draw-call cost still needs measurement.

## Opaque WGPU shadow material slots

A full receiver-image check exposed an additional opaque-shadow issue: bulk
chunk draws used material instance zero, so a fragment face-rule test could not
identify an imported single-sided caster. Such chunks now issue opaque draws
with their actual material slots and a face-aware depth pipeline. Chunks that
contain no single-sided opaque models use the original vertex-only, one-draw
shadow path. This decision is cached when the chunk is rebuilt rather than
rescanning its draws in each cascade/frame. Both sun and cube submission paths
use the corrected selection.

`imported_single_sided_back_does_not_cast_a_sun_shadow` places a downward-facing
imported model above a floor. It compares shadows-on/off receiver images for
single-sided and double-sided source materials and requires the latter to add
a visible shadow. It passes. A temporary negative control restored the old
bulk submission and the test failed as expected, with 457 changed receiver
pixels for both variants. The fixed source was restored before final tests.
The pre-existing light/shadow-cache regression also passed.

The software-material runner now includes four WGPU image tests. Full local
software GL/WGPU evidence is `/tmp/heron-shadow-materials-ci-final.log`, with
negative-control evidence in `/tmp/heron-shadow-grouping-negative-control.log`.
This proves the sun receiver regression and preserves the fast-path structure;
it does not quantify performance or independently verify cube face silhouettes.

## Opaque WGPU prepass face rules

An opaque imported material without a normal map must still retain its material
slot in the depth/normal/id prepass. Previously those draws were merged under
slot zero, losing source sidedness. The forward pass discarded the back face,
but its prepass depth could leave a hole in a receiver behind it. Imported face
rules now prevent that merge. Ordinary opaque brush surfaces still share the
material-independent prepass range.

`imported_opaque_back_does_not_occlude_the_receiver` compares a plain receiver
with the same receiver behind a single-sided or double-sided opaque model.
Normal maps are deliberately absent. Before the fix both variants changed
1,122 receiver pixels and the test failed. After the fix all fourteen regular
headless GPU tests pass on the native AMD Vulkan adapter, including this image
regression. Evidence: `/tmp/heron-prepass-negative.log` and
`/tmp/heron-prepass-fixed-regressions.log`. The automatic `imported_` CI filter
also includes this new fifth material image test. This test verifies occlusion,
not back-face normal-map lighting.

The shared software-material command also passed production OpenGL checks and
all five WGPU image tests on required llvmpipe Vulkan, with no skipped tests or
downloads. Evidence: `/tmp/heron-prepass-software-ci.log`. Remote CI execution
remains unverified.
