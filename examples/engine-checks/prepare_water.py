#!/usr/bin/env python3
"""Prepare disposable projects after the core water_volume_contract export test."""
from pathlib import Path
import shutil,json,re
root=Path(__file__).resolve().parents[2];base=Path('/tmp/heron-water-check')
# Minimal Unity project using only engine-bundled modules.
project=base/'unity';(project/'Packages').mkdir(parents=True,exist_ok=True);(project/'ProjectSettings').mkdir(exist_ok=True)
(project/'Packages/manifest.json').write_text(json.dumps({'dependencies':{'com.unity.modules.physics':'1.0.0','com.unity.modules.imgui':'1.0.0','com.unity.modules.jsonserialize':'1.0.0'}}))
(project/'ProjectSettings/ProjectVersion.txt').write_text('m_EditorVersion: 6000.3.24f1\n')
# Copy only project scaffolding. All geometry/importer sources come from the fresh export.
project=base/'flax';seed=root/'examples/engine-checks/flax'
for name in ['flax.flaxproj','scene-template.json']:
 shutil.copy2(seed/name,project/name)
(project/'Source/Game').mkdir(parents=True,exist_ok=True)
for name in ['GameTarget.Build.cs','GameEditorTarget.Build.cs']:shutil.copy2(seed/'Source'/name,project/'Source'/name)
shutil.copy2(seed/'Source/Game/Game.Build.cs',project/'Source/Game/Game.Build.cs')
shutil.copy2(seed/'scene-template.json',project/'Content/ExportCheck.scene');shutil.copy2(seed/'Content/GameSettings.json',project/'Content/GameSettings.json')
export=project/'Content/arena';text=(export/'HeronBrushLevel.cs').read_text();namespace=re.search(r'namespace (Game.HeronLevel_\w+)',text)[1]
for file in export.glob('*.cs'):shutil.copy2(file,project/'Source/Game'/file.name)
(project/'water-namespace.txt').write_text(namespace)
(project/'Source/Game/WaterCheckPlugin.cs').write_text('using '+namespace+';\n'+(seed/'Source/Game/WaterCheckPlugin.cs').read_text())
# Defold sample scaffold, retaining freshly generated water resources.
project=base/'defold';seed=root/'examples/engine-checks/defold'
for name in ['game.project']:shutil.copy2(seed/name,project/name)
for file in (seed/'main').iterdir():
 if file.is_file() and file.name not in ['arena.json','arena.collection']:shutil.copy2(file,project/'main'/file.name)
text=(project/'main/main.collection').read_text().replace('/main/arena.collection','/main/water.collection');(project/'main/main.collection').write_text(text)
text=(project/'main/entity_runtime.lua').read_text().replace('/main/arena.json','/main/water.json');(project/'main/entity_runtime.lua').write_text(text)
for name in ['player.script','water_checks.lua']:
 text=(project/'main'/name).read_text().replace('main.arena_water','main.water_water');(project/'main'/name).write_text(text)
text=(project/'game.project').read_text().replace('/main/arena.json','/main/water.json').replace('[bootstrap]','[bootstrap]\nrender = /main/water.renderc');(project/'game.project').write_text(text)
print('Prepared offline native projects')

import subprocess,sys
subprocess.run([sys.executable,str(root/'examples/engine-checks/unity/install_check.py'),str(base/'unity'),'Assets/Water','VerifyWater.cs'],check=True)
