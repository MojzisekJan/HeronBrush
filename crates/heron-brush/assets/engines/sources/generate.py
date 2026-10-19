#!/usr/bin/env python3
"""Unify the engine logos in this directory into 96px rounded cards in the parent."""
import io, os, subprocess
from PIL import Image, ImageDraw, ImageFont
import numpy as np

OUT = os.path.join(os.path.dirname(__file__), "..")
CANVAS = 96
RADIUS = 18


def load(path, w=512):
    if path.endswith(".svg"):
        buf = subprocess.run(["rsvg-convert", "-w", str(w), "-h", str(w), "--keep-aspect-ratio", path],
                             check=True, capture_output=True).stdout
        return drop_white(Image.open(io.BytesIO(buf)).convert("RGBA"))
    return drop_white(Image.open(path).convert("RGBA"))


def drop_white(im):
    """Makes a white background transparent, for a mark supplied as a flat image.

    A logo with no alpha cannot be trimmed and composites as a white square over the
    card, hiding it. Only touched when the image really is opaque throughout and its
    corner is white, so a mark that brought its own alpha is left alone."""
    a = np.array(im)
    if a[:, :, 3].min() < 255 or tuple(a[0, 0, :3]) < (250, 250, 250):
        return im
    # Near-white goes fully transparent; what is left keeps its own coverage, so an
    # antialiased edge fades out rather than showing a white fringe.
    # int32, not int16: the intermediate reaches -51000 for black and wraps in 16 bits,
    # which turns the mark inside out and leaves only its antialiased fringe.
    lightness = a[:, :, :3].min(axis=2).astype(np.int32)
    a[:, :, 3] = np.clip(255 - (lightness - 200) * 255 // 55, 0, 255)
    return Image.fromarray(a, "RGBA")


def trim(im):
    box = im.getbbox()
    return im.crop(box) if box else im


def recolor(im, rgb):
    a = np.array(im)
    a[:, :, 0], a[:, :, 1], a[:, :, 2] = rgb
    return Image.fromarray(a, "RGBA")


def card(logo=None, text=None, text_rgb=(110, 110, 116)):
    out = Image.new("RGBA", (CANVAS, CANVAS), (0, 0, 0, 0))
    mask = Image.new("L", (CANVAS, CANVAS), 0)
    ImageDraw.Draw(mask).rounded_rectangle([1, 1, CANVAS - 2, CANVAS - 2], radius=RADIUS, fill=255)
    out = Image.composite(Image.new("RGBA", (CANVAS, CANVAS), (244, 244, 245, 255)), out, mask)
    draw = ImageDraw.Draw(out)
    draw.rounded_rectangle([1, 1, CANVAS - 2, CANVAS - 2], radius=RADIUS, outline=(210, 210, 214, 255), width=2)
    if logo is not None:
        logo = trim(logo)
        inner = int(CANVAS * 0.68)
        scale = min(inner / logo.width, inner / logo.height)
        logo = logo.resize((max(1, int(logo.width * scale)), max(1, int(logo.height * scale))), Image.LANCZOS)
        out.alpha_composite(logo, ((CANVAS - logo.width) // 2, (CANVAS - logo.height) // 2))
    if text is not None:
        size = 46 if len(text) <= 1 else 30
        font = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", size)
        tb = draw.textbbox((0, 0), text, font=font)
        draw.text(((CANVAS - (tb[2] - tb[0])) / 2 - tb[0], (CANVAS - (tb[3] - tb[1])) / 2 - tb[1]),
                  text, font=font, fill=text_rgb + (255,))
    return out


card(logo=load("icon_color.svg")).save(f"{OUT}/godot.png")
card(logo=load("three_js_logo.svg")).save(f"{OUT}/threejs.png")
card(logo=load("raylib_logo.png")).save(f"{OUT}/raylib.png")

bevy = trim(load("bevy_logo_dark.svg"))
bird = bevy.crop((0, 0, 174, bevy.height))  # the wordmark starts near x=195
card(logo=recolor(bird, (61, 68, 81))).save(f"{OUT}/bevy.png")

card(logo=load("defold_logo.svg")).save(f"{OUT}/defold.png")
# Unity's mark is a solid black glyph; on the light card that reads as heavy, so
# it takes the same dark slate the Bevy bird does. Unreal's is the same: the
# black artwork of the two the user supplied, since the card is light.
card(logo=recolor(load("unity_logo.svg"), (61, 68, 81))).save(f"{OUT}/unity.png")
card(logo=recolor(load("unreal_logo_black.png"), (61, 68, 81))).save(f"{OUT}/unreal.png")
card(text="OBJ").save(f"{OUT}/obj.png")
# Cocos has no mark supplied yet; a lettered tile until one is.
card(logo=load("flax_logo.png")).save(f"{OUT}/flax.png")
# Supplied as black artwork on white; the same dark slate as the others.
card(logo=recolor(load("cocos_logo.png"), (61, 68, 81))).save(f"{OUT}/cocos.png")
card(text=".map", text_rgb=(70, 90, 120)).save(f"{OUT}/map.png")


def tile(path, out):
    """An app icon that is already a coloured rounded tile, cut out of its padding and
    given the cards' size and corners, as Stride's own tile is."""
    im = trim(Image.open(path).convert("RGBA"))
    im = im.resize((CANVAS - 4, CANVAS - 4), Image.LANCZOS)
    mask = Image.new("L", im.size, 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, im.width - 1, im.height - 1], radius=RADIUS - 2, fill=255)
    im.putalpha(Image.fromarray(np.minimum(np.array(im)[:, :, 3], np.array(mask)), "L"))
    canvas = Image.new("RGBA", (CANVAS, CANVAS), (0, 0, 0, 0))
    canvas.alpha_composite(im, (2, 2))
    canvas.save(out)


# GDevelop's mark, from the macOS app's icon.
tile("gdevelop_icon.png", f"{OUT}/gdevelop.png")
print("wrote godot, threejs, raylib, bevy, defold, unity, unreal, flax, cocos, obj, map, gdevelop")
