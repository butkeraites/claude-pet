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
| `corpo_px` | altura do corpo: D = 12% da altura do monitor ÷ `corpo_px` |
| `escala_padrao` | escala inteira para prévias (×3 numa célula de 48) |
| `estados` | estado semântico → tags (`idle`: a primeira tag dá a pose fixa e as rajadas alternam entre todas) |
| `chao` | tags com os pés no chão (o lint confere a linha dos pés) |

## Onde as skins moram

- `skins/` vai para o git: só arte que pode ser redistribuída (a xadrez
  `_teste`, gerada por `cargo xtask skin-teste`).
- `skins-locais/` fica fora do git e entra **só** na imagem Docker local,
  que nunca vai a registry (decisão 0011). É onde moram as skins feitas de
  packs comprados, como o Zeca. O importador e o lint recusam uma skin não
  redistribuível dentro de `skins/`.

## O Zeca

```sh
bin/pet skin-instalar ~/Downloads/"Cute Parrots! -Pixel Art Asset Pack.zip"
bin/pet subir                 # a skin entra na imagem
bin/pet skin-aprovar zeca     # depois de ver a folha de contato
```

`skin-instalar` descompacta o zip numa pasta temporária fora do repo,
confere o layout (pasta «Parrot 2» com `Parrot.aseprite`), roda `cargo
xtask zeca` com e sem contorno, o lint, a cobertura e as prévias, e põe a
folha de contato e os GIFs em `tmp/previa-zeca-m2/`. Rodar de novo dá os
mesmos bytes.

### Como o Zeca é montado (`cargo xtask zeca`)

1. **Importa** o Parrot 2 pelo `.aseprite` (90 quadros, 21 tags, 100 ms por
   quadro), com o `Parrot.png` de reserva. Os nomes viram ids:
   `Sit(Idle)` → `sit_idle`, `Sit(End)` → `stand`, `Sleep(Idle)` → `sleep`,
   `End Dive` → `dive_end`… (o original fica no `data` da tag).
2. **Troca de paleta:** nenhuma no visual "Malandro rosa" (decisão 0023).
3. **Veste** cada quadro com o chapéu-palheta e a gravata-borboleta:
   - âncora no olho branco (a maior mancha branca);
   - chapéu em (olho − 4, olho − 6), gravata em (olho + 1, olho + 6);
   - correções em `arte/zeca/ancoras.json`;
   - nos quadros de clarão (silhueta branca), a âncora vem do quadro com a
     mesma silhueta e os acessórios ficam brancos.
4. **Chapéu voa e volta** (decisão 0024): `arte/zeca/chapeu_voando.json`
   troca os quadros das tags do susto, do mergulho e do pouso. Cada quadro
   diz o corpo do pack, a duração e onde está o chapéu: solto na célula
   (variante, x, y) ou assentado.
5. **Tags compostas** (`arte/zeca/zeca.toml`): `stand_look_sit`, `nod`,
   `short_flight`, `big_flight`. Quadros iguais apontam para a mesma
   célula da folha.
6. **Contorno creme** opcional (`--contorno`, skin `zeca-contorno`): 1 pixel
   de arte `#F7E7C5` por fora, pisando na linha 33.

O `Death` fica fora: o Zeca não morre.

### Mexer na arte

Tudo em `arte/zeca/` é arte nossa (MIT) e vai para o git:

| Arquivo | O quê |
|---|---|
| `paleta.toml` | cores do olho, do bico e do clarão; troca de paleta; letra → cor das grades; cor do contorno |
| `acessorios/*.txt` | grades de texto: `chapeu`, `gravata` e as variantes da cambalhota (`chapeu-inclinado-*`, `chapeu-de-lado-*`, `chapeu-de-ponta-cabeca`, `chapeu-torto-*`, `chapeu-amassado`); `.` é transparente, `;` comenta |
| `ancoras.json` | a regra do olho e as correções: `oculto`, `variante`, `x`/`y` absolutos, `dx`/`dy` relativos, por tag ou por quadro |
| `chapeu_voando.json` | as trajetórias do chapéu solto |
| `zeca.toml` | metadados, `pe`, `chao`, estados, composições, tags excluídas |

Para ver onde cada peça caiu: `cargo xtask zeca --pack <zip> --ancoras`
imprime, por quadro do pack, a caixa do olho, o clarão e as posições. O
`zeca` avisa quando um acessório sai da célula, cobre o olho ou o bico, quando
uma cor sai da paleta do pack e quando o chapéu solto passa a menos de 4
pixels do corpo (o contorno creme juntaria os dois).

Regras de estilo (`docs/pesquisa/06-inspiracoes.md`):
- contorno de 1 pixel na tinta do pack, `#1D2427` (nunca `#000000`);
- só cores da paleta do pack, com no máximo uma rampa nova de 3 tons (a da
  palha);
- sem anti-aliasing;
- luz de cima-esquerda;
- os pés na mesma linha.

## Conferir

| Comando | O quê |
|---|---|
| `cargo xtask lint-skin <pasta>…` | erros (não carrega, duração fora de 16–5000 ms, não redistribuível em `skins/`) e avisos (cores, alfa parcial, corpo pulando > 3 px, pés fora da linha, borda da célula); o `bin/pet verificar` roda em todas |
| `cargo xtask cobertura <pasta>` | `cobertura.md`: cada estado do catálogo (`pet_core::estados`) como nativo, receita, reserva ou faltando |
| `cargo xtask contato <pasta> [--copia <pasta>]` | folha de contato (todas as tags, fundo escuro e claro, ×4) e um GIF por tag, mais o `repouso.gif` (o parado como o daemon toca), em `tmp/contato/<id>/` |
| `cargo xtask skin-importar …` | importa um pack qualquer (`.aseprite` ou tiras PNG) num esqueleto de skin |

## Aprovar (decisão 0026)

Sem personagem aprovado, o pet fica **escondido** (`tela: sem_personagem`).
A skin `_teste` só aparece com `PET_DEBUG=1`.

- `bin/pet skin-aprovar [id]` calcula no host a impressão digital da skin
  que você viu (`sha256sum skin.json sheet.json sheet.png | sha256sum`) e
  pede ao pet para aprovar. O pet só aprova se a skin **da imagem** tiver a
  mesma impressão (senão falta `bin/pet subir`). Ele guarda uma cópia dos
  três arquivos e o `aprovacao.json` em `/state/skins/<id>/` e troca de
  personagem na hora.
- Quem aparece: a skin da imagem, se aprovada; senão a cópia aprovada de
  `/state` (se a da imagem mudou, sumiu ou quebrou); senão ninguém.
  `bin/pet estado` mostra `skin.origem` (`imagem` ou `snapshot`),
  `skin.sha256` e os avisos.
- **Depois de reconstruir a skin** (arte nova, pack reinstalado, `bin/pet
  subir`): a imagem tem uma impressão nova, ainda não aprovada. O pet
  continua com a cópia aprovada e avisa "mudou depois da aprovação". Veja a
  folha de contato nova e rode `bin/pet skin-aprovar` de novo; sem isso,
  nada muda na tela.
- `bin/pet skin-revogar [id]` apaga a aprovação e a cópia: o pet some.
- A aprovação é por id. Para usar `zeca-contorno`, aprove essa skin e ponha
  `aparencia.skin = "zeca-contorno"` em `config/claude-pet.toml`.
- Para conferir o personagem com as rotas de debug (nitidez, foto
  mascarada): `PET_DEBUG_PERSONAGEM=1` na pilha de dev, ou
  `scripts/verificar-ao-vivo.sh --personagem`. Precisa da tela acesa e
  **desbloqueada**: com o lock do Omarchy o Hyprland só desenha a tela de
  senha.
