"""Shared asset dimensions: 32 map units per metre, Steve/player height 64.

Profiles are absolute dimensions, so saving repeatedly never compounds scale.
All profiled assets have their lowest point at Z=0. Unlisted maps are untouched.
"""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent
PROFILES = json.loads((ROOT / 'scale_profiles.json').read_text())['assets']


def bounds(brushes):
    return ([min(b['min'][i] for b in brushes) for i in range(3)],
            [max(b['max'][i] for b in brushes) for i in range(3)])


def normalize_editor(editor, path):
    path = Path(path).resolve()
    if path.parent != ROOT or path.stem not in PROFILES:
        return
    before = json.loads(editor.call('list_brushes'))
    if not before:
        raise RuntimeError(f'{path.stem}: no geometry to scale')
    lo, hi = bounds(before)
    target = PROFILES[path.stem]['units']
    factors = [target[i] / (hi[i] - lo[i]) for i in range(3)]
    if all(abs(hi[i] - lo[i] - target[i]) < 1e-5 for i in range(3)) and abs(lo[2]) < 1e-5:
        return
    # A nearest-grid snap may change the outer size by one eighth unit.
    # Do not rescale already aligned geometry back off-grid on the next save.
    on_grid = all(abs(v * 8 - round(v * 8)) < 1e-5 for v in lo + hi)
    if on_grid and abs(lo[2]) < 1e-5 and all(abs(hi[i] - lo[i] - target[i]) <= 0.12501 for i in range(3)):
        return
    # Explicit brush IDs avoid moving incidental lights or player markers.
    editor.call('select', ids=[b['id'] for b in before])
    editor.call('scale_selection', factors=factors, center=[0, 0, 0])
    if abs(lo[2] * factors[2]) > 1e-7:
        editor.call('translate_selection', delta=[0, 0, -lo[2] * factors[2]])
    after = json.loads(editor.call('list_brushes'))
    if len(after) != len(before):
        raise RuntimeError(f'{path.stem}: scaling lost brushes; refusing to save')
    low, high = bounds(after)
    if abs(low[2]) > 1e-4 or any(abs(high[i] - low[i] - target[i]) > 1e-4 for i in range(3)):
        raise RuntimeError(f'{path.stem}: scaled dimensions do not match profile')


def normalize_geometry(name, solids, brushes):
    """Apply the same profile to the custom thumbnail renderer's geometry."""
    if name not in PROFILES:
        return
    points = [p for pts, _ in solids for p in pts]
    lo = [min(p[i] for p in points) for i in range(3)]
    hi = [max(p[i] for p in points) for i in range(3)]
    factors = [PROFILES[name]['units'][i] / (hi[i] - lo[i]) for i in range(3)]
    def transform(p):
        return tuple((p[i] - (lo[2] if i == 2 else 0)) * factors[i] for i in range(3))
    solids[:] = [([transform(p) for p in pts], color) for pts, color in solids]
    brushes[:] = [([[transform(p) for p in face] for face in faces], color) for faces, color in brushes]
