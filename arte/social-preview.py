#!/usr/bin/env python3
"""Gera a imagem de social preview do GitHub (1280x640) a partir da arte CC0.

Reprodutível, como os geradores de `arte/`: usa os GIFs do Zeca em `docs/img/`
(folha de contato «escuro | claro»; aqui só a metade escura, recortada), uma
fonte monoespaçada do sistema e um gradiente navy. Escreve
`docs/img/social-preview.png`.

    python3 arte/social-preview.py

A imagem é setada à mão em GitHub → Settings → Social preview (não há API).
"""
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

RAIZ = Path(__file__).resolve().parents[1]
OUT = RAIZ / "docs" / "img" / "social-preview.png"
W, H = 1280, 640
BG_SPRITE = (11, 12, 22)  # fundo sólido dos GIFs, para recortar

# Uma fonte monoespaçada, tentando as comuns de cada SO.
_FONTES = [
    "/System/Library/Fonts/SFNSMono.ttf",  # macOS (SF Mono)
    "/System/Library/Fonts/Menlo.ttc",
    "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",  # Linux
    "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
    "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
]


def _achar_fonte():
    for c in _FONTES:
        if Path(c).exists():
            return c
    return None


_FONTE = _achar_fonte()


def font(sz):
    return ImageFont.truetype(_FONTE, sz) if _FONTE else ImageFont.load_default()


def sprite(name, scale, tol=14):
    im = Image.open(RAIZ / "docs" / "img" / name).convert("RGBA")
    # Só a metade de fundo escuro (0..184; a clara fica em 196+), pulando a
    # linha fina de 3px no topo (y<38) acima do chapéu.
    im = im.crop((0, 38, 185, im.height))
    px = im.load()
    for y in range(im.height):
        for x in range(im.width):
            r, g, b, _ = px[x, y]
            if abs(r - BG_SPRITE[0]) <= tol and abs(g - BG_SPRITE[1]) <= tol and abs(b - BG_SPRITE[2]) <= tol:
                px[x, y] = (0, 0, 0, 0)
    im = im.crop(im.getbbox())
    w, h = im.size
    return im.resize((int(w * scale), int(h * scale)), Image.NEAREST)


def gerar():
    # fundo: gradiente vertical navy + brilho radial verde atrás dos sprites
    top, bot = (17, 27, 46), (7, 11, 20)
    img = Image.new("RGB", (W, H))
    d = ImageDraw.Draw(img)
    for y in range(H):
        t = y / (H - 1)
        d.line([(0, y), (W, y)], fill=tuple(int(top[i] * (1 - t) + bot[i] * t) for i in range(3)))
    glow = Image.new("L", (W, H), 0)
    gd = ImageDraw.Draw(glow)
    gx, gy, gr = 960, 330, 340
    for r in range(gr, 0, -4):
        gd.ellipse([gx - r, gy - r, gx + r, gy + r], fill=int(42 * (1 - r / gr)))
    img = Image.composite(Image.new("RGB", (W, H), (34, 211, 170)), img, glow)

    greens = sprite("zeca-idle.gif", 1.5)
    blue = sprite("zeca-azul.gif", 1.1)
    fire = sprite("zeca-fogo.gif", 1.1)
    img.paste(blue, (762, 150), blue)
    img.paste(fire, (1028, 330), fire)
    img.paste(greens, (878, 268), greens)

    d = ImageDraw.Draw(img)
    WHITE, GRAY, ACCENT = (240, 243, 248), (150, 162, 178), (52, 211, 170)
    d.text((80, 92), "Zeca", font=font(132), fill=WHITE)
    d.text((86, 250), "A pixel-art desktop pet", font=font(40), fill=GRAY)
    d.text((86, 300), "that reacts to ", font=font(40), fill=GRAY)
    w1 = d.textlength("that reacts to ", font=font(40))
    d.text((86 + w1, 300), "Claude Code", font=font(40), fill=ACCENT)
    d.text((86, 392), "celebrates · calls you · naps · bring your own sprite", font=font(24), fill=GRAY)
    d.line([(86, 470), (640, 470)], fill=(40, 52, 72), width=2)
    d.text((86, 496), "Linux · macOS · Windows", font=font(28), fill=WHITE)
    d.text((86, 540), "open source — MIT code, CC0 art", font=font(24), fill=GRAY)
    d.text((86, 584), "github.com/butkeraites/claude-pet", font=font(24), fill=ACCENT)

    img.save(OUT)
    return img


if __name__ == "__main__":
    gerar()
    print("escrito:", OUT)
