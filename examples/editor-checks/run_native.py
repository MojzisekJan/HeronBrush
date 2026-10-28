#!/usr/bin/env python3
"""Replay editor input in the native editor and validate every declared artifact.

On Linux the editor runs on an isolated Xephyr display. On macOS it opens an ordinary
window: the script's input is injected into egui, so the desktop's pointer and keyboard
are untouched, but the window is visible while the check runs.
"""
import argparse
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]


def artifacts(script):
    result = []
    for line in Path(script).read_text().splitlines():
        words = shlex.split(line, comments=True)
        if words and words[0] in {"snapshot", "shot"}:
            name = words[1] if len(words) > 1 else words[0]
            if not name or Path(name).name != name or name in {".", ".."}:
                raise ValueError(f"Unsafe artifact name: {name}")
            result.append((words[0], name))
    if not any(kind == "snapshot" for kind, _ in result):
        raise ValueError("A checked input fixture must declare at least one snapshot")
    return result


def validate(script, output, working=None):
    output = Path(output)
    snapshots = {}
    for kind, name in artifacts(script):
        path = output / f"{name}.{'json' if kind == 'snapshot' else 'png'}"
        if not path.is_file():
            raise AssertionError(f"Missing {kind}: {path}")
        if kind == "snapshot":
            value = json.loads(path.read_text())
            if not isinstance(value.get("map"), str):
                raise AssertionError(f"Snapshot has no map: {path}")
            snapshots[name] = value
    if {"base", "changed", "undo"} <= snapshots.keys():
        assert snapshots["base"]["map"] != snapshots["changed"]["map"], "GUI edit did not change the map"
        assert snapshots["base"]["map"] == snapshots["undo"]["map"], "One Undo did not restore the map"
    if Path(script).name == "native-basics.script":
        for left, right in [("base", "dup_undo"), ("base", "base_again"),
                            ("duplicated", "move_undo"), ("duplicated", "dup_redo"),
                            ("changed", "redo"), ("changed", "delete_undo"),
                            ("changed", "cut_undo"), ("changed", "saved")]:
            assert snapshots[left]["map"] == snapshots[right]["map"], f"{right} differs from {left}"
        assert snapshots["base"]["brushes"] == 1
        assert snapshots["duplicated"]["brushes"] == 2
        assert snapshots["moved"]["map"] != snapshots["duplicated"]["map"]
        assert snapshots["changed"]["brushes"] == 2
        assert snapshots["deleted"]["brushes"] == 0
        assert snapshots["cut"]["brushes"] == 0
    if Path(script).stem in {"native-material", "native-material-handles"}:
        faces = [line for line in snapshots["changed"]["map"].splitlines() if line.startswith("( ")]
        assert len(faces) == 6
        assert sum("brick_wall" in line for line in faces) == 1, "Material must change exactly one face"
        assert sum(" stone " in line for line in faces) == 5, "Other face materials changed"
        assert snapshots["changed"]["map"] == snapshots["redo"]["map"]
        assert snapshots["changed"]["map"] == snapshots["saved"]["map"]
    if Path(script).stem == "native-fillet-grid":
        assert snapshots["preview"]["map"] == snapshots["base"]["map"], "Preview must not edit the map"
        faces = [line for line in snapshots["changed"]["map"].splitlines() if line.startswith("( ")]
        assert len(faces) > 6, "Fillet must add rounded faces"
        for face in faces:
            for point in face.split(")")[:3]:
                for coordinate in point.strip(" (").split():
                    value = float(coordinate)
                    assert abs(value / 2.0 - round(value / 2.0)) < 1e-6, "Fillet UI must use the selected Grid 2"
    if Path(script).stem == "native-fillet":
        assert snapshots["preview"]["map"] == snapshots["base"]["map"], "Preview changed the map before apply"
        assert snapshots["preview"]["modified"] == snapshots["base"]["modified"]
    if Path(script).stem == "native-uv":
        assert snapshots["selected"]["face_selection"] == 1
        before = [line for line in snapshots["base"]["map"].splitlines() if line.startswith("( ")]
        after = [line for line in snapshots["changed"]["map"].splitlines() if line.startswith("( ")]
        assert len(before) == len(after) == 6
        assert sum(left != right for left, right in zip(before, after)) == 1, "UV must change exactly one face"
        assert snapshots["redo"]["map"] == snapshots["changed"]["map"]
    if Path(script).stem.startswith(("native-texture-copy", "native-texture-transfer")):
        faces = [line for line in snapshots["changed"]["map"].splitlines() if line.startswith("( ")]
        assert len(faces) == 6
        assert sum("brick_wall" in line for line in faces) == 2, "Source and target must both have brick"
        assert sum(" stone " in line for line in faces) == 4, "Other faces changed"
        assert snapshots["changed"]["map"].count("// heron_brush color ") == 4, "Paint still hides the copied texture"
        assert snapshots["changed"]["map"] == snapshots["redo"]["map"]
        assert snapshots["changed"]["map"] == snapshots["saved"]["map"]
        if Path(script).stem.endswith("-handles"):
            assert snapshots["base"]["face_selection"] == 0, "Stale explicit face overrides the handle"
            assert snapshots["changed"]["face_selection"] == 0
            assert snapshots["changed"]["selection"] == 1
    if Path(script).stem == "native-fillet-selection":
        counts = {"base":0, "picked_2d":1, "added_2d":2, "removed_2d":1,
                  "selected_two":2, "cleared":0, "picked_3d":1, "added_3d":2,
                  "removed_3d":1, "kept_on_ctrl_empty":1, "cleared_again":0}
        for name, count in counts.items():
            assert snapshots[name]["selection"] == count, f"Wrong brush selection at {name}"
            assert snapshots[name]["map"] == snapshots["base"]["map"], f"Selecting brushes edited geometry at {name}"
        assert all(value["tool"] == "Fillet" and value["brushes"] == 2 for value in snapshots.values())
        assert snapshots["preview"]["map"] == snapshots["base"]["map"]
        assert snapshots["preview"]["modified"] is False
    if Path(script).stem in {"native-paint-drag", "native-paint-drag-2d"}:
        faces=[line for line in snapshots["changed"]["map"].splitlines() if line.startswith("( ")]
        assert len(faces)==(12 if Path(script).stem.endswith("-2d") else 6)
        assert sum("brick_wall" in line for line in faces)==2, "One drag must paint both crossed faces"
        assert snapshots["changed"]["map"]==snapshots["redo"]["map"]==snapshots["saved"]["map"]
        assert snapshots["base"]["selection"]==snapshots["changed"]["selection"]==0
        assert snapshots["base"]["camera_position"]==snapshots["changed"]["camera_position"]
        if "eyedropper" in snapshots:
            assert snapshots["eyedropper"]["map"]==snapshots["changed"]["map"]
    if Path(script).stem=="native-middle-material":
        for name in ["changed","pasted_2d"]:
            faces=[line for line in snapshots[name]["map"].splitlines() if line.startswith("( ")]
            assert len(faces)==6 and sum("brick_wall" in line for line in faces)==2
            assert snapshots[name]["face_selection"]==snapshots["base"]["face_selection"]==1
        assert snapshots["changed"]["camera_position"]==snapshots["base"]["camera_position"]
        assert snapshots["panned"]["camera_position"]!=snapshots["base"]["camera_position"], "Middle drag must pan"
        for name in ["panned","undo_2d","panned_2d","saved"]:
            assert snapshots[name]["map"]==snapshots["base"]["map"], "Middle pan or Undo changed scene"
    if "saved" in snapshots and working is not None:
        assert snapshots["saved"].get("modified") is False, "Save left unsaved changes"
        # to_map_text serializes scene data; save also writes a separate resource header.
        scene = lambda text: "".join(line for line in text.splitlines(keepends=True)
                                    if not line.startswith("// heron material-sources "))
        assert scene(Path(working).read_text()) == scene(snapshots["saved"]["map"]), "Saved file differs from the editor"
    return snapshots


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("script", type=Path)
    parser.add_argument("--map", type=Path, default=ROOT / "examples/editor-checks/cube.map")
    parser.add_argument("--shots", type=Path, default=Path("/tmp/heron-native-tools"))
    parser.add_argument("--cwd", type=Path, help="Working directory offered as a file chooser place")
    parser.add_argument("--binary", type=Path, default=ROOT / "target/debug/heron-brush")
    parser.add_argument("--welcome", action="store_true", help="Start without opening a map")
    parser.add_argument("--timeout", type=int, default=120)
    args = parser.parse_args()
    args.shots = args.shots.resolve()
    args.shots.mkdir(parents=True, exist_ok=True)
    expected = artifacts(args.script)
    # A previous successful run must never supply evidence for an interrupted run.
    for kind, name in expected:
        for suffix in ([".json"] if kind == "snapshot" else [".png", "-3d.png"]):
            (args.shots / f"{name}{suffix}").unlink(missing_ok=True)
    (args.shots / "CHECK.json").unlink(missing_ok=True)
    profile = Path(tempfile.mkdtemp(prefix="profile-", dir=args.shots))
    config = profile / "config/heron-brush"
    config.mkdir(parents=True)
    preferences = args.shots / "config/heron-brush/preferences"
    if preferences.exists():
        shutil.copyfile(preferences, config / "preferences")
    else:
        (config / "preferences").write_text("ui/Language=english\n")
    working = args.shots / "working.map"
    if args.map.resolve() != working:
        shutil.copyfile(args.map, working)
    env = {**os.environ,
           "XDG_CONFIG_HOME": str(profile / "config"),
           "XDG_DATA_HOME": str(profile / "data"),
           "HERON_BRUSH_SCRIPT": str(args.script.resolve()),
           "HERON_BRUSH_SHOTS": str(args.shots)}
    command = [str(args.binary.resolve()), "--new-instance"] + ([] if args.welcome else [str(working)])

    def run_editor(env):
        with (args.shots / "editor.log").open("w") as editor_log:
            subprocess.run(command, env=env, cwd=args.cwd or ROOT, stdout=editor_log,
                           stderr=subprocess.STDOUT, check=True, timeout=args.timeout)

    if sys.platform == "darwin":
        # A sleeping display never delivers the window's screenshot: wake it and keep it on.
        command[:0] = ["caffeinate", "-d", "-u"]
        run_editor(env)
    else:
        with (args.shots / "xephyr.log").open("w") as display_log:
            display = subprocess.Popen(
                ["Xephyr", "-displayfd", "1", "-screen", "1440x1000", "-nolisten", "tcp"],
                stdout=subprocess.PIPE, stderr=display_log, text=True,
            )
            try:
                number = display.stdout.readline().strip()
                if not number:
                    raise RuntimeError(f"Xephyr did not start; see {args.shots / 'xephyr.log'}")
                run_editor({**env, "DISPLAY": ":" + number, "LIBGL_ALWAYS_SOFTWARE": "1"})
            finally:
                display.terminate()
                display.wait(timeout=10)
    snapshots = validate(args.script, args.shots, working)
    report = {"script": str(args.script.resolve()), "snapshots": list(snapshots),
              "artifacts": len(expected), "passed": True}
    (args.shots / "CHECK.json").write_text(json.dumps(report, indent=2) + "\n")
    print(f"PASS: {args.script.name}, {len(snapshots)} snapshots, {len(expected)} artifacts ({args.shots})")


if __name__ == "__main__":
    main()
