#!/usr/bin/env python3
"""Build a production Defold export with an explicit normal-map sampler using Bob.

The output folder must be new or empty. No engine installation is changed.
"""
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    project = args.output.resolve() if args.output else Path(tempfile.mkdtemp(prefix='heron-defold-material-'))
    if project.exists() and any(project.iterdir()):
        parser.error('output must be new or empty')
    project.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, HERON_DEFOLD_SAMPLER_OUTPUT=str(project))
    subprocess.run(['cargo', 'test', '-p', 'editor_core', '--lib', '--offline',
                    'defold_project_shader_receives_requested_normal_texture'], cwd=ROOT, env=env, check=True)
    material = (HERE / 'main/model.material').read_text().replace('/main/model.vp', '/normal.vp').replace('/main/model.fp', '/normal.fp')
    material += '\nsamplers { name: "tex_normal" wrap_u: WRAP_MODE_REPEAT wrap_v: WRAP_MODE_REPEAT filter_min: FILTER_MODE_MIN_LINEAR filter_mag: FILTER_MODE_MAG_LINEAR }\n'
    (project / 'normal.material').write_text(material)
    shutil.copy2(HERE / 'main/model.vp', project / 'normal.vp')
    (project / 'normal.fp').write_text('''varying mediump vec2 var_texcoord0;
uniform lowp sampler2D tex0;
uniform lowp sampler2D tex_normal;
void main() { gl_FragColor=vec4(texture2D(tex_normal,var_texcoord0).rgb + texture2D(tex0,var_texcoord0).rgb*0.0001,1.0); }
''')
    (project / 'game.project').write_text('[project]\ntitle = Heron material binding check\n[bootstrap]\nmain_collection = /arena.collectionc\n[display]\nwidth = 320\nheight = 240\n')
    (project / 'input').mkdir(exist_ok=True)
    (project / 'input/game.input_binding').write_text('')
    sdk = Path(os.environ.get('DEFOLD_HOME', Path.home() / '.local/opt/Defold'))
    jar = next((sdk / 'packages').glob('defold-*.jar'))
    java = next((sdk / 'packages').glob('jdk-*/bin/java'))
    with (project / 'bob.log').open('w') as log:
        result = subprocess.run([str(java), '--enable-native-access=ALL-UNNAMED', '-cp', str(jar),
                                 'com.dynamo.bob.Bob', '--root', str(project), '--platform', 'x86_64-linux',
                                 '--archive', 'build'], stdout=log, stderr=subprocess.STDOUT)
    if result.returncode:
        print((project / 'bob.log').read_text(), file=sys.stderr)
        return result.returncode
    print('DEFOLD_MATERIAL_BINDINGS PASS: Bob compiled the production model with albedo and normal samplers', project)
    return 0


if __name__ == '__main__':
    sys.exit(main())
