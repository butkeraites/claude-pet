# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

# claude-pet — contexto para o Claude Code

O **Zeca** é um papagaio de pixel art que mora na tela do Renan (Omarchy,
Hyprland 0.56, Wayland) e reage ao Claude Code rodando no terminal:
comemora quando o Claude termina, chama quando o Claude precisa dele, dorme
quando ninguém mexe. Fica sempre por cima de tudo, pode ser arrastado e
segue o monitor ativo. Roda em Docker. **Sem som** (decisão 0002).

O plano completo, com marcos M0–M7 e como verificar cada um, está em
`PLANO.md`. As decisões, com o porquê, estão em `DECISIONS.md`.

## Estado do repositório

**M0 (fundação) concluído. M1 (overlay) concluído na branch `m1-overlay`,
com o portão fechado com a tela acesa** (atualização da decisão 0005):
nitidez exata, sem fantasma, parado a 0,70 commit/s e +0,58 ponto de CPU do
Hyprland; sob repintura de tela cheia, +0,20 de CPU e +1,6 de GPU. A camada
única fica; o plano B não é necessário. Fotos em `docs/fotos/m1/`. Falta só
o clique manual (clicar ao lado do pet chega na janela de baixo).

O daemon acha o Hyprland pelo `hyprland.lock`, conecta ao Wayland (Rust
puro, SCTK), cria a camada OVERLAY `claude-pet` no monitor focado e desenha
a skin em blocos D×D de pixels do monitor, com orçamento de commits.
Repositório privado em `github.com/butkeraites/claude-pet`. Ainda não há
hooks (M3).

**M2 (Zeca) na branch `m2-zeca`:** o pack *Cute Parrots!* (exclusiveOlive,
zip em `~/Downloads`, fora do repo) vira o Zeca: Parrot 2 verde no visual
"Malandro rosa" (bico rosa original, chapéu-palheta de faixa laranja,
gravata-borboleta rosa dentro do contorno do pack) e o "chapéu voa e volta"
no susto e no mergulho, caindo na cabeça depois do pouso (decisões
0023–0025 e a revisão 0028). `bin/pet skin-instalar <zip>` gera
`skins-locais/zeca` e `zeca-contorno` (fora do git) e as prévias em
`tmp/previa-zeca-m2/`. O Zeca só aparece depois de `bin/pet subir` e
`bin/pet skin-aprovar zeca`, que só aprova a skin da folha de contato vista
(decisões 0026 e 0029); sem aprovação, a produção fica conectada e escondida
(`tela: sem_personagem`). Aprovar e revogar trocam na tela na hora, e o
config é relido a cada aprovação (`zeca-contorno` sem reiniciar). A pilha de
dev (`PET_DEBUG=1`) mostra a skin xadrez `_teste`, ou o personagem aprovado
com `PET_DEBUG_PERSONAGEM=1`. Formato, arte e aprovação em `docs/SKINS.md`.

## Comandos

O `~/.cargo/bin` **não está no PATH** do Renan; o `bin/pet` acrescenta.
Fora dele, use `~/.cargo/bin/cargo`.

| Comando | O que faz |
|---|---|
| `bin/pet verificar` | portão antes de **todo** commit: fmt, clippy, testes, compose, plugin |
| `bin/pet subir` / `parar` / `logs` / `estado` | compose e estado do pet |
| `~/.cargo/bin/cargo test` | testes do workspace (os quadros dourados regeneram com `PET_ATUALIZAR_OURO=1`) |
| `docker compose -f docker-compose.yml -f docker-compose.dev.yml up --build` | modo desenvolvimento (sem restart, debug, skin `_teste`) |
| `bin/pet foto` | foto do pet (grim no monitor inteiro); em debug, só os pixels opacos do pet sobre fundo neutro |
| `scripts/verificar-ao-vivo.sh` | verificação do M1 na tela de verdade; termina com a produção de pé |
| `scripts/medir-custo.sh` | CPU do Hyprland, GPU e commits/s: escondido × parado, com e sem carga de repintura, e estresse |
| `cargo xtask skin-teste` / `nitidez` / `fantasma` / `carga` | gera a skin xadrez; compara captura e quadro esperado; acha pixel velho e fantasma; repintura invisível para medir custo |
| `bin/pet skin-instalar <zip\|pasta>` | o pack vira o Zeca em `skins-locais/` (com e sem contorno, `--estrito`), com lint, cobertura (`--nativos mvp`) e prévias em `tmp/previa-zeca-m2/` |
| `bin/pet skin-aprovar [id]` / `skin-revogar [id]` | aprova o conteúdo exato da skin da folha de contato vista (impressão digital conferida; cópia em `/state`) ou tira a aprovação |
| `cargo xtask zeca --pack <zip\|pasta> [--contorno] [--ancoras] [--estrito]` | monta o Zeca; `--ancoras` mostra o encaixe quadro a quadro; `--estrito` reprova aviso fora de `arte/zeca/avisos-aceitos.txt` |
| `cargo xtask skin-importar` / `lint-skin` / `cobertura` / `contato` | importa um pack; confere a skin; estados cobertos (`--nativos mvp`); folha de contato com a impressão digital e GIFs (escuro e claro lado a lado) |
| `scripts/verificar-ao-vivo.sh --personagem` / `scripts/medir-custo.sh --personagem` | a verificação e a medição do M1 com o personagem no lugar da `_teste` (aprovam só para o teste se faltar aprovação e revogam no fim) |

## Arquitetura em uma tela

- Um binário Rust (`crates/claude-pet`) no container `alpine`, uid 1000,
  rootfs somente leitura. Threads: principal (calloop, a partir do M1),
  ingress HTTP, leitor do `.socket2.sock` (M4), watchdog.
- `crates/pet-core` é **puro**: cérebro, pontuação, animador, skin,
  raster. **Nunca** depende de crates Wayland — os testes ficam rápidos.
- Uma camada OVERLAY do tamanho do monitor focado, criada com output NULL,
  nunca redimensionada, sem subsurfaces; o Zeca anda dentro do buffer.
  Cada pixel de arte vira um bloco D×D inteiro de pixels do monitor.
- Eventos do Claude Code chegam por `POST 127.0.0.1:27380/v1/evento`
  vindos do plugin `bichinho` (hooks async → `plugin/scripts/avisar.sh`).

## Regras de ouro

- **Hooks:** sempre `async`, só metadados (lista branca do jq), sempre
  `exit 0`. Conteúdo (prompt, código, resposta, título de janela) nunca sai
  do host nem vai para log.
- **Hyprland:** o daemon **nunca** abre o `.socket.sock` e nunca chama
  `hyprctl dispatch`/`keyword`. Só lê eventos do `.socket2.sock`.
  `hyprctl` só aparece em scripts de teste do host.
- **Config do Hyprland** (`~/.config/hypr/*.lua`): só pela skill
  `omarchy` e com consentimento do Renan.
- **Arte:** o pack e tudo derivado dele (sheet, GIFs, folhas de contato,
  fotos) ficam em `skins-locais/` ou `tmp/` (gitignored), nunca no git nem
  em `docs/`; o xtask recusa gravar arte de pack em outra pasta do repo. Só
  `arte/zeca/` (acessórios, âncoras, trajetórias) é nossa e vai para o git.
  A skin `_teste` nunca vira personagem.
- **Personagem só com aprovação:** o Zeca aparece só com a impressão
  digital aprovada pelo Renan (`bin/pet skin-aprovar`, decisões 0026 e 0029).
  Nunca aprove por ele: aprovação de teste se revoga no fim (os scripts ao
  vivo fazem isso sozinhos, até numa falha).
- **Orçamento de commits Wayland:** média ≤ 2/s parado, 0 dormindo,
  rajadas ≤ 30 fps (decisão 0005).
- **Registro por tarefa:** cada tarefa ganha uma linha no `PROGRESS.md` e um
  commit; decisão nova ou mudança de rumo vai para o `DECISIONS.md`
  (acrescentar, nunca reescrever).

## Pegadinhas conhecidas

- O Hyprland desconecta um cliente do socket2 com 64 eventos acumulados:
  drene sempre, numa thread só para isso.
- `idle_prompt` se repete a cada ~60 s; nunca trate como aviso novo.
- O Stop não chega quando o usuário aperta Esc no meio da resposta.
- `hyprctl output create headless` não aceita nome: descubra o nome novo
  por diff em `hyprctl -j monitors`.
- Arquivo de dev do compose **nunca** se chama `compose.override.yml`
  (seria mesclado sozinho em produção).
- Nunca passe `GDK_SCALE` (definido em `~/.config/hypr/monitors.lua`) para
  o container.
- `docker kill` cancela a política de restart; para simular crash, mate o
  processo pelo host (`kill -9`).
- Com a tela apagada (DPMS) o Hyprland não desenha: o `grim` espera para
  sempre (use `timeout` e confira `dpmsStatus`), não chega frame callback e
  camadas destruídas ficam no `hyprctl -j layers` com `pid: -1` até a tela
  acender. A checagem de camada filtra `pid > 0`.
- Toda POST no ingress precisa de `Content-Length` (curl: `-d ''`); sem ele
  a resposta é 411.
- Nunca capture com `grim -g`: a geometria lógica passa por filtro
  bilinear. Use `grim -o <monitor>` e recorte em pixels do monitor.
- Com a tela apagada o pet não faz commit nenhum depois do primeiro quadro:
  ele espera o frame callback, que só vem quando o monitor desenha
  (decisão 0018). O ritmo parado só se mede com a tela acesa.
- Captura do monitor tem o que estiver na tela (janelas, texto): fica em
  `tmp/` e é apagada. Para o git e para PR, só a foto mascarada da skin
  `_teste`; a foto do Zeca tem pixels do pack e fica em `tmp/`.
- **Sessão bloqueada:** o lock do Omarchy (quickshell, ext-session-lock)
  apaga a tela depois de um tempo parado e, bloqueado, o Hyprland desenha só a
  tela de senha: nenhuma camada aparece, nem acendendo a tela. O logind não
  marca `LockedHint`; quem diz é
  `/usr/share/omarchy/bin/omarchy-hyprland-session-locked` (sai 0 bloqueada)
  ou `LOCK` em `solitaryBlockedBy` no `hyprctl -j monitors`. Os scripts ao
  vivo dão NÃO VERIFICADO nesse caso. Para acender ou apagar a tela:
  `omarchy-brightness-display on|off`.
- `pkill -f`/`pgrep -f` com um padrão que aparece na própria linha de
  comando acha o shell que está rodando: para parar um daemon de teste,
  guarde o PID.
- O `shellcheck` não está instalado no host (o `bin/pet verificar` pula).
  Rodado pela imagem oficial, que depois foi removida:
  `docker run --rm --network none -v "$PWD:/mnt:ro" -w /mnt
  koalaman/shellcheck:stable -x bin/pet scripts/*.sh`.

## Convenções

- Português em docs, comentários, commits, CLI, config e balões; inglês
  nos identificadores Rust e nas chaves semânticas da skin (`idle`,
  `done_small`, …).
- Commit: uma frase em português, citando a tarefa `(T1.2)` ou a decisão
  `(decisão 0007)` quando fizer sentido, terminando com
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Um PR por marco (`mN-tema`), revisado pelo Renan; corpo do PR termina com
  `🤖 Generated with [Claude Code](https://claude.com/claude-code)`.
- Nunca commitar `.env`, `config/claude-pet.toml`, `skins-locais/*`,
  `tmp/`, `target/`.
