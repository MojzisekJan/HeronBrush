#!/usr/bin/env python3
"""Run the real Godot import and tests; GODOT may override the executable."""
from pathlib import Path
import os, subprocess
base = Path(__file__).resolve().parent
godot = os.environ.get("GODOT", "godot")
(base/"evidence").mkdir(exist_ok=True)
for name, args in [("import", ["--editor", "--import"])] + [("verify_"+t, ["--max-fps", "60", "--script", "verify_"+t+".gd"]) for t in ["contract", "demo", "portal", "doors", "damage"]]:
    with (base/"evidence"/(name+".log")).open("w") as log:
        result = subprocess.run([godot, "--headless", "--path", str(base), *args], stdout=log, stderr=subprocess.STDOUT, timeout=120)
    output = (base/"evidence"/(name+".log")).read_text()
    if result.returncode or (name != "import" and "ALL CHECKS PASSED" not in output):
        raise SystemExit(f"{name} failed: inspect evidence/{name}.log")
    print(name + ": passed")
