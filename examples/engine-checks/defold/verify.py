#!/usr/bin/env python3
"""Re-export, compile and exercise real Defold physics. Needs the Linux SDK."""
from pathlib import Path
import os, subprocess, sys, tempfile, zipfile
base=Path(__file__).resolve().parent
sdk=Path(os.environ.get("DEFOLD_HOME", "/tmp/heron-defold/Defold"))
jar=next((sdk/"packages").glob("defold-*.jar"))
java=next((sdk/"packages").glob("jdk-*/bin/java"))
subprocess.run([sys.executable,str(base/"export.py")],check=True)
evidence=base/"evidence";evidence.mkdir(exist_ok=True)
with (evidence/"gameplay-build.txt").open("w") as log:
 subprocess.run([str(java),"--enable-native-access=ALL-UNNAMED","-cp",str(jar),"com.dynamo.bob.Bob","--root",str(base),"--max-cpu-threads","2","build"],stdout=log,stderr=subprocess.STDOUT,check=True,timeout=120)
with tempfile.TemporaryDirectory(prefix="defold-gameplay-") as tmp:
 engine=Path(tmp)/"dmengine"
 with zipfile.ZipFile(jar) as archive: engine.write_bytes(archive.read("libexec/x86_64-linux/dmengine"))
 engine.chmod(0o755)
 with (evidence/"gameplay-runtime.txt").open("w") as log:
  subprocess.run([str(engine),"--config=check.enabled=2","./game.projectc"],cwd=base/"build/default",stdout=log,stderr=subprocess.STDOUT,check=True,timeout=30)
output=(evidence/"gameplay-runtime.txt").read_text()
assert "GAMEPLAY_CHECK all checks passed" in output,output
assert "ERROR:" not in output,output
print("Defold native gameplay passed:",output.count("DEBUG:SCRIPT: PASS"),"assertions")
