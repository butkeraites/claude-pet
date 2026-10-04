# Skins: o personagem como dado

O pet desenha uma **skin**: uma pasta com três arquivos. O código não sabe
que o personagem é um papagaio (decisão 0012); trocar de bicho é trocar de
skin.

| Arquivo | O quê |
|---|---|
| `skin.json` | metadados, âncoras e o mapa estado semântico → tags |
| `sheet.json` | a folha no formato **json-array** do Aseprite: quadros (retângulo, duração, `spriteSourceSize`/`sourceSize`) e `meta.frameTags` (nome, de, até, direção; `data` guarda o nome original do pack) |
| `sheet.png` | as células, RGBA de 8 bits, sem alfa parcial |

Campos do `skin.json`:

| Campo | Para quê |
|---|---|
| `formato` | versão do formato (1) |
| `id`, `nome`, `autor`, `licenca`, `fonte` | quem é e de onde veio |
| `redistribuivel` | `false` para arte de pack comprado: nunca vai para o git |
| `folha`, `dados` | nomes de `sheet.png` e `sheet.json` (só na própria pasta) |
| `celula` | tamanho da célula em pixels de arte (`[48, 48]` no Zeca) |
| `pe` | onde os pés pisam (x no meio dos pés, y na linha do chão) |
| `toque` | área clicável (o corpo), `[x, y, w, h]` |
| `corpo_px` | altura da figura parada: D = a fração da altura do monitor ÷ `corpo_px` (no Zeca, 19: sentado com o chapéu). A fração vem do config, não da skin: `aparencia.tamanho` = `pequeno` (~10%), `normal` (~12%) ou `grande` (~16%), e trocar não pede outra aprovação (decisão 0042) |
| `escala_padrao` | escala inteira para prévias (×3 numa célula de 48) |
| `estados` | estado semântico → tags (`idle`: a primeira tag dá a pose fixa e as rajadas do repouso vão pelas tags, em ordem; uma tag pode repetir) |
| `chao` | tags com os pés no chão (o lint confere a linha dos pés) |

## Onde as skins moram

- `skins/` vai para o git: só arte que pode ser redistribuída (a xadrez
  `_teste`, gerada por `cargo xtask skin-teste`, e o Zeca original,
  `zeca-livre` e `zeca-livre-escuro`, gerados por `cargo xtask zeca-livre`).
- `skins-locais/` fica fora do git e entra **só** na imagem Docker local,
  que nunca vai a registry (decisão 0011). É onde moram as skins feitas de
  packs comprados, como o Zeca.
- Arte derivada de pack (folha, GIFs, folha de contato, fotos) só é gravada,
  dentro do repositório, em `skins-locais/` ou `tmp/`: o importador, o `zeca`
  e o `contato` recusam qualquer outra pasta do repo (`skins/`, `docs/`,
  `arte/`…) para uma skin não redistribuível; o lint acusa uma dentro de
  `skins/`. Fora do repositório, qualquer lugar.

## Os dois Zecas

O Zeca tem duas artes, e as duas podem ficar instaladas ao mesmo tempo:

| Skin | Arte | Onde | Licença |
|---|---|---|---|
| `zeca-livre-escuro` | o Zeca original com o anel de 1 px por fora, para tema escuro | `skins/` (no git) | CC0 1.0 |
| `zeca-livre` | o Zeca original, para tema claro | `skins/` (no git) | CC0 1.0 |
| `zeca` | o Zeca feito do pack *Cute Parrots!* | `skins-locais/` (fora do git) | a do pack: não pode ser redistribuído |
| `zeca-contorno` | o mesmo, com contorno creme | `skins-locais/` (fora do git) | a do pack |

Quem aparece é o `aparencia.skin` do `config/bichinho.toml` (sem a chave, o
`zeca`), e cada uma precisa da própria aprovação (decisões 0026 e 0066).

## O Zeca original (arte livre, CC0)

Desenhado do zero, sem nenhum pixel do pack, por um gerador em Python que é a
fonte da arte: `arte/zeca-livre/zeca.py` (paleta, grades das peças, rig e 22
animações; decisão 0065). Arte original feita com o Claude para o projeto
bichinho, dedicada ao domínio público pela **CC0 1.0**
(`arte/zeca-livre/LICENSE`). As notas de arte (o que cada animação faz, a
regra de diferenciação e o que ainda pede um pixel artist) estão em
`arte/zeca-livre/notas.md`.

```sh
bin/pet skin-livre                # refaz as duas skins, com lint, cobertura e prévias em tmp/previa-zeca-livre/
bin/pet subir                     # as skins entram na imagem
bin/pet skin-aprovar zeca-livre-escuro   # depois de ver a folha de contato
```

- `cargo xtask zeca-livre` roda o gerador numa pasta temporária
  (`zeca.py --quadros`, sem ImageMagick) e grava o `arte/zeca-livre/anims.json`
  e as duas skins, montadas pela receita `arte/zeca-livre/skin.toml`: as tags
  são pedaços das animações do gerador (`anim`, o trecho `quadros = [de,
  ate]` e `vezes`), quadros iguais numa célula só da folha. O toque e o
  `corpo_px` saem da pose parada sem anel, então as duas variantes têm o
  mesmo tamanho na tela; no escuro o `pe` fica uma linha abaixo (o anel
  debaixo dos pés vira o chão).
- `cargo xtask zeca-livre --conferir` não grava nada: roda o gerador duas
  vezes, compara os bytes e confere que o manifesto e as skins do git são o
  que sai agora. O `bin/pet verificar` roda quando há `python3`. **PNG da arte
  livre nunca se edita à mão:** mexa no `zeca.py` ou no `skin.toml` e rode
  `bin/pet skin-livre`.
- `python3 arte/zeca-livre/zeca.py` grava as prévias da própria arte (quadros
  dos dois visuais, folhas, GIFs com o fundo escuro, o escuro com anel e o
  claro, e a vitrine) em `tmp/zeca-livre/`.
- **Tema escuro = sempre a variante com anel** (o diretor de arte manda: o
  contorno `#2B2136` tem 1,27:1 contra o fundo do hackerman). A escolha é por
  skin, `zeca-livre-escuro` ou `zeca-livre`, no config; a montagem confere que
  o escuro é o padrão com o anel, sem mexer em pixel nenhum do miolo.

### Estados do Zeca original

| Estado | Tag | O que ele faz |
|---|---|---|
| `idle` | `respira`, `blink`, `ginga` | pose neutra; com pelo menos 4 s entre elas, uma respirada com o chapéu atrasando, uma piscada e a ginga (queixo erguido, olho malandro, o quadril vai e o pé bate). ~16 s por ciclo, 0,95 commit/s |
| `working` | `work_session` | debruça e dá o passo, bate na tecla duas vezes e endireita |
| `thinking` | `peck` | bica o grão, com o chapéu escorregando até o bico |
| `waiting`, `alert` | `attention_call` | apruma com o «!», pula e abana a asa duas vezes, sai com o olho malandro |
| `ready`, `hello`, `giggle` | `chirp` | pia com as notas e o olho feliz |
| `nod` (T0) | `nod` | a tirada de chapéu: a cabeça abaixa e o chapéu tomba (o respira e a ginga sobem a cabeça: o aceno não se confunde com o repouso) |
| `wave`, `bye` | `wave` | abana a asa |
| `done_small` (T1) | `hop` | o pulo comemorando, com o chapéu rodopiando e a poeira do pouso |
| `done_medium` (T2) | `short_flight` | decola, bate asas e pousa |
| `done_big` (T3) | `big_flight` | decola, voa mais tempo, pousa e pia |
| `error` | `scared` | o susto: encolhe, arregala, o chapéu voa e cai torto |
| `yawn` | `yawn` | o bocejo |
| `sleep` | `nap` | adormece com o chapéu escorregando, ronca duas vezes e acorda |
| `wake` | `wake` | se espreguiça e o chapéu pula |
| `dangle` | `fly` | bate asas em laço enquanto é arrastado |
| `land` | `landing` | pousa com poeira dos dois lados e o chapéu quicando |

O corpo parado tem 34 pixels de arte (do topo do chapéu aos pés): no eDP-1,
D = 4 no `pequeno` e no `normal` e 6 no `grande`; no 4K, 6, 7 e 9.

## O Zeca do pack

```sh
bin/pet skin-instalar ~/Downloads/"Cute Parrots! -Pixel Art Asset Pack.zip"
bin/pet subir                 # a skin entra na imagem
bin/pet skin-aprovar zeca     # depois de ver a folha de contato
```

`skin-instalar` passa o zip ao `cargo xtask zeca`, que o lê em memória
(CRC conferido, teto de tamanho, sem extrair nada no disco; o lixo do macOS
é ignorado; uma pasta descompactada também serve), monta o Zeca com e sem
contorno com `--estrito`, roda o lint, a cobertura (os estados do MVP têm de
ser nativos) e as prévias, e põe a folha de contato e os GIFs em
`tmp/previa-zeca-m2/`. Rodar de novo dá os mesmos bytes. Layout inesperado
(sem «Parrot 2/Parrot.aseprite») é erro com o endereço do pack.

### Como o Zeca é montado (`cargo xtask zeca`)

1. **Importa** o Parrot 2 pelo `.aseprite` (90 quadros, 21 tags, 100 ms por
   quadro), com o `Parrot.png` de reserva. Os nomes viram ids:
   `Sit(Idle)` → `sit_idle`, `Sit(End)` → `stand`, `Sleep(Idle)` → `sleep`,
   `End Dive` → `dive_end`… (o original fica no `data` da tag). Tags com
   outra direção (ping-pong, reversa) são expandidas na ordem em que tocam.
2. **Troca de paleta:** nenhuma no visual "Malandro rosa" (decisão 0023).
3. **Veste** cada quadro com o chapéu-palheta e a gravata-borboleta:
   - âncora no olho branco (a maior mancha branca);
   - chapéu em (olho − 4, olho − 6), gravata em (olho + 1, olho + 6);
   - correções em `arte/zeca/ancoras.json`;
   - a gravata só pinta o **miolo** do corpo (pixel opaco com os 4 vizinhos
     opacos): o contorno de 1 pixel do pack sempre ganha (decisão 0028);
   - o chapéu vai por cima, com a aba pousando no contorno do topo da cabeça;
   - nos quadros de clarão (silhueta branca), a âncora vem do quadro com a
     mesma silhueta e os acessórios ficam brancos.
4. **Chapéu voa e volta** (decisões 0024 e 0028):
   `arte/zeca/chapeu_voando.json` troca os quadros das tags do susto, do
   mergulho e do pouso. Cada quadro diz o corpo do pack, a duração e onde
   está o chapéu: solto (variante e centro `cx`/`cy`, para a cambalhota girar
   em volta de um ponto fixo) ou assentado. No mergulho o chapéu fica no ar
   até depois do pouso: a tag nova `landing_mergulho` (`"base": "landing"`)
   fecha o `big_flight` com o chapéu caindo na cabeça.
5. **Tags compostas** (`arte/zeca/zeca.toml`): `stand_look_sit`, `nod`,
   `short_flight`, `big_flight`. Quadros iguais apontam para a mesma
   célula da folha.
6. **Contorno creme** opcional (`--contorno`, skin `zeca-contorno`): 1 pixel
   de arte `#F7E7C5` por fora, pisando na linha 33.

O `Death` fica fora: o Zeca não morre. Parado, ele respira três vezes e
levanta para olhar uma (~18 s por ciclo, ~0,9 commit/s).

### Mexer na arte

Tudo em `arte/zeca/` é arte nossa (MIT) e vai para o git:

| Arquivo | O quê |
|---|---|
| `paleta.toml` | cores do olho, do bico e do clarão; troca de paleta; letra → cor das grades (nunca `#000000`); cor do contorno |
| `acessorios/*.txt` | grades de texto: `chapeu`, `gravata` e as variantes da cambalhota (`chapeu-inclinado-*`, `chapeu-de-lado-*`, `chapeu-de-ponta-cabeca`, `chapeu-torto-*`, `chapeu-amassado`); `.` é transparente, `;` comenta |
| `ancoras.json` | a regra do olho e as correções: `oculto`, `variante`, `x`/`y` absolutos, `dx`/`dy` relativos, por tag ou por quadro (índice escrito sem zero à esquerda) |
| `chapeu_voando.json` | as trajetórias do chapéu solto (`cx`/`cy` ou `x`/`y`); com `base`, uma tag nova feita dos corpos de outra |
| `zeca.toml` | metadados, `pe`, `chao`, estados, composições, tags excluídas |
| `avisos-aceitos.txt` | avisos do `zeca` já olhados e aceitos, cada um com o porquê |

Dado errado para o `zeca` em vez de sumir: correção para quadro que a tag
não tem, índice «01», deslocar uma peça que a tag escondeu (para recriá-la
num quadro, dê `x` e `y`), `oculto` junto com posição, chapéu solto com
canto e centro juntos, tag ou variante que não existe.

Para ver onde cada peça caiu: `cargo xtask zeca --pack <zip> --ancoras`
imprime, por quadro do pack, a caixa do olho, o clarão e as posições. O
`zeca` avisa quando:
- um acessório sai da célula, cobre o olho ou o bico;
- a gravata cairia fora do miolo do corpo (e não foi desenhada);
- uma cor de acessório encosta no transparente sem tinta em volta (a única
  abertura do desenho aprovado é a ponta de palha da aba);
- o chapéu assentado afunda no corpo ou não encosta na cabeça;
- o chapéu solto fica a menos de 4 pixels do corpo ou de 2 da borda (o
  contorno creme juntaria os dois), pula mais de 3 pixels entre dois quadros
  ou sobe e desce aos trancos;
- o contorno creme junta manchas soltas;
- uma cor sai da paleta do pack.

Com `--estrito`, qualquer aviso fora de `avisos-aceitos.txt` reprova antes
de gravar (o `skin-instalar` usa).

Regras de estilo (`docs/pesquisa/06-inspiracoes.md`):
- contorno de 1 pixel na tinta do pack, `#1D2427` (nunca `#000000`);
- toda cor dentro da tinta, como no pack (só o branco do clarão, das bolhas
  e do risco encosta no transparente);
- só cores da paleta do pack, com no máximo uma rampa nova de 3 tons (a da
  palha);
- sem anti-aliasing;
- luz de cima-esquerda;
- os pés na mesma linha.

**Para o M6:** o chapéu solto mora em coordenadas da célula. Enquanto as
tags do mergulho tocam, a célula não pode descer pela tela (ou o chapéu
precisa virar uma trilha separada), senão ele desce junto com o Zeca
(decisão 0028).

## Conferir

| Comando | O quê |
|---|---|
| `cargo xtask lint-skin <pasta>…` | erros (não carrega, duração fora de 16–5000 ms, não redistribuível em `skins/`) e avisos (cores, alfa parcial, preto puro, corpo pulando > 3 px, pés fora da linha, borda da célula); o `bin/pet verificar` roda em todas |
| `cargo xtask cobertura <pasta> [--nativos mvp]` | `cobertura.md`: cada estado do catálogo (`pet_core::estados`) como nativo, receita, reserva ou faltando; `--nativos` (uma lista, ou `mvp` = a tabela do PLANO, item 4, com o `nod` do T0) reprova se algum desses não for nativo |
| `cargo xtask contato <pasta> [--copia <pasta>]` | folha de contato (todas as tags, fundo escuro e claro, ×4, impressão digital no título e em `contato.sha256`) e um GIF por tag com o fundo escuro e o claro lado a lado, mais o `repouso.gif` (o parado como o daemon toca), em `tmp/contato/<id>/`; `--copia` põe a folha e os GIFs dos estados que mais importam, com o nome do estado (`<id>-parado.gif`, `-pulinho`, `-esperando`, `-trabalhando`, `-voo-curto`, `-voo-grande`, `-susto`, `-pouso`), numa pasta só |
| `cargo xtask zeca-livre [--conferir]` | o Zeca original (CC0) do gerador `arte/zeca-livre/zeca.py`: grava o manifesto e as skins `zeca-livre` e `zeca-livre-escuro`; `--conferir` só confere (gerador duas vezes, mesmos bytes, igual ao git) |
| `cargo xtask skin-importar …` | importa um pack qualquer (`.aseprite` ou tiras PNG) num esqueleto de skin (o id é o nome da pasta) |

## Reações (M3, decisões 0019, 0020 e 0030)

O cérebro emite três reações: `nod` (T0, uma resposta sem trabalho),
`done_small` (T1, uma resposta com trabalho) e `bye` (o Claude saiu). O
`bin/pet tocar` aceita qualquer estado. O animador toca a reação uma vez e
volta à pose, escolhendo a tag assim:
1. a primeira tag do estado de mesmo nome em `estados`;
2. senão, a do primeiro estado de reserva que a skin tem, pelas reservas do
   catálogo (`pet_core::estados`, as mesmas do `cobertura.md`), nunca o
   `idle` (tocar o repouso não é reação);
3. senão, uma tag com esse nome.

| Reação | Zeca original | Zeca do pack | `_teste` (debug) |
|---|---|---|---|
| `nod` | `nod` (a tirada de chapéu, 810 ms) | `nod` (composta: levanta e senta, 600 ms) | `wave` (reserva) |
| `done_small` | `hop` (o pulo) | `chirp` | `done_small` |
| `bye` | `wave` (abana a asa) | `chirp` | não anima |

Os três são nativos obrigatórios do personagem (`--nativos mvp`): sem um
`nod` próprio, o aceno cairia no `wave`, que no Zeca do pack é o mesmo pio do
pulinho. No Zeca do pack o `nod` ainda é um pedaço da rajada do repouso
(`stand_look_sit`) e dá para confundir; no original ele é um gesto próprio. Sem personagem aprovado, nada toca na tela: a reação fica em
`/v1/estado.ultima_reacao`.

## Aprovar (decisões 0026 e 0029)

Sem personagem aprovado, o pet fica **escondido** (`tela: sem_personagem`).
A skin `_teste` só aparece com `PET_DEBUG=1`.

- `bin/pet skin-aprovar [id]` calcula no host a impressão digital da skin
  (`sha256sum skin.json sheet.json sheet.png | sha256sum`), confere que é a
  da folha de contato em `tmp/previa-zeca-m2/contato-<id>.sha256` (no Zeca
  original, `tmp/previa-zeca-livre/contato-<id>.sha256`; a mesma que está no
  título da folha) e pede ao pet para aprovar. O pet só aprova se
  a skin **da imagem** tiver a mesma impressão (senão falta `bin/pet subir`).
  Ele guarda uma cópia dos três arquivos e o `aprovacao.json` em
  `/state/skins/<id>/` e troca de personagem na hora, com o pet na tela ou
  não.
- Quem aparece: a skin da imagem, se aprovada; senão a cópia aprovada de
  `/state` (se a da imagem mudou, sumiu ou quebrou); senão ninguém.
  `bin/pet estado` mostra `skin.origem` (`imagem` ou `snapshot`),
  `skin.sha256` e os avisos.
- **Depois de reconstruir a skin** (arte nova, pack reinstalado): `bin/pet
  skin-instalar` gera as prévias de novo; olhe a folha, rode `bin/pet subir`
  e `bin/pet skin-aprovar`. Até lá o pet continua com a cópia aprovada antiga
  e avisa "mudou depois da aprovação"; aprovar sem prévias novas é recusado.
- `bin/pet skin-revogar [id]` apaga a aprovação e a cópia: o pet some.
- A aprovação é por id. Para usar `zeca-contorno`: ponha `aparencia.skin =
  "zeca-contorno"` em `config/bichinho.toml` e rode `bin/pet skin-aprovar
  zeca-contorno` — o pet relê o config a cada aprovação, sem reiniciar.

### Trocar de Zeca

As aprovações ficam guardadas por skin em `/state/skins/<id>/`: o Zeca
original e o do pack podem estar aprovados ao mesmo tempo, e quem aparece é
o `aparencia.skin`.

1. Ponha o id em `config/bichinho.toml`, na seção `[aparencia]`: `skin =
   "zeca-livre-escuro"` (o original, tema escuro), `"zeca-livre"` (o
   original, tema claro), `"zeca"` ou `"zeca-contorno"` (o do pack). Sem a
   chave, vale o `zeca`.
2. Se a skin **ainda não foi aprovada:** olhe a folha de contato dela
   (`tmp/previa-zeca-livre/` ou `tmp/previa-zeca-m2/`) e rode `bin/pet
   skin-aprovar <id>`, que relê o config e troca na hora.
3. Se ela **já está aprovada:** `bin/pet parar && bin/pet subir` (o pet lê o
   config ao subir; o `subir` sozinho não recria o container quando só o
   config muda). Aprovar de novo também relê o config, mas regrava a
   aprovação com a hora nova.

Sem aprovação da skin configurada, o pet fica escondido (`tela:
sem_personagem`), e o `bin/pet skin-aprovar` de outro id avisa quando o
config pede outra skin.
- Para conferir o personagem com as rotas de debug (nitidez, foto
  mascarada): `scripts/verificar-ao-vivo.sh --personagem` (aprova só para o
  teste se faltar aprovação e revoga no fim) ou `PET_DEBUG_PERSONAGEM=1` na
  pilha de dev. Precisa da tela acesa e **desbloqueada**: com o lock do
  Omarchy o Hyprland só desenha a tela de senha. A foto do Zeca tem pixels
  do pack: fica em `tmp/`, nunca no git nem em `docs/`.
