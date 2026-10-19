#!/usr/bin/env python3
"""Generate deterministic inputs for benchmark_map_loading.py without dependencies."""
import argparse
from pathlib import Path


def box(index):
    x = index % 100 * 24
    y = index // 100 % 100 * 24
    z = index // 10000 * 24
    a, b, c = x + 16, y + 16, z + 16
    return f'''{{
( {x} {y} {z} ) ( {x} {b} {z} ) ( {x} {y} {c} ) stone 0 0 0 1 1
( {x} {y} {z} ) ( {x} {y} {c} ) ( {a} {y} {z} ) stone 0 0 0 1 1
( {x} {y} {z} ) ( {a} {y} {z} ) ( {x} {b} {z} ) stone 0 0 0 1 1
( {a} {b} {c} ) ( {a} {y} {c} ) ( {x} {b} {c} ) stone 0 0 0 1 1
( {a} {b} {c} ) ( {x} {b} {c} ) ( {a} {b} {z} ) stone 0 0 0 1 1
( {a} {b} {c} ) ( {a} {b} {z} ) ( {a} {y} {c} ) stone 0 0 0 1 1
}}
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    for count in [1000, 10000, 50000]:
        (args.output / f"boxes-{count}.map").write_text(
            '{\n"classname" "worldspawn"\n' + ''.join(box(i) for i in range(count)) + '}\n')
    groups = '{ "classname" "worldspawn" }\n'
    groups += ''.join(f'{{ "classname" "func_group" "_tb_type" "_tb_group" "_tb_id" "{i}" }}\n' for i in range(10000))
    groups += ''.join(f'{{ "classname" "light" "_tb_group" "{i}" }}\n' for i in range(10000))
    (args.output / "groups-10000.map").write_text(groups)
    dragon = (Path(__file__).resolve().parents[2] / "assets/dragon.map").read_text()
    body = dragon[dragon.index('{') + 1:dragon.rfind('}')]
    body = '\n'.join(line for line in body.splitlines() if not line.startswith('// heron_brush id'))
    (args.output / "dragon-x40.map").write_text('// Format: Valve\n{\n"classname" "worldspawn"\n' + body * 40 + '\n}\n')


if __name__ == "__main__":
    main()
