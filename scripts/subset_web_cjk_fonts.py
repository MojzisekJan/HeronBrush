#!/usr/bin/env python3
"""Cut the Chinese, Japanese and Korean fonts of the web build down to the text it shows.

A browser page cannot read the system's fonts, so the web build carries Noto Sans CJK
in crates/heron-brush-web/fonts, holding only the characters of the interface
translations and the manual. Run this again after those change.

Those fonts are downloaded only for a Chinese, Japanese or Korean interface
(scripts/pack_web_bundle.py). Everyone else gets the faces in fonts/picker, which hold
just the languages' own names, so the language list still reads.

    python3 -m pip install fonttools
    python3 scripts/subset_web_cjk_fonts.py <folder with NotoSans{SC,TC,JP,KR}-Regular.otf>

The source fonts are the region subsets of Noto Sans CJK:
https://github.com/notofonts/noto-cjk/tree/main/Sans/SubsetOTF
"""
from pathlib import Path
import sys

import re

from fontTools import subset
from fontTools.ttLib import TTFont

REGIONS = ['SC', 'TC', 'JP', 'KR']
# The language whose name each region's picker face draws.
PICKER = {'SC': 'zh', 'TC': 'zh-TW', 'JP': 'ja', 'KR': 'ko'}


def main(source: Path):
    root = Path(__file__).resolve().parent.parent
    texts = [*(root / 'crates/editor_core/src/i18n').glob('*.rs'), *(root / 'docs/manual').glob('*.md')]
    characters = {chr(code) for code in range(0x20, 0x7f)}
    for path in texts:
        characters.update(c for c in path.read_text(encoding='utf8') if ord(c) > 0x2000)
    # Each region's language by its own name, as the language list shows it.
    names = dict(re.findall(r'language!\(\w+, "([^"]*)", "[^"]*", "([^"]*)"', (root / 'crates/editor_core/src/i18n.rs').read_text(encoding='utf8')))
    picker = {region: {c for c in names[code] if ord(c) > 0x2000} for region, code in PICKER.items()}
    options = subset.Options()
    options.layout_features = ['*']
    options.name_IDs = ['*']
    options.notdef_outline = True
    fonts = root / 'crates/heron-brush-web/fonts'
    (fonts / 'picker').mkdir(exist_ok=True)
    for region in REGIONS:
        for wanted, destination, layout in [(characters, fonts / f'NotoSans{region}-Regular.otf', ['*']), (picker[region], fonts / 'picker' / f'NotoSans{region}-Regular.otf', [])]:
            font = TTFont(source / f'NotoSans{region}-Regular.otf')
            options.layout_features = layout
            subsetter = subset.Subsetter(options)
            subsetter.populate(unicodes=[ord(c) for c in wanted])
            subsetter.subset(font)
            font.save(destination)
            print(f'{destination.relative_to(fonts)}: {len(font.getBestCmap())} characters, {destination.stat().st_size} bytes')


if __name__ == '__main__':
    main(Path(sys.argv[1]))
