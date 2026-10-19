# Shared model geometry in exports

Repeated imported OBJ/GLB models can share one exported mesh while retaining
separate entities, names, properties and transforms. There is no new map format
or required runtime component.

For ordinary static models, the engine exports keep source geometry in one asset
and put position, rotation and positive axis scales on each instance. Source
surfaces are cached once per model during export. Generated mesh files are shared
only after their complete bytes match; the hash selects comparison candidates.
Materials and supplied vertex normals remain part of those bytes.

Portable glTF/GLB uses several nodes referring to one mesh. These nodes retain
source transforms, including mirrored scales. It does not emit a keyframe playback
controller. Standalone OBJ keeps flattened geometry because OBJ has no standard
scene-instance representation.

## Cases that retain baked geometry

The common engine build retains baked geometry for mirrored scales, generated
collision, animation/motion properties, `func_*` and `trigger_*` classes, custom
origin rules and occluders. An entity combining imported geometry with brushes or
patches also stays baked. Byte-identical mesh files can still be shared in this
path; different collision or material data prevents accidental reuse.

This preserves the existing collision and animation coordinate frames. Extending
canonical instancing to those cases needs separate engine checks. This change
reduces exported geometry files; it does not add GPU instanced draw calls or promise
that every engine loader shares its GPU buffers. Godot's native check verifies
that its three nodes reference the same mesh resource.

Imported skin PNGs are packaged with the other generated images in
`<level>_paint/`, with project-relative resource references. GLB assets also retain
embedded images.

## Checks

`repeated_models_share_assets_without_changing_world_vertices` checks different
positions, rotations and nonuniform positive scales, all 13 export targets,
portable glTF mesh/node counts, world vertices and a 20-instance mesh-size fixture.
The report's triangle count still describes the placed scene, rather than only the
unique assets. Additional scene-export tests check the baked fallback and full
byte equality before file reuse.

With Godot already installed:

```sh
python3 examples/editor-checks/verify_instances.py
```

The helper uses offline Cargo, creates a fresh temporary project, imports the
actual generated OBJ and PNG files, then checks three MeshInstance3D nodes sharing
one resource. It compares world vertices and inverse-transpose-transformed normals
with the editor's baked model data. No downloads or installations occur. Logs and
exported fixtures remain in the printed temporary folder. `--root` accepts a new
or empty destination so stale files cannot hide missing assets.

The Cocos loader now receives an explicit YXZ quaternion instead of interpreting
the angle array as yaw/pitch/roll. Its quaternion is checked against the common
basis calculation. The loader uses the documented
[Node.setRotation quaternion API](https://docs.cocos.com/creator/3.8/api/en/class/Node).
A native Cocos run remains unperformed.
