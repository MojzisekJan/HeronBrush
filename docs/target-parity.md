# Export target parity

Jan's priority on 13 October 2026 is to bring Bevy and Three.js up to the
raylib target, then Defold, Godot and Flax; Unity was added to the goal as well. Completion requires actual runtime
checks, not just preserving properties or compiling generated code.

The reference is `demos/motion_lab/motion_lab.map` and the raylib sample's
`anim-check`: ride both legs of the shuttle's L path, ride the six-metre lift,
stand on the rotor for ten turns without drift, walk and jump off platforms,
activate the door and lift through their exported pads, pass through the open
door and be blocked by the closed door. Also preserve the common demo's geometry,
materials, transforms, exact collision, properties and links. Door/portal gameplay
in the common demo remains part of the sample runtime, not the generic exporter.

## Current evidence

| Target | Proven | Scope / limits |
| --- | --- | --- |
| raylib | Native motion_lab check rerun successfully on Linux on 13 October | Common-demo doors/collider poses and both portal mouths now pass game-check; C reader limits remain documented |
| Bevy | Fresh exported GLB colliders in actual Avian: complete motion_lab checks; viewer compiles | Common-demo gameplay is separate from export parity |
| Three.js | Portable runtime matches 27 editor poses; Chromium HTML and independent importer motion_lab rides, rotor walking, jumps, both pads and open/closed door checks pass | Both browser entry points passed; FPS gameplay is separate |
| Godot | Existing motion_lab controller check rerun successfully on Linux, including ten rotor turns | Fresh export/import passed; FPS gameplay is separate |
| Defold | Fresh native motion_lab and common-demo door/portal checks | Capsule contact permits at most two airborne frames |
| Unity | Native motion_lab, 27 editor poses and playback controls | Common final/live native regression passed; renderer pipeline scope is documented |
| Flax | Fresh native PhysX motion_lab: animation, triggers, carrying, walking, jumps and door passage; 27 editor poses | Null renderer does not prove rendered materials |

Export a fresh fixture through the real editor MCP:

```sh
python3 examples/engine-checks/export_motion_lab.py bevy
python3 examples/engine-checks/export_motion_lab.py three
```

Exports default to `/tmp/heron-motion-<target>`. `--output` chooses another
project folder; `--binary` chooses the editor executable. The source map remains
unchanged. Short-lived runtime failures caused by a blocked display are not
target failures: the raylib test needs a working X11/OpenGL context.

## Three.js regression

```sh
cargo test --locked -p editor_core --lib portable_animation -- --test-threads=1
node examples/engine-checks/threejs/verify_animation.mjs /path/to/three.core.js
python3 examples/engine-checks/export_motion_lab.py three
node examples/engine-checks/threejs/verify_motion_lab.cjs /tmp/heron-motion-three/motion_lab.html
```

The browser check requires Playwright with Chromium. It enables the HTML's
opt-in `?verify` inspection, then steps the same capsule player and Octree as
interactive play at 60 Hz. The export currently loads Three.js 0.186.0 from
the CDN. Node's pose check should use that version too. Linux Chromium with
SwiftShader passed with shuttle/lift drift below floating-point precision,
rotor drift below 0.000001 m over 120 seconds, no airborne ride frames, open-door
passage to z=6.937 m and closed-door stop at z=5.750 m. This proves the motion
fixture; it does not prove portal, combat or enemy gameplay.

The moving Octree pads both sides of its bounds by 0.00001 m. Without that
broadphase padding the lift lost its top collision face at some heights and
the ride check reported 48 airborne frames. Static collision is retained;
only animated collision trees are rebuilt.

## Bevy regression

```sh
python3 examples/engine-checks/export_motion_lab.py bevy
cargo test --locked --manifest-path examples/bevy-motion-check/Cargo.toml
cargo test --locked --manifest-path examples/bevy-motion-check/Cargo.toml exported_motion_lab \
    -- --ignored --nocapture
```

`HERON_BEVY_MOTION` can select a different fixture folder. The ignored check
reads the actual exported GLB collision primitives and passes them through
the sample's Avian collider builders. It requires a fresh export, but no
window or renderer. The check passed: no airborne ride frames, six-metre shuttle/lift travel, rotor drift 0.00026 m, and open-door passage to z=6.7667 m versus a closed-door stop at z=5.7 m. Seven regular tests also passed; the full viewer passed `cargo check`.

## Flax regression

```sh
python3 examples/engine-checks/prepare_motion_lab.py flax
/path/to/FlaxEditor -headless -null -project /tmp/heron-flax-motion -exit -std -heroncheck -heronmotion
```

The importer attaches exported animation and volume trigger components. The
optional `HeronBrushMotionPlayer` drives the same components with a native
CharacterController and full support poses. The test uses this exported helper,
not a replacement physics controller. It passed on Flax 1.12.6912 with no
airborne ride frames, rotor drift 0.000108 m and open/closed door positions
z=7.4167/5.65 m. The Null renderer verifies native physics, not rendered materials.
The pose check consumes `/tmp/heron-portable-animation-check`, written by the
editor_core portable animation test above.

## Godot regression

```sh
python3 demos/motion_lab/build.py --export-only --binary target/debug/heron-brush
godot --headless --editor --path demos/motion_lab/godot --import
godot --headless --path demos/motion_lab/godot --fixed-fps 60 --script verify.gd
```

The import step is necessary after generating new painted texture files.

## Defold regression

```sh
python3 examples/engine-checks/prepare_motion_lab.py defold
# Build the prepared project with Bob, then run its game.projectc:
dmengine --config=check.enabled=3 --config=display.vsync=1 --config=display.update_frequency=60 game.projectc
```

The prepared project enables fixed physics steps. The generated animation and
sample player use `fixed_update` together; the test observes contact afterwards
in `late_update`. The player collision group is included on both sides of the
exported collision masks.

The complete Defold motion check passed on 13 October: shuttle horizontal drift
0.0334 m, lift vertical drift 0.01667 m with one airborne frame, rotor drift
0.008744 m with no airborne frames, walking/jumping and both pads. The original
common-demo door/portal regression also passed. Trigger activation uses entry
messages; Bullet also sends collision messages for sensors, and acting on both
caused repeated toggles that closed a door immediately.

## Reference game regression

```sh
make -C examples/engine-checks/raylib game-check RAYLIB=/path/to/raylib
```

This calls the actual raylib sample's systems without a renderer: both exported
portal mouths place the player at their destination and observe the cooldown;
all three ordinary doors open, move their exact collision, close and restore
that collision. It found and fixed the `wait=0` door case, which previously
never opened. The raylib portal sets destination yaw; preserving rotated
momentum/full look orientation is not claimed by this sample.

## Result

All seven targets passed the editor-authored motion/collision contract above.
The shared demo data check also passed (48 entities, 11 textures, 362 property
fields, eight links, 329 visible triangles, 207 static collision triangles and
19 exact hulls). This is export/runtime parity, not a port of all raylib FPS
combat, enemy or portal systems to every engine.

## Unity regression

```sh
cargo build --locked -p heron-brush
cargo test --locked -p editor_core --lib portable_animation -- --test-threads=1
python3 examples/engine-checks/prepare_motion_lab.py unity
/path/to/Unity -batchmode -nographics -projectPath /tmp/heron-unity-motion \
    -executeMethod VerifyMotion.Run -logFile /tmp/heron-unity-motion-runtime.log
```

Verified with Unity 6000.3.24f1 on Linux after activating the editor license.
The native CharacterController and PhysX check passed: no airborne ride frames,
shuttle drift zero, lift vertical drift below 0.000001 m, rotor drift 0.00002479 m,
walking and jumping, both pads and open/closed doorway positions z=7.416667/5.64 m.
The same process compares 27 poses and all orientation axes with editor output.
`HeronBrushMotionPlayer` is an optional exported helper with one shared manual
animation/trigger clock per level. It carries the capsule using full support
poses, including rotation, and disposes back to automatic component updates.

The native test exposed two issues that compilation did not: a skin-width gap
losing floor contact on the descending lift, and inward-facing static collider
triangles that let the player fall through the floor. The walker maintains a
downward grounded bias. The collision helper now reflects X without reversing
the already clockwise source triangles again. Its asset hash includes a schema
version so reimports do not silently reuse the earlier inward-facing meshes.
The same collision helper is used by the final and live Unity importers.

The Unity common export/live check also passed after the winding correction:
19/19 material slots retained textures, 20 native colliders, valid ray hits,
move/delete/undo snapshots and persisted scene collision assets.

## Cocos Creator installation check

Checked on Linux on 13 October 2026. The local download
`CocosDashboard-v2.2.1-win-112616.exe` is a Windows PE executable. The two
files extracted into `~/Downloads/cocos` are Windows payload/certificate data,
not an installed Linux editor. The separate Cocos2d-x 4.0 source checkout is
a different product and cannot validate the Cocos Creator 3 importer.

The official [Creator installation documentation](https://docs.cocos.com/creator/3.8/manual/en/getting-started/install/)
lists Windows and macOS Dashboard environments and installers. No supported
native Linux Creator installation was found. The
[official download page](https://www.cocos.com/en/creator-download) mentions
Linux in engine runtime changes; that does not establish Linux editor support.
Cocos remains an export implementation without a native Creator verification
on this machine. Verify it using Creator on Windows/macOS; Wine is a separate,
unverified compatibility route, not evidence of native Linux support.

Update, 18 October 2026: Cocos Creator 3.8.8 on macOS built and ran the export
natively (web-desktop); the target was reworked in the process. See
[Cocos Creator export](cocos-export.md).

## General project integrations, 14 October 2026

The seven available native targets now have independent non-FPS fixtures.
Profiles and class bindings belong to `.heron-brush/exports.json`, with paths
relative to the project root and authored values overriding typed defaults.

| Target | Independent project proof | Native integration choice |
| --- | --- | --- |
| Godot 4.7.1 | Puzzle PackedScene, exported boolean/array values, signal and forward NodePath, floor collision | Class build rules select scenes, scripts, native node type, collision, groups and signals |
| Defold 1.13.2 | Project script, `.go`, number/bool/vector/URL/quaternion overrides, custom physics groups/mass and falling body | Native game object/component resources and collision/material hints |
| Three.js 0.186.0 | Project factories, typed properties, forward references, game-driven collision and parent transform, isolated copies and cleanup | Callback registry creates arbitrary native objects; game selects blocking shapes and owns physics |
| Bevy 0.19/Avian 0.7 | Project ECS bundles, typed arrays, actions, two copies with scoped links, dynamic exported GLB hull on floor | Factory registry inserts native project components/bundles |
| Unity 6000.3.24f1 | Native prefab, project component fields/actions, references, dynamic body; two exports, persisted transactional reexport | Native prefab and field bindings, optional body/mass hints; final and live custom projects passed |
| Flax 1.12.6912 | Native prefab/script fields/actions, references and dynamic hull; two exports and persisted reexport | Native prefab/script/field bindings and project callback; final and live custom projects passed |
| raylib 6 | Own C components/actions, type/value reading, scoped copies, moving convex collider and falling sphere | Full JSON plus compact text; physics/gameplay is project code |

Unity/Flax final importers preserve unrelated same-name objects and placement.
Their generated class namespaces are scoped per export. Missing resources leave
the prior map intact. Live tests also verified retry after a missing prefab.
Unity Editor, Hub and licensing helper processes were closed after verification.

These are engine-appropriate integration choices, not a required Heron Brush
game schema. They do not make every native engine API identical. Geometry
formats remain asset interchange: glTF/GLB now retain typed project defaults in
versioned `extras.heron` while preserving authored `extras.entity`; OBJ carries
geometry/materials. They do not generate gameplay or physics components.

Unreal and Cocos Creator have no available supported native editor here and
remain outside this machine's native completion evidence. Cocos2d-x is a
different product and does not validate the Creator target.

### Stride 4.4 preparation, 14 October 2026

Stride writes native model, material and sky assets, Bullet colliders, animation
and triggers, verified natively in Stride 4.4 on macOS; see
[Stride export](stride-export.md). Windows and Linux runs remain.


## Water adapters

Water now has its own explicit completion scope rather than being inferred from
motion_lab parity. Godot, Bevy, Three.js, Defold, Unity, Flax, Stride, Cocos Creator,
GDevelop and raylib share optional water
volumes, swimming parameters and baseline surface/view adapters. Native Godot
4.5 (rendered, Metal), Stride, Cocos, GDevelop and raylib (rendered), Avian, Defold/Bullet,
Unity and Flax/PhysX checks and both actual Three.js browser entry
points passed. See [water export](water-export.md) for reproduction and limits.
Unity material state/shader checks ran without graphics; Flax rendering remains
unverified because its installed Vulkan shader include setup fails. These checks
do not imply parity with Godot Lantern Lane's depth-aware water shader.
