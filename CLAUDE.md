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
Repositório privado em `github.com/butkeraites/claude-pet`.

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
**Pendente:** a conferência na tela da arte revista (nitidez, foto
mascarada, reaprovação com o pet na tela) e a medição de custo com o
personagem: a sessão ficou bloqueada. Com a tela acesa e desbloqueada, rode
`scripts/verificar-ao-vivo.sh --personagem` e `scripts/medir-custo.sh
--personagem` (aprovam só para o teste e revogam no fim).

**M3 (hooks → reação) na branch `m3-hooks`, rebaseada sobre a `m2-zeca`**
(publicada, sem PR nem merge; a pilha é `m1-overlay` ← `m2-zeca` ←
`m3-hooks`): fio v1 validado no `/v1/evento`, plugin `bichinho` (13 hooks
async → `avisar.sh`), cérebro mínimo (T0 `nod`, T1 `done_small`, `bye`
quando o Claude sai) e `bin/pet testar`. Na integração (decisão 0030) um
`/v1/comando` só serve as reações (`tocar`, `esconder`, `mostrar`) e as
aprovações (`aprovar_skin`, `revogar_skin`); as reações tocam pelos estados
do `skin.json` com as reservas do catálogo: no Zeca, o aceno é a tag
composta `nod` (levanta e senta) e o pulinho e o tchau são o pio; na
`_teste`, o aceno cai no `wave`. O `nod` é nativo obrigatório do MVP, e o
config relido a cada aprovação vale também para o cérebro. Gate ao vivo
refeito com o Zeca (decisões 0021 e 0030). O plugin **não** está
instalado: até o merge na `main`, só por sessão, com `claude --plugin-dir
~/Documents/claude-pet/plugin`; depois do merge, pela worktree estável
(README). Sem personagem aprovado a produção reage só no `/v1/estado`
(`ultima_reacao`, `turnos`). A revisão adversarial da integração (decisões
0031–0034) prendeu os hooks no 127.0.0.1 (sem curlrc, proxy nem `~/.jq`),
ensinou o cérebro a esperar o Stop que chega depois do prompt seguinte e a
reabrir o turno quando outro Stop hook segura o Claude, fez o `tocar` dizer
o que tocou e refez o gate com um turno de Write. **Pendente:** no Zeca o
aceno (`nod`: levanta e senta) é um pedaço da rajada do repouso
(`stand_look_sit`), e dá para confundir os dois; mudar pede o Renan (muda o
`skin.json`, a impressão e a aprovação). As reações com o Zeca ainda não
foram vistas na tela: a sessão estava bloqueada nos dois gates.

## Comandos

O `~/.cargo/bin` **não está no PATH** do Renan; o `bin/pet` acrescenta.
Fora dele, use `~/.cargo/bin/cargo`.

| Comando | O que faz |
|---|---|
| `bin/pet verificar` | portão antes de **todo** commit: fmt, clippy, testes, compose, plugin |
| `bin/pet subir` / `parar` / `logs` / `estado` | compose e estado do pet |
| `bin/pet testar rapido` / `pequeno` | eventos sintéticos pelo `avisar.sh` de verdade (`PET_TESTE=1`) → `nod` / `done_small` |
| `bin/pet tocar <reação>` / `esconder` / `mostrar` | `/v1/comando` (não persiste); o `tocar` diz a tag que a skin tocou e se apareceu na tela |
| `claude --plugin-dir ~/Documents/claude-pet/plugin` | o plugin numa sessão só (até o merge, nunca instalar) |
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
  vindos do plugin `bichinho` (hooks async → `plugin/scripts/avisar.sh`),
  são validados campo a campo (`pet_core::evento`, decisão 0019) e vão
  pelo canal do calloop para o cérebro (`pet_core::cerebro`, decisões
  0020 e 0032), que mora no laço principal, conta os prazos da chegada de
  cada evento e funciona mesmo sem compositor.
- Comandos chegam por `POST /v1/comando`, sempre `{"cmd", "arg"}`: as
  reações (`tocar`, 200 com a tag e se apareceu na tela; `esconder` e
  `mostrar`, 204) e as aprovações (`aprovar_skin`, `revogar_skin`; 200
  depois de o laço trocar o personagem), com as mesmas checagens de `Host`,
  `X-Pet` e `Content-Type` (decisões 0030 e 0033). Só as aprovações passam
  pelo cadeado: uma reação nunca espera uma aprovação. Uma reação toca a tag do estado de mesmo nome no
  `skin.json`, ou a reserva do catálogo (`pet_core::estados`), nunca o
  repouso.

## Regras de ouro

- **Hooks:** sempre `async`, só metadados (lista branca do jq), sempre
  `exit 0`. Conteúdo (prompt, código, resposta, título de janela) nunca sai
  do host nem vai para log. Os metadados só vão ao 127.0.0.1: todo curl que
  fala com o pet leva `-q --noproxy '*'` (nenhum curlrc, nenhum proxy) e o
  jq do `avisar.sh` roda sem `~/.jq` (decisão 0031).
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
  vivo fazem isso sozinhos, até numa falha). **Nunca revogue uma aprovação
  que você não fez:** antes de mexer, leia `skin` no `/v1/estado` e o
  `/state/skins/<id>/aprovacao.json` (`docker exec claude-pet-pet-1 cat …`);
  a que já estava lá é do Renan. Para testar reações, `bin/pet testar` e
  `bin/pet tocar`, que nunca mexem em aprovação.
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
- Hooks async chegam fora de ordem: o Stop pode chegar depois do prompt
  seguinte, e um Stop hook de outro plugin manda a continuação segundos
  depois da festa, com o mesmo `prompt_id`. O cérebro espera 0,8 s pelo
  Stop do turno trocado e reabre o turno comemorado (decisão 0032).
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
- Plugin: **nunca** `claude plugin marketplace add` / `install` antes do
  merge na `main` (sessões de outros projetos rodariam a branch). Para
  testar ao vivo, `claude --plugin-dir ~/Documents/claude-pet/plugin`.
- Hook async não aparece em lugar nenhum: para ver o que chegou, pilha de
  dev e `curl -H 'X-Pet: 1' 127.0.0.1:27380/v1/debug/eventos` (só
  metadados validados).
- `claude` aninhado (tmux, testes) a partir de uma sessão do Claude: tire
  `CLAUDECODE` e as `CLAUDE_*` do ambiente antes, como no gate do M3. No
  tmux, mande o texto com `tmux send-keys -l` e o `Enter` num `send-keys`
  separado: juntos, o Claude Code trata como colagem e não envia.
  O próprio Claude Code põe `CLAUDE_CODE_ENTRYPOINT` (`cli` no terminal,
  `sdk-cli` no `-p`), e o cérebro só conta `cli` (`sessoes.origens`).
- No 2.1.288 o `UserPromptSubmit` vem **sem** `source`, e o `SessionEnd`
  vem com o `prompt_id` do `/exit`.
- `bin/pet testar` precisa do pet de pé; as sessões de teste somem em 60 s
  e nunca se misturam com as reais.
- O `shellcheck` não está instalado no host (o `bin/pet verificar` pula).
  Rodado pela imagem oficial, que depois foi removida:
  `docker run --rm --network none -v "$PWD:/mnt:ro" -w /mnt
  koalaman/shellcheck:stable -x bin/pet scripts/*.sh plugin/scripts/avisar.sh`.

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
