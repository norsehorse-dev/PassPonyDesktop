#!/usr/bin/env python3
"""make-fonts.py -- regenerate the Simplified-Chinese font subset from the catalog.

    tools/make-fonts.py                         # find Noto Sans CJK on the system
    tools/make-fonts.py /path/to/NotoSansCJK-Regular.ttc

Inter, JetBrains Mono and Lucide ship pre-subset (Latin, mono, icons); this script
covers the one face that has to track the translations: Noto Sans SC, subset to
exactly the Han and full-width-punctuation characters the `zh-Hans` catalog draws,
plus the language picker's own native name. Han is the only script egui's built-in
fallback does not cover, so without this every Chinese glyph is a tofu box.

Output (safe to delete and re-run):

    passpony-desktop/assets/fonts/NotoSansSC-subset.ttf

Requires fonttools (`pip install fonttools`). The `theme::font_coverage` test reads
the same catalog and fails if a Chinese string uses a character the subset lacks —
so after adding or editing a zh-Hans string, run this and commit the result.
"""

import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
SRC = os.path.join(ROOT, "passpony-desktop", "src")
# The short-string table and the long-form Settings prose both carry Chinese.
CATALOG = os.path.join(SRC, "i18n_data.rs")
HELP = os.path.join(SRC, "i18n_help.rs")
OUT = os.path.join(ROOT, "passpony-desktop", "assets", "fonts", "NotoSansSC-subset.ttf")

# Where Noto Sans CJK might live, in priority order. The SC face inside the .ttc
# is font-number 2 (JP, KR, SC, TC, HK).
SOURCE_CANDIDATES = (
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    "/System/Library/Fonts/PingFang.ttc",
    "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
)
SC_FONT_NUMBER = 2


def chinese_chars():
    """Every character above the CJK-radicals floor drawn by the zh-Hans UI: the
    short-string ZH table, the long-form ZH_LONG prose, and the picker's native
    name (which lives in i18n.rs, not either catalog)."""
    chars = set("简体中文")
    blocks = [
        (CATALOG, r"pub const ZH: Strings = Strings \{(.*?)\n\};"),
        (HELP, r"pub const ZH_LONG: LongForm = LongForm \{(.*?)\n\};"),
    ]
    for path, pattern in blocks:
        m = re.search(pattern, open(path, encoding="utf-8").read(), re.S)
        if not m:
            print("could not find the ZH block in %s" % path, file=sys.stderr)
            sys.exit(1)
        chars |= {c for c in m.group(1) if ord(c) > 0x2E7F}
    return "".join(sorted(chars))


def main(argv):
    source = argv[1] if len(argv) == 2 else None
    if source is None:
        source = next((p for p in SOURCE_CANDIDATES if os.path.isfile(p)), None)
    if not source or not os.path.isfile(source):
        print(
            "Noto Sans CJK not found. Pass the path to NotoSansCJK-Regular.ttc,\n"
            "or install it (Debian/Ubuntu: `apt install fonts-noto-cjk`).",
            file=sys.stderr,
        )
        return 1

    text = chinese_chars()
    print("master  %s (SC face #%d)" % (source, SC_FONT_NUMBER))
    print("subsetting %d Simplified-Chinese characters" % len(text))

    cmd = [
        sys.executable, "-m", "fontTools.subset", source,
        "--font-number=%d" % SC_FONT_NUMBER,
        "--text=%s" % text,
        "--output-file=%s" % OUT,
        "--no-hinting", "--desubroutinize",
    ]
    subprocess.run(cmd, check=True)

    # Verify coverage the same way the Rust test will.
    from fontTools.ttLib import TTFont
    cmap = TTFont(OUT).getBestCmap()
    missing = [c for c in text if ord(c) not in cmap]
    if missing:
        print("  MISSING %d glyph(s): %s" % (len(missing), "".join(missing)), file=sys.stderr)
        return 1
    print("  %s  %d glyphs, %d bytes" % (OUT, len(cmap), os.path.getsize(OUT)))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
