import re
import struct
import sys

src = open(sys.argv[1]).read()

body = src[src.index("iso_font[256 * 16] = {"):]
body = body[body.index("{") + 1:body.index("};")]
raw = bytes(int(v, 16) for v in re.findall(r"0x[0-9a-fA-F]{2}\b", body))
if len(raw) != 4096:
    sys.exit(f"expected 4096 font bytes, got {len(raw)}")

# XNU guarda cada fila con el píxel izquierdo en el bit menos significativo
# (bit 0). PSF espera el píxel izquierdo en el bit más significativo, así que
# se invierte el orden de bits de cada byte; sin esto los glifos salen en
# espejo.
def reverse_bits(b):
    return int(f"{b:08b}"[::-1], 2)

data = bytes(reverse_bits(b) for b in raw)

with open("xnu-console.psf", "wb") as f:
    # PSF2: magic, version, header size, flags (unicode table), glyphs,
    # char size, height, width.
    f.write(struct.pack("<IIIIIIII", 0x864AB572, 0, 32, 1, 256, 16, 16, 8))
    f.write(data)
    # Tabla Unicode de PSF2: por glifo, sus code points en UTF-8 seguidos de
    # 0xFF (no los uint16 + 0xFFFF de PSF1). Latin-1: el glifo i es U+00XX.
    for i in range(256):
        f.write(chr(i).encode("utf-8") + b"\xff")
