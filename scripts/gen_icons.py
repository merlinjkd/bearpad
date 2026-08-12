#!/usr/bin/env python3
"""Generate BearPad app icons: rounded-rect dark tile, text lines, cyan caret."""
from PIL import Image, ImageDraw
import os

S = 1024
R = 205  # corner radius (~20% => squircle-ish rounded rect)
img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
d = ImageDraw.Draw(img)

# rounded rectangle background, subtle vertical gradient #1e1e1e -> #2d2d2d
top = (30, 30, 34)
bot = (45, 45, 49)
for y in range(S):
    t = y / S
    c = tuple(int(top[i] + (bot[i] - top[i]) * t) for i in range(3))
    d.rounded_rectangle([0, y, S - 1, y + 1], radius=R, fill=c + (255,))
# re-mask corners after per-row drawing (rounded_rectangle per row loses corner alpha)
mask = Image.new("L", (S, S), 0)
ImageDraw.Draw(mask).rounded_rectangle([0, 0, S - 1, S - 1], radius=R, fill=255)
img.putalpha(mask)

# subtle border
d = ImageDraw.Draw(img)
d.rounded_rectangle([0, 0, S - 1, S - 1], radius=R, outline=(92, 92, 96, 255), width=6)

TEXT = (212, 212, 212, 235)   # #d4d4d4
DIMMED = (212, 212, 212, 120) # dimmer lines
CARET = (128, 222, 234, 255)  # #80DEEA

lines = [
    (300, 350, 660, DIMMED),   # short line above caret
    (300, 470, 720, TEXT),     # caret line
    (300, 590, 560, TEXT),     # line below caret
    (300, 710, 640, DIMMED),
]
for x0, y, x1, col in lines:
    d.rounded_rectangle([x0, y, x1, y + 26], radius=13, fill=col)

# caret: vertical bar between the two middle lines
cx = 775
d.rounded_rectangle([cx, 436, cx + 30, 606], radius=15, fill=CARET)

# ---------------------------------------------------------------------------
os.makedirs("src-tauri/icons", exist_ok=True)
out = "src-tauri/icons"

def save_png(im, path):
    im.save(path, "PNG")

# Tauri bundle files
save_png(img.resize((32, 32), Image.LANCZOS), f"{out}/32x32.png")
save_png(img.resize((128, 128), Image.LANCZOS), f"{out}/128x128.png")
save_png(img.resize((256, 256), Image.LANCZOS), f"{out}/128x128@2x.png")
save_png(img.resize((512, 512), Image.LANCZOS), f"{out}/icon.png")

# Windows .ico: proper size ladder 16..256
sizes = [16, 24, 32, 48, 64, 128, 256]
img.resize((256, 256), Image.LANCZOS).save(
    f"{out}/icon.ico", format="ICO", sizes=[(s, s) for s in sizes]
)

# macOS .icns via iconset + iconutil
ic = f"{out}/BearPad.iconset"
os.makedirs(ic, exist_ok=True)
for s in [16, 32, 64, 128, 256, 512, 1024]:
    im = img.resize((s, s), Image.LANCZOS)
    im.save(f"{ic}/icon_{s}x{s}.png")
    im.save(f"{ic}/icon_{s}x{s}@2x.png")
os.system(f"iconutil -c icns {ic} -o {out}/icon.icns")
os.system(f"rm -rf {ic}")

print("icons written:", sorted(os.listdir(out)))
