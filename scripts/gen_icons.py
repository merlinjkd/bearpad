#!/usr/bin/env python3
"""Compose BearPad icons: original white bear paw (from v0.2.8 artwork) on a
black rounded-corner tile (macOS-style), generated for every platform."""
from PIL import Image, ImageDraw
import os, subprocess

S = 1024
R = 229  # macOS Big Sur squircle-ish radius (~22.4%)

# original bear paw artwork from v0.2.8 (transparent bg)
orig = Image.open(os.path.join(os.path.dirname(__file__), "..", "src", "assets", "bearpaw.png")).convert("RGBA")
bbox = orig.getbbox()
paw = orig.crop(bbox)
# scale paw to ~78% of tile, centered
scale = (S * 0.78) / max(paw.size)
paw = paw.resize((max(1, int(paw.width * scale)), max(1, int(paw.height * scale))), Image.LANCZOS)

tile = Image.new("RGBA", (S, S), (0, 0, 0, 0))
d = ImageDraw.Draw(tile)
d.rounded_rectangle([0, 0, S - 1, S - 1], radius=R, fill=(0, 0, 0, 255))
tile.paste(paw, ((S - paw.width) // 2, (S - paw.height) // 2), paw)

out = os.path.join(os.path.dirname(__file__), "..", "src-tauri", "icons")
os.makedirs(out, exist_ok=True)

tile.resize((32, 32), Image.LANCZOS).save(f"{out}/32x32.png")
tile.resize((128, 128), Image.LANCZOS).save(f"{out}/128x128.png")
tile.resize((256, 256), Image.LANCZOS).save(f"{out}/128x128@2x.png")
tile.resize((512, 512), Image.LANCZOS).save(f"{out}/icon.png")

sizes = [16, 24, 32, 48, 64, 128, 256]
tile.resize((256, 256), Image.LANCZOS).save(f"{out}/icon.ico", format="ICO", sizes=[(s, s) for s in sizes])

ic = f"{out}/BearPad.iconset"
os.makedirs(ic, exist_ok=True)
for s in [16, 32, 64, 128, 256, 512, 1024]:
    im = tile.resize((s, s), Image.LANCZOS)
    im.save(f"{ic}/icon_{s}x{s}.png")
    im.save(f"{ic}/icon_{s}x{s}@2x.png")
subprocess.run(["iconutil", "-c", "icns", ic, "-o", f"{out}/icon.icns"], check=True)
subprocess.run(["rm", "-rf", ic], check=True)
print("icons written:", sorted(os.listdir(out)))
