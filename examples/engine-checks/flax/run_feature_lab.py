#!/usr/bin/env python3
"""Run the feature lab's Flax export through the generated importer, headless.

    python3 examples/engine-checks/flax/run_feature_lab.py --root /tmp/feature-runs

Steps: export the feature lab for Flax through the editor (export_feature_lab.py),
write the editor's poses of fl_mover (an editor_core test), make a Flax project
at <root>/heron-flax-feature-project with the export under Content and the
generated scripts plus feature_lab/FeatureLabCheck.cs under Source/Game, then
start the Flax editor with -headless -null -heronfeaturelab. The check imports
the GLBs, builds the level, logs "FEATURE_LAB PASS/FAIL" lines and exits.
With --gpu the editor opens on the display instead, runs the same checks, then
points its viewport at each station and saves what it draws as PNGs in
<root>/flax-feature-shots.

FLAX_EDITOR picks the editor binary (default
~/Downloads/flax/Binaries/Editor/Linux/Development/FlaxEditor).
"""
import argparse
import os
import shutil
import subprocess
import sys
import threading
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--root", type=Path, default=Path("/tmp"),
                        help="parent folder of heron-feature-flax and the project")
    parser.add_argument("--no-export", action="store_true", help="reuse the existing export")
    parser.add_argument("--gpu", action="store_true",
                        help="run the editor on the display (Vulkan) and save screenshots")
    args = parser.parse_args()
    root = args.root.resolve()
    editor = Path(os.environ.get("FLAX_EDITOR", Path.home() /
                                 "Downloads/flax/Binaries/Editor/Linux/Development/FlaxEditor"))
    if not args.no_export:
        subprocess.run([sys.executable, str(ROOT / "examples/engine-checks/export_feature_lab.py"),
                        "flax", "--root", str(root)], check=True)
    export = root / "heron-feature-flax"
    project = root / "heron-flax-feature-project"
    if project.exists():
        shutil.rmtree(project)
    (project / "Content").mkdir(parents=True)
    shutil.copy2(HERE / "flax.flaxproj", project / "flax.flaxproj")
    shutil.copytree(HERE / "Source", project / "Source",
                    ignore=shutil.ignore_patterns("*Check*.cs", "MotionCheck.cs"))
    shutil.copy2(HERE / "Content/GameSettings.json", project / "Content/GameSettings.json")
    shutil.copy2(HERE / "scene-template.json", project / "Content/FeatureLab.scene")
    for item in export.iterdir():
        if item.suffix == ".cs":
            shutil.copy2(item, project / "Source/Game" / item.name)
        elif item.is_dir():
            shutil.copytree(item, project / "Content" / item.name)
        else:
            shutil.copy2(item, project / "Content" / item.name)
    level = (export / "HeronBrushLevel.cs").read_text()
    namespace = next(line[len("namespace "):].strip() for line in level.splitlines()
                     if line.startswith("namespace Game.HeronLevel_"))
    check = (HERE / "feature_lab/FeatureLabCheck.cs").read_text()
    (project / "Source/Game/FeatureLabCheck.cs").write_text(f"using {namespace};\n" + check)

    poses = root / "feature-lab-poses.json"
    subprocess.run(["cargo", "test", "-q", "-p", "editor_core", "--lib",
                    "writes_feature_lab_mover_poses_for_engine_checks"], cwd=ROOT, check=True,
                   env=dict(os.environ, HERON_FEATURE_LAB_POSES=str(poses)))
    # Flax writes LastProject.txt under XDG_CONFIG_HOME; keep it out of the person's.
    env = dict(os.environ, HERON_FEATURE_LAB_POSES=str(poses),
               XDG_CONFIG_HOME=str(root / "flax-config"))
    log = root / ("flax-feature-gpu.txt" if args.gpu else "flax-feature-runtime.txt")
    command = [str(editor), "-project", str(project), "-std", "-heronfeaturelab"]
    if not args.gpu:
        command[1:1] = ["-headless", "-null", "-exit"]
    shots = root / "flax-feature-shots"
    if args.gpu:
        shutil.rmtree(shots, ignore_errors=True)
        shots.mkdir()
        env["HERON_FEATURE_LAB_SHOTS"] = str(shots)
    process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                               text=True, errors="replace", env=env)
    # A script compile error leaves the editor open on its error bar; stop it then.
    timer = threading.Timer(900, process.kill)
    timer.start()
    with log.open("w") as out:
        for line in process.stdout:
            out.write(line)
    status = process.wait(timeout=60)
    timer.cancel()
    lines = [line for line in log.read_text(errors="replace").splitlines() if "FEATURE_LAB" in line]
    print("\n".join(lines))
    print(f"Flax editor status {status}; log {log}")
    failures = compare_extracted_images(project / "Content")
    if failures or not any("FEATURE_LAB_DONE 0" in line for line in lines):
        sys.exit(status or 1)


def compare_extracted_images(content):
    """Flax's GLB import writes each embedded image as <model>_tex_<n>.png beside the
    model. Each must equal, pixel for pixel, an image the export started from: a
    project texture, a baked paint image or one of the model's maps."""
    from PIL import Image

    def pixels(path):
        image = Image.open(path).convert("RGBA")
        return image.size, image.tobytes()

    sources = {}
    for folder in ["textures", "feature_lab_paint"]:
        for path in sorted((content / folder).glob("*.png")):
            sources.setdefault(pixels(path), path.name)
    extracted = sorted((content / "feature_lab").glob("*_tex_*.png"))
    unmatched = [path.name for path in extracted if pixels(path) not in sources]
    matched = {sources[pixels(path)] for path in extracted if pixels(path) in sources}
    ok = extracted and not unmatched
    print(f"{'PASS' if ok else 'FAIL'} extracted images: {len(extracted)} PNGs, "
          f"{len(matched)} distinct sources, unmatched {unmatched}")
    found = {pixels(path) for path in extracted}
    missing = sorted(p.name for p in (content / "feature_lab_paint").glob("*.png")
                     if pixels(p) not in found)
    print(f"INFO export images not among Flax's extracted ones: {missing}")
    return 0 if ok else 1


if __name__ == "__main__":
    main()
