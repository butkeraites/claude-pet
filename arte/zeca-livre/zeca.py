#!/usr/bin/env python3
"""Zeca - papagaio malandro de mesa (estilo A "chibi redondo", refinado). Arte original.

Arte original feita com o Claude para o projeto bichinho, dedicada ao dominio publico pela
CC0 1.0 (LICENSE, ao lado).  Este arquivo E a fonte da arte: paleta, grades das pecas, rig e
animacoes; os PNG sao gerados a partir dele, nunca editados a mao.

Tudo e desenhado pixel a pixel em grades de texto (1 caractere = 1 pixel) e montado por um
rig de pecas: cada peca tem posicao absoluta no quadro idle 48x48 e cada quadro de animacao
e uma pose (dict de variantes + deslocamentos) que o motor compoe de tras para a frente.

    python3 zeca.py [--saida DIR]   # tudo: frames/, frames/escuro/, frames/folhas/, gifs/,
                                    # anims.json e vitrine.png em DIR (padrao: tmp/zeca-livre/
                                    # na raiz do repositorio, fora do git), e a verificacao
    python3 zeca.py --quadros DIR   # so os quadros dos dois visuais e o anims.json, sem o
                                    # ImageMagick; sai 1 se a verificacao reprovar.  E o que o
                                    # `cargo xtask zeca-livre` usa para montar as skins

A saida de --quadros e deterministica: os mesmos bytes a cada execucao (o `cargo xtask
zeca-livre --conferir` roda duas vezes e compara).  Os GIFs e a vitrine passam pelo
ImageMagick, que grava data nos arquivos: sao previas, nao entram na comparacao.

Rodada 2: 10 estados (entrou 'work' = trabalhando), lint de furos de fundo / recorte na borda /
recorte solto / orfaos / cores / margem nos dois visuais (lint, check_all) e releitura dos PNG
exportados (verify_exports).  Rodada 3 (no repositorio): o trecho 'respira' sem quadros
repetidos, efeito sem contorno e poeira fora do corpo (regra nova no lint), e as transicoes e
os gestos que o pet pede (aceno, tchau, bocejo, acordar, entrar e sair do trabalho, do sono e
da chamada, decolar e pousar).  Detalhes do que mudou e por que em notas.md.

So stdlib para compor, escrever e reler PNG (zlib + struct); ImageMagick ('magick') so para
ampliar, montar a vitrine e gerar os GIFs.  Ferramentas de revisao: dev_zoom, dev_strip, dev_sheet
(gravam em <saida>/processo/).
"""
import os, sys, struct, zlib, json, subprocess, shutil

HERE = os.path.dirname(os.path.abspath(__file__))
# saida padrao: tmp/zeca-livre/ na raiz do repositorio (gitignored); `--saida` troca
SAIDA = os.path.normpath(os.path.join(HERE, '..', '..', 'tmp', 'zeca-livre'))
CELL = 48
FLOOR = 44          # linha do contorno de baixo dos pes

# =================================================================== paleta ==
PALETTE = {
    'K': '#2B2136',   # contorno: ameixa escura (nunca #000)
    'A': '#2E7487',   # petroleo: pontas das penas (claro o bastante p/ fundo escuro)
    'S': '#2B7D55',   # verde sombra (frio)
    'M': '#4DAF4B',   # verde medio
    'L': '#8FD653',   # verde claro: cabeca e barriga
    'H': '#D3EF78',   # brilho quente
    'R': '#A6E8D6',   # luz de recorte (rim) fria; tambem gota de suor, z/Z e poeira
    'W': '#FFF8EC',   # branco quente
    '1': '#FFC4D0',   # rosa claro
    '2': '#F27FA6',   # rosa base
    '3': '#B8457E',   # rosa sombra
    'y': '#FBE7A1',   # palha clara
    'Y': '#E2B864',   # palha base
    'B': '#A8743E',   # palha sombra
    'o': '#F59A3A',   # laranja
    'O': '#C25A2C',   # laranja sombra
}
# cor extra SO da variante de tema escuro: anel de 1 px por fora do contorno
RING = '#5E5A86'
SOFT_CHARS = {'k'}          # contorno contextual: K na silhueta, sombra por dentro
RIM_CHARS = {'m': 'M', 's': 'S', 'a': 'A'}   # preenchimento que vira luz de recorte R na borda exposta

# ==================================================================== pecas ==
class Part:
    """Grade de texto em coordenadas absolutas do idle (x, y = canto sup. esq.).

    role  : papel no rig ('body', 'head', 'wing'...): variantes da mesma peca dividem o papel
    soft  : cor que o contorno contextual 'k' assume quando fica por dentro da silhueta
    over  : papeis sobre os quais o 'k' desta peca vira sombra (vinco) em vez de contorno duro
    """
    def __init__(self, name, x, y, grid, role, soft='S', over=(), fill=None):
        rows = [r.strip() for r in grid.strip('\n').split('\n')]
        w = max(len(r) for r in rows)
        for i, r in enumerate(rows):
            assert len(r) == w, f'{name}: linha {i} tem {len(r)} != {w}: {r!r}'
            for ch in r:
                assert ch == '.' or ch in PALETTE or ch in SOFT_CHARS or ch in RIM_CHARS or ch == '~', (name, ch)
        self.name, self.x, self.y, self.rows, self.role = name, x, y, rows, role
        self.w, self.h = w, len(rows)
        self.soft = soft          # cor do contorno 'k' quando fica por dentro
        self.over = set(over)     # pecas sobre as quais o contorno vira sombra
        self.fill = fill          # cor do contorno 'k' coberto pelo chapeu (cabeca: o proprio preenchimento)

P = {}
def part(name, x, y, grid, role=None, soft='S', over=(), fill=None):
    P[name] = Part(name, x, y, grid, role or name.split('_')[0].lower(), soft, over, fill)
    return P[name]


# ------------------------------------------------------------ chapeu (gerador)
# O boater e sempre o mesmo objeto: aba de 24 px de palha + 2 tampas, copa de 12 px
# (contorno incluso) com tampo claro, lateral e fita laranja de 2 fileiras.  A
# inclinacao e um cisalhamento em degraus de comprimento fixo (run) alinhados na ponta
# da aba: 24 divide por 2,3,4,6,8,12, entao todos os degraus tem o mesmo tamanho e
# tampo, fita e aba andam juntos (sem "escada" desencontrada).
HAT_W, HAT_H = 28, 19
HAT_SEAT = (13, 9)          # ponto da grade que pousa na cabeca (meio da copa, linha da aba)
BRIM0, BRIM1 = 2, 25        # 24 px de palha (divisivel por 2,3,4,6,8,12: degraus iguais)
CROWN0, CROWN1 = 9, 20      # copa de 12 px com contorno: cai em 2 degraus na inclinacao do idle

def hat_grid(run=6, dirn=1, shadow=True, crun=None):
    """run: px por degrau da aba (0 = reto); dirn=+1 sobe para a direita (inclinado p/ tras),
    dirn=-1 desce para a direita (caido para a frente).
    crun: degrau proprio do tampo da copa, alinhado no MEIO da copa (6 = um degrau so, 6+6;
    0 = tampo reto).  A fita fica sempre sentada na aba; a diferenca de altura entre tampo e
    aba e absorvida pela lateral (Y) da copa.  None = a copa segue os degraus da aba (v1)."""
    W = HAT_W
    nseg = 1 if run == 0 else (BRIM1 - BRIM0 + 1) // run
    def seg(x):
        if run == 0:
            return 0
        return min(max((x - BRIM0) // run, 0), nseg - 1)
    sm = seg(HAT_SEAT[0])
    def off(x):                 # deslocamento vertical da coluna x (0 no ponto de apoio)
        return -dirn * (seg(x) - sm)
    cw = CROWN1 - CROWN0 + 1    # 12
    def ctop(x):                # nivel do tampo da copa na coluna x
        xi = min(max(x, CROWN0 + 1), CROWN1 - 1)
        if crun is None:
            return off(xi)
        if crun == 0 or run == 0:
            return off(HAT_SEAT[0])
        k = (xi - CROWN0) // crun                       # 0..(cw/crun - 1)
        kc = (HAT_SEAT[0] - CROWN0) // crun
        steps = cw // crun
        # mesma inclinacao total da aba sob a copa, distribuida em degraus iguais
        rise = off(CROWN0 + 1) - off(CROWN1 - 1)
        per = rise / max(steps - 1, 1) if steps > 1 else 0
        return off(HAT_SEAT[0]) - round((k - kc) * per)
    g = [['.'] * W for _ in range(HAT_H)]
    def put(x, y, c):
        if 0 <= y < HAT_H:
            g[y][x] = c
    R0 = HAT_SEAT[1]
    for x in range(BRIM0 - 1, BRIM1 + 2):
        t = R0 + off(min(max(x, BRIM0), BRIM1))
        if x in (BRIM0 - 1, BRIM1 + 1):          # tampas arredondadas
            put(x, t + 1, 'K'); put(x, t + 2, 'K')
            continue
        put(x, t, 'K')
        put(x, t + 1, 'y')
        put(x, t + 2, 'B' if x > BRIM1 - 3 else 'Y')
        put(x, t + 3, 'K')
        if shadow:
            put(x, t + 4, '~')
        if CROWN0 <= x <= CROWN1:
            top = R0 + ctop(x) - 5                       # contorno de cima da copa
            bot = t - 1                                  # ultima fileira da fita (sentada na aba)
            if crun is None:
                bot = R0 + off(min(max(x, CROWN0 + 1), CROWN1 - 1)) - 1
            if x in (CROWN0, CROWN1):
                # lateral: do tampo (o mais alto entre esta coluna e a vizinha) ate a aba
                nb = CROWN0 + 1 if x == CROWN0 else CROWN1 - 1
                top2 = min(top, R0 + ctop(nb) - 5)
                for r in range(top2, max(bot, R0 + off(min(max(nb, BRIM0), BRIM1)) - 1) + 1):
                    put(x, r, 'K')
            else:
                last = x >= CROWN1 - 2
                put(x, top, 'K')
                put(x, top + 1, 'y')
                for r in range(top + 2, bot - 1):
                    put(x, r, 'B' if last else 'Y')
                put(x, bot - 1, 'O' if last else 'o')
                put(x, bot, 'O' if last else 'o')
    return '\n'.join(''.join(r) for r in g)

HAT_SEAT_IDLE = (23, 16)    # rodada 2: chapeu 1 px mais alto em relacao ao olho (fresta sob a aba)

def hat_part(name, run, dirn, shadow=True, seat=None, crun=6):
    """Registra um chapeu cujo HAT_SEAT cai em `seat` (coordenadas absolutas do idle)."""
    seat = seat or HAT_SEAT_IDLE
    return part(name, seat[0] - HAT_SEAT[0], seat[1] - HAT_SEAT[1], hat_grid(run, dirn, shadow, crun), role='hat')

# ================================================================ grades ==
# Coordenadas absolutas do quadro idle (chao: contorno de baixo dos pes em y=44).
# '.' vazio | 'k' contorno contextual | '~' sombra projetada (escurece o que esta embaixo)
# m/s/a = M/S/A que viram luz de recorte R quando a borda fica exposta

# ---- cauda: leque curto, 3 pontas petroleo, nao encosta no chao
part('TAIL', 7, 35, """
......kkkk
....kkMMMM
..kkMMMMMM
.kAMMSMMMk
kAAMSSMMk.
kAAKAAAk..
.kk.kkk...
""")

# ---- pes: o de perto planta o peso; o de longe vai a frente, relaxado
part('FOOT_N', 18, 42, """
..KoK..
KoooOOK
.KKKKK.
""")
part('FOOT_F', 28, 42, """
.KOK...
KOOOOOK
.KKKKK.
""")
# pe de longe batendo o ritmo: dedos levantados (o calcanhar fica)
part('FOOT_F_UP', 28, 39, """
......K.
.....KOK
....KOK.
.KOKOK..
KOOOOK..
.KKKK...
""", role='foot')

# ---- corpo em pera: estreito sob a cabeca, barriga redonda, peito estufado
part('BODY', 14, 30, """
......kkkkkkk.......
....kkMMMMMMMkk.....
...kMMMMMMMMMMMk....
..kMMLLLLLLLLLLLk...
..kMLLLLLLLLLHHLMk..
.kMLLLLLLLLLLHHHLMk.
.kMLLLLLLLLLLLHLLMk.
kSMLLLLLLLLLLLLLLMk.
kSMLLLLLLLLLLLLLLmk.
.kSMLLLLLLLLLLLLMmk.
..kSSMMMMmmmmmmmsk..
...kkkkkkkkkkkkkk...
""")

# ---- asa dobrada: oval com 3 pontas de pena petroleo atras
part('WING', 13, 32, """
....kkkkk..
...kLLMMMk.
..kLLMMMMMk
.kMMMMMMMMk
kMMMMMMMMMk
kMMMMMMMMk.
kAMMMMMMk..
kAAMMMSk...
.kAAASk....
..kkkk.....
""", over=('body',))

# ---- cabeca: esfera com luz de cima-esquerda (brilho em mancha, sombra em crescente)
part('HEAD', 14, 18, """
.......kkkkkk.......
.....kkLLLLLLkk.....
...kkLLLLLLLLLLkk...
..kLHHHHLLLLLLLLLk..
.kLLHHHLLLLLLLLLLMk.
.kLLLHHLLLLLLLLLLMk.
kLLLLLLLLLLLLLLLLMMk
kLLLLLLLLLLLLLLLMMMk
kLLLLLLLLLLLLLLLMMSk
kMLLLLLLLLLLLLLMMMSk
.kMLLLLLLLLLLLMMMSk.
.kMMLLLLLLLLLMMMSSk.
..kMMMLLLLLMMMMSSk..
...kkMMMMMMMMSSkk...
.....kkSSSSSSkk.....
.......kkkkkk.......
""", over=('body', 'wing'), fill='L')

part('BLUSH', 19, 29, """
.11.
1122
""")

# ---- gravata-borboleta grande, em 3/4 (laco de tras mais estreito)
part('BOW', 26, 31, """
KKSSSSSKK.
K2KKKSSK3K
K223KKK33K
K233K3K33K
K333K3K33K
K333KKK33K
K3KK...K3K
KK......K.
""")

# ---- bico em gancho (todos com a raiz na mesma coluna e a mesma dobradica)
part('BEAK', 31, 20, """
kKKKK....
k1111KK..
k111222K.
k1222222K
k2222223K
K2222223K
kKK22223K
k33KK223K
.kk33K23K
...kkK3K.
......K..
""")
# queixo erguido (cabeca inclinada p/ tras ~20 graus): gancho aponta mais para a frente
part('BEAK_UP', 31, 20, """
kKKKKK....
k11111KK..
k1122222K.
k12222222K
K22222223K
kKKK22233K
k333KK33K.
.kkkkK3K..
......K...
""", role='beak', over=('head',))
# piando: mandibula de baixo abre na dobradica (presa ao rosto) e o gancho sobe 1 px (o bico
# de cima do papagaio e articulado): fica um V de fundo na frente, garganta K e lingua 3 na base
part('BEAK_OPEN', 31, 20, """
kKKKK....
k1111KK..
k111222K.
k1222222K
k2222223K
K2222223K
kKKK2223K
kKKK..23K
kK3K..K3K
k33K...K.
k222K....
k2223K...
.kkkK....
""", role='beak', over=('head',))
# bocao: grito de susto / comemoracao (mesma familia, mandibula 1 px mais baixa, garganta funda)
part('BEAK_WIDE', 31, 20, """
kKKKK....
k1111KK..
k111222K.
k1222222K
k2222223K
K2222223K
kKKK2223K
kKKK..23K
kKKK..K3K
kK3K...K.
k33K.....
k222K....
k2223K...
.kkkK....
""", role='beak', over=('head',))

# ---- olhos (8x9 ancorados em (23, 21); arregalado e 10x10)
part('EYE', 22, 21, """
..KKKK..
.KWWWWK.
KWWWWKKK
KWWWKWKK
KWWWKKKK
KWWWKKKK
KWWWWKKK
.KWWWWK.
..KKKK..
""")
# malandro: palpebra a meio mastro, linha fina
part('EYE_SLY', 22, 21, """
........
........
........
KKKKKKKK
KWWWKKKK
KWWWKWKK
KWWWWKKK
.KWWWWK.
..KKKK..
""", role='eye')
part('EYE_HALF', 22, 21, """
........
........
........
........
.KKKKKK.
KKWWKKKK
.KWWWKK.
..KKKK..
........
""", role='eye')
part('EYE_SHUT', 22, 21, """
........
........
........
........
........
KK....KK
.KKKKKK.
........
........
""", role='eye')
part('EYE_WIDE', 21, 20, """
..KKKKKK..
.KWWWWWWK.
KWWWWWWWWK
KWWWWWWWWK
KWWWWWKKWK
KWWWWKKKWK
KWWWWWKKWK
KWWWWWWWWK
.KWWWWWWK.
..KKKKKK..
""", role='eye')
part('EYE_HAPPY', 22, 21, """
........
........
........
..KKKK..
.KK..KK.
KK....KK
........
........
........
""", role='eye')
part('EYE_DOWN', 22, 21, """
..KKKK..
.KWWWWK.
KWWWWWWK
KWWWWWWK
KWWWWKKK
KWWWKWKK
KWWWKKKK
.KWWKKK.
..KKKK..
""", role='eye')


# ---- variantes de corpo (squash & stretch) -------------------------------------------
# agachado: 2 px mais largo de cada lado, 2 fileiras mais baixo, senta sobre os pes
part('BODY_SQ', 13, 33, """
......kkkkkkkkkk......
...kkkMMMMMMMMMMkkk...
..kMMMMMMMMMMMMMMMMk..
.kMMLLLLLLLLLLLLLHLMk.
kSMLLLLLLLLLLLLLHHHLMk
kSMLLLLLLLLLLLLLLHLLMk
kSMLLLLLLLLLLLLLLLLMmk
.kSMLLLLLLLLLLLLLMmsk.
..kSSMMMMmmmmmmmmssk..
....kkkkkkkkkkkkkk....
""", role='body')
# esticado: 2 px mais estreito, 2 fileiras mais alto, pernas aparecem
part('BODY_ST', 15, 27, """
.....kkkkkkkk.....
...kkMMMMMMMMkk...
..kMMMMMMMMMMMMk..
.kMMLLLLLLLLLLLMk.
.kMLLLLLLLLLLHHLk.
kSMLLLLLLLLLLHHHMk
kSMLLLLLLLLLLLHLMk
kSMLLLLLLLLLLLLLMk
kSMLLLLLLLLLLLLLMk
kSMLLLLLLLLLLLLMmk
.kSMLLLLLLLLLLMmk.
.kSMMLLLLLLLLMmsk.
..kSSMMMMMmmmssk..
....kkkkkkkkkk....
""", role='body')
# quanto cabeca / asa / cauda andam com cada corpo (relativo ao idle)
BODY_META = {
    'BODY':    dict(neck=(0, 0),  wing=(0, 0),  tail=(0, 0)),
    'BODY_SQ': dict(neck=(0, 3),  wing=(0, 3),  tail=(-2, 0)),   # cauda fica 2+ px acima do chao
    'BODY_ST': dict(neck=(0, -3), wing=(0, -2), tail=(1, -2)),
}
# pes no ar: dedos pendurados para baixo-frente (pernas desenhadas ate o corpo)
part('FOOT_N_AIR', 19, 43, """
.KoK.
KooOK
.KoOK
..KK.
""", role='foot')
part('FOOT_F_AIR', 27, 43, """
.KOK.
KOOOK
.KOOK
..KK.
""", role='foot')
# pes recolhidos (voo): dois tocos laranja colados na barriga
part('FOOT_N_TUCK', 21, 41, """
KooK
KooK
.KK.
""", role='foot')
part('FOOT_F_TUCK', 26, 41, """
KOOK
KOOK
.KK.
""", role='foot')
LEG_X = {'FOOT_N': 20, 'FOOT_F': 29, 'FOOT_F_UP': 29, 'FOOT_N_AIR': 21, 'FOOT_F_AIR': 29}

# asa de longe (voo): mesma forma, mais escura (S com pontas A), pintada atras do corpo
FAR_RECOLOR = {'L': 'S', 'H': 'S', 'M': 'S', 'S': 'A', 'm': 'S', 's': 'A', 'a': 'A'}
def far_wing(name, src_name, dx, dy):
    p = P[src_name]
    grid = '\n'.join(''.join(FAR_RECOLOR.get(c, c) for c in r) for r in p.rows)
    return part(name, p.x + dx, p.y + dy, grid, role='farwing')

# ---- asas abertas (voo / comemoracao): pintadas atras da cabeca, presas nas costas
part('WING_UP', 2, 17, """
.kk.................
kAAk.kk.............
kAAAkAAk.kk.........
.kSAAKAAAkAkk.......
.kSSAAKAAAAAMk......
..kSSSAAAAAAMMk.....
..kSMMSSSAAAMMMk....
...kSMMMMSSSSMMMk...
...kSMMMMMMMMMMMMk..
....kSMMMMMMMLLLLLk.
.....kSMMMMMMLLLLLk.
......kSMMMMMLLLLLk.
.......kSSMMMMLLLLk.
........kkSMMMLLLk..
..........kkMMMLk...
............kkkk....
""", role='wing', over=('body',))
part('WING_MID', 2, 27, """
.....kkkkkkkkkkkk...
...kkLLLLLLLLLLLMk..
..kMLMMMMMMMMMMMMMk.
.kMMMMMMMMMMMMMMMMk.
kSSMMSSMMMMMMMMMMk..
kASSSSSSSMMMMMMMk...
kAAKASSKASSSSSSk....
.kAkAAkAAAkkkkkk....
..k.kk.kkkk.........
""", role='wing', over=('body',))
# asa de longe usa a forma diagonal antiga (so a borda de ataque aparece acima das costas)
part('WING_DOWN_DIAG', 6, 31, """
........kkkkk..
......kkMMMLk..
.....kMMMMMLk..
....kSMMMMMLk..
...kSMMMMMMLk..
...kSMMMMMMk...
..kSSMMMMMMk...
..kASSMMMMk....
.kAASSSMMMk....
.kAAKASSSk.....
kAAkAAkSk......
kAk.kAk.k......
.k...k.........
""", role='wing', over=('body',))
# batida para baixo: a asa de perto desce POR BAIXO da barriga (pontas sob o corpo, longe da cauda)
part('WING_DOWN', 9, 31, """
......kkkkk.
.....kMMMLLk
....kSMMMMLk
....kSMMMMLk
...kSMMMMMMk
...kSMMMMMk.
...kSSMMMMk.
..kASSMMMk..
..kAASSSMk..
..kAAKASk...
.kAAkAAk....
.kAk.kAk....
..k...k.....
""", role='wing', over=('body',))
far_wing('WING_FAR_UP', 'WING_UP', 3, -2)
far_wing('WING_FAR_MID', 'WING_MID', 2, -3)
far_wing('WING_FAR_DOWN', 'WING_DOWN_DIAG', 4, -7)


# ---- bicada: bico a 45 graus e apontando para o chao (raiz desce junto da cara)
part('BEAK_45', 28, 27, """
....kK...
...k11K..
..k1111K.
.k111112K
k2222211K
kKK22222K
k3K22222K
k3K22222K
.k3K2223K
.kkK223K.
...K233K.
...KK3K..
....K....
""", role='beak', over=('head',))
part('BEAK_DOWN', 28, 28, """
......kK..
....kk11K.
..kk11111K
.kK222211K
kk3K22222K
k33K22222K
.k3K22222K
.k3K22223K
..kK2223K.
...K2233K.
...KK33K..
....KKK...
""", role='beak', over=('head',))

# ---- susto: corpo eriçado (borda em serrilha nas costas e na barriga) e topete em pe
part('BODY_PUFF', 13, 29, """
.......kkkkkkk........
.....kkMMMMMMMkk......
....kMMMMMMMMMMMk.....
..kkMMLLLLLLLLLLLk....
.kSMMLLLLLLLLLLHHLMk..
kSSMLLLLLLLLLLLHHHLMk.
.kSMLLLLLLLLLLLLHLLMk.
kSSMLLLLLLLLLLLLLLLMMk
.kSMLLLLLLLLLLLLLLLMmk
kSSMLLLLLLLLLLLLLLMmsk
.kSSMLLLLLLLLLLLLMmssk
..kkSSMMMMMMMmmmmsskk.
....kkkkkkkkkkkkkkk...
""", role='body')
BODY_META['BODY_PUFF'] = dict(neck=(0, -1), wing=(-1, -1), tail=(-1, -1))
# inclinado para a frente (bicada / trabalhando): cisalhamento do BODY - ombro +3/+2, barriga parada
part('BODY_LEAN', 14, 32, """
.........kkkkkkk.....
.......kkMMMMMMMkk...
.....kkMMLLLLLLLLLk..
.....kMLLLLLLLLLHHMk.
....kMLLLLLLLLLLHHHMk
...kMLLLLLLLLLLLLHLMk
..kSMLLLLLLLLLLLLLLMk
..kSMLLLLLLLLLLLLLmk.
...kSSMMMMmmmmmmmsk..
....kkkkkkkkkkkkkk...
""", role='body')
BODY_META['BODY_LEAN'] = dict(neck=(0, 0), wing=(1, 1), tail=(0, 0))
part('CREST', 18, 13, """
.k....k....
kLk..kLk.k.
kLLk.kLLkLk
.kLLkkLLLLk
..kLLLLLLk.
""", role='crest', over=('head',))
part('NAPE', 11, 22, """
.k.....
kMk....
.kMk...
kMMMk..
.kMMk..
kMMk...
.kk....
""", role='nape', over=('head', 'body', 'wing'))
part('TAIL_FAN', 4, 32, """
........kkkkkk
......kkMMMMMM
...kkkMMSSMMMM
.kkASSSSSMMMMM
kAAAASSAAMMMMk
kAkAAAAAKMMMk.
.k.kkAkkAkkk..
.....k..k.....
""", role='tail')
part('TAIL_UP', 6, 30, """
.k........
kAk.......
kAAkk.....
kAAAAkk...
.kAAAMMkk.
kAAAMMMMMk
kAAAMSSMMM
.kkAASSSkk
...kkkk...
""", role='tail')

# ---- efeitos (ancorados no quadro, nao no corpo)
part('FX_NOTE', 0, 0, """
...KKK..
...KyyK.
...KyKyK
...KyKK.
.KKKyK..
KyyyyK..
KyyyKK..
.KKKK...
""", role='fx')
part('FX_BANG', 0, 0, """
KKKK
KooK
KooK
KooK
KOOK
KKKK
.KK.
KooK
KKKK
""", role='fx')
part('FX_Z', 0, 0, """
AAAA
..A.
.A..
AAAA
""", role='fx')
part('FX_Z2', 0, 0, """
AAA
.A.
A..
AAA
""", role='fx')
part('FX_SWEAT', 0, 0, """
.K..
KRK.
KRRK
KRRK
.KK.
""", role='fx')
part('FX_SPARK', 0, 0, """
..K..
.KyK.
KyyyK
.KyK.
..K..
""", role='fx')
part('FX_SPARK2', 0, 0, """
.KK.
KyyK
KyyK
.KK.
""", role='fx')
part('FX_CRUMB', 0, 0, """
KKKK
KyyK
KKKK
""", role='fx')
part('FX_SEED', 0, 0, """
.KKKK.
KyyYYK
.KKKK.
""", role='fx')
part('FX_CONF1', 0, 0, """
22
""", role='fx')
part('FX_CONF2', 0, 0, """
o
o
""", role='fx')
part('FX_CONF3', 0, 0, """
AA
""", role='fx')
part('FX_DUST', 0, 0, """
.KKK.
KWWWK
.KKK.
""", role='fx')
part('FX_DUST2', 0, 0, """
.KK.
KRRK
.KK.
""", role='fx')
# faisca pequena de trabalho (petroleo, sem contorno: le nos dois temas)
part('FX_SPARK_A', 0, 0, """
.A.
AAA
.A.
""", role='fx')
part('FX_DOT_A', 0, 0, """
AA
AA
""", role='fx')
# tecla de teclado que ele bica enquanto o Claude trabalha (solta / apertada)
part('FX_KEY', 0, 0, """
.KKKKKKK.
KWWWWWWWK
KWAAAAAWK
KWWWWWWWK
KRRRRRRRK
.KKKKKKK.
""", role='fx')
part('FX_KEY_DN', 0, 0, """
.KKKKKKK.
KWAAAAAWK
KWWWWWWWK
KRRRRRRRK
.KKKKKKK.
""", role='fx')
part('FX_JOLT', 0, 0, """
KK......
KyK..KK.
.KyK.KyK
..KK.KyK
......K.
KKKK....
KyyyK...
.KKKK...
""", role='fx')

# ---- chapeus (gerados): inclinacoes do mesmo objeto
# (run da aba, direcao, modo do tampo).  Rodada 2: TODOS com tampo reto - a copa fica um
# retangulo limpo (o tampo chato e a marca do boater) e so a aba escalona; nada de "chamine"
# (8+2), "dois tijolos" ou escada de 3 degraus, e a copa nao "pula" quando o chapeu troca de
# inclinacao no meio de uma animacao (comparacao em processo/v2_r16_* e v2_r31/r32)
HATS = {
    'HAT_6':  (6, 1, 0),         # NEUTRO (rodada 2): inclinado para tras, o malandro "encostado"
    'HAT':    (8, 1, 0),         # menos inclinado (assentando depois do pouso)
    'HAT_4':  (4, 1, 0),         # bem inclinado (queixo erguido na ginga / rodopio no ar)
    'HAT_12': (12, 1, 0),        # quase reto
    'HAT_0':  (0, 1, 0),         # reto (pouso, prensado)
    'HAT_F6': (6, -1, 0),        # caido para a frente (sono, susto)
    'HAT_F4': (4, -1, 0),        # bem caido para a frente (golpe da bicada, escorregando)
}
for n, (run, dirn, crun) in HATS.items():
    hat_part(n, run, dirn, crun=crun)
    hat_part(n + '_AR', run, dirn, shadow=False, crun=crun)      # sem sombra: fora da cabeca

# ================================================================== motor ==
# sombra projetada ('~' nas grades): escurece um degrau o que ja esta pintado
DARKEN = {'H': 'L', 'L': 'M', 'M': 'S', 'S': 'A', 'A': 'K', 'R': 'M', 'W': 'R',
          '1': '2', '2': '3', '3': 'K', 'y': 'Y', 'Y': 'B', 'B': 'O', 'o': 'O', 'O': 'K'}
SHADOW_RECEIVERS = {'head', 'body'}
RIM_ON = True

def compose(layers):
    """layers: [(Part, dx, dy)] de tras para a frente -> grade 48x48 de chars (None = vazio)."""
    cv = [[None] * CELL for _ in range(CELL)]
    own = [[-1] * CELL for _ in range(CELL)]
    shade = set()                      # pixels que recebem a sombra projetada da aba ('~')
    for li, (p, dx, dy) in enumerate(layers):
        for j, row in enumerate(p.rows):
            for i, ch in enumerate(row):
                if ch == '.':
                    continue
                x, y = p.x + i + dx, p.y + j + dy
                if not (0 <= x < CELL and 0 <= y < CELL):
                    if ch == '~':
                        continue
                    raise ValueError(f'{p.name} sai da celula em {(x, y)}')
                if ch == '~':
                    if cv[y][x] is not None and layers[own[y][x]][0].role in SHADOW_RECEIVERS:
                        shade.add((x, y))
                    continue
                cv[y][x] = ch
                own[y][x] = li
                shade.discard((x, y))          # peca nova por cima: a sombra era da de baixo
    # o bico nunca fica por cima da aba: com o chapeu escorregado sobre o rosto, o que sobra do
    # bico acima da aba (mesma coluna, 1-3 px) e recortado
    for y in range(CELL):
        for x in range(CELL):
            if own[y][x] >= 0 and layers[own[y][x]][0].role == 'beak':
                for d in (1, 2, 3):
                    if y + d < CELL and own[y + d][x] >= 0 and layers[own[y + d][x]][0].role == 'hat' \
                            and own[y + d][x] > own[y][x]:
                        cv[y][x] = None; own[y][x] = -1; shade.discard((x, y))
                        break
    # contorno contextual: 'k' vira K na silhueta ou contra peca de tras de outro material,
    # e vira a cor 'soft' da peca quando fica por cima de uma peca listada em 'over'
    out = [r[:] for r in cv]
    for y in range(CELL):
        for x in range(CELL):
            if cv[y][x] != 'k':
                continue
            li = own[y][x]
            p = layers[li][0]
            hard = False
            for dy in (-1, 0, 1):
                for dx in (-1, 0, 1):
                    if dx == dy == 0:
                        continue
                    X, Y = x + dx, y + dy
                    if not (0 <= X < CELL and 0 <= Y < CELL) or cv[Y][X] is None:
                        hard = True
                    else:
                        lj = own[Y][X]
                        if lj < li and layers[lj][0].role not in p.over and layers[lj][0].role != p.role:
                            hard = True
            if hard:
                out[y][x] = 'K'
            elif p.fill and any(_in(x + dx, y + dy) and own[y + dy][x + dx] > li and
                                layers[own[y + dy][x + dx]][0].role == 'hat' for dx, dy in N8):
                out[y][x] = p.fill          # contorno da cabeca sob a aba: e testa, nao vinco
            else:
                out[y][x] = p.soft
    # luz de recorte marcada a mao: m/s/a viram R quando encostam (4-viz) num K DA MESMA PECA
    # que esta na silhueta dela: encosta no vazio ou numa peca de tras de outro papel (a perna
    # de longe atras da barriga nao apaga o recorte).  Senao voltam a cor base.
    def exposed_k(X, Y, li):
        if not (0 <= X < CELL and 0 <= Y < CELL) or out[Y][X] != 'K' or own[Y][X] != li:
            return False
        for ex, ey in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            U, V = X + ex, Y + ey
            if not (0 <= U < CELL and 0 <= V < CELL) or out[V][U] is None:
                return True
            lj = own[V][U]
            if lj < li and layers[lj][0].role != layers[li][0].role:
                return True
        return False
    for y in range(CELL):
        for x in range(CELL):
            c = out[y][x]
            if c in RIM_CHARS:
                edge = any(exposed_k(x + dx, y + dy, own[y][x]) for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)))
                out[y][x] = 'R' if (edge and RIM_ON) else RIM_CHARS[c]
    # bolsao de ate 3 px fechado por contorno entre a aba e a cabeca (degrau da aba sobre a
    # cabeca redonda quando o chapeu atrasa para cima): o contorno se encontra ali
    for comp in holes(out):
        if len(comp) <= 3 and any(_in(x + dx, y + dy) and own[y + dy][x + dx] >= 0 and
                                  layers[own[y + dy][x + dx]][0].role == 'hat'
                                  for x, y in comp for dx, dy in N4):
            for x, y in comp:
                out[y][x] = 'K'
    # sombra projetada por ultimo, sobre as cores ja resolvidas (inclusive o contorno da cabeca
    # que ficou por dentro, sob a aba): a faixa de sombra sai continua
    out = clean_orphans(out)
    for x, y in shade:
        if out[y][x] is not None and out[y][x] != 'K':
            out[y][x] = DARKEN.get(out[y][x], out[y][x])
    return clean_orphans(out), own

CLEAN = set('HLMSARyYBoO123')
def clean_orphans(cv):
    """Limpeza de pixel orfao criado pela montagem (sombra do chapeu caindo num buraco,
    recorte isolado, sobreposicao de pecas): um pixel de preenchimento sem vizinho
    da mesma cor vira a cor dominante dos vizinhos.  Contorno K e branco W nunca mudam."""
    from collections import Counter
    for _ in range(3):
        changed = False
        for y in range(CELL):
            for x in range(CELL):
                c = cv[y][x]
                if c not in CLEAN:
                    continue
                n8 = [cv[y + dy][x + dx] for dy in (-1, 0, 1) for dx in (-1, 0, 1)
                      if (dx or dy) and 0 <= x + dx < CELL and 0 <= y + dy < CELL]
                if c in n8:
                    continue
                n4 = [cv[y + dy][x + dx] for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))
                      if 0 <= x + dx < CELL and 0 <= y + dy < CELL and cv[y + dy][x + dx] in CLEAN]
                pool = n4 or [v for v in n8 if v in CLEAN]
                if pool:
                    cnt = Counter(pool)
                    best = max(cnt, key=lambda v: (cnt[v], n8.count(v)))
                    cv[y][x] = best
                    changed = True
        if not changed:
            break
    return cv

def hex2rgb(h):
    h = h.lstrip('#')
    return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))

def write_png(path, cv, colors=None):
    """RGBA 8 bits, alfa so 0 ou 255."""
    colors = colors or PALETTE
    h = len(cv); w = len(cv[0])
    raw = bytearray()
    for y in range(h):
        raw.append(0)
        for x in range(w):
            ch = cv[y][x]
            raw += b'\0\0\0\0' if ch is None else bytes(hex2rgb(colors[ch]) + (255,))
    def chunk(t, d):
        c = struct.pack('>I', len(d)) + t + d
        return c + struct.pack('>I', zlib.crc32(t + d) & 0xffffffff)
    png = b'\x89PNG\r\n\x1a\n'
    png += chunk(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, 6, 0, 0, 0))
    png += chunk(b'IDAT', zlib.compress(bytes(raw), 9))
    png += chunk(b'IEND', b'')
    with open(path, 'wb') as f:
        f.write(png)

# ===================================================================== rig ==
# Hierarquia: tudo (x, y) -> corpo (body_d) -> cabeca (head_d) -> olho (eye_d)
#             -> chapeu / bico / bochecha presos ao olho.  Pes ficam no chao
#             (seguem x, y mas nao body_d), a menos que air=True.
def pose(body='BODY', head='HEAD', eye='EYE', beak='BEAK', hat='HAT_6', wing='WING', tail='TAIL',
         foot_n='FOOT_N', foot_f='FOOT_F', bow='BOW', blush='BLUSH',
         x=0, y=0, body_d=(0, 0), head_d=(0, 0), eye_d=(0, 0), hat_d=(0, 0), beak_d=(0, 0),
         wing_d=(0, 0), tail_d=(0, 0), bow_d=(0, 0), foot_n_d=(0, 0), foot_f_d=(0, 0),
         air=False, fx=(), back_fx=(), bow_back=False, extra=(), far_wing=None, far_d=(0, 0),
         foot_n_front=False):
    """far_wing: asa de longe (mais escura) atras do corpo; foot_n_front: pe de perto pintado
    por cima do corpo (agachado: os 3 dedos aparecem em vez de sumir sob a barriga)."""
    def add(a, b): return (a[0] + b[0], a[1] + b[1])
    meta = BODY_META.get(body, BODY_META['BODY'])
    G = (x, y)
    B = add(G, body_d)
    Hd = add(add(B, head_d), meta['neck'])
    E = add(Hd, eye_d)
    F = B if air else G
    L = []
    def put(name, off):
        if name:
            L.append((P[name], off[0], off[1]))
    for name, off in back_fx:
        put(name, off)
    if far_wing:
        put(far_wing, add(add(B, far_d), meta['wing']))
    put(tail, add(add(B, tail_d), meta['tail']))
    late = []
    for fname, fd, dark in ((foot_f, foot_f_d, True), (foot_n, foot_n_d, False)):
        if not fname:
            continue
        if foot_n_front and not dark:
            mark = len(L)
        fo = add(F, fd)
        if fname in LEG_X and body:
            # perna automatica: da barriga ate o topo do pe (coluna de 1 px com contorno)
            bp = P[body]
            lx = LEG_X[fname] + fo[0]
            col = lx - (bp.x + B[0])
            bottom = None
            if 0 <= col < bp.w:
                for j in range(bp.h - 1, -1, -1):
                    if bp.rows[j][col] != '.':
                        bottom = bp.y + B[1] + j
                        break
            top_foot = P[fname].y + fo[1]
            if bottom is not None and top_foot - bottom > 1:
                n = top_foot - bottom
                c = 'O' if dark else 'o'
                L.append((Part('LEG', lx - 1, bottom, '\n'.join(['K' + c + 'K'] * n), 'foot'), 0, 0))
        put(fname, fo)
        if foot_n_front and not dark:
            late, L[mark:] = L[mark:], []
    put(body, B)
    L.extend(late)
    put(wing, add(add(B, wing_d), meta['wing']))
    if bow_back:                       # bicada: a gravata fica no pescoco, atras do queixo
        put(bow, add(add(B, meta['neck']), bow_d))
    put(head, Hd)
    for name, off, rel in extra:       # pecas presas a cabeca (topete, penas) ou ao corpo
        put(name, add(Hd if rel == 'head' else B, off))
    put(blush, E)
    if not bow_back:
        put(bow, add(Hd, bow_d))
    put(beak, add(E, beak_d))
    put(eye, E)
    put(hat, add(E, hat_d))
    for name, off in fx:              # efeitos: coordenadas absolutas do quadro
        put(name, off)
    return L

def light_outline(cv, color='Q', diag=False):
    """Variante para tema escuro: anel de 1 px por fora do contorno."""
    out = [r[:] for r in cv]
    nb = ((1, 0), (-1, 0), (0, 1), (0, -1)) + (((1, 1), (1, -1), (-1, 1), (-1, -1)) if diag else ())
    for y in range(CELL):
        for x in range(CELL):
            if cv[y][x] is None and any(0 <= x + dx < CELL and 0 <= y + dy < CELL and cv[y + dy][x + dx] is not None
                                        for dx, dy in nb):
                out[y][x] = color
    # anel solto num vao de 1 px (chapeu flutuando, asa encostada) vira ruido: sai
    for y in range(CELL):
        for x in range(CELL):
            if out[y][x] == color and cv[y][x] is None and not any(
                    0 <= x + dx < CELL and 0 <= y + dy < CELL and out[y + dy][x + dx] == color and cv[y + dy][x + dx] is None
                    for dx in (-1, 0, 1) for dy in (-1, 0, 1) if dx or dy):
                out[y][x] = None
    # bolsao de fundo de ate 3 px que o proprio anel fechou (ex.: entre a ponta do bico e a
    # gravata) vira anel: senao fica um furo escuro dentro do adesivo
    for comp in holes(out):
        if len(comp) <= 3 and all(cv[y][x] is None for x, y in comp):
            for x, y in comp:
                out[y][x] = color
    return out

def render_full(layers, rim=True):
    global RIM_ON
    RIM_ON = rim
    cv, own = compose(layers)
    RIM_ON = True
    return cv, own

def render(layers, rim=True):
    return render_full(layers, rim)[0]


# ================================================================ animacoes ==
# Cada quadro: (duracao em ms, pose).  Atalhos de pose abaixo; o chapeu atrasa um
# quadro em relacao a cabeca (follow-through) usando hat_d.
SLY = dict(eye='EYE_SLY')
CHIN = dict(beak='BEAK_UP', hat='HAT_4', eye_d=(0, -1))          # queixo erguido: chapeu ainda mais para tras
AIR = dict(air=True, foot_n='FOOT_N_AIR', foot_f='FOOT_F_AIR')
CREST = [('CREST', (0, 0), 'head'), ('NAPE', (0, 0), 'head')]
SCARE = dict(body='BODY_PUFF', eye='EYE_WIDE', tail='TAIL_FAN', extra=CREST)
SCARE_UP = dict(body='BODY_PUFF', eye='EYE_WIDE', tail='TAIL_FAN', extra=CREST[:1])   # asa erguida atras: so o topete (a nuca viraria hachura sobre a asa)
SCARE_HAT = dict(body='BODY_PUFF', eye='EYE_WIDE', tail='TAIL_FAN')   # chapeu de volta na cabeca: topete e nuca baixam
SQ = dict(body='BODY_SQ', foot_n_front=True, foot_n_d=(-1, 0), foot_f_d=(1, 0), bow_d=(0, -2))   # agachado: pes abrem, dedos aparecem; gravata nao desce ate os pes
LEAN = dict(body='BODY_LEAN', tail='TAIL_UP')

def F(ms, **kw):
    return (ms, kw)

ANIMS = {}
# -- idle: respira (cabeca sobe 1, chapeu atrasa), depois a "ginga": queixo erguido, olho malandro;
#    nas batidas o quadril vai (corpo +1) e a cabeca fica (contrapeso), o pe de tras bate com o
#    calcanhar no chao e a cauda balanca um quadro depois
ANIMS['idle'] = [
    F(420),
    F(140, head_d=(0, -1), hat_d=(0, 1)),
    F(420, head_d=(0, -1)),
    F(140, hat_d=(0, -1)),
    F(380),
    F(140, head_d=(0, -1), hat_d=(0, 1)),
    F(300, head_d=(0, -1)),
    F(120, head_d=(-1, 0), hat_d=(0, 1), **SLY, **CHIN),
    F(220, body_d=(1, 0), head_d=(-2, -1), **SLY, **CHIN, foot_f='FOOT_F_UP'),
    F(200, head_d=(-1, 0), **SLY, **CHIN, tail_d=(0, 1)),
    F(220, body_d=(1, 0), head_d=(-2, -1), **SLY, **CHIN, foot_f='FOOT_F_UP'),
    F(260, head_d=(-1, 0), **SLY, **CHIN, tail_d=(0, 1)),
    F(140, hat_d=(0, -1)),
    F(300),
]
ANIMS['blink'] = [
    F(500),
    F(50, eye='EYE_HALF'),
    F(80, eye='EYE_SHUT'),
    F(60, eye='EYE_HALF'),
    F(500),
]
# -- piar: notas saem a 2 px do bico e sobem na mesma coluna
ANIMS['chirp'] = [
    F(260),
    F(120, head_d=(0, 1), eye='EYE_HALF', hat_d=(0, -1)),
    F(130, head_d=(0, -2), beak='BEAK_OPEN', eye='EYE_HAPPY', hat_d=(0, 1), fx=[('FX_NOTE', (39, 11))]),
    F(110, head_d=(0, -1), eye='EYE_HAPPY', fx=[('FX_NOTE', (39, 6))]),
    F(130, head_d=(0, -2), beak='BEAK_OPEN', eye='EYE_HAPPY', fx=[('FX_NOTE', (39, 1)), ('FX_NOTE', (39, 11))]),
    F(110, head_d=(0, -1), eye='EYE_HAPPY', fx=[('FX_NOTE', (39, 6))]),
    F(160, hat_d=(0, -1), fx=[('FX_NOTE', (39, 1))]),
    F(320),
]
# -- bicar: olha, recua, inclina o corpo (ombro a frente, barriga parada), a cabeca GIRA e o bico
#    desce pela cara ate o grao; o chapeu escorrega ate a raiz do bico e volta no tranco
SEED = [('FX_SEED', (36, 42))]
ANIMS['peck'] = [
    F(240, fx=SEED),
    F(200, eye='EYE_DOWN', fx=SEED),
    F(140, head_d=(-1, -1), eye='EYE_DOWN', hat_d=(0, 1), fx=SEED),
    F(70, **LEAN, head_d=(3, 1), beak='BEAK_45', eye='EYE_DOWN', hat='HAT_F6', hat_d=(1, 0), fx=SEED),
    F(110, **LEAN, head_d=(5, 2), beak='BEAK_DOWN', eye='EYE_DOWN', eye_d=(0, 1), hat='HAT_F4', hat_d=(3, 1),
      bow_d=(-1, 2), fx=SEED),
    F(90, **LEAN, head_d=(5, 2), beak='BEAK_DOWN', eye='EYE_SHUT', eye_d=(0, 1), hat='HAT_F4', hat_d=(4, 3),
      bow_d=(-1, 2), fx=[('FX_CRUMB', (40, 37)), ('FX_CRUMB', (43, 40))]),
    F(100, head_d=(1, -1), beak='BEAK_OPEN', eye='EYE_HAPPY', hat='HAT_12', hat_d=(0, -2),
      fx=[('FX_CRUMB', (41, 32)), ('FX_CRUMB', (43, 36))]),
    F(130, eye='EYE_HAPPY', hat_d=(0, 1), fx=[('FX_CRUMB', (43, 41))]),
    F(110, beak='BEAK_OPEN', eye='EYE_HAPPY', head_d=(0, -1)),
    F(130, eye='EYE_HAPPY'),
    F(110, beak='BEAK_OPEN', eye='EYE_HAPPY', head_d=(0, -1)),
    F(300),
]
# -- pulo / comemorar
# poeira do pouso, lado esquerdo: o agachado deixa a cauda em x5-14 ate y41 e a asa ate o chao, entao
# o tufo vai para tras da ponta da cauda (x1-5), com 1 px de fundo ate ela (o espelho exato do tufo da
# direita cairia dentro do rabo).  Usado tambem no 'landing'.
DUST_L = ('FX_DUST', (1, 42))
DUST2_L = ('FX_DUST2', (1, 41))
ANIMS['hop'] = [
    F(220),
    F(120, **SQ, eye='EYE_SHUT', hat_d=(0, 1)),
    F(70, body='BODY_ST', y=-2, eye='EYE_HAPPY', beak='BEAK_WIDE', wing='WING_UP', hat_d=(0, 1)),
    F(90, y=-5, **AIR, eye='EYE_HAPPY', beak='BEAK_WIDE', wing='WING_UP', hat='HAT_6_AR', hat_d=(1, -3),
      fx=[('FX_SPARK2', (7, 24)), ('FX_CONF1', (40, 10))]),
    F(120, y=-6, **AIR, eye='EYE_HAPPY', beak='BEAK_WIDE', wing='WING_MID', hat='HAT_4_AR', hat_d=(1, -3),
      fx=[('FX_SPARK', (3, 14)), ('FX_CONF1', (41, 13)), ('FX_CONF2', (7, 7)), ('FX_CONF3', (40, 3))]),
    F(110, y=-4, **AIR, eye='EYE_HAPPY', beak='BEAK_OPEN', wing='WING_UP', hat='HAT_0_AR', hat_d=(1, -6),
      fx=[('FX_SPARK2', (4, 17)), ('FX_CONF1', (42, 17)), ('FX_CONF2', (8, 11)), ('FX_CONF3', (39, 8))]),
    # rodada 3: o confete laranja passa atras da asa erguida (antes ficava pintado na asa, em (9,16))
    F(80, body='BODY_ST', y=-2, eye='EYE_HAPPY', wing='WING_UP', hat='HAT_12_AR', hat_d=(0, -4),
      fx=[('FX_CONF1', (43, 22)), ('FX_CONF3', (38, 14))]),
    # rodada 3: o tufo da esquerda sai de baixo da cauda e da asa (antes em (11,41), lia como ponta
    # branca do rabo) para o unico lugar com folga, atras da ponta da cauda
    F(100, **SQ, eye='EYE_SHUT', hat='HAT_6_AR', hat_d=(0, -8),
      fx=[DUST_L, ('FX_DUST', (36, 41)), ('FX_CONF2', (10, 22)), ('FX_CONF3', (37, 20))]),
    # rodada 3: o tufo da esquerda sobe 1 px e encolhe no mesmo lugar (sem espaco para abrir para fora);
    # o confete petroleo que caia por cima do bico em (36,26) saiu: os dois confetes acabam no quadro 7
    F(100, **SQ, eye='EYE_HAPPY', hat_d=(0, 1), fx=[DUST2_L, ('FX_DUST2', (39, 40))]),
    F(100, eye='EYE_HAPPY', head_d=(0, -1), hat_d=(0, -2)),
    F(160, eye='EYE_HAPPY', fx=[('FX_SPARK2', (39, 13))]),
    F(300),
]
# -- voar: corpo inclinado ~15 graus (cabeca 2 a frente e abaixo, cauda 2 acima), asa de longe
#    mais escura, chapeu com atraso de 1 quadro (repete a altura do quadro anterior)
FLYP = dict(air=True, foot_n='FOOT_N_TUCK', foot_f='FOOT_F_TUCK', tail='TAIL_FAN', head_d=(2, 2), tail_d=(0, -2))
ANIMS['fly'] = [
    F(90, y=-6, **FLYP, wing='WING_UP', far_wing='WING_FAR_UP', body_d=(0, 1), hat_d=(0, -1)),
    F(80, y=-6, **FLYP, wing='WING_MID', far_wing='WING_FAR_MID', hat_d=(0, 1)),
    F(90, y=-6, **FLYP, wing='WING_DOWN', far_wing='WING_FAR_DOWN', body_d=(0, -1), hat_d=(0, 1)),
    F(80, y=-6, **FLYP, wing='WING_MID', far_wing='WING_FAR_MID', hat_d=(0, -1)),
]
# -- susto: antecipacao (encolhe e fecha o olho), take (arregala, grita, topete, chapeu voa),
#    tremedeira com gota, chapeu cai torto sobre o olho, meio-olho, ajeita o chapeu
ANIMS['scared'] = [
    F(240),
    F(80, **SQ, eye='EYE_SHUT', hat_d=(0, 1)),
    F(90, y=-3, **SCARE_UP, beak='BEAK_WIDE', wing='WING_UP', hat='HAT_4_AR', hat_d=(1, -5), fx=[('FX_JOLT', (38, 6))]),
    F(110, y=-3, **SCARE_UP, beak='BEAK_WIDE', wing='WING_UP', hat='HAT_6_AR', hat_d=(2, -6), fx=[('FX_JOLT', (39, 5))]),
    F(100, **SCARE, beak='BEAK_OPEN', hat='HAT_0_AR', hat_d=(1, -8)),
    F(70, x=-1, **SCARE, beak='BEAK_OPEN', hat='HAT_12_AR', hat_d=(1, -6), fx=[('FX_SWEAT', (9, 15))]),
    F(70, x=1, **SCARE, beak='BEAK_OPEN', hat='HAT_12_AR', hat_d=(0, -3), fx=[('FX_SWEAT', (8, 17))]),
    F(70, x=-1, **SCARE_HAT, beak='BEAK_OPEN', hat='HAT_F6', hat_d=(1, 2), fx=[('FX_SWEAT', (8, 19))]),
    F(70, x=1, **SCARE_HAT, hat='HAT_F6', hat_d=(1, 2), fx=[('FX_SWEAT', (7, 21))]),
    F(240, body='BODY_PUFF', eye='EYE_WIDE', hat='HAT_F6', hat_d=(1, 2), fx=[('FX_SWEAT', (7, 24))]),
    F(320, eye='EYE_HALF', hat='HAT_F6', hat_d=(1, 1)),
    F(300, eye='EYE_SLY', hat_d=(0, 0)),
]
ZZZ = dict(eye='EYE_SHUT', hat='HAT_F6', tail_d=(0, 1))
ANIMS['sleep'] = [      # respira devagar: cabeca afunda e a asa sobe 1 px no "inspira"; chapeu escorrega
    F(520, **ZZZ, head_d=(0, 1), hat_d=(1, 2), fx=[('FX_Z2', (38, 15))]),
    F(520, **ZZZ, head_d=(0, 1), hat_d=(1, 2), wing_d=(0, -1), fx=[('FX_Z2', (39, 11))]),
    F(520, **ZZZ, head_d=(0, 2), hat_d=(2, 3), wing_d=(0, -1), fx=[('FX_Z2', (40, 7)), ('FX_Z', (38, 15))]),
    F(520, **ZZZ, head_d=(0, 2), hat_d=(2, 3), fx=[('FX_Z', (39, 10))]),
    F(520, **ZZZ, head_d=(0, 1), hat_d=(1, 2), fx=[('FX_Z', (40, 5))]),
]
# -- chamar atencao: pulo alto (pico segurado 2 quadros), grita, abana a asa, olhada de lado
ANIMS['attention'] = [
    F(120, **SQ, head_d=(1, 1), wing='WING', hat_d=(0, 1), fx=[('FX_BANG', (41, 9))]),
    F(80, y=-4, body='BODY_ST', beak='BEAK_OPEN', wing='WING_UP', hat_d=(0, 2), fx=[('FX_BANG', (41, 6))]),
    F(110, y=-8, **AIR, beak='BEAK_WIDE', wing='WING_MID', hat='HAT_6_AR', hat_d=(0, 0), fx=[('FX_BANG', (41, 4))]),
    F(110, y=-8, **AIR, beak='BEAK_WIDE', wing='WING_UP', hat='HAT_6_AR', hat_d=(0, -1), fx=[('FX_BANG', (41, 3))]),
    F(80, y=-3, **AIR, beak='BEAK_OPEN', wing='WING_MID', hat='HAT_6', hat_d=(0, -1), fx=[('FX_BANG', (41, 6))]),
    F(120, **SQ, beak='BEAK', wing='WING', hat_d=(0, 1), fx=[('FX_BANG', (41, 9))]),
    F(110, head_d=(1, 0), beak='BEAK_OPEN', wing='WING_UP', fx=[('FX_BANG', (41, 8))]),
    F(110, head_d=(1, 0), beak='BEAK', wing='WING_MID', fx=[('FX_BANG', (41, 8))]),
    F(110, head_d=(1, 0), beak='BEAK_OPEN', wing='WING_UP', fx=[('FX_BANG', (41, 8))]),
    F(160, head_d=(1, 1), eye='EYE_SLY', beak='BEAK', wing='WING', fx=[('FX_BANG', (41, 9))]),
]
# -- trabalhando (o estado mais visto enquanto o Claude Code roda): inclinado sobre uma tecla, o
#    bico bate 2 vezes rapido e pausa; o chapeu desce 1 px a frente a cada batida e volta 1 quadro
#    depois; a cada ciclo sobe uma faisca petroleo
WORK = dict(body='BODY_LEAN', beak='BEAK_45', eye='EYE_DOWN', hat='HAT_12', foot_f_d=(-2, 0), bow_back=True, bow_d=(-1, -1))
KEYUP = ('FX_KEY', (35, 39)); KEYDN = ('FX_KEY_DN', (35, 40))
ANIMS['work'] = [
    F(120, **WORK, head_d=(5, -2), fx=[KEYUP]),                           # pronto: curvado, ponta 1 px acima da tecla
    F(80, **WORK, head_d=(5, 1), hat_d=(1, 1), fx=[KEYDN]),               # bate 1: tecla afunda, chapeu desce a frente
    F(80, **WORK, head_d=(5, -2), hat_d=(1, 1), fx=[KEYUP]),              # sobe; o chapeu volta 1 quadro depois
    F(80, **WORK, head_d=(5, 1), hat_d=(1, 1), fx=[KEYDN]),               # bate 2
    F(90, **WORK, head_d=(5, -2), hat_d=(1, 1), fx=[KEYUP, ('FX_SPARK_A', (43, 33))]),
    F(300, **WORK, head_d=(5, -3), fx=[KEYUP, ('FX_SPARK_A', (43, 29))]),  # pausa: ergue um pouco e confere
    F(160, **dict(WORK, eye='EYE_HALF'), head_d=(5, -2), fx=[KEYUP, ('FX_DOT_A', (44, 25))]),
]

# ------------------------------------------------- rodada 3: gestos e transicoes ---------------
# Tudo pelo rig, com as pecas de sempre.  Cada gesto comeca e termina na pose neutra (o pet volta
# a ela no fim de cada reacao); cada transicao liga a pose neutra a um laco de estado (a critica:
# work_00, sleep_00, attention_00 e fly_00 ficavam a 400-900 px de diferenca do idle_00 e o app
# trocava de uma vez).

# -- aceno (T0, resposta sem trabalho): a cabeca baixa e o chapeu tomba para a frente, a "tirada
#    de chapeu" do malandro, e volta.  Gesto proprio: nao e pedaco do respira nem da ginga (que
#    sobem a cabeca), entao nao se confunde com o repouso.
ANIMS['nod'] = [
    F(100, head_d=(0, 1), eye='EYE_SLY', hat_d=(0, -1)),                      # comeca a baixar; o chapeu atrasa
    F(180, head_d=(0, 2), eye='EYE_HAPPY', hat='HAT_12', hat_d=(1, 0)),       # baixa: o chapeu tomba a frente
    F(110, head_d=(0, 1), eye='EYE_HAPPY', hat_d=(0, 1)),                     # sobe; o chapeu atrasa
    F(120, eye='EYE_SLY', hat_d=(0, 1)),                                      # chapeu assentando
    F(300),
]
# -- tchau (e o aceno de asa): a asa de perto abana duas vezes, olho feliz, bico abrindo
ANIMS['wave'] = [
    F(100, head_d=(0, -1), eye='EYE_HAPPY', wing='WING_MID', hat_d=(0, 1)),
    F(120, head_d=(0, -1), eye='EYE_HAPPY', beak='BEAK_OPEN', wing='WING_UP'),
    F(100, head_d=(0, -1), eye='EYE_HAPPY', wing='WING_MID'),
    F(120, head_d=(0, -1), eye='EYE_HAPPY', beak='BEAK_OPEN', wing='WING_UP'),
    F(120, eye='EYE_HAPPY', wing='WING_MID', hat_d=(0, -1)),
    F(300),
]
# -- bocejo (cansado; comeco da soneca): o olho pesa, o queixo sobe com o bico bem aberto, fecha
#    devagar e volta com o olho pesado
ANIMS['yawn'] = [
    F(160, eye='EYE_HALF'),
    F(120, head_d=(0, -1), eye='EYE_SHUT', beak='BEAK_OPEN', hat_d=(0, 1)),
    F(520, head_d=(-1, -2), eye='EYE_SHUT', beak='BEAK_WIDE', hat='HAT_4', hat_d=(0, 1)),
    F(160, head_d=(0, -1), eye='EYE_SHUT', beak='BEAK_OPEN'),
    F(260, head_d=(0, 1), eye='EYE_HALF', hat_d=(0, -1)),
    F(240, eye='EYE_HALF'),
    F(300),
]
# -- acordar (fim da soneca): pisca pesado, se espreguica (peito estufado, asas para cima, bico
#    aberto) e o chapeu pula e assenta
ANIMS['wake'] = [
    F(200, eye='EYE_HALF'),
    F(100, head_d=(0, 1), eye='EYE_SHUT', hat_d=(0, 0)),
    F(360, body='BODY_ST', eye='EYE_SHUT', beak='BEAK_WIDE', wing='WING_UP', hat_d=(0, -2)),   # espreguica
    F(120, eye='EYE_HALF', wing='WING_MID', hat_d=(0, -1)),           # desce; o chapeu ainda no ar
    F(120, hat_d=(0, 1)),                                             # o chapeu cai e assenta
    F(300),
]
# -- entrar no trabalho (2 quadros): debruca sobre a tecla e da o passo (o pe de longe recua 2 px,
#    a base fechada do work_00); sair: endireita com o pe ainda atras e volta ao neutro
ANIMS['work_in'] = [
    # a tecla so aparece com o passo dado: com o pe de longe ainda no lugar, pe e tecla fechariam um
    # furo de fundo debaixo do bico
    F(120, **LEAN, head_d=(3, 0), beak='BEAK_45', eye='EYE_DOWN', hat='HAT_12', bow_back=True, bow_d=(-1, -1)),
    F(100, **WORK, head_d=(4, -1), fx=[KEYUP]),
]
ANIMS['work_out'] = [
    F(120, **LEAN, head_d=(3, -1), beak='BEAK_45', eye='EYE_DOWN', hat='HAT_12', foot_f_d=(-2, 0), bow_back=True,
      bow_d=(-1, -1), fx=[KEYUP]),
    F(160, eye='EYE_SLY', hat_d=(0, 1)),
]
# -- adormecer (3 quadros): o olho pesa, a cabeca afunda e o chapeu escorrega sobre o olho ate a
#    pose do sleep_00; acordar do sono: abre o olho, a cabeca sobe e empurra o chapeu, que assenta
ANIMS['sleep_in'] = [
    F(240, eye='EYE_HALF'),
    F(240, head_d=(0, 1), eye='EYE_HALF', hat_d=(0, 1)),
    F(260, **ZZZ, head_d=(0, 1), hat_d=(1, 1)),
]
ANIMS['sleep_out'] = [
    F(200, head_d=(0, 1), eye='EYE_HALF', hat='HAT_F6', hat_d=(1, 2), tail_d=(0, 1)),
    F(120, head_d=(0, -1), hat_d=(0, -1)),
    F(160, hat_d=(0, 1)),
]
# -- chamar atencao: entra aprumando a cabeca com o "!" ja aceso (o agachado do attention_00 vira
#    a antecipacao do pulo); sai com o olho malandro, sem o "!", e o chapeu assenta
ANIMS['attention_in'] = [
    F(120, head_d=(0, -1), hat_d=(0, 1), fx=[('FX_BANG', (41, 9))]),
]
ANIMS['attention_out'] = [
    F(140, eye='EYE_SLY', hat_d=(0, 1)),
    F(160),
]
# -- decolar: o agachado do pulo, estica com as asas para cima e sobe batendo (ate o fly_00, em
#    y-6 e inclinado); pousar: desce com os pes para baixo, agacha no impacto com poeira dos dois
#    lados (o tufo da esquerda no lugar novo), o chapeu quica e assenta
ANIMS['takeoff'] = [
    F(120, **SQ, eye='EYE_SHUT', hat_d=(0, 1)),
    F(80, body='BODY_ST', y=-2, wing='WING_UP', hat_d=(0, 1)),
    F(80, y=-4, **AIR, wing='WING_MID', far_wing='WING_FAR_MID', head_d=(1, 1), hat_d=(0, 1)),
]
ANIMS['landing'] = [
    F(90, y=-3, **AIR, wing='WING_UP', far_wing='WING_FAR_UP', head_d=(1, 1), hat_d=(0, -1)),
    F(100, **SQ, eye='EYE_SHUT', hat_d=(0, 1), fx=[DUST_L, ('FX_DUST', (36, 41))]),
    F(100, **SQ, eye='EYE_HAPPY', hat_d=(0, -1), fx=[DUST2_L, ('FX_DUST2', (39, 40))]),
    F(120, eye='EYE_HAPPY', hat_d=(0, 1)),
    F(300),
]

def anim_layers(name):
    return [(ms, pose(**kw)) for ms, kw in ANIMS[name]]

def dev_strip(name, out, zoom=4, bg='#5A5F7A'):
    """Tira de quadros de uma animacao (para revisar o movimento)."""
    tmp = os.path.join(SAIDA, 'processo', '_strip')
    os.makedirs(tmp, exist_ok=True)
    tiles = []
    for i, (ms, layers) in enumerate(anim_layers(name)):
        f = os.path.join(tmp, f'{i:02d}.png')
        write_png(f, render(layers))
        t = os.path.join(tmp, f'{i:02d}t.png')
        mk(f, '-background', bg, '-flatten', '-filter', 'point', '-resize', f'{zoom*100}%',
           '-gravity', 'north', '-background', '#30324A', '-splice', '0x18', '-fill', '#E0E2F0',
           '-font', 'Adwaita-Mono', '-pointsize', '13', '-annotate', '+0+2', f'{i} {ms}ms', t)
        tiles.append(t)
    mk(*tiles, '-background', '#30324A', '-splice', '4x0', '+append', out)
    shutil.rmtree(tmp)


# ============================================================== verificacao ==
def lum(hexc):
    def lin(c):
        c /= 255
        return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4
    r, g, b = hex2rgb(hexc)
    return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)

def contrast(a, b):
    la, lb = lum(a), lum(b)
    return (max(la, lb) + 0.05) / (min(la, lb) + 0.05)

def orphans(cv):
    """Pixels sem nenhum vizinho (8-viz) da mesma cor."""
    out = []
    for y in range(CELL):
        for x in range(CELL):
            c = cv[y][x]
            if c is None:
                continue
            if not any(0 <= x + dx < CELL and 0 <= y + dy < CELL and cv[y + dy][x + dx] == c
                       for dx in (-1, 0, 1) for dy in (-1, 0, 1) if dx or dy):
                out.append((x, y, c))
    return out

N4 = ((1, 0), (-1, 0), (0, 1), (0, -1))
N8 = tuple((dx, dy) for dy in (-1, 0, 1) for dx in (-1, 0, 1) if dx or dy)

def _in(x, y):
    return 0 <= x < CELL and 0 <= y < CELL

def _components(cells, nb):
    """Componentes conexos de um conjunto de (x, y)."""
    cells = set(cells); seen = set(); out = []
    for c in sorted(cells):
        if c in seen:
            continue
        comp = []; st = [c]; seen.add(c)
        while st:
            x, y = st.pop(); comp.append((x, y))
            for dx, dy in nb:
                n = (x + dx, y + dy)
                if n in cells and n not in seen:
                    seen.add(n); st.append(n)
        out.append(sorted(comp))
    return out

def holes(cv):
    """Furos de fundo: componentes transparentes (4-viz) que nao alcancam a borda da celula.
    Inclui o caso 'pixel de alfa 0 com os 4 vizinhos opacos' e os bolsoes de 2-3 px."""
    empty = {(x, y) for y in range(CELL) for x in range(CELL) if cv[y][x] is None}
    return [c for c in _components(empty, N4)
            if not any(x in (0, CELL - 1) or y in (0, CELL - 1) for x, y in c)]

def rim_problems(cv, own=None, layers=None):
    """Luz de recorte R (do corpo, nao dos efeitos): (a) nunca na borda da silhueta (os 4 vizinhos
    tem de ser opacos) e (b) sempre em trechos de 2 px ou mais (8-viz), nunca cisco solto."""
    def fx(x, y):
        return own is not None and own[y][x] >= 0 and layers[own[y][x]][0].role == 'fx'
    rs = [(x, y) for y in range(CELL) for x in range(CELL) if cv[y][x] == 'R' and not fx(x, y)]
    edge = [(x, y) for x, y in rs if any(not _in(x + dx, y + dy) or cv[y + dy][x + dx] is None for dx, dy in N4)]
    comps = _components(rs, N8)
    return edge, [c for c in comps if len(c) < 2], comps

def speckles(cv):
    """Ilhas de 1 px: pixel de preenchimento (nao K) sem vizinho 4-viz da mesma cor."""
    return [(x, y) for y in range(CELL) for x in range(CELL)
            if cv[y][x] not in (None, 'K') and not any(_in(x + dx, y + dy) and cv[y + dy][x + dx] == cv[y][x]
                                                       for dx, dy in N4)]

def k_clumps(cv, box=None):
    """Blocos 2x2 de contorno (contorno dobrado = 'sobrancelha pesada')."""
    x0, y0, x1, y1 = box or (0, 0, CELL - 1, CELL - 1)
    return [(x, y) for y in range(y0, y1) for x in range(x0, x1)
            if all(cv[y + dy][x + dx] == 'K' for dx in (0, 1) for dy in (0, 1))]

# Efeitos sem contorno (confete, faisca e ponto do trabalho, z do sono) e a poeira: pintados por
# cima do corpo leem como marca no proprio Zeca (o confete do hop_08 virava um risco no bico, o
# tufo do pouso uma ponta branca do rabo).  Os de contorno fechado (brilho, nota, "!", gota,
# farelo, tracinhos) e os aderecos que ele toca (grao, tecla) podem passar na frente.
FX_SEM_CORPO = {'FX_CONF1', 'FX_CONF2', 'FX_CONF3', 'FX_SPARK_A', 'FX_DOT_A', 'FX_Z', 'FX_Z2',
                'FX_DUST', 'FX_DUST2'}

def fx_sobre_o_corpo(layers):
    """Pixels de efeito de FX_SEM_CORPO pintados por cima de uma peca do personagem (rodada 3)."""
    own = [[-1] * CELL for _ in range(CELL)]
    out = []
    for li, (p, dx, dy) in enumerate(layers):
        for j, row in enumerate(p.rows):
            for i, ch in enumerate(row):
                if ch in '.~':
                    continue
                x, y = p.x + i + dx, p.y + j + dy
                if not _in(x, y):
                    continue
                if p.name in FX_SEM_CORPO and own[y][x] >= 0 and layers[own[y][x]][0].role != 'fx':
                    out.append((x, y, p.name, layers[own[y][x]][0].name))
                own[y][x] = li
    return out

def lint(cv, own=None, layers=None):
    """Todas as regras duras de um quadro.  Devolve dict de listas (vazio = aprovado)."""
    bad = {}
    if layers is not None:
        sobre = fx_sobre_o_corpo(layers)
        if sobre: bad['efeito_no_corpo'] = sobre
    orp = orphans(cv)
    if orp: bad['orfaos'] = orp
    hl = holes(cv)
    if hl: bad['furos'] = hl
    edge, small, _ = rim_problems(cv, own, layers)
    if edge: bad['recorte_na_borda'] = edge
    if small: bad['recorte_solto'] = small
    cols = {c for r in cv for c in r if c is not None}
    if len(cols) > 16: bad['cores'] = sorted(cols)
    pts = [(x, y) for y in range(CELL) for x in range(CELL) if cv[y][x] is not None]
    if pts and (min(p[0] for p in pts) < 1 or max(p[0] for p in pts) > CELL - 2 or
                min(p[1] for p in pts) < 1 or max(p[1] for p in pts) > FLOOR):
        bad['fora_da_margem'] = (min(p[0] for p in pts), min(p[1] for p in pts), max(p[0] for p in pts), max(p[1] for p in pts))
    return bad

def check_all(verbose=True):
    """Roda o lint em todos os quadros, no visual padrao e na variante com anel."""
    allc = set()
    report = {}
    for name in ANIMS:
        for i, (ms, layers) in enumerate(anim_layers(name)):
            cv, own = render_full(layers)
            cols = {c for r in cv for c in r if c is not None}
            allc |= cols
            bad = lint(cv, own, layers)
            ring = light_outline(cv, 'Q')
            hr = holes(ring)
            if hr: bad['furos_no_anel'] = hr
            orr = orphans(ring)
            if orr: bad['orfaos_no_anel'] = orr
            report[f'{name}_{i:02d}'] = (len(cols), bad, len(speckles(cv)), len(k_clumps(cv)))
            if verbose and bad:
                print(f'{name}_{i:02d}: ' + '; '.join(f'{k} {v}' for k, v in bad.items()))
    return allc, report


# ================================================================ exportar ==
DARK, LIGHT = '#0B0C16', '#EFF1F5'
RING_COLORS = dict(PALETTE, Q=RING)
KEY = {'idle': 8, 'blink': 2, 'chirp': 4, 'peck': 4, 'hop': 4, 'fly': 0, 'scared': 3, 'sleep': 2, 'attention': 3, 'work': 1,
       'nod': 1, 'wave': 1, 'yawn': 2, 'wake': 3, 'work_in': 0, 'work_out': 0, 'sleep_in': 1, 'sleep_out': 1,
       'attention_in': 0, 'attention_out': 0, 'takeoff': 2, 'landing': 1}
LOOPS = {'idle', 'fly', 'sleep', 'attention', 'work'}
# trechos que o app pode tocar separados (ex.: respirar em loop e sortear a ginga).  Rodada 3: o
# 'respira' e so 0-3 (os quadros 4-6 repetiam os 0-2, e a costura 6->0 descia cabeca e chapeu
# juntos, sem o atraso do chapeu)
SEGMENTS = {'idle': {'respira': {'quadros': [0, 3], 'loop': True},
                     'ginga': {'quadros': [7, 13], 'loop': False, 'intervalo_s': [8, 20]}}}
# transicoes da pose neutra para cada laco de estado e de volta (a critica pediu a lista no manifesto)
TRANSITIONS = {'work': {'entrar': 'work_in', 'sair': 'work_out'},
               'sleep': {'entrar': 'sleep_in', 'sair': 'sleep_out'},
               'attention': {'entrar': 'attention_in', 'sair': 'attention_out'},
               'fly': {'entrar': 'takeoff', 'sair': 'landing'}}
TITLES = {'idle': 'idle (ginga)', 'blink': 'piscar', 'chirp': 'piar', 'peck': 'bicar / comer', 'hop': 'pulo / comemorar',
          'fly': 'voar', 'scared': 'susto', 'sleep': 'dormir', 'attention': 'chamar atenção', 'work': 'trabalhando',
          'nod': 'aceno (tirada de chapéu)', 'wave': 'tchau (asa)', 'yawn': 'bocejo', 'wake': 'acordar (espreguiça)',
          'work_in': 'entrar no trabalho', 'work_out': 'sair do trabalho', 'sleep_in': 'adormecer',
          'sleep_out': 'acordar do sono', 'attention_in': 'entrar na chamada', 'attention_out': 'sair da chamada',
          'takeoff': 'decolar', 'landing': 'pousar'}

def frames_of(name, ring=False):
    out = []
    for ms, layers in anim_layers(name):
        cv = render(layers)
        out.append((ms, light_outline(cv, 'Q') if ring else cv))
    return out

def manifest():
    """O anims.json: quadros, ms, loop, paleta, anel, uso (tema, idle, transicoes) e gatilhos.  Os
    caminhos dos quadros sao relativos a pasta de saida do gerador."""
    m = {
        'celula': CELL, 'chao_y': FLOOR, 'paleta': PALETTE, 'anel_tema_escuro': RING,
        'licenca': 'CC0-1.0 (arte original feita com o Claude para o projeto bichinho; LICENSE ao lado do zeca.py)',
        'uso': {
            'tema': 'Tema CLARO: use "arquivo" (visual padrao). Tema ESCURO: use SEMPRE "escuro" (frames/escuro/, '
                    'anel de 1 px #5E5A86 por fora do contorno) - o contorno #2B2136 tem so 1,27:1 contra #0B0C16.',
            'idle': 'Toque em loop SO o trecho "respira" (quadros 0-3, 1,12 s). Sorteie a "ginga" (quadros 7-13) a cada '
                    '8-20 s (campo intervalo_s); em loop direto ela vira um tique a cada 3,4 s. No bichinho o '
                    'orcamento de commits (ate 2/s parado) nao deixa o respira em loop: a pose neutra fica parada e o '
                    'respira, o piscar e a ginga tocam em rajadas, com pelo menos 4 s entre elas.',
            'transicoes': {
                'regra': 'As acoes (blink, chirp, peck, hop, scared) e os gestos (nod, wave, yawn, wake) comecam na '
                         'pose neutra do idle e voltam a ela (o peck mostra o grao ja no 1o quadro; o scared termina '
                         'com o olho malandro, que vira o olho neutro no idle). Os lacos de estado (work, fly, sleep, '
                         'attention) entram e saem pela pose neutra com as transicoes abaixo: toque "entrar", o laco '
                         'quantas vezes quiser e "sair".',
                'lacos': TRANSITIONS,
            },
        },
        'gatilhos_sugeridos': {
            'work': 'Claude Code executando (o estado mais visto)', 'attention': 'pedindo permissao / esperando resposta',
            'hop': 'tarefa concluida', 'scared': 'erro ou falha de teste', 'sleep': 'ocioso por muito tempo',
            'chirp': 'notificacao / saudacao', 'peck': 'variacao do ocioso', 'blink': 'variacao do ocioso',
            'fly': 'entrando ou saindo da tela', 'idle': 'parado',
            'nod': 'resposta sem trabalho (aceno discreto)', 'wave': 'tchau / aceno', 'yawn': 'cansado, comeco da soneca',
            'wake': 'fim da soneca',
        },
        'animacoes': {},
    }
    for name in ANIMS:
        items = [{'arquivo': f'frames/{name}_{i:02d}.png', 'escuro': f'frames/escuro/{name}_{i:02d}.png', 'ms': ms}
                 for i, (ms, _) in enumerate(ANIMS[name])]
        m['animacoes'][name] = {'titulo': TITLES[name], 'loop': name in LOOPS, 'quadros': items}
        if name in SEGMENTS:
            m['animacoes'][name]['trechos'] = SEGMENTS[name]
    return m

def export_frames(out=None, folhas=True):
    """Os quadros dos dois visuais (frames/ e frames/escuro/), as folhas horizontais (frames/folhas/,
    se `folhas`) e o anims.json em `out`.  Mesmos bytes a cada execucao."""
    out = out or SAIDA
    fdir = os.path.join(out, 'frames'); edir = os.path.join(fdir, 'escuro'); sdir = os.path.join(fdir, 'folhas')
    for d in (fdir, edir) + ((sdir,) if folhas else ()):
        os.makedirs(d, exist_ok=True)
    for name in ANIMS:
        for ring, d in ((False, fdir), (True, edir)):
            fr = frames_of(name, ring)
            for i, (ms, cv) in enumerate(fr):
                write_png(os.path.join(d, f'{name}_{i:02d}.png'), cv, RING_COLORS if ring else PALETTE)
            if not folhas:
                continue
            # folha horizontal (1x) para motores de jogo
            sheet = [[None] * (CELL * len(fr)) for _ in range(CELL)]
            for i, (ms, cv) in enumerate(fr):
                for y in range(CELL):
                    sheet[y][i * CELL:(i + 1) * CELL] = cv[y]
            write_png(os.path.join(sdir, f'{name}{"_escuro" if ring else ""}.png'), sheet, RING_COLORS if ring else PALETTE)
    with open(os.path.join(out, 'anims.json'), 'w', encoding='utf-8') as f:
        json.dump(manifest(), f, indent=1, ensure_ascii=False)
        f.write('\n')

def export_gifs(out=None, zoom=4):
    """Previa 4x (escuro | escuro+anel | claro) com cabecalho fixo e GIF transparente 1x."""
    out = out or SAIDA
    gdir = os.path.join(out, 'gifs'); g1 = os.path.join(gdir, '1x')
    os.makedirs(g1, exist_ok=True)
    tmp = os.path.join(out, 'processo', '_gif'); os.makedirs(tmp, exist_ok=True)
    S = CELL * zoom
    for name in ANIMS:
        plain = frames_of(name); ringed = frames_of(name, True)
        head = os.path.join(tmp, 'head.png')
        mk('-size', f'{3*S}x42', 'xc:#1A1B2A', '-fill', '#E6E8F4', '-font', 'Adwaita-Sans-SemiBold', '-pointsize', '15',
           '-gravity', 'northwest', '-annotate', '+8+4', f'Zeca · {TITLES[name]}',
           '-fill', '#9EA2BC', '-font', 'Adwaita-Mono', '-pointsize', '12',
           '-annotate', '+8+24', 'escuro', '-annotate', f'+{S+8}+24', 'escuro + anel', '-annotate', f'+{2*S+8}+24', 'claro', head)
        args1 = ['-dispose', 'Background']
        argsp = []
        for i, ((ms, cv), (_, cr)) in enumerate(zip(plain, ringed)):
            a = os.path.join(tmp, f'{i:02d}a.png'); b = os.path.join(tmp, f'{i:02d}b.png')
            write_png(a, cv); write_png(b, cr, RING_COLORS)
            p = os.path.join(tmp, f'{i:02d}p.png')
            mk(head, '(', '(', a, '-background', DARK, '-flatten', '-filter', 'point', '-resize', f'{zoom*100}%', ')',
               '(', b, '-background', DARK, '-flatten', '-filter', 'point', '-resize', f'{zoom*100}%', ')',
               '(', a, '-background', LIGHT, '-flatten', '-filter', 'point', '-resize', f'{zoom*100}%', ')',
               '+append', ')', '-append', p)
            argsp += ['-delay', str(max(2, round(ms / 10))), p]
            args1 += ['-delay', str(max(2, round(ms / 10))), a]
        mk(*argsp, '-loop', '0', os.path.join(gdir, f'{name}.gif'))
        mk(*args1, '-loop', '0', os.path.join(g1, f'{name}.gif'))
    shutil.rmtree(tmp)


def _label(path, text, bg, fg, size=16, w=None, h=None, font='Adwaita-Sans', gravity='west'):
    args = ['-background', bg, '-fill', fg, '-font', font, '-pointsize', str(size)]
    if w and h:
        args += ['-size', f'{w}x{h}', '-gravity', gravity]
    mk(*args, f'label:{text}', path)

def vitrine(out=None):
    """Todos os estados em 6x (quadro-chave) e 1x (todos os quadros), em tres paineis empilhados:
    escuro padrao, escuro com anel e claro; rotulos, duracoes e paleta."""
    out = out or SAIDA
    tmp = os.path.join(out, 'processo', '_vit'); os.makedirs(tmp, exist_ok=True)
    CW, Z, NC = 372, 6, 5
    names = list(ANIMS)
    panels = []
    for pid, (bg, fg, sub, ring) in enumerate((
            (DARK, '#C9CCE0', 'fundo escuro #0B0C16  ·  visual padrão (recorte frio nas bordas de baixo-direita)', False),
            (DARK, '#C9CCE0', 'fundo escuro #0B0C16  ·  variante de tema escuro com anel #5E5A86 (a recomendada para o escuro)', True),
            (LIGHT, '#4A4E66', 'fundo claro #EFF1F5  ·  visual padrão', False))):
        cells = []
        for name in names:
            fr = frames_of(name, ring)
            cols = RING_COLORS if ring else PALETTE
            key = os.path.join(tmp, f'{pid}_{name}_k.png')
            write_png(key, fr[KEY[name]][1], cols)
            big = os.path.join(tmp, f'{pid}_{name}_b.png')
            mk(key, '-background', bg, '-flatten', '-filter', 'point', '-resize', f'{Z*100}%', big)
            ones = []
            for i, (ms, cv) in enumerate(fr):
                f = os.path.join(tmp, f'{pid}_{name}_{i:02d}.png')
                write_png(f, cv, cols)
                ones.append(f)
            rows = []
            for r0 in range(0, len(ones), 7):
                rr = os.path.join(tmp, f'{pid}_{name}_r{r0}.png')
                mk(*ones[r0:r0 + 7], '-background', bg, '-alpha', 'remove', '-alpha', 'off',
                   '-bordercolor', bg, '-border', '1x0', '+append', rr)
                rows.append(rr)
            strip = os.path.join(tmp, f'{pid}_{name}_s.png')
            mk(*rows, '-background', bg, '-gravity', 'west', '-splice', '0x4', '-append', strip)
            total = sum(ms for ms, _ in fr)
            lab = os.path.join(tmp, f'{pid}_{name}_l.png')
            _label(lab, f'{TITLES[name]}  ·  {len(fr)} quadros  ·  {total/1000:.2f} s{"  (loop)" if name in LOOPS else ""}',
                   bg, fg, 15, CW, 26, 'Adwaita-Sans-SemiBold')
            cap6 = os.path.join(tmp, f'{pid}_{name}_c6.png')
            _label(cap6, f'6x · quadro {KEY[name]}', bg, fg, 11, CW, 16, 'Adwaita-Mono')
            cap1 = os.path.join(tmp, f'{pid}_{name}_c1.png')
            _label(cap1, '1x · todos os quadros', bg, fg, 11, CW, 16, 'Adwaita-Mono')
            cell = os.path.join(tmp, f'{pid}_{name}_cell.png')
            mk(lab, cap6, big, cap1, strip, '-background', bg, '-gravity', 'west', '-append',
               '-gravity', 'north', '-extent', f'{CW}x{26+16+CELL*Z+16+2*(CELL+4)+12}', cell)
            cells.append(cell)
        rowsimg = []
        for r in range(0, len(cells), NC):
            ri = os.path.join(tmp, f'{pid}_row{r}.png')
            mk(*cells[r:r + NC], '-background', bg, '-splice', '14x0', '+append', ri)
            rowsimg.append(ri)
        head = os.path.join(tmp, f'{pid}_head.png')
        _label(head, '  ' + sub, bg, fg, 18, NC * (CW + 14) + 14, 44, 'Adwaita-Sans-SemiBold')
        p = os.path.join(tmp, f'{pid}_panel.png')
        mk(head, *rowsimg, '-background', bg, '-gravity', 'west', '-append', '-bordercolor', bg, '-border', '0x10', p)
        panels.append(p)
    body = os.path.join(tmp, 'body.png')
    mk(*panels, '-append', body)
    W = int(subprocess.run(['magick', 'identify', '-format', '%w', body], capture_output=True, text=True).stdout)
    title = os.path.join(tmp, 'title.png')
    _label(title, f'  Zeca · arte original (CC0) · rodada 3 · {len(names)} animações · célula 48×48 · '
                  'chão em y=44 · 16 cores (+1 só no anel do tema escuro)', '#1A1B2A', '#E6E8F4', 22, W, 56,
           'Adwaita-Sans-SemiBold')
    sw = []
    for ch, hx in list(PALETTE.items()) + [('anel', RING)]:
        s = os.path.join(tmp, f'sw_{ch}.png')
        mk('-size', '56x40', f'xc:{hx}', '(', '-background', '#1A1B2A', '-fill', '#C9CCE0', '-font', 'Adwaita-Mono',
           '-pointsize', '11', '-size', '74x30', '-gravity', 'center', f'label:{ch} {hx}', ')',
           '-background', '#1A1B2A', '-gravity', 'center', '-append', '-bordercolor', '#1A1B2A', '-border', '4', s)
        sw.append(s)
    pal = os.path.join(tmp, 'pal.png')
    mk(*sw, '-background', '#1A1B2A', '+append', pal)
    mk(pal, '-background', '#1A1B2A', '-gravity', 'center', '-extent', f'{W}x92', pal)
    mk(title, body, pal, '-append', '+repage', os.path.join(out, 'vitrine.png'))
    shutil.rmtree(tmp)

# ============================================================= ferramentas ==
def mk(*args):
    subprocess.run(['magick'] + [str(a) for a in args], check=True)

def dev_sheet(poses, out, labels=None, zoom=8):
    """Folha de inspecao: cada pose em 8x com grade (fundo cinza), 6x escuro, 6x claro, 1x."""
    tmp = os.path.join(SAIDA, 'processo', '_tmp')
    os.makedirs(tmp, exist_ok=True)
    cols = []
    for i, layers in enumerate(poses):
        cv = render(layers)
        src = os.path.join(tmp, f'p{i}.png')
        write_png(src, cv)
        z = os.path.join(tmp, f'p{i}_z.png')
        # 8x com grade
        mk(src, '-background', '#5A5F7A', '-flatten', '-filter', 'point', '-resize', f'{zoom*100}%',
           '(', '-size', f'{CELL*zoom}x{CELL*zoom}', 'xc:none', '-fill', 'none', '-stroke', '#00000030',
           '-draw', ' '.join(f'line {k*zoom},0 {k*zoom},{CELL*zoom}' for k in range(CELL)) + ' ' +
                    ' '.join(f'line 0,{k*zoom} {CELL*zoom},{k*zoom}' for k in range(CELL)), ')',
           '-composite', z)
        d6 = os.path.join(tmp, f'p{i}_d.png'); l6 = os.path.join(tmp, f'p{i}_l.png')
        mk(src, '-background', '#0B0C16', '-flatten', '-filter', 'point', '-resize', '600%', d6)
        mk(src, '-background', '#EFF1F5', '-flatten', '-filter', 'point', '-resize', '600%', l6)
        one = os.path.join(tmp, f'p{i}_1.png')
        mk('(', src, '-background', '#0B0C16', '-flatten', ')', '(', src, '-background', '#EFF1F5', '-flatten', ')',
           '+append', '-background', '#30324A', '-gravity', 'center', '-extent', '120x64', one)
        col = os.path.join(tmp, f'p{i}_col.png')
        lab = labels[i] if labels else str(i)
        mk('-background', '#30324A', '-fill', '#E0E2F0', '-font', 'Adwaita-Mono', '-pointsize', '16',
           f'label:{lab}', z, d6, l6, one, '-gravity', 'center', '-append', col)
        cols.append(col)
    mk(*cols, '-background', '#30324A', '-splice', '8x0', '+append', out)
    shutil.rmtree(tmp)

def dev_zoom(poses, out, labels=None, zoom=10, bg='#5A5F7A', crop=None, ones=False):
    """So o zoom com grade, varias poses lado a lado (para comparar variantes).
    crop=(x, y, w, h) em pixels da celula; ones=True acrescenta 1x escuro/claro embaixo."""
    tmp = os.path.join(SAIDA, 'processo', '_tmpz')
    os.makedirs(tmp, exist_ok=True)
    cols = []
    cx, cy, cw, ch = crop or (0, 0, CELL, CELL)
    for i, layers in enumerate(poses):
        cv = render(layers)
        src = os.path.join(tmp, f'p{i}.png')
        write_png(src, cv)
        z = os.path.join(tmp, f'p{i}_z.png')
        mk(src, '-crop', f'{cw}x{ch}+{cx}+{cy}', '+repage', '-background', bg, '-flatten',
           '-filter', 'point', '-resize', f'{zoom*100}%',
           '(', '-size', f'{cw*zoom}x{ch*zoom}', 'xc:none', '-fill', 'none', '-stroke', '#00000030',
           '-draw', ' '.join(f'line {k*zoom},0 {k*zoom},{ch*zoom}' for k in range(cw)) + ' ' +
                    ' '.join(f'line 0,{k*zoom} {cw*zoom},{k*zoom}' for k in range(ch)), ')',
           '-composite', z)
        if ones:
            o = os.path.join(tmp, f'p{i}_o.png')
            mk('(', src, '-background', '#0B0C16', '-flatten', ')', '(', src, '-background', '#EFF1F5', '-flatten', ')',
               '+append', '-background', '#30324A', '-gravity', 'center', '-extent', f'{max(cw*zoom,100)}x56', o)
            mk(z, o, '-gravity', 'center', '-background', '#30324A', '-append', z)
        lab = labels[i] if labels else str(i)
        col = os.path.join(tmp, f'p{i}_c.png')
        mk('-background', '#30324A', '-fill', '#E0E2F0', '-font', 'Adwaita-Mono', '-pointsize', '18',
           f'label:{lab}', z, '-gravity', 'center', '-append', col)
        cols.append(col)
    mk(*cols, '-background', '#30324A', '-splice', '6x0', '+append', out)
    shutil.rmtree(tmp)



def read_png(path):
    """Leitor PNG minimo (RGBA 8 bits, sem entrelacamento) para verificar o que foi exportado."""
    data = open(path, 'rb').read(); pos = 8; idat = b''
    while pos < len(data):
        ln = struct.unpack('>I', data[pos:pos + 4])[0]; typ = data[pos + 4:pos + 8]; ch = data[pos + 8:pos + 8 + ln]
        pos += 12 + ln
        if typ == b'IHDR':
            w, h, bd, ct = struct.unpack('>IIBB', ch[:10])
        elif typ == b'IDAT':
            idat += ch
    raw = zlib.decompress(idat); bpp = 4; stride = w * bpp
    rows = []; prev = bytearray(stride); k = 0
    for y in range(h):
        f = raw[k]; k += 1; line = bytearray(raw[k:k + stride]); k += stride
        for x in range(stride):
            a = line[x - bpp] if x >= bpp else 0; b = prev[x]; c = prev[x - bpp] if x >= bpp else 0
            if f == 1: line[x] = (line[x] + a) & 255
            elif f == 2: line[x] = (line[x] + b) & 255
            elif f == 3: line[x] = (line[x] + ((a + b) >> 1)) & 255
            elif f == 4:
                p_ = a + b - c; pa, pb, pc = abs(p_ - a), abs(p_ - b), abs(p_ - c)
                line[x] = (line[x] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        rows.append(bytes(line)); prev = line
    return [[tuple(rows[y][x * 4:x * 4 + 4]) for x in range(w)] for y in range(h)]

def verify_exports(out=None):
    """Relê os PNG exportados (padrao e escuro) e confere as regras duras pixel a pixel."""
    import glob
    out = out or SAIDA
    inv = {hex2rgb(v): k for k, v in RING_COLORS.items()}
    res = {}
    for variant, pat in (('padrao', 'frames/*.png'), ('escuro', 'frames/escuro/*.png')):
        files = sorted(glob.glob(os.path.join(out, pat)))
        alphas, cols, bad = set(), set(), []
        for f in files:
            px = read_png(f)
            cv = [[None if p[3] == 0 else inv.get(p[:3], '?') for p in row] for row in px]
            alphas |= {p[3] for row in px for p in row}
            fc = {c for row in cv for c in row if c}
            cols |= fc
            prob = {}
            if '?' in fc: prob['cor_fora_da_paleta'] = 1
            if len(fc) > (17 if variant == 'escuro' else 16): prob['cores'] = len(fc)
            if orphans(cv): prob['orfaos'] = len(orphans(cv))
            if holes(cv): prob['furos'] = len(holes(cv))
            if variant == 'padrao':
                edge = [(x, y) for y in range(CELL) for x in range(CELL) if cv[y][x] == 'R' and
                        any(not _in(x + dx, y + dy) or cv[y + dy][x + dx] is None for dx, dy in N4)]
                if edge: prob['recorte_na_borda'] = len(edge)
            pts = [(x, y) for y in range(CELL) for x in range(CELL) if cv[y][x]]
            if min(p[0] for p in pts) < (0 if variant == 'escuro' else 1) or max(p[0] for p in pts) > (CELL - 1 if variant == 'escuro' else CELL - 2) \
                    or min(p[1] for p in pts) < (0 if variant == 'escuro' else 1) or max(p[1] for p in pts) > (FLOOR + 1 if variant == 'escuro' else FLOOR):
                prob['margem'] = 1
            if prob:
                bad.append((os.path.basename(f), prob))
        res[variant] = dict(quadros=len(files), alfas=sorted(alphas), cores=len(cols), preto=(0, 0, 0) in [hex2rgb(RING_COLORS[c]) for c in cols if c in RING_COLORS],
                            problemas=bad)
    return res

def verificar(out):
    """Lint das composicoes e releitura dos PNG exportados em `out`.  True se tudo passou."""
    allc, rep = check_all(verbose=True)
    bad = {k: o for k, (n, o, s, c) in rep.items() if o}
    sp = sum(s for n, o, s, c in rep.values()) / len(rep)
    kc = sum(c for n, o, s, c in rep.values()) / len(rep)
    print(f'lint (composicao): {len(rep)} quadros, {len(allc)} cores, {len(bad)} quadros com problema; '
          f'ilhas de 1 px/quadro {sp:.1f}; blocos 2x2 de K/quadro {kc:.1f}')
    ok = not bad and len(allc) <= 16
    for variant, r in verify_exports(out).items():
        print(f'PNG exportados ({variant}): {r["quadros"]} quadros, alfas {r["alfas"]}, {r["cores"]} cores, '
              f'preto puro: {r["preto"]}, problemas: {r["problemas"] or "nenhum"}')
        ok = ok and not r['problemas'] and r['alfas'] == [0, 255] and not r['preto']
    return ok

USO = 'uso: python3 zeca.py [--saida DIR] | python3 zeca.py --quadros DIR'

def main(argv):
    global SAIDA
    if argv[:1] == ['--quadros'] and len(argv) == 2:
        # so os quadros e o manifesto, sem ImageMagick (o `cargo xtask zeca-livre` usa)
        out = argv[1]
        os.makedirs(out, exist_ok=True)
        shutil.rmtree(os.path.join(out, 'frames'), ignore_errors=True)
        export_frames(out, folhas=False)
        sys.exit(0 if verificar(out) else 1)
    if argv[:1] == ['--saida'] and len(argv) == 2:
        SAIDA = os.path.abspath(argv[1])
    elif argv:
        print(USO, file=sys.stderr)
        sys.exit(2)
    os.makedirs(SAIDA, exist_ok=True)
    for d in ('frames', 'gifs'):
        shutil.rmtree(os.path.join(SAIDA, d), ignore_errors=True)
    export_frames(SAIDA)
    export_gifs(SAIDA)
    vitrine(SAIDA)
    ok = verificar(SAIDA)
    print(f'saida em {SAIDA}')
    sys.exit(0 if ok else 1)

if __name__ == '__main__':
    main(sys.argv[1:])
