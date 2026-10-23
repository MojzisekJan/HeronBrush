#!/usr/bin/env python3
"""Build the feature lab's Defold export with Bob and run it in dmengine.

    python3 examples/engine-checks/defold/run_feature_lab.py --root /tmp/feature-runs

Steps: export the feature lab for Defold through the editor (export_feature_lab.py),
write the editor's poses of fl_mover (an editor_core test), add the check files
from feature_lab/ and the sample's /main/model.material, build with Bob, then run
dmengine inside a nested Xephyr X server. The check script prints PASS and FAIL
lines; this script takes a screenshot of the Xephyr screen at each "VIEW" line
with xwd, saves PNGs and checks a few pixels in them. It exits non-zero on any
failure.

DEFOLD_HOME picks the Defold editor folder (default ~/.local/opt/Defold). It
needs DISPLAY (for Xephyr), xwd, stdbuf and Pillow; without a display, use
--no-run to stop after the Bob build.
"""
import argparse
import os
import shutil
import struct
import subprocess
import sys
import time
import zipfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]

# Wraps the generated render script: counts the frames in which each predicate is
# drawn and sends the totals to the check script once a second.
RENDER_PREFIX = """-- Check wrapper round the exported render script.
local hb_names,hb_counts,hb_frame={},{},0
local hb_predicate,hb_draw=render.predicate,render.draw
render.predicate=function(tags) local p=hb_predicate(tags);hb_names[p]=tags[1];return p end
render.draw=function(p,...) local n=hb_names[p];if n then hb_counts[n]=true end;return hb_draw(p,...) end
local hb_totals={}
"""
RENDER_SUFFIX = """
local hb_update=update
function update(self,dt)
 hb_counts={};hb_update(self,dt);hb_frame=hb_frame+1
 for n in pairs(hb_counts) do hb_totals[n]=(hb_totals[n] or 0)+1 end
 if hb_frame%30==0 then msg.post("main:/probe#check","heron_draws",hb_totals) end
end
"""


def xwd_to_png(data, path):
    """Saves a 24 or 32 bit per pixel ZPixmap xwd dump as PNG, with Pillow."""
    from PIL import Image
    fields = struct.unpack(">25I", data[:100])
    header_size, width, height = fields[0], fields[4], fields[5]
    bits_per_pixel, bytes_per_line, ncolors = fields[11], fields[12], fields[19]
    raw = {24: ("RGB", "BGR"), 32: ("RGBX", "BGRX")}.get(bits_per_pixel)
    if raw is None:
        raise RuntimeError(f"xwd dump has {bits_per_pixel} bits per pixel")
    offset = header_size + ncolors * 12
    image = Image.frombuffer(raw[0], (width, height), data[offset:offset + bytes_per_line * height],
                             "raw", raw[1], bytes_per_line, 1)
    image.convert("RGB").save(path)


def check_screenshots(evidence):
    """Pixel checks on the screenshots. Each view looks at its station's centre."""
    from PIL import Image

    def centre(name):
        image = Image.open(evidence / f"{name}.png").convert("RGB")
        w, h = image.size
        box = image.crop((w // 2 - 10, h // 2 - 10, w // 2 + 10, h // 2 + 10))
        pixels = list(box.getdata())
        return [sum(p[i] for p in pixels) / len(pixels) for i in range(3)]

    def share(name, colour):
        """Share of pixels whose colour, scaled to its brightest channel, is the paint's."""
        image = Image.open(evidence / f"{name}.png").convert("RGB").resize((480, 250))
        top = max(colour)
        target = [c / top for c in colour]
        hits = 0
        for p in image.getdata():
            m = max(p)
            if m > 40 and all(abs(p[i] / m - target[i]) < 0.08 for i in range(3)):
                hits += 1
        return hits / (480 * 250)

    def woody(rgb):
        return rgb[0] - rgb[2] > 25

    results = []
    hidden, hidden_top = centre("hidden_face"), centre("hidden_top")
    results.append((not woody(hidden) and woody(hidden_top),
                    "fl_hidden's hidden side shows what is behind it; its top shows wood",
                    f"side {hidden}, top {hidden_top}"))
    skip = centre("skip_top")
    results.append((abs(skip[0] - skip[2]) < 15 and skip[0] > 120,
                    "fl_skip's top is open: the floor shows through", f"{skip}"))
    for view, name, colour in [("colours_back", "red", (220, 30, 40)),
                               ("colours_back", "blue", (40, 60, 230)),
                               ("colours_front", "green", (30, 200, 60)),
                               ("colours_front", "yellow", (240, 220, 30))]:
        part = share(view, colour)
        results.append((part > 0.01, f"painted {name} face drawn", f"{part:.1%} of {view}"))
    sky = Image.open(evidence / "sky.png").convert("RGB").getpixel((10, 10))
    results.append((sky != (20, 31, 43) and sky[2] > 120, "sky pass covers the clear colour",
                    f"top-left pixel {sky}"))
    failures = 0
    for ok, label, detail in results:
        failures += not ok
        print(f"{'PASS' if ok else 'FAIL'} screenshot: {label} ({detail})")
    return failures


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--root", type=Path, default=Path("/tmp"),
                        help="parent folder; the project is heron-feature-defold inside it")
    parser.add_argument("--no-export", action="store_true", help="reuse the existing export")
    parser.add_argument("--no-run", action="store_true", help="stop after the Bob build")
    parser.add_argument("--xephyr-display", type=int, default=77,
                        help="display number for the nested Xephyr server (default 77)")
    parser.add_argument("--shots", type=Path, help="only check the screenshots in this folder")
    args = parser.parse_args()
    if args.shots:
        sys.exit(check_screenshots(args.shots))
    root = args.root.resolve()
    project = root / "heron-feature-defold"
    sdk = Path(os.environ.get("DEFOLD_HOME", Path.home() / ".local/opt/Defold"))
    jar = next((sdk / "packages").glob("defold-*.jar"))
    java = next((sdk / "packages").glob("jdk-*/bin/java"))

    if not args.no_export:
        subprocess.run([sys.executable, str(ROOT / "examples/engine-checks/export_feature_lab.py"),
                        "defold", "--root", str(root)], check=True)
    poses = project / "check/poses.json"
    shutil.copytree(HERE / "feature_lab", project, dirs_exist_ok=True)
    (project / "main").mkdir(exist_ok=True)
    for name in ["model.material", "model.vp", "model.fp"]:
        shutil.copy2(HERE / "main" / name, project / "main" / name)
    subprocess.run(["cargo", "test", "-q", "-p", "editor_core", "--lib",
                    "writes_feature_lab_mover_poses_for_engine_checks"], cwd=ROOT, check=True,
                   env=dict(os.environ, HERON_FEATURE_LAB_POSES=str(poses)))
    render = (project / "feature_lab.render_script").read_text()
    (project / "check/check.render_script").write_text(RENDER_PREFIX + render + RENDER_SUFFIX)

    evidence = root / "defold-feature-evidence"
    if evidence.exists():
        shutil.rmtree(evidence)
    evidence.mkdir(parents=True)
    with (evidence / "bob.txt").open("w") as log:
        status = subprocess.run([str(java), "--enable-native-access=ALL-UNNAMED", "-cp", str(jar),
                                 "com.dynamo.bob.Bob", "--root", str(project),
                                 "--max-cpu-threads", "3", "distclean", "build"],
                                stdout=log, stderr=subprocess.STDOUT, timeout=600).returncode
    print(f"Bob build: status {status}, log {evidence / 'bob.txt'}")
    if status != 0:
        sys.exit(status)
    if args.no_run:
        return
    engine = root / "dmengine"
    with zipfile.ZipFile(jar) as archive:
        engine.write_bytes(archive.read("libexec/x86_64-linux/dmengine"))
    engine.chmod(0o755)
    # dmengine runs in its own nested X server, so a screenshot holds only the game
    # and never another window that happens to cover it.
    display = f":{args.xephyr_display}"
    xephyr = subprocess.Popen(["Xephyr", display, "-screen", "1280x720", "-ac", "-br", "-noreset"],
                              stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    env = dict(os.environ, DISPLAY=display)
    for _ in range(50):
        if subprocess.run(["xwininfo", "-root"], env=env, capture_output=True).returncode == 0:
            break
        time.sleep(0.1)
    lines = []
    # stdbuf: dmengine buffers a piped stdout, and a late VIEW line would miss its view.
    process = subprocess.Popen(["stdbuf", "-oL", "-eL", str(engine), "./game.projectc"],
                               cwd=project / "build/default", env=env,
                               stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1)
    for line in process.stdout:
        lines.append(line)
        print(line, end="")
        if "VIEW " in line:
            name = line.split("VIEW ", 1)[1].strip()
            dump = subprocess.run(["xwd", "-silent", "-root"], env=env, capture_output=True)
            if dump.returncode == 0:
                xwd_to_png(dump.stdout, evidence / f"{name}.png")
            else:
                print(f"xwd failed for {name}: {dump.stderr.decode().strip()}")
    status = process.wait(timeout=60)
    xephyr.terminate()
    (evidence / "runtime.txt").write_text("".join(lines))
    passes = sum("SCRIPT: PASS" in line for line in lines)
    print(f"dmengine status {status}; {passes} passes; evidence in {evidence}")
    failures = check_screenshots(evidence)
    sys.exit(status or failures)


if __name__ == "__main__":
    main()
