# Créditos e licenças de terceiros

O código deste repositório é MIT (`LICENSE`). Abaixo, o que vem de fora.

## Arte que NÃO está no repositório

- **Cute Parrots! — Pixel Art Asset Pack**, por exclusiveOlive —
  https://exclusiveolive.itch.io/cute-parrots-pixel-art-asset-pack
  Licença do autor: pode ser usado em projetos comerciais e não comerciais e
  pode ser editado; **não pode ser redistribuído nem revendido, mesmo
  editado**. Por isso o pack e o Zeca derivado dele ficam em `skins-locais/`
  (fora do git) e só entram na imagem Docker local. Cada usuário precisa
  baixar o próprio pack. Crédito apreciado: obrigado, exclusiveOlive!

## Arte e dados no repositório

- `arte/zeca/` — paleta, âncoras, trajetórias do chapéu voando, receita da
  skin e acessórios (chapéu-palheta e as variantes da cambalhota,
  gravata-borboleta) desenhados para este projeto; mesma licença do código
  (MIT). Só coordenadas e cores da paleta do pack; nenhum pixel do pack.
- `skins/_teste/` — skin xadrez de QA gerada por `cargo xtask skin-teste`;
  MIT.
- `assets/fonte/monogram/` — **monogram**, a fonte de pixel dos balões, de
  Vinícius Menézio (datagoblin): https://datagoblin.itch.io/monogram, em
  domínio público pela **CC0 1.0** (o texto da licença em
  `assets/fonte/monogram/CC0-1.0.txt`, os créditos originais em
  `credits.txt` e a origem em `LICENCA.md`). O repositório guarda só o JSON
  de bitmaps do pacote; `cargo xtask fonte` o assa em
  `crates/pet-core/src/fonte/glifos.rs`, que vai dentro do binário (decisão
  0052). Obrigado, datagoblin!
