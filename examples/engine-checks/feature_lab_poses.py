#!/usr/bin/env python3
"""Write the editor's own poses of the feature lab's animated entities, for engine checks.

Run after `cargo build -p heron-brush`:

    python3 examples/engine-checks/feature_lab_poses.py /tmp/feature-lab-poses.json

The editor evaluates each pose through MCP `sample_animation`: fl_mover's looping
default clip every 1/3 s from 0 to 3.67 s, and fl_door's `open` clip every 1/6 s. Each
sample places four probe points (the pivot and a point 64 units along each map axis
from it) the way the editor moves the entity's brushes, and writes them in the export
frame: Y up, metres, `(y, z, x) / 32` of the map's `(x, y, z)`. An engine check moves
the same rest points with its runtime's pose of the entity and compares.
"""
import argparse
import json
import math
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from export_feature_lab import MAP, ROOT, Editor  # noqa: E402

SCALE = 32.0
# The mover loops, so its last frame is its first; the door's `open` clip plays once.
SAMPLES = {"fl_mover": range(0, 120, 10), "fl_door": range(0, 31, 5)}


def rotate(v, degrees):
    """The editor's turn: X, then Y, then Z (keyframe_animation.rs transform_time)."""
    x, y, z = v
    for axis, angle in enumerate(degrees):
        s, c = math.sin(math.radians(angle)), math.cos(math.radians(angle))
        if axis == 0:
            x, y, z = x, c * y - s * z, s * y + c * z
        elif axis == 1:
            x, y, z = c * x + s * z, y, -s * x + c * z
        else:
            x, y, z = c * x - s * y, s * x + c * y, z
    return (x, y, z)


def export_frame(p):
    return [p[1] / SCALE, p[2] / SCALE, p[0] / SCALE]


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("output", type=Path)
    parser.add_argument("--text", type=Path,
                        help="also write `name time x y z x y z` lines, rest then posed, "
                             "one per probe, for a C check")
    parser.add_argument("--binary", type=Path, default=ROOT / "target/debug/heron-brush")
    args = parser.parse_args()
    out = {}
    with tempfile.TemporaryDirectory(prefix="heron-feature-poses-") as scratch:
        editor = Editor(args.binary.resolve(), Path(scratch))
        try:
            editor.call("open_map", path=str(MAP))
            graph = json.loads(editor.call("entity_graph"))
            ids = {n["name"]: str(n["id"]) for n in graph["nodes"] if n.get("name")}
            for name, frames in SAMPLES.items():
                samples = []
                for frame in frames:
                    pose = json.loads(editor.call("sample_animation", entity=ids[name], frame=frame))
                    pivot, offset = pose["pivot"], pose["position"]
                    probes = [(0, 0, 0), (64, 0, 0), (0, 64, 0), (0, 0, 64)]
                    rest, posed = [], []
                    for probe in probes:
                        turned = rotate(probe, pose["rotation_degrees"])
                        rest.append(export_frame([pivot[i] + probe[i] for i in range(3)]))
                        posed.append(export_frame([pivot[i] + turned[i] + offset[i]
                                                   for i in range(3)]))
                    samples.append({"time": frame / 30.0, "rest": rest, "posed": posed,
                                    "offset": offset, "rotation_degrees": pose["rotation_degrees"]})
                out[name] = samples
        finally:
            editor.close()
    args.output.write_text(json.dumps(out, indent=1))
    if args.text:
        lines = [f"{name} {s['time']!r} " + " ".join(repr(v) for v in rest + posed)
                 for name, samples in out.items() for s in samples
                 for rest, posed in zip(s["rest"], s["posed"])]
        args.text.write_text("\n".join(lines) + "\n")
    print(f"{args.output}: {sum(len(s) for s in out.values())} poses")


if __name__ == "__main__":
    main()
