import re
import struct
import sys

src = open(sys.argv[1]).read()

body = src[src.index("iso_font[256 * 16] = {"):]
body = body[body.index("{") + 1:body.index("};")]
data = bytes(int(v, 16) for v in re.findall(r"0x[0-9a-fA-F]{2}\b", body))
if len(data) != 4096:
    sys.exit(f"expected 4096 font bytes, got {len(data)}")

with open("xnu-console.psf", "wb") as f:
    # PSF2: magic, version, header size, flags (unicode table), glyphs,
    # char size, height, width.
    f.write(struct.pack("<IIIIIIII", 0x864AB572, 0, 32, 1, 256, 16, 16, 8))
    f.write(data)
    # Latin-1: glyph i is U+00XX; each table entry ends with 0xFFFF.
    for i in range(256):
        f.write(struct.pack("<HH", i, 0xFFFF))

with open("xnu-console.bdf", "w") as f:
    f.write("STARTFONT 2.1\n")
    f.write("FONT -nixdots-XNU Console-medium-r-normal--16-160-75-75-c-80-iso10646-1\n")
    f.write("SIZE 16 75 75\n")
    f.write("FONTBOUNDINGBOX 8 16 0 -4\n")
    f.write("STARTPROPERTIES 9\n")
    f.write('FAMILY_NAME "XNU Console"\n')
    f.write('WEIGHT_NAME "Medium"\n')
    f.write('SLANT "R"\n')
    f.write('SETWIDTH_NAME "Normal"\n')
    f.write("PIXEL_SIZE 16\n")
    f.write('SPACING "C"\n')
    f.write("FONT_ASCENT 12\n")
    f.write("FONT_DESCENT 4\n")
    f.write("DEFAULT_CHAR 0\n")
    f.write("ENDPROPERTIES\n")
    f.write("CHARS 256\n")
    for i in range(256):
        f.write(f"STARTCHAR U+{i:04X}\n")
        f.write(f"ENCODING {i}\n")
        f.write("SWIDTH 500 0\n")
        f.write("DWIDTH 8 0\n")
        f.write("BBX 8 16 0 -4\n")
        f.write("BITMAP\n")
        for row in data[i * 16:(i + 1) * 16]:
            f.write(f"{row:02X}\n")
        f.write("ENDCHAR\n")
    f.write("ENDFONT\n")
