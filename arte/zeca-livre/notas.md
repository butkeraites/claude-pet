# Zeca: arte original (esboço A "chibi redondo"), rodadas 2 e 3

Zeca é um papagaio malandro de mesa, 100% original. Ele é verde, de perfil para a direita, tem
olho branco grande com pupila e brilho, bico rosa em gancho, cabeça e barriga verde-claras, asas
mais escuras e pés laranja. Usa um chapéu de palha (boater) com fita laranja e uma gravata-borboleta
rosa. Tudo o que ele faz é gesto de pássaro: asa, bico, pé e penas.

**Licença:** arte original feita com o Claude para o projeto bichinho, dedicada ao domínio público
pela **CC0 1.0** (`LICENSE`, nesta pasta). O crédito de cortesia é bem-vindo, não obrigatório.

A rodada 2 aplicou a crítica do diretor de arte (nota 6,8 na rodada 1); a crítica final dela deu
**8,4/10, publicável**. A rodada 3 (seção 11) é a entrada no repositório: as duas correções rápidas
da crítica final, o que o próprio rig resolve e os gestos e transições que o pet pede.

Para gerar tudo (quadros, folhas, GIFs, vitrine e a verificação), rode `python3 zeca.py`: leva
cerca de 40 s e grava em `tmp/zeca-livre/`, na raiz do repositório (fora do git). O `anims.json`
desta pasta e as skins `skins/zeca-livre/` e `skins/zeca-livre-escuro/` saem do
`cargo xtask zeca-livre`, que roda `python3 zeca.py --quadros` (só os quadros e o manifesto, sem
ImageMagick, em ~4 s) e monta as skins pela receita `skin.toml`; o `cargo xtask zeca-livre
--conferir` roda o gerador duas vezes, compara os bytes e confere que o que está no git é o que sai
dele.

---

## 1. Entregas

Nesta pasta (no git):

| arquivo | conteúdo |
|---|---|
| `zeca.py` | Fonte da arte: paleta, grades das peças, gerador do chapéu, motor do rig, 22 animações, exportação, vitrine, verificação (lint) e releitura dos PNG |
| `anims.json` | Manifesto com quadros, ms, loop, paleta, anel, licença, `uso` (tema, idle, transições de cada laço), `gatilhos_sugeridos` e `trechos` do idle; os caminhos dos quadros são relativos à pasta de saída do gerador |
| `skin.toml` | A receita das skins: as tags (pedaços das animações), o mapa dos estados do pet, os pés e as tags no chão |
| `LICENSE` | A dedicação CC0 1.0, o crédito de cortesia e o texto legal |
| `notas.md` | Este arquivo |

Na saída do gerador (`tmp/zeca-livre/`, fora do git; `python3 zeca.py --saida DIR` troca):

| arquivo | conteúdo |
|---|---|
| `frames/<anim>_NN.png` | 134 quadros 48×48 RGBA, com alfa só 0/255, no visual padrão (é o do tema claro) |
| `frames/escuro/<anim>_NN.png` | Os mesmos 134 quadros com anel de 1 px `#5E5A86` por fora. **Use estes no tema escuro** |
| `frames/folhas/` | Folha horizontal 1x de cada animação, padrão e `_escuro` |
| `gifs/<anim>.gif` | Prévia 4x em três painéis: escuro, escuro com anel e claro, com rótulo em cada painel |
| `gifs/1x/<anim>.gif` | GIF transparente em tamanho real. Os quadros e as durações conferem pixel a pixel com os PNG |
| `anims.json` | O mesmo manifesto desta pasta |
| `vitrine.png` | As 22 animações, cada uma com o quadro-chave em 6x e todos os quadros em 1x, em três painéis empilhados: escuro padrão, escuro com anel e claro. Tem rótulos e paleta |

A vitrine e a prancha da rodada 2, a comparação antes × depois, os renders de cada rodada
(`processo/v2_r16` a `v2_r53`) e os scripts de auditoria do diretor ficaram fora do repositório: as
referências a eles nas seções 3 e 8 são a história da rodada 2.

## 2. Paleta: 16 cores, mais 1 só na variante de tema escuro

A paleta não mudou. A gravata é que passou a usar só `2` e `3` (item 10).

| char | hex | papel | contraste com escuro / claro |
|---|---|---|---|
| `K` | `#2B2136` | contorno: ameixa escura, nunca preto | 1,27 / 13,5 |
| `A` | `#2E7487` | petróleo: pontas de pena, z do sono, faísca do trabalho | 3,68 / 4,68 |
| `S` | `#2B7D55` | verde sombra (frio); vinco entre peças | 3,86 / 4,46 |
| `M` | `#4DAF4B` | verde médio: asa, sombra suave | 7,00 / 2,46 |
| `L` | `#8FD653` | verde claro: cabeça e barriga | 11,1 / 1,56 |
| `H` | `#D3EF78` | brilho quente | 15,2 / 1,13 |
| `R` | `#A6E8D6` | luz de recorte fria, gota, tecla | 14,0 / 1,23 |
| `W` | `#FFF8EC` | branco quente: olho, tecla | 18,4 / 1,07 |
| `1` | `#FFC4D0` | rosa claro (só no bico e na bochecha) | 13,0 / 1,32 |
| `2` | `#F27FA6` | rosa base: bico; luz da gravata | 7,75 / 2,22 |
| `3` | `#B8457E` | rosa sombra: base da gravata, boca, língua | 3,89 / 4,43 |
| `y` `Y` `B` | `#FBE7A1` `#E2B864` `#A8743E` | palha clara / base / sombra | — |
| `o` `O` | `#F59A3A` `#C25A2C` | laranja: fita, pés, "!" / sombra | 8,87 / 4,44 (escuro) |
| anel | `#5E5A86` | só em `frames/escuro/`: 1 px por fora do `K` | 3,04 / 5,66 |

As rampas fazem hue shift e a luz vem de cima-esquerda. O verde vai de 74° (brilho) a 193°
(petróleo), e o rosa puxa para o magenta na sombra. Palha e laranja escurecem para ocre e tijolo,
não para o frio. O recorte `R` é frio e vem do lado oposto à luz, então só acende nas bordas de
baixo-direita.

## 3. O que mudou, item por item

### Prioridade alta

**1. Furos de fundo dentro do sprite (76 de 80 quadros).** Agora são 0 em 89 quadros, nos dois visuais.
- **Gravata.** Apliquei a correção sugerida. A linha 0 ficou `KKSSSSSKK.`: fecha (32,31) e o vão de
  cima passa a mostrar pescoço. A linha 1 ficou `K2KKKSSK3K`. Fui um passo além na linha 0 porque,
  com a gravata atrás da cabeça (bicada, trabalhando), o vão de cima também furava.
- **Asas abertas.** A raiz de `WING_UP` e `WING_MID` agora entra por baixo da cabeça e do corpo,
  que são pintados por cima. Assim a ponta nunca fica só encostando na cabeça ou no chapéu. A borda
  de baixo da `WING_MID` ganhou 1 px e fecha contra a cauda. A base da `TAIL_FAN` também entra sob o
  corpo, porque no voo a cauda sobe 2 px.
- **Chapéu e cabeça.** Quando o chapéu atrasa para cima, abre um bolsão de 1 px entre o degrau da aba
  e a cabeça redonda. O motor fecha esse bolsão com contorno, porque é ali que os dois contornos se
  encontram.
- **Lint.** A verificação ganhou a função `holes()`: nenhum componente transparente (4-viz) pode ficar
  sem ligação com a borda da célula. Isso cobre o pixel de alfa 0 com os 4 vizinhos opacos e os
  bolsões de 2 a 3 px. A regra roda nos 89 quadros e na variante com anel. Também corrigi o
  `light_outline`: bolsão de até 3 px fechado pelo próprio anel vira anel, porque antes aparecia
  um furo escuro entre a ponta do bico e a gravata. Onde o anel de um efeito e o do corpo fechavam
  áreas grandes, afastei o efeito: tracinhos do susto, faísca e o chapéu no alto do susto.

**2. Recorte `R` na borda da silhueta (67 de 80 quadros).** Agora são 0.
- **Cauda.** Troquei todo `a` por `A`. As duas últimas linhas viraram `.kk.kkk...` e a linha de dentes
  saiu: o contorno fecha em y41 com um entalhe só. O miolo da cauda ficou o do paintover (pontas
  `A` atrás, faixa `M`, `S` sob a asa), com 4 cores em vez de 5.
- **`BEAK_UP`.** A linha 0 virou `kKKKKK....`, e o `1` não encosta mais no fundo.
- **Lint.** `rim_problems()` aceita `R` (fora dos efeitos) só com os 4 vizinhos opacos e em trechos de
  2 px ou mais (8-viz). Hoje todos os trechos têm de 3 a 12 px.
- **Motor.** O recorte agora só acende contra o contorno da **própria** peça. Esse contorno conta como
  exposto contra o vazio ou contra uma peça de trás de outro papel. Por isso a perna de longe não
  corta mais a linha da barriga.

**3. Aba colada no olho e no bico.** Todos os chapéus subiram 1 px em relação ao olho
(`HAT_SEAT_IDLE = (23, 16)`). Entre a aba e o olho ficam 2 px de testa, e a linha de cima recebe
a sombra `~`. No idle_00, os blocos 2×2 de `K` caíram de 14 para 7, e os 7 que sobram estão na
pupila, que é cheia de propósito. Com o `HAT_6` neutro, a ponta da aba fica 2 px acima do bico.
- **Canto da cabeça.** Não precisei mudar o canto de cima à direita da `HEAD`. Resolvi no motor,
  com duas regras gerais que valem para qualquer chapéu:
  - O contorno da cabeça coberto pelo chapéu vira preenchimento (`fill='L'`), e não vinco `S`. Antes,
    a sombra escurecia esse vinco para `A` e deixava pontinhos petróleo na testa.
  - A sombra projetada passou a ser aplicada **depois** da resolução do contorno e da limpeza, e por
    isso a faixa sob a aba sai contínua.

  O resultado é verificado no despejo do idle_00: cabeça e raiz do bico fecham sob a aba sem pixel solto.
- **Atraso do chapéu.** Agora ele encosta no olho em vez de fundir. No idle_01, os únicos blocos de
  `K` estão na pupila.

**4. Bicada que vira bule.** Contando todo pixel opaco, como na crítica, a área no golpe fica em 691
e 667 px, contra 710 no neutro (−3% e −6%). Era 540 contra 676, ou seja, −20%. Sem os efeitos
(grão e farelo), são 677 e 643 contra 696.
- **`BODY_LEAN` novo.** É o `BODY` cisalhado: ombro 3 px à frente e 2 px abaixo, barriga parada. A asa
  e a `TAIL_UP` vêm junto. Ele é usado nos quadros 3 a 5 e no "trabalhando".
- **A cabeça gira em vez de afundar.** Aqui me afastei da receita (`head_d` até (4,6)) de propósito.
  Com o bico pendurado na base de uma cabeça baixa, testei 7 variantes (`v2_r23` e `v2_r24`), e todas
  liam como chinelo ou bota rosa. O papagaio bica girando a cabeça. Por isso o bico desce pela frente
  da cara, a cabeça só vai a (5,2) e o vinco com o corpo continua visível. O olho, que olha para o
  grão, mantém a cara legível no golpe.
- **`BEAK_DOWN` refeito.** Tem a mesma massa do bico normal, girado cerca de 60°: saiu de uma rotação
  RotSprite (`processo/v2_rotsprite.py`) limpa à mão. O gancho aponta para baixo e a ponta pousa no
  grão em y42.
- **`BEAK_45` também cresceu.** Tem a mesma massa e é usado na aproximação e no "trabalhando".
- **`TAIL_UP`.** É um leque de 3 penas erguido para trás e para cima, com pontas `A` separadas.
- **Gravata.** Fica sempre visível: na frente, sob a garganta, deslocada (−1,+2). Nenhum quadro usa
  `bow=None`.
- **Grão.** Mudou para (36,42), longe do pé de longe. Na aproximação fica a 1 px da ponta do bico
  (medido) e no golpe a ponta toca o grão.
- **Gag do chapéu (feito).** No quadro 5 o chapéu escorrega até a raiz do bico e cobre o olho. No 6,
  o tranco da cabeça faz ele pular e voltar.

**5. Voo.** O corpo inclina cerca de 15°: cabeça 2 px à frente e abaixo, cauda 2 px acima.
- **Chapéu.** `hat_d` faz (0,−1), (0,+1), (0,+1), (0,−1), e o chapéu repete exatamente a altura do
  quadro anterior. Conferi pelo deslocamento: chapéu −4, −3, −4, −5 contra olho −3, −4, −5, −4.
- **Asa de longe.** `far_wing()` gera uma cópia mais escura de cada asa (`L/M`→`S`, `S`→`A`), pintada
  atrás do corpo. Na batida para cima e no meio ela dá profundidade. Na batida para baixo, a borda de
  ataque aparece acima das costas.
- **Batida para baixo.** A `WING_DOWN` foi redesenhada: desce por baixo da barriga, com as pontas sob
  o corpo, longe da cauda. Erguer a asa antiga não bastava, porque a cauda também subiu com a
  inclinação.

**6. Efeitos que somem.**
- **`FX_JOLT`.** São tracinhos de 2 a 3 px com miolo `y` e contorno `K`, legíveis nos dois temas.
- **z do sono.** Saiu a caixa: agora é só a letra em `A`, `FX_Z2` = `AAA/.A./A../AAA` e
  `FX_Z` = `AAAA/..A./.A../AAAA`. Testei o pixel de `R` no traço de cima (`v2_r52_z.png`): no claro
  ele some (1,23:1) e encurta o traço, e no escuro quase não se nota. Ficou só `A`.

**7. Estado "trabalhando" (`work`, novo).** São 7 quadros em 0,91 s, em loop.
- **Pose.** Curvado (`BODY_LEAN`), com `BEAK_45` e `EYE_DOWN`, sobre uma tecla de teclado (`FX_KEY`,
  com o sublinhado em petróleo).
- **Ritmo.** Duas batidas rápidas de 80 ms afundam a tecla, depois vem uma pausa de 300 ms com a
  cabeça um pouco erguida, conferindo.
- **Chapéu e faísca.** O `HAT_12` desce 1 px para a frente a cada batida e volta 1 quadro depois. A
  cada ciclo sobe uma faísca `A`.
- **Gravata.** Fica atrás da cabeça, no peito, para não empilhar com o bico.
- **Pés.** O pé de longe recua 2 px, uma base fechada de quem está debruçado. Uma busca automática
  passou por 144 combinações de cabeça, gravata e tecla, das quais 22 não tinham furo. Comparei 4
  delas a olho (`v2_r39` a `v2_r42`).

### Prioridade média

**8. Ginga e pose-base.**
- **Pose-base.** Peito +1 (`BODY`, e `BOW` +1 no mundo), cabeça e tudo o que é preso a ela −1, e pé
  de longe de x26 para x28.
- **Chapéu do neutro.** O `HAT_6` passou a ser o neutro: o malandro "encostado".
- **Batidas.** Nos quadros 8 e 10, `body_d=(1,0)` e `head_d=(-2,-1)`: o quadril vai e a cabeça fica.
  A cauda balança nos quadros 9 e 11.
- **Pé que bate.** `FOOT_F_UP` foi refeito: calcanhar no chão, dedos numa diagonal limpa de 3 px e
  nenhum `K` por dentro.
- **Chapéu do queixo erguido.** Usa o `HAT_4` de copa reta: o de copa em escada serrilhava
  (`v2_r31`).

**9. Corpo e asa salpicados.**
- **Asa.** Campo `M` limpo, ombro `L` em diagonal de 2×2, rêmiges `A` em faixa diagonal de (14,38)
  a (17,40) e nenhum `K` por dentro. A borda `S` da frente é o próprio vinco da asa sobre o corpo.
- **Barriga.** Aparece um crescente `L` sob a gravata e na frente da asa, e o recorte vira uma linha
  contínua de 11 px na base e na diagonal da frente.
- **Ilhas de 1 px no idle_00.** Caíram de 21 para 6, o mesmo número do paintover. As 6 que sobram
  são de propósito: o brilho do olho, a ponta do gancho, o vinco diagonal da asa (2 px), o início do
  crescente da cabeça e a sombra da aba contornando a nuca.

**10. Gravata e bico empilhados.** A gravata desceu um tom: base `3` e luz `2`, sem `1`. O bico
segue em `1/2/3`. Em 1x os dois se separam pelo valor.

**11. Agachado (`BODY_SQ`).**
- **Cauda.** Passou para (−2,0) e fica 3 px acima do chão.
- **Pés.** Abrem 1 px para fora, e o pé de perto é pintado **depois** do corpo
  (`foot_n_front=True`), então os 3 dedos aparecem.
- **Gravata.** Sobe 2 px: antes ela afundava até os pés e fechava um furo.
- **Recorte.** O corpo agachado ganhou uma linha de recorte embaixo.

**12. Bico aberto.**
- **`BEAK_OPEN`.** A mandíbula de baixo desce 1 px a mais e abre um V de 2 px de fundo na frente, sob
  o gancho, que sobe 1 px (o bico de cima do papagaio é articulado). A garganta é `K` e a língua `3`
  fica só na base.
- **`BEAK_WIDE`.** É da mesma família, com a mandíbula 1 px mais baixa e a garganta mais funda.
- **Leitura.** Agora lê como bico aberto em 1x, no claro e no escuro (`v2_r20` e `v2_r21`).

**13. Susto sem antecipação.** Entrou o quadro 1, de 80 ms: agachado, olho fechado e chapéu prensado.
Depois vem o take, com olho arregalado.

**14. Chamar atenção.** O pico agora é y=−8: a barriga vai de y40 a y32, e o topo do chapéu fica em
y3-4. O pico é segurado por 2 quadros (220 ms) e o "!" acompanha sem bater no chapéu.

**15. Poeira, notas e grão.**
- **Poeira.** Sai em par, um tufo de cada lado dos pés. No quadro seguinte abre 2 px para fora,
  menor e em `R`, e depois some.
- **Notas.** Saem em x39, com pelo menos 2 px de fundo até o bico, e sobem na mesma coluna (y11 → 6 →
  1). Não usei (41,12): a nota tem 8 px de largura e passaria da margem de 1 px que o anel precisa.

**16. Tema escuro.**
- **Manifesto.** O `anims.json` registra em `uso.tema` que o app usa `frames/escuro/` (anel) sempre
  que o tema for escuro, e o visual padrão no claro.
- **Recorte no visual padrão.** Não há mais `R` isolado: todo trecho tem de 3 a 12 px (8-viz) e fica
  só nas bordas de baixo-direita. No corpo neutro é uma linha contínua de 11 px, na base da barriga e
  na diagonal da frente. A sugestão era usar trechos de 3 a 6 px, mas aqui o recorte segue a borda
  inteira e lê como um traço só.

### Prioridade baixa

**17. Chapéu.** Testei os três modos de copa em todas as inclinações (`v2_r16`, `v2_r31` e `v2_r32`):
o degrau seguindo a aba, um degrau no meio e o tampo reto.
- **Tampo reto em todos.** Ganhou: a copa vira um retângulo limpo, e o tampo chato é a marca do
  boater. Sumiram a "chaminé" do `HAT` (8+2), os "dois tijolos" do `HAT_F6` e a escada do `HAT_4`.
  A copa também não "pula" quando o chapéu troca de inclinação no meio da animação.
- **Recorte do bico pela aba.** É uma regra no motor: pixel de bico com a aba logo abaixo, a 1-3 px na
  mesma coluna, sai. Isso resolve o sono.
- **Topete e nuca.** Ficam escondidos enquanto o chapéu está na cabeça. Nos dois quadros com a asa
  erguida atrás fica só o topete, porque a nuca virava hachura `K` sobre a asa (`v2_r49` a `v2_r51`).

**18. Apresentação.**
- **Título dos GIFs.** Tem margem de 8 px e cada painel tem rótulo.
- **Trechos do idle no manifesto.**
  - `respira`: `{"quadros": [0, 6], "loop": true}`.
  - `ginga`: `{"quadros": [7, 13], "loop": false, "intervalo_s": [8, 20]}`.
  - Em `uso.idle` está escrito para fazer loop só da respiração.

**19. Identidade.** A regra de diferenciação está na seção 5 e vale para versões futuras.

## 4. Escolhas de desenho

- **Proporção chibi do esboço.** Cabeça 20×16, corpo em pera 20×12, olho 8×9 e bico 9×11. Chão em
  y=44. Todo quadro deixa 1 px de margem para o anel.
- **Contorno contextual `k`.**
  - Vira `K` na silhueta e vinco `S` sobre uma peça "parente".
  - Na rodada 2 ganhou uma terceira saída: sob o chapéu, o contorno da cabeça vira o próprio
    preenchimento.
- **Cabeça que gira.** Na bicada e no "trabalhando", a inclinação da cabeça é mostrada pelo bico
  girado (45° e 60°, com a mesma massa do bico normal), não por uma cabeça afundada. A cabeça
  redonda não muda de silhueta ao girar: quem conta o giro são o bico, o olho e o chapéu.
- **Chapéus.** São 7 inclinações do mesmo objeto, todas com tampo reto:
  - `HAT_6`: neutro.
  - `HAT_4`: queixo erguido e rodopio.
  - `HAT`: assentando.
  - `HAT_12`: quase reto, no trabalhando.
  - `HAT_0`: prensado.
  - `HAT_F6` e `HAT_F4`: caindo para a frente.
- **Tecla.** É o único adereço novo, uma tecla de teclado. Ela afunda 1 px quando é bicada e liga o
  pet ao contexto "Claude Code trabalhando" sem dar a ele gesto humano.

## 5. Regra de diferenciação (model sheet)

Zeca não pode derivar para personagem comercial conhecido. Valem sempre:

- Fita do chapéu **sempre laranja** e gravata **sempre rosa**: nunca pretas.
- Bico **sempre rosa**, nunca amarelo.
- **Sem paletó, colete, luvas, guarda-chuva, charuto ou braço humano.** Todo gesto é feito com asa,
  bico, pé ou penas.
- Chapéu de palha **de copa baixa e tampo reto** (boater), inclinado para trás no neutro.
- Proporção chibi, de cabeça grande, olho grande e corpo em pera.
- **"Zeca" é o único nome.**

## 6. O rig (para as próximas animações)

- **Peças.** `part(nome, x, y, grade, role, soft, over, fill)` registra uma grade em coordenadas
  absolutas do idle. São 74 peças, entre elas:
  - 5 corpos: normal, agachado, esticado, eriçado e inclinado.
  - 7 olhos.
  - 6 bicos: fechado, queixo erguido, piando, grito, 45° e golpe.
  - 4 asas de perto em uso, mais 3 de longe geradas por `far_wing()`. A asa diagonal antiga
    (`WING_DOWN_DIAG`) fica registrada só como molde da asa de longe da batida para baixo.
  - 3 caudas.
  - 7 pés.
  - 14 chapéus gerados: 7 inclinações, cada uma com e sem sombra.
  - 19 efeitos.
- **Hierarquia.** Tudo `(x, y)` → corpo (`body_d` mais `BODY_META`) → cabeça (`head_d`) → olho
  (`eye_d`). Chapéu, bico e bochecha ficam presos ao olho; a gravata, à cabeça (ou ao pescoço, com
  `bow_back`). Os pés ficam no chão, a menos que `air=True`.
- **Opções novas da pose.**
  - `far_wing` e `far_d`: asa de longe.
  - `foot_n_front`: pé de perto por cima do corpo.
- **Regras automáticas.**
  - Pernas.
  - Sombra da aba, aplicada por último.
  - Contorno contextual com `fill` sob o chapéu.
  - Recorte só contra o contorno da própria peça.
  - Recorte do bico pela aba.
  - Fechamento de bolsão entre aba e cabeça.
  - Limpeza de órfão de montagem.
  - Anel do tema escuro, que fecha bolsões de até 3 px.
- **Nova animação.** Acrescente `ANIMS['nome'] = [F(ms, **pose), ...]`, mais `KEY` e `TITLES`.
  Vitrine, GIFs, manifesto e verificação pegam tudo sozinhos. O lint reprova furo, recorte na borda
  ou solto, órfão, mais de 16 cores e saída da margem e, desde a rodada 3, efeito sem contorno ou
  poeira por cima do corpo. Para virar estado do pet, a animação entra na receita da skin
  (`cargo xtask zeca-livre`, seção 11).

## 7. As 10 animações da rodada 2 (as 12 da rodada 3 estão na seção 11)

| anim | quadros | duração | loop | batidas |
|---|---|---|---|---|
| `idle` | 14 | 3,40 s | trecho `respira` | respira ×2 com o chapéu atrasando → queixo erguido e olho malandro → ginga ×2 (quadril vai, cabeça fica, calcanhar no chão e dedos batendo) → cauda balança → volta |
| `blink` | 5 | 1,19 s | — | aberto → meio → "‿" → meio → aberto |
| `chirp` | 8 | 1,34 s | — | antecipa → bico abre em V e sai a ♪ (2 px do bico) → fecha → abre e sai a ♪♪ → notas sobem na coluna → assenta |
| `peck` | 12 | 1,73 s | — | olha o grão → recua → inclina (ombro à frente) e aproxima a 1 px → cabeça gira e o bico pousa no grão → chapéu escorrega até o bico, farelo → tranco: chapéu pula e volta → mastiga ×2 |
| `hop` | 12 | 1,57 s | — | agacha (dedos à mostra) → estica → sobe com as asas → pico com o chapéu rodopiando → cai → pousa agachado com poeira dos dois lados → chapéu pousa 1 quadro depois e quica → brilho |
| `fly` | 4 | 0,34 s | sim | asa cima (com a de longe atrás) → meio → baixo (por baixo da barriga, a de longe acima das costas) → meio; corpo inclinado, chapéu com atraso de 1 quadro |
| `scared` | 12 | 1,76 s | — | **antecipação** (encolhe e fecha o olho) → take (arregala, grita, topete, asas, chapéu voa, tracinhos) → pousa → treme ±1 px com gota → chapéu cai torto sobre o olho → meio-olho → ajeita o chapéu |
| `sleep` | 5 | 2,60 s | sim | olho fechado, chapéu escorregado sobre o olho (bico recortado pela aba), cabeça afunda e sobe, asa sobe no "inspira", z/Z sobem |
| `attention` | 10 | 1,11 s | sim | agacha → pulo de 8 px com o pico segurado 2 quadros → grita → pousa → abana a asa ×2 → olhada de lado; "!" o tempo todo |
| `work` | 7 | 0,91 s | sim | debruçado sobre a tecla → bate ×2 (80 ms, a tecla afunda e o chapéu desce e volta 1 quadro depois) → pausa de 300 ms conferindo → faísca sobe |

## 8. Processo e verificação

Em toda rodada fiz render, olhei com zoom de 4x a 16x e em 1x/2x nos dois fundos, critiquei e
redesenhei. Os arquivos estão em `processo/`.

- **Fase 1, modelo base (r16-r19 e r30-r33).**
  - r16: chapéus em 3 modos de copa.
  - r17-r19: idle refeito (aba, gravata, asa, barriga, cauda e pose-base), comparado com a rodada 1
    e com o paintover.
  - r30-r33: ginga e chapéu do queixo, e o tampo reto em todos.
- **Fase 2, peças (r20-r29).**
  - r20-r21: bicos abertos, 6 candidatos lidos em 1x, 2x, 4x e 16x.
  - r22-r26: golpe da bicada, 4 rodadas, até trocar "cabeça afunda" por "cabeça gira".
  - r27-r29: asas abertas, asa de longe e nova batida para baixo.
- **Fase 3, animações (r34-r46 e r49-r52).** Tiras de cada animação. "Trabalhando" levou 6 rodadas
  (r38-r43), inclusive uma busca automática de poses sem furo. Também passaram por aqui o susto, o
  agachado, o sono, o "chamar atenção" e os z.
- **Fase 4, apresentação (r47-r53).** Cabeçalho dos GIFs, vitrine em painéis empilhados, painel com
  anel e voo e pulo no escuro com anel.

A verificação roda em `python3 zeca.py`. O lint é aplicado nas composições, com dono de cada pixel, e
os PNG exportados são relidos e conferidos de novo, nos dois visuais. As métricas comparáveis abaixo
foram tiradas com os scripts do diretor (`_critica/check.py`, `edges.py`, `speckle.py`, `clumps.py`,
`motion.py` e `lift.py`):

| medida | rodada 1 | rodada 2 |
|---|---|---|
| quadros / estados | 80 / 9 | 89 / 10 |
| quadros com furo de fundo (padrão / anel) | 76 / 66 | **0 / 0** |
| quadros com recorte `R` na borda da silhueta | 67 | **0** |
| trechos de recorte com menos de 3 px | vários | **0** (3 a 12 px) |
| ilhas de 1 px por quadro (média) / no idle_00 | 19,5 / 21 | **8,4 / 6** |
| blocos 2×2 de `K` por quadro (média) / no idle_00 | 7,0 / 14 | **5,8 / 7** (os 7 na pupila) |
| área opaca no golpe da bicada × neutro (todo pixel opaco) | 540 × 676 (−20%) | **691 e 667 × 710** (−3% e −6%) |
| altura do pulo do "chamar atenção" | 4 px | **8 px**, segurado 2 quadros |
| borda com menos de 1,5:1 no claro | — | **0** de 14 128 px |
| silhueta efetiva no `#0B0C16` (visual padrão / anel) | 97,8% / 100% | 97,3% / **100%** |
| alfas, cores, preto puro, órfãos estritos | 0/255, 16 (+1), não, 0 | 0/255, 16 (+1), não, 0 |
| GIF 1x idêntico aos PNG (pixels e durações) | sim | sim (89/89) |

## 9. Problemas abertos

- **O neutro ainda é quase vertical.** O "encostado" pedido (peito +1, cabeça −1, pé aberto, `HAT_6`)
  dá a atitude, mas são mudanças de 1-2 px. Quem mais vende o malandro continua sendo a ginga.
- **Desvio de receita na bicada.** A cabeça vai a (5,2) e gira, em vez de ir a (4,6). Com o bico
  pendurado, nenhuma das 7 variantes testadas deixou de parecer bota rosa.
- **Batida para baixo do voo.** Lê bem na prévia 4x, mas em 1x a asa sob a barriga é a parte mais
  fraca do ciclo. A de cima, com a asa de longe, é a mais forte, e por isso é o quadro-chave da vitrine.
- **"Trabalhando" muda a base.** O pé de longe recua 2 px em relação ao neutro. Falta um quadro de
  entrada que mostre o passo; hoje o app troca direto. *(Rodada 3: `work_in` e `work_out`.)*
- **Ilhas de 1 px.** As que sobram se concentram no take do susto (16-17 por quadro): a serrilha do
  corpo eriçado e o topete.
- **Tema escuro no visual padrão.** 96% da borda continua sendo `K`, que tem 1,27:1. O recorte ajuda
  na base, mas a solução de verdade para o escuro é a variante com anel, que o manifesto manda usar.
- **Confete.** O de 2 px quase some em 1x e funciona como textura. Não mexi.
- **Transições.** Ainda faltam as transições entre estados: dormir e acordar, decolar e pousar,
  entrar e sair do trabalho. Também faltam andar e virar para a câmera. O rig cobre tudo isso com as
  peças atuais. *(Rodada 3: as transições de cada laço; andar e virar continuam faltando.)*

## 10. Autoavaliação: **8,3 / 10**

O que subiu:

- Todos os itens altos e médios da crítica foram atendidos, e os baixos também. O pacote técnico está
  fechado nos dois visuais, com zero furo, zero recorte solto e zero órfão. O lint novo impede que
  esses problemas voltem.
- **Bicada.** Deixou de ser bule: lê como bicada de verdade e não perde volume.
- **Voo.** Tem profundidade.
- **Bicos abertos.** Leem em 1x.
- **Cara.** Ficou mais limpa e alegre sem a sobrancelha dupla.
- **"Trabalhando".** Existe e é legível, e é o estado mais visto.

O que segura a nota:

- O neutro ainda é tímido na atitude.
- A batida para baixo do voo é fraca em 1x.
- Faltam as transições entre estados.
- O susto ainda tem mais ruído que o resto.

## 11. Rodada 3: no repositório (2026-10-04)

A crítica final do diretor de arte deu **8,4/10, publicável**, com duas correções rápidas para já e
uma lista do que o próprio rig resolve. Tudo foi feito no gerador (nenhum PNG editado à mão), e o
`cargo xtask zeca-livre --conferir` garante que os bytes saem iguais a cada execução.

### Correções da crítica final

- **Trecho `respira` = quadros 0-3** (era 0-6). Os quadros 4, 5 e 6 repetiam, pixel a pixel, os 0, 1
  e 2, e a costura 6→0 descia cabeça e chapéu juntos: uma expiração em cada duas perdia o atraso do
  chapéu. Agora 0-1-2-3 sobe e desce com o atraso em 1,12 s, e a costura 3→0 é o chapéu
  assentando. O idle completo (0-13) não mudou.
- **Confete fora do bico no `hop_08`.** O confete petróleo de 2 px que caía sobre o bico em
  (36-37, 26) saiu: os dois confetes acabam no `hop_07`.
- **Regra nova no lint** (`fx_sobre_o_corpo`): efeito sem contorno (confete, faísca e ponto do
  trabalho, z do sono) e poeira não podem ser pintados por cima de peça do Zeca, porque leem como
  marca no corpo. Os de contorno fechado (brilho, nota, "!", gota, farelo, tracinhos) e os adereços
  que ele toca (grão, tecla) podem passar na frente. A regra achou mais um: o confete laranja do
  `hop_06` pintado na ponta da asa erguida; agora ele passa atrás da asa (some nesse quadro e
  reaparece no `hop_07`).
- **Poeira do pouso.** O tufo da esquerda colado no rabo (`KWWWK` em (11-15, 42) no `hop_07`, `RR`
  em (10-11, 41) no `hop_08`) foi para trás da ponta da cauda, o único lugar com folga: (1, 42) no
  quadro do impacto e (1, 41), menor, no seguinte (sem espaço para abrir para fora, ele sobe 1 px).

### Gestos e transições (pelo rig, com as peças de sempre)

Cada gesto começa e termina na pose neutra (o pet volta a ela no fim de toda reação). Cada laço de
estado ganhou entrada e saída, listadas em `uso.transicoes.lacos` do manifesto.

| anim | quadros | duração | batidas |
|---|---|---|---|
| `nod` | 5 | 0,81 s | aceno discreto (T0): a cabeça baixa 2 px com o olho feliz e o chapéu tomba para a frente (a "tirada de chapéu" do malandro), volta com o chapéu atrasando. Gesto próprio: o respira e a ginga sobem a cabeça, o aceno abaixa |
| `wave` | 6 | 0,86 s | tchau: a asa de perto abana duas vezes (meio → cima), olho feliz, bico abrindo |
| `yawn` | 7 | 1,76 s | bocejo: o olho pesa, o queixo sobe com o bico bem aberto e o olho fechado, fecha devagar |
| `wake` | 6 | 1,20 s | acordar: pisca pesado, se espreguiça (corpo esticado, asas para cima, bico aberto, o chapéu pula) e o chapéu cai e assenta |
| `work_in` | 2 | 0,22 s | debruça sobre a tecla (a gravata vai para o pescoço) e dá o passo: o pé de longe recua 2 px e a tecla aparece |
| `work_out` | 2 | 0,28 s | endireita com o pé ainda atrás e volta ao neutro com o olho malandro |
| `sleep_in` | 3 | 0,74 s | o olho pesa, a cabeça afunda e o chapéu escorrega sobre o olho até a pose do `sleep_00` |
| `sleep_out` | 3 | 0,48 s | abre o olho, a cabeça sobe e empurra o chapéu, que assenta |
| `attention_in` | 1 | 0,12 s | apruma a cabeça com o "!" aceso; o agachado do `attention_00` vira a antecipação do pulo |
| `attention_out` | 2 | 0,30 s | sai com o olho malandro, sem o "!", e o chapéu assenta |
| `takeoff` | 3 | 0,28 s | o agachado do pulo, estica com as asas para cima e sobe batendo (asa de longe atrás) até o `fly_00` |
| `landing` | 5 | 0,71 s | desce com os pés para baixo, agacha no impacto com poeira dos dois lados, o chapéu quica e assenta |

Total: 22 animações, 134 quadros, 16 cores (+1 do anel), zero furo, recorte solto, órfão ou efeito
sobre o corpo nos dois visuais, e nenhuma peça solta juntada pelo anel (abaixo).

### O anel não junta peças soltas (revisão, decisão 0068)

A revisão achou o defeito da poeira de volta, só no tema escuro: o `light_outline` pintava de anel
todo pixel de fundo encostado no contorno, então um vão de 1-2 px entre duas peças virava anel e as
colava. A correção do tufo esquerdo (acima) só valia no visual padrão; no escuro, o que o diretor
manda usar sempre, o tufo continuava preso ao rabo pelo anel. O mesmo colava o chapéu voando no
topete (o take do susto), as notas e o «!» no bico aberto, a tecla no bico e o grão no pé, em 27 dos
134 quadros.

Agora cada pixel do anel é de uma peça (componente 8-viz do quadro): o vão que encosta em duas peças
fica de fundo (um entalhe) e, onde os anéis de duas peças se encostariam, o da menor (o efeito, o
chapéu no ar, a poeira) cede. O miolo não muda, e nos 107 quadros sem vão estreito o anel é o mesmo
de antes. A regra entrou no lint (`anel_junta_pecas`: cada mancha do escuro tem uma peça do padrão,
e só uma) e na montagem das skins (`anel_sem_ponte` no `cargo xtask zeca-livre`).

### Como o pet usa (skins `zeca-livre` e `zeca-livre-escuro`)

O `cargo xtask zeca-livre` monta as duas skins com a receita `skin.toml`: o tema claro usa os
quadros padrão e o escuro, sempre os com anel (decisão 0066; a montagem confere que o escuro é o
padrão com o anel, sem mexer no miolo). Cada estado do pet vira uma sequência dos quadros acima:
o repouso é a pose neutra com rajadas do respira (0-3), do piscar e da ginga, a cada 4 s ou mais
(o orçamento de commits não deixa o respira em laço); o aceno (T0) é o `nod`; o pulinho (T1), o
`hop`; o trabalho e a chamada tocam com a entrada e a saída; o dormindo é só o laço do `sleep`
(o estado entre o bocejo e o acordar; a 1,9 troca/s, dentro dos 2 fps do dormindo; decisão 0068),
e o `sleep_in`/`sleep_out` ficam no manifesto para quando o pet tocar a entrada e a saída de um
laço; o voo curto e o voo grande são `takeoff` + `fly` + `landing`. O mapa inteiro, com o porquê de cada um, está no `skin.toml` e no
`docs/SKINS.md`.

### Para um pixel artist (M9, T9.2)

O rig só desloca peças; estes itens pedem desenho à mão e ficam para a publicação:

- **Bicos girados (`BEAK_45` e `BEAK_DOWN`).** Têm a massa certa, mas perderam o gancho: colados na
  gravata, viram uma massa rosa que lê como luva. É o item mais importante, porque o "trabalhando",
  o estado mais visto, usa o `BEAK_45`. Pedido: silhueta assimétrica, borda da frente convexa, ponta
  recurvada para o peito e 1-2 px de mandíbula `3` atrás do gancho; na bicada, a gravata 2 px para
  trás.
- **Ícone pequeno do app** em 16, 32 e 64 px (busto: cabeça, chapéu, bico e olho), com versão com o
  anel para fundos escuros. Reduzir o sprite não serve: em 16 px vira borrão, em 32 px o contorno
  do olho, do gancho e da aba se quebra.
- **Pose-base em S.** A neutra continua vertical; falta linha de ação (peito 2-3 px à frente, cabeça
  e chapéu 2 px para trás, vinco de pescoço, pé de longe adiantado), propagada para respira,
  piscar, piar e os quadros de entrada e saída.
- **Penas.** Contorno escuro salpicado dentro das pontas de pena na cauda do voo, na asa de longe e
  nas asas abertas do pulo; separar as penas por entalhe de fundo na borda, não por `K` no campo.
- **Susto.** O take ainda tem 16-17 ilhas de 1 px por quadro (serrilha do corpo eriçado e topete):
  juntar em 3-4 dentes de 2 px e fazer o topete com 2 penas de 2 px.

A regra de diferenciação (seção 5) vale também para os textos de divulgação e para toda arte nova.
