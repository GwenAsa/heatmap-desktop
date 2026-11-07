#!/usr/bin/env python3
"""Turns raw window captures into README-ready images: rounded corners, soft shadow, subtle gradient backdrop.
Usage: scripts/frame.py INPUT.png OUTPUT.png [padding]"""
import sys
from PIL import Image, ImageDraw, ImageFilter

def frame(src, dst, pad=44, radius=16):
    im = Image.open(src).convert("RGBA")
    w, h = im.size
    W, H = w + pad * 2, h + pad * 2
    bg = Image.new("RGBA", (W, H))
    px = bg.load()
    for y in range(H):                       # diagonal gradient #0b1220 -> #112a3f with a faint green glow bottom-right
        for x in range(W):
            t = (x / W + y / H) / 2
            r = int(11 + (17 - 11) * t); g = int(18 + (42 - 18) * t); b = int(32 + (63 - 32) * t)
            px[x, y] = (r, g, b, 255)
    shadow = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    ImageDraw.Draw(shadow).rounded_rectangle((pad, pad + 10, pad + w, pad + h + 10), radius, fill=(0, 0, 0, 170))
    bg = Image.alpha_composite(bg, shadow.filter(ImageFilter.GaussianBlur(18)))
    mask = Image.new("L", (w, h), 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, w - 1, h - 1), radius, fill=255)
    bg.paste(im, (pad, pad), mask)
    ImageDraw.Draw(bg).rounded_rectangle((pad, pad, pad + w - 1, pad + h - 1), radius, outline=(255, 255, 255, 40))
    bg.convert("RGB").save(dst, optimize=True)

if __name__ == "__main__":
    frame(sys.argv[1], sys.argv[2], int(sys.argv[3]) if len(sys.argv) > 3 else 44)
