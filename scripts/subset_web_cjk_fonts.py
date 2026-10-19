#!/usr/bin/env python3
"""Cut the Chinese, Japanese and Korean fonts of the web build down to the text it shows.

A browser page cannot read the system's fonts, so the web build carries Noto Sans CJK
in crates/heron-brush-web/fonts, holding only the characters of the interface
translations and the manual. Run this again after those change.

    python3 -m pip install fonttools
    python3 scripts/subset_web_cjk_fonts.py <folder with NotoSans{SC,TC,JP,KR}-Regular.otf>

The source fonts are the region subsets of Noto Sans CJK:
https://github.com/notofonts/noto-cjk/tree/main/Sans/SubsetOTF
"""
from pathlib import Path
import sys

from fontTools import subset
from fontTools.ttLib import TTFont

REGIONS = ['SC', 'TC', 'JP', 'KR']


def main(source: Path):
    root = Path(__file__).resolve().parent.parent
    texts = [*(root / 'crates/editor_core/src/i18n').glob('*.rs'), *(root / 'docs/manual').glob('*.md')]
    characters = {chr(code) for code in range(0x20, 0x7f)}
    for path in texts:
        characters.update(c for c in path.read_text(encoding='utf8') if ord(c) > 0x2000)
    options = subset.Options()
    options.layout_features = ['*']
    options.name_IDs = ['*']
    options.notdef_outline = True
    for region in REGIONS:
        font = TTFont(source / f'NotoSans{region}-Regular.otf')
        subsetter = subset.Subsetter(options)
        subsetter.populate(unicodes=[ord(c) for c in characters])
        subsetter.subset(font)
        destination = root / f'crates/heron-brush-web/fonts/NotoSans{region}-Regular.otf'
        font.save(destination)
        print(f'{destination.name}: {len(font.getBestCmap())} characters, {destination.stat().st_size} bytes')


if __name__ == '__main__':
    main(Path(sys.argv[1]))
