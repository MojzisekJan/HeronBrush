# Contract for integrating exported levels into a game

The exporter builds level data. Engine adapters turn that data into meshes,
materials, scene nodes and collision shapes. Project-owned scripts or systems
implement gameplay. The FPS demo is an integration test of these boundaries;
it is not the specification of every game built with this editor.

## Data to preserve

- Geometry, surface material assignments, UVs, solid face colours and gradients.
- Full transforms, including pitch, yaw, roll and nonuniform scale. Brush
  geometry may already contain its authored transform; do not apply it twice.
- Collision geometry and the distinction between blocking shapes and sensors.
  Concave static meshes and convex moving shapes are different. Bounds are a
  fallback for a consumer that cannot use a hull, not an equivalent shape.
- Entity class, name, typed properties, arbitrary custom strings, tags and groups.
- Resolved target references. Consumers should create the entities before wiring
  references, so forward references work.

The neutral JSON format is currently version 2. Rotation arrays are `[x,y,z]`
in radians, applied in YXZ order. The mesh and transform frame is right-handed,
Y-up, measured in metres when scale is 32 map units/metre. Each native adapter
must convert the coordinate system once and preserve the complete orientation.
Native lights and camera/player conventions may need a local basis adjustment.

Engine GLBs embed available PNG/JPEG albedo images, including baked solid and
gradient paint. Ordinary texture coordinates repeat, while generated paint
clamps. External native materials remain external. Other image formats produce
a warning when they cannot be embedded; the current conversion does not promise
complete normal, roughness or emission texture transfer.

Unknown gameplay properties retain their original values. For example, the
legacy FPS kit uses `travel=96` in map units; its game code converts that to
three metres at scale 32. The exporter cannot safely infer units from an
arbitrary property name. Document units in the game's entity definitions.

## Configuration belongs to the project

Use a project FGD to define the entity classes and property types. Godot projects
can supply their own scene/script mappings through FGD annotations. Other games
can map classnames and custom properties in their importer or ECS systems.
A reference to a Godot `.tscn` or `.gd` is metadata in another engine, not code
that that engine can execute.

Export profiles provide scale, project/output directories, texture/material
search locations, collision/optimization options, tool-texture names, naming
and grouping. Engine-specific fields include Defold material/sampler/resource
paths and Unreal/Flax content paths. MCP must expose the same implemented
options as the exporter; this is covered by its schema regression test.
For Godot, use `res://textures` and `res://materials` when the paths refer to the
project root. Bare relative paths are resolved from the export directory.

Keep game code separate from generated meshes and scene data. A re-export must
not require editing generated gameplay code. The checked-in examples document
which files are generated and which belong to the sample game. Raylib also writes
a full JSON sidecar because its compact text format flattens arrays and newlines.
Defold keeps full JSON beside its narrower `go.property` representation.
Custom importers
should update their own generated root and preserve unrelated game content.

Each solid's `.glb` holds its visual mesh as glTF mesh 0, normally one primitive
per texture. Visual index lists are 16-bit (never the restart value 65535). A
texture with more than 65,535 vertices becomes several primitives sharing one
material, because raylib 5.5 and 6.0 store indices as 16-bit and truncate wider
ones. Map textures through each entity's `surfaces[].primitive`, which lists every
primitive; do not assume one primitive per texture. Collision meshes are outside
the glTF scene and may use 32-bit indices.

## What a successful test proves

A parsed export alone does not prove that an engine can import or draw it.
A rendered screenshot does not prove physics or gameplay. Test separately:

1. Data contract: files exist, references resolve, properties/types survive,
   primitive indices and collision shapes are valid.
2. Native import and rendering, including painted faces and all image resources.
3. Game-side integration: physics sensors, movers, portal wiring or other
   systems provided by the test game's code.
4. Re-export and configuration changes in a separate project, including different
   resource paths and units. Windows/macOS remain separate native platform tests.

See [the Bevy example](../demos/bevy_fps/README.md) and the engine checks in
`examples/engine-checks/` for reproducible results and explicit limitations.
These checks do not imply that every engine target has identical capabilities.

## Project profiles and class bindings, 14 October 2026

Saved maps use the nearest project `.heron-brush/exports.json`. Paths are relative
 to the profile's project root, so moving the whole project preserves the export
 destinations. Unsaved maps retain the legacy global preferences. Saved maps
 without a profile start from defaults, avoiding output into a different game's
 directory. The Export window's Entity bindings editor and MCP use this same
 profile; per-call MCP overrides do not change it.

```json
{
  "version": 1,
  "targets": {
    "godot": {
      "project_dir": ".",
      "directory": "maps",
      "entities": {
        "beacon": {
          "build": {"scene": "res://beacon.tscn", "auto": "true"},
          "defaults": {"charge": 9, "armed": true, "sequence": [2, 3, 5]}
        }
      }
    }
  }
}
```

Profiles accept partial target options. Class bindings supplement a project's
FGD or define classes without one. Existing FGD property types stay authoritative;
authored map values override defaults. Strings, numbers, booleans and arrays
are supported. Structured project data can be represented by a JSON string.
Unknown build rules, malformed values and unreadable profiles produce an error.

The Godot custom-game fixture verified an owned PackedScene, charge 7 overriding
9, native boolean/array exports, a forward NodePath reference, native Area3D
body_entered signal and CharacterBody3D floor collision. A PackedScene can receive
linked signals even when the class binding does not specify a separate script.

### Three.js project factories and collider updates

`loadLevel(url, {factories, blocks})` accepts project callbacks selected by
`three_factory` or classname. A callback receives the typed entity and a context
with `root`, `entities`, `byName`, `colliderParts` and the original level data.
All entity roots exist before callbacks run. The game owns its native objects,
components and gameplay; the importer does not require an FPS schema.

`verify_custom_game.cjs` runs a separate editor-exported puzzle in Chromium. It
checks typed defaults, authored overrides, forward references, custom geometry
and actions, project-driven moving brush collision, a transformed level parent,
and loading another copy without changing the first one. It uses local Three.js
0.186.0 modules. `advance` refreshes collision poses for game-owned movement too.

### Babylon.js project factories

`loadLevel(BABYLON, scene, url, {factories})` in the exported `heron_babylon.js`
calls a project callback chosen by `babylon_factory` or classname with the entity's
record and the level, once every entity's node exists and every target is resolved
to a record. A record carries the typed `properties`, the same as plain `values`, the
node, its meshes and its Havok body. The scene is switched to Babylon's right-handed
mode, which is the level's frame, so nothing is converted. The feature lab's run in
Chromium checks typed values, the authored override of a class default, and the
links; see [Babylon.js export](babylon-export.md) and
[its run-time checks](feature-runtime-babylon.md).

### PlayCanvas project factories

`loadLevel(pc, app, url, {factories})` in the exported `heron_playcanvas.js` calls a
project callback chosen by `playcanvas_factory` or classname with the entity's record
and the level, once every entity exists and every target is resolved to a record. A
record carries the typed `properties`, the same as plain `values`, the entity, its
mesh instances and, on the entity, its rigidbody and collision components.
PlayCanvas's frame is the level's, so nothing is converted. The feature lab's run in
Chromium checks typed values, the authored override of a class default, and the
links; see [PlayCanvas export](playcanvas-export.md) and
[its run-time checks](feature-runtime-playcanvas.md).

### Bevy project factories

Bevy exports include `heron_entities.rs`. Attach `HeronEntityData::from_json` to
each imported root, register callbacks in `HeronEntityFactories`, and configure
after creating all roots. Callbacks receive a native World/Entity, typed
properties and a name-to-Entity table, and insert arbitrary project bundles.
`bevy_factory` selects a callback; otherwise the classname does.

Use `HeronEntityScope(level_root)` when loading several maps or copies to keep
references within one level. The plugin configures newly loaded entities once;
remove `HeronEntityConfigured` to request explicit reconfiguration. It avoids
cloning property data on every frame after configuration.

The custom-game fixture passed with native Avian physics: typed boolean/array
properties, an authored integer override, project component actions, two copies
with scoped forward references, and an exported dynamic convex GLB settling on
the exported floor. The viewer also preserves a root for metadata-only solids.

### Unity project prefabs and persistent ownership

`unity_prefab` selects a native `Assets/*.prefab`. The importer keeps the prefab
connection and applies typed values directly to project-owned component fields
and writable properties. Matching property names can be used automatically;
`unity_property_<Component>__<field>` selects a specific component and rejects
a missing field. Numbers, booleans, strings, arrays/lists, vectors, colours,
quaternions, enums and native object/component references are converted to the
declared field type. Reference strings name entities within the imported map or
Unity asset paths. Arbitrary gameplay units remain the project's responsibility.

`unity_body_type` selects static/kinematic/dynamic; `unity_mass` configures a
positive native Rigidbody mass. Dynamic exported geometry needs convex shapes
and cannot also use the exported animation driver. Prefab-owned materials and
components remain project assets.

Each export has a stable namespace based on its project-relative Assets folder
and level name. Multiple map exports therefore compile in one project. The
importer finds its own script and stores its Unity asset GUID on the generated
root. It validates resources before building, keeps the old root until success,
preserves its parent/placement, and leaves unrelated same-name objects intact.
The custom-game native test passed two-map import, native prefab fields/actions,
forward component references, dynamic convex settling, missing-prefab failure,
reexport placement and persisted ownership after saving/reopening a scene.

### Flax native project scripts and prefabs

`flax_prefab` selects a project Content-relative prefab.
`flax_script_<id>` adds a project script by its full managed type name. Typed
properties can match public fields/writable properties, or use
`flax_property_<Script>__<field>` for a checked explicit binding. Actor/script
references name entities within the imported map. Scalar values, arrays/lists,
vectors, colours and quaternions follow the declared native field type.
`flax_body_type` and `flax_mass` configure native rigidbodies; a prefab selected
for a dynamic/kinematic body must have a RigidBody root.

Each final export uses a namespace based on its Content directory and level
name. `Build` creates another copy; `Rebuild` replaces the scene-persistent
owned root after success and preserves its parent/placement. The Tools menu
uses Rebuild. Native Flax 1.12.6912/PhysX verified project prefabs and script
fields, actions, forward references, dynamic convex floor collision, two levels,
and ownership after saving/reloading. The complete motion_lab and 27 editor
pose regression also passed after these changes. Null graphics proves physics
and integration here, not material rendering.

### Raylib project-owned components

The separate custom C puzzle consumes the production text reader and collision
adapter without the FPS gameplay kit. It checks property types/values, a forward
reference, actions, independent map copies, a game-driven convex collider and
a falling sphere against the exported floor. Raylib itself has no built-in
rigidbody engine; project physics remains game code. Rich structured values
remain in the JSON sidecar, while the compact text loader has documented caps
and flattened scalar arrays.

### Portable assets and units

glTF/GLB retain authored string properties in `extras.entity`, and add versioned
`extras.heron` with typed FGD/project defaults and build-rule metadata. Native
GLTFLoader in Chromium verified integer overrides, boolean/array defaults and
project-owned consumption. GLTFLoader and OBJLoader loaded the same twelve
triangles with matching bounds. These files preserve assets and metadata; native
gameplay/physics is supplied by the consuming project.

The OBJ engine target now honours units-per-metre, matching glTF/GLB and the
other export targets. It previously ignored the scale control and emitted map
units. The standalone mesh writer retains its previous map-unit default.
Choose scale 1 in the engine target when raw map units are required.

## Completed project integration checks

### Defold project-owned entity configuration, 14 October 2026

Defold exports now interpret the following entity properties, which can be
declared with defaults in any project FGD. No FPS classnames are required.
The hints remain in the complete JSON metadata, but do not become gameplay
`go.property` declarations in the generated properties script.

| Property | Effect |
| --- | --- |
| `defold_body_type` | `static`, `kinematic`, `dynamic`, `trigger`, or `none` |
| `defold_mass` | Positive mass for dynamic bodies; zero for other types |
| `defold_group` | Native collision group |
| `defold_mask` | Comma/space-separated native collision masks; empty disables all masks |
| `defold_friction`, `defold_restitution` | Surface physics values |
| `defold_linear_damping`, `defold_angular_damping` | Damping between zero and one |
| `defold_locked_rotation` | Boolean or `true`/`false`/`0`/`1` |
| `defold_material`, `defold_sampler` | Per-entity model material and colour sampler |
| `defold_component_<id>` | Attach a project component resource, for example `/main/relic.script` |

Custom component ids use letters, digits and underscores. `mesh`, `collision`
and `properties` are reserved for generated components. Project resource paths
start with `/`. Re-export writes the generated level, not the referenced project
scripts. `defold_go=/main/beacon.go` replaces a point entity with a project prefab.
`defold_property_<component>__<property>` supplies native property overrides.
Numbers, booleans, strings (hashes), three-component vectors and four-component
vectors retain their native types. For URL and quaternion values, add
`defold_property_type_<component>__<property>` with `url` or `quat`. Solid
entities cannot be replaced by a `.go` because that would discard their brushes.

Explicit hints override the legacy classname-based physics defaults. Without
hints, existing exports retain their defaults. Geometry/collision generation is
still controlled by the FGD build directives and export settings. A body type
cannot create a collider when collision generation was disabled. Dynamic bodies
cannot also be driven by exported animation.

Final and live exports reject invalid hints before publishing files.
`examples/engine-checks/defold/prepare_custom_game.py` prepares a separate game
with its own FGD and project component. Native Defold 1.13.2/Bullet 3.25 verified
the `relic` component's authored metadata `charge=7`, native component charge 9,
boolean/vector overrides,
a separate `.go` prefab with charge 11, dynamic mass 2, custom group
pair `artifact`/`geometry`, and settling on the floor at y=0.5799999 m.
The fixture uses no FPS kit or classname-based mover convention.

### Remaining functionality

- Publish versioned loader APIs and schema fixtures, including long names,
  structured properties, missing resources and changes in map-unit scale.
- Expand native material mapping beyond albedo and keep an explicit choice
  between generated materials and project-owned replacements.
### Unperformed verification

- Repeat editor restart/domain-reload checks on Windows/macOS; final Unity/Flax
  ownership already persists in the Linux native scene checks.
- Unreal needs a supported installed editor for native checks.
- Stride, GDevelop and Cocos Creator passed native checks on macOS on 18 October
  2026: models, materials, physics, animation, triggers and sky. See
  [Stride](stride-export.md), [GDevelop](gdevelop-export.md) and
  [Cocos Creator](cocos-export.md). Windows/Linux runs remain.

### Check project paths without starting an engine

Run the read-only diagnostic from the checkout:

```sh
python3 scripts/check_export_paths.py /path/to/project/.heron-brush/exports.json
python3 scripts/check_export_paths.py /path/to/project/.heron-brush/exports.json --json
```

It reports each configured target's explicit project and output directories.
Relative paths use the directory above `.heron-brush`, matching the profile
loader. A missing explicit project directory or an output path blocked by a file
or dangling symlink is an error. An output directory that has not been created
is a warning. Engine project markers are advisory: a custom or incomplete
project remains usable. Empty paths retain automatic discovery or selection at
export time; the diagnostic does not guess an engine's project root.

The command writes no files, runs no engine or build, and needs only the Python
standard library. Exit status is 0 for no path errors, 1 for path errors, and 2
for an unreadable or malformed profile. Duplicate JSON keys and non-standard
NaN/Infinity literals are rejected. `--json` returns the same findings as
structured output after a profile is successfully read.

This is a filesystem preflight, not complete export validation. Options, entity
bindings, filesystem permissions, textures, engine resources and native imports
still need the editor/exporter and their own checks. Local validation passed twenty
small filesystem tests and the actual `demos/canal_relay` profile, without
compiling or running an engine. The tests are included through portable-core's
existing `test_*.py` discovery; remote CI has not run here.


### Portability of configured export paths

The same diagnostic warns when `project_dir` or `directory` is absolute, escapes
the profile root through `..`, or follows a symlink outside the project. Absolute
paths within the project include a relative-path suggestion. Suggestions are
advisory: the tool never rewrites the profile or copies external folders.
A deliberately separate engine project is supported and remains a warning.

Foreign platform paths, such as Windows drive/UNC paths on Linux, retain their
original spelling and are reported as unverified on the current host. They are
not incorrectly checked beneath the local project root. Backslashes in relative
POSIX-host paths also receive a portability warning because their meaning differs
on Windows. Symlinked project roots and sibling paths with a shared name prefix
are handled using path components rather than string-prefix comparisons.

```sh
python3 scripts/check_export_paths.py /path/to/project/.heron-brush/exports.json --require-portable-paths
```

Default exit codes remain unchanged. With `--require-portable-paths`, portability
findings also produce exit status 1. A missing engine marker or a future output
directory alone does not fail this stricter check. JSON output adds per-target
`portability` findings with a `field`, `kind`, original `configured` value and,
where useful, a resolved path or `relative_suggestion`.

`configured_paths_portable` applies only to the explicit host paths checked here.
It is false for path errors or portability findings. It does not certify that a
whole project is self-contained: automatic engine-root discovery, undeclared material
collections, individual textures/models and engine virtual resources such as `res://`
are outside this audit. They must not be interpreted as ordinary host paths.


### Inspect material sources declared by selected maps

Add one or more explicit map arguments, relative to the command's working
directory or absolute:

```sh
python3 scripts/check_export_paths.py /path/to/project/.heron-brush/exports.json --map /path/to/project/maps/arena.map --require-portable-paths
```

The diagnostic reads the first `// heron material-sources` declaration within
the map's first 256 KiB. It resolves each declared folder or package path relative
to that map's directory, matching the editor. A `folder` must be a directory;
`wad` denotes the editor's package sources, including WAD/PAK/PK3/ZIP, and must
be a file. Package contents and images are never opened. Missing sources or wrong
file types are errors. Absolute paths, foreign-platform paths and sources outside
the project receive the same portability findings as export paths. Relative
suggestions for map sources use the map directory as their base.

A missing declaration is unverified, not evidence of no material dependencies.
Likewise, metadata beyond the bounded inspection range is not audited. An
explicit empty list verifies only that the declaration contains no sources;
automatic materials, individual texture names, model references and engine
resources still need separate checks. Invalid metadata and oversized declaration
lines are reported; no partial result is certified portable. At most 2048
material sources per declaration are inspected.

JSON output adds `maps` and `map_source_paths_portable` only when `--map` is
supplied. Each map reports `declaration_found`, `sources`, availability, warnings,
errors and `paths_portable`. Source errors produce exit 1. With
`--require-portable-paths`, unverified declarations and portability findings also
produce exit 1. Unreadable profiles still produce exit 2; unreadable selected maps
are recorded as individual map errors so other maps can still be checked.

Twenty small tests pass, including map-relative sources, external links, missing
sources, declaration selection, bounded reads and CLI exit status. The canal_relay
profile and all three maps pass the strict configured-path/declaration check.
This validation uses only Python filesystem metadata and bounded header reads.
