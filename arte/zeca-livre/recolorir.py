#!/usr/bin/env python3
"""recolorir.py — variantes de paleta do Zeca original (CC0), de forma
determinística (não editado à mão): pega o `sheet.png` do Zeca e troca a rampa
do corpo (os verdes) e os acentos frios por uma rampa nova, mantendo a mesma
arte/forma. Os detalhes quentes (bico de palha, bochecha rosa) ficam.

É um exemplo de "traga seu sprite" para o README: a mesma arte do gerador
`zeca.py`, só com outra paleta. Reproduzível: mesmos bytes a cada execução.

Uso:  python3 recolorir.py <entrada_sheet.png> <variante> <saida_sheet.png>
      variante: azul | fogo
"""
import sys

from PIL import Image

# As variantes mapeiam a rampa do corpo do Zeca (verdes S/M/L/H), o acento frio
# (A petróleo) e a luz de recorte (R) para uma paleta nova. O resto (contorno,
# anel, rosa, palha, laranja, branco) fica.
VARIANTES = {
    "azul": {
        "#2B7D55": "#2B5F8E",  # S  verde sombra  -> azul escuro
        "#4DAF4B": "#3F8FD1",  # M  verde médio   -> azul
        "#8FD653": "#5BB6E8",  # L  verde claro   -> azul claro
        "#D3EF78": "#9CDAEF",  # H  brilho        -> ciano claro
        "#2E7487": "#2E5A8E",  # A  petróleo      -> azul acento
        "#A6E8D6": "#A6D2E8",  # R  rim           -> rim azul
    },
    "fogo": {
        "#2B7D55": "#8E2B2B",  # S  -> vermelho escuro
        "#4DAF4B": "#D13F3F",  # M  -> vermelho
        "#8FD653": "#EF6B5B",  # L  -> coral
        "#D3EF78": "#F5A98A",  # H  -> coral claro
        "#2E7487": "#8E4A2E",  # A  -> marrom quente
        "#A6E8D6": "#F0C8A6",  # R  -> rim quente
    },
}


def hex_para_rgb(h):
    h = h.lstrip("#")
    return (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16))


def recolorir(entrada, variante, saida):
    mapa = {hex_para_rgb(k): hex_para_rgb(v) for k, v in VARIANTES[variante].items()}
    im = Image.open(entrada).convert("RGBA")
    px = im.load()
    w, h = im.size
    trocados = 0
    for y in range(h):
        for x in range(w):
            r, g, b, a = px[x, y]
            if a > 0 and (r, g, b) in mapa:
                nr, ng, nb = mapa[(r, g, b)]
                px[x, y] = (nr, ng, nb, a)
                trocados += 1
    im.save(saida)
    print(f"recolorido ({variante}): {trocados} pixels, {saida}")


if __name__ == "__main__":
    if len(sys.argv) != 4 or sys.argv[2] not in VARIANTES:
        sys.exit("uso: recolorir.py <entrada.png> <azul|fogo> <saida.png>")
    recolorir(sys.argv[1], sys.argv[2], sys.argv[3])
