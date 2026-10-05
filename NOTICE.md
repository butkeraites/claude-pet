# Créditos e licenças de terceiros

O código deste repositório é MIT (`LICENSE`), menos a arte do Zeca original
(`arte/zeca-livre/`, com o gerador `zeca.py`, e as skins
`skins/zeca-livre/` e `skins/zeca-livre-escuro/`), que é CC0 1.0. Abaixo, o
que vem de fora e o que tem outra licença.

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
- `arte/zeca-livre/` — o **Zeca original**: arte original feita com o Claude
  para o projeto bichinho, desenhada do zero (nenhum pixel do pack), dedicada
  ao domínio público pela **CC0 1.0** (a dedicação, o crédito de cortesia e o
  texto legal em `arte/zeca-livre/LICENSE`). O gerador `zeca.py` é a fonte da
  arte; tudo o que sai dele também é CC0. O crédito não é obrigatório, mas é
  bem-vindo: "arte original feita com o Claude para o projeto bichinho".
- `skins/zeca-livre/` e `skins/zeca-livre-escuro/` — as skins do Zeca
  original (tema claro e tema escuro, com o anel), geradas do `zeca.py` por
  `cargo xtask zeca-livre`; **CC0 1.0**, como a arte (o `CREDITS.md` de cada
  uma aponta para `arte/zeca-livre/LICENSE`).
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

## Protocolos

- `crates/pet-wayland/protocolos/hyprland-toplevel-mapping-v1.xml` — o
  protocolo `hyprland_toplevel_mapping_v1`, do hyprland-protocols
  (https://github.com/hyprwm/hyprland-protocols, commit 9830bfb5 de
  2025-04-01): Copyright © 2025 WhySoBad, **BSD-3-Clause** (o aviso de
  copyright e a licença estão no próprio XML, que vai sem mudança nenhuma).
  O código de cliente sai dele pelo `wayland-scanner` (decisão 0056) e vai
  dentro do binário: quem distribuir o binário leva junto este aviso e o
  texto da licença que está no XML.
