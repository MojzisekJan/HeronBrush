#!/usr/bin/env python3
"""Export demos/feature_lab/feature_lab.map to every target through the real editor MCP.

Run from any directory after `cargo build -p heron-brush`:

    python3 examples/engine-checks/export_feature_lab.py            # all 14 targets
    python3 examples/engine-checks/export_feature_lab.py godot glb  # some of them

Each target goes to its own folder, /tmp/heron-feature-<target> unless --root
names another parent. Folders are emptied first so stale files cannot hide a
missing one. The editor runs with scratch XDG folders, so it never touches the
person's preferences. check_feature_lab.py reads the result.
"""
import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MAP = ROOT / "demos/feature_lab/feature_lab.map"
TEXTURES = ROOT / "demos/feature_lab/textures"
TARGETS = ["godot", "defold", "three", "bevy", "raylib", "unity", "unreal", "flax",
           "stride", "cocos", "gdevelop", "obj", "gltf", "glb"]
NAME = "feature_lab"
# A project class without an FGD entry: typed defaults the authored map overrides
# (charge) or leaves alone (armed, sequence, label).
BINDINGS = {
    "fl_probe": {
        "defaults": {"charge": 9, "armed": True, "sequence": [2, 3, 5], "label": "probe"},
    },
}


def output_dir(root, target):
    return root / f"heron-feature-{target}"


class Editor:
    def __init__(self, binary, scratch):
        env = dict(os.environ, XDG_CONFIG_HOME=str(scratch / "config"),
                   XDG_DATA_HOME=str(scratch / "data"), XDG_CACHE_HOME=str(scratch / "cache"))
        self.proc = subprocess.Popen([str(binary), "--mcp"], stdin=subprocess.PIPE,
                                     stdout=subprocess.PIPE, text=True, bufsize=1, env=env)
        self.serial = 0
        self.rpc("initialize", {})

    def rpc(self, method, params):
        self.serial += 1
        self.proc.stdin.write(json.dumps({"jsonrpc": "2.0", "id": self.serial,
                                          "method": method, "params": params}) + "\n")
        self.proc.stdin.flush()
        while True:
            line = self.proc.stdout.readline()
            if not line:
                raise RuntimeError("the editor closed the connection")
            message = json.loads(line)
            if message.get("id") == self.serial:
                return message

    def call(self, tool, **arguments):
        result = self.rpc("tools/call", {"name": tool, "arguments": arguments}).get("result", {})
        text = "\n".join(c.get("text", "") for c in result.get("content", []))
        if result.get("isError"):
            raise RuntimeError(f"{tool}: {text}")
        return text

    def close(self):
        self.proc.stdin.close()
        self.proc.wait(timeout=60)


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("targets", nargs="*", metavar="target",
                        help=f"any of {', '.join(TARGETS)}; all when omitted")
    parser.add_argument("--root", type=Path, default=Path("/tmp"),
                        help="parent of the heron-feature-<target> folders")
    parser.add_argument("--binary", type=Path, default=ROOT / "target/debug/heron-brush")
    args = parser.parse_args()
    targets = args.targets or TARGETS
    unknown = sorted(set(targets) - set(TARGETS))
    if unknown:
        parser.error(f"unknown target {', '.join(unknown)}")
    failed = []
    with tempfile.TemporaryDirectory(prefix="heron-feature-xdg-") as scratch:
        editor = Editor(args.binary.resolve(), Path(scratch))
        try:
            print(editor.call("open_map", path=str(MAP)))
            for target in targets:
                project = output_dir(args.root.resolve(), target)
                if project.exists():
                    shutil.rmtree(project)
                project.mkdir(parents=True)
                # The game project owns its images: copy them in, as a real project
                # would have them, and let the export find them there.
                shutil.copytree(TEXTURES, project / "textures")
                options = {"project_dir": str(project), "texture_dir": "textures",
                           "entities": BINDINGS}
                if target == "defold":
                    options.update(model_material="/main/model.material", model_sampler="tex0")
                try:
                    report = editor.call("export", engine=target, path=str(project), name=NAME,
                                         scale=32, options=options)
                except RuntimeError as error:
                    failed.append(target)
                    print(f"{target}: FAILED {error}", file=sys.stderr)
                    continue
                (project / "export-report.txt").write_text(report)
                first = report.splitlines()[0] if report else ""
                print(f"{target}: {project} {first[:120]}")
        finally:
            editor.close()
    if failed:
        sys.exit(f"export failed for {', '.join(failed)}")


if __name__ == "__main__":
    main()
