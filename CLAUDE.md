# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

# bichinho — contexto para o Claude Code

O **Zeca** é um papagaio de pixel art que mora na tela do Renan (Omarchy,
Hyprland 0.56, Wayland) e reage ao Claude Code rodando no terminal:
comemora quando o Claude termina, chama quando o Claude precisa dele, dorme
quando ninguém mexe. Fica sempre por cima de tudo, pode ser arrastado e
segue o monitor ativo. Roda em Docker. **Sem som** (decisão 0002).

O app se chama **bichinho** (binário, crate, camada, compose; decisão 0036);
o repositório de desenvolvimento continua `claude-pet`
(`~/Documents/claude-pet`).

O plano completo, com marcos M0–M9 e como verificar cada um, está em
`PLANO.md`. As decisões, com o porquê, estão em `DECISIONS.md`.

## Estado do repositório

**Na `main` (tags `v0.1.0`–`v0.4.1`):**
- M0 (fundação) e M1 (overlay nítido e barato; portão fechado com a tela
  acesa, decisão 0005);
- M2 (o Zeca do pack, aprovado pelo Renan pela folha de contato, decisões
  0023–0029);
- M3 (hooks → reação: plugin `bichinho`, cérebro mínimo com `nod`,
  `done_small` e `bye`, `bin/pet testar`, decisões 0019–0022 e 0030–0034);
- a preparação multiplataforma (PR #4, `v0.3.1`, decisões 0035–0046): a
  costura de plataforma (T8.0: o Motor e os traits no `pet-core`, o Wayland
  em `pet-wayland`), o hook nativo `bichinho avisar` em exec form (T8.1) e o
  tamanho do Zeca no config (TP.2, `aparencia.tamanho`);
- o M4 (PR #5, `v0.4.0`, decisões 0047–0064): arrastar com posições salvas
  por monitor, seguir o monitor ativo pelo socket2 com o poof, a fonte
  monogram e o balão mínimo, a proteção de tela e a soneca, a janela de cada
  sessão (`term` no fio v1 e o anel de ativações), o clique que foca o
  terminal pelo foreign-toplevel com o mapeamento do Hyprland, a presença
  pelo `ext_idle_notifier_v1` e os avisos em ciclo;
- o **Zeca original** em CC0 (PR #6, `v0.4.1`, decisões 0065–0070): o
  gerador em `arte/zeca-livre/` e as skins `zeca-livre` (tema claro) e
  `zeca-livre-escuro` (com o anel), redistribuíveis.

O plugin instalado é o 0.2.0 (exec form), da worktree estável na `main`, com
o `~/.local/bin/bichinho` dela. O andamento por tarefa está no
`PROGRESS.md`.

**Na branch `m5-cerebro` (sem PR ainda):** a primeira metade do M5, o
cérebro (T5.1–T5.10, decisões 0071–0080): a pesquisa com o 2.1.288
(`docs/pesquisa/10-cerebro-m5.md`), o hook com `orig` e `crn`, as correntes
de agentes e os turnos de máquina, a pontuação T0–T3 com os pesos no config,
os prazos do estado, os avisos com a escalada L1–L4, a festa e a tela (a
mesclagem, a base, os selos, o sono, a discrição), tudo num registro de
intenções do Motor com cenários dourados (`cenarios/`, `docs/CENARIOS.md`),
`bin/pet simular` e `bin/pet eventos --salvar`. As reações e os balões já vão
para a tela; o desenho do resto (a base segurada, os selos, os voos, o
confete, o pulso, a faixa) e a leitura do `screencast` no socket2 são a
segunda metade, na mesma branch.

**Na tela do Renan:** o `zeca-livre-escuro` no tamanho `pequeno` (D = 4 no
eDP-1), aprovado em 2026-10-05 (decisão 0070). O `zeca` do pack continua
instalado e aprovado: para voltar, é só trocar `aparencia.skin` no
`config/bichinho.toml`.

O Zeca só aparece com a skin da imagem aprovada (`bin/pet skin-aprovar`, que
só aprova a skin da folha de contato vista; decisões 0026 e 0029). Sem
aprovação a produção fica conectada e escondida (`tela: sem_personagem`).
Aprovar e revogar trocam na tela na hora, e o config é relido a cada
aprovação. A pilha de dev (`PET_DEBUG=1`) mostra a skin xadrez `_teste`, ou o
personagem aprovado com `PET_DEBUG_PERSONAGEM=1`. Formato, arte e aprovação
em `docs/SKINS.md`.

**Rumo (2026-10-03, decisões 0035–0042):** lançamento open source para
Linux, macOS e Windows, com o app **bichinho** (o personagem continua Zeca);
o PLANO tem o M8 (multiplataforma) e o M9 (publicação). A pesquisa está em
`docs/pesquisa/09-multiplataforma.md`. O próximo marco é o M5 (cérebro
completo).

**Pendentes** (pedem a tela acesa e desbloqueada, ou o Renan):
- a conferência na tela do M4 (`scripts/verificar-m4.sh` e `--manual`, com
  o checklist do HDMI, da tampa e da suspensão) e o
  `scripts/e2e-monitor.sh --autorizo` (o Renan digita «sim» a cada vez);
- a conferência do Zeca original na tela (nitidez, reações, tamanho
  `pequeno`), a medição de custo com o personagem e a regressão do T8.0
  (`scripts/verificar-ao-vivo.sh --personagem` e `scripts/medir-custo.sh
  --personagem`, que aprovam só para o teste e revogam no fim);
- no Zeca do pack, o aceno (`nod`, levanta e senta) é um pedaço da rajada
  do repouso (`stand_look_sit`) e dá para confundir: mudar pede uma folha de
  contato nova e a reaprovação do Renan. No Zeca original o aceno é a
  tirada de chapéu, sem imagem em comum com o repouso.

## Comandos

O `~/.cargo/bin` **não está no PATH** do Renan; o `bin/pet` acrescenta.
Fora dele, use `~/.cargo/bin/cargo`.

| Comando | O que faz |
|---|---|
| `bin/pet verificar` | portão antes de **todo** commit: fmt, clippy, testes, compose, plugin |
| `rustup target add x86_64-pc-windows-msvc aarch64-apple-darwin` | com os alvos instalados, o `verificar` também passa o clippy do núcleo, dos esboços e do daemon para Windows e macOS (sem linkar, sem SDK) |
| `bin/pet subir` / `parar` / `logs` / `estado` | compose e estado do pet |
| `bin/pet testar rapido` / `pequeno` / `medio` / `grande` / `pergunta` / `dois-prontos` | eventos sintéticos pelo hook de verdade (`bichinho avisar` do PATH ou `PET_BICHINHO`, com o commit dele; sem ele, o `avisar.sh` de reserva, com aviso), com `PET_TESTE=1` (só sessões de teste) → `nod` / `done_small` / `done_medium` com o confete / `done_big` com a chuva (o segundo em 10 min vira T2) / a chamada e o aceno / uma festa só com "2 prontos"; mostra o turno, as intenções da sessão e o `/v1/estado.desenho` (decisão 0087) |
| `bin/pet instalar-host` | copia o binário estático da imagem para `~/.local/bin/bichinho` (o hook do plugin 0.2.0 o acha pelo PATH); só a imagem do commit da worktree estável, de árvore limpa (decisão 0045). `--da-branch` pula a conferência: só com `PET_BIN_HOST` numa pasta de teste fora do PATH |
| `~/.cargo/bin/cargo build -p bichinho` + `PATH="$PWD/target/debug:$PATH" claude --plugin-dir plugin` | o hook da branch só numa sessão (e `PET_BICHINHO=$PWD/target/debug/bichinho bin/pet testar`) |
| `bin/pet tocar <reação>` / `esconder` / `mostrar` | `/v1/comando` (não persiste); o `tocar` diz a tag que a skin tocou e se apareceu na tela |
| `bin/pet clique [esquerdo\|direito]` | clica no pet como o mouse (`/v1/comando` `clique`, decisão 0057) e mostra o que ele fez: `focou` o terminal da sessão do aviso mais urgente (o endereço da janela e se o desktop já confirmou), `balao` com o porquê de não focar, `lista` das sessões ou `soneca` |
| `claude --plugin-dir ~/Documents/claude-pet/plugin` | o plugin da branch numa sessão só (nunca instalar antes do merge) |
| `~/.cargo/bin/cargo test` | testes do workspace (os quadros e os cenários dourados regeneram com `PET_ATUALIZAR_OURO=1`; leia o diff) |
| `bin/pet simular <nome\|arquivo>` | roda `cenarios/<nome>.jsonl` (ou o arquivo) no cérebro e no Motor num relógio falso, offline, e imprime as intenções (o `bichinho simular` da branch: `PET_BICHINHO` ou o cargo); a saída é o `.esperado.jsonl` (decisão 0078) |
| `bin/pet eventos [--salvar <arquivo>]` | o `/v1/debug/eventos` de um pet de debug (`PET_PORTA`); `--salvar` grava um cenário com pseudônimos pelo `bichinho cenario`. Recusa um pet sem debug (a produção nunca guarda eventos) |
| `docker compose -f docker-compose.yml -f docker-compose.dev.yml up --build` | modo desenvolvimento (sem restart, debug, skin `_teste`) |
| `bin/pet foto` | foto do pet (grim no monitor inteiro); em debug, só os pixels opacos do pet sobre fundo neutro |
| `scripts/verificar-ao-vivo.sh` | verificação do M1 na tela de verdade; termina com a produção de pé |
| `scripts/verificar-m4.sh [--manual]` | verificação do M4 contra a produção: o desktop no `/v1/estado` (os protocolos do clique e o da presença), a janela ativa igual à do Hyprland (`hyprctl -j`, só leitura) e, desbloqueado, dois `foot` com título-canário e sessões de teste para o clique levar a cada um (recusa se uma sessão real tem aviso, conferido antes de cada clique); `--manual` guia o Renan no arraste, no restart, na proteção de tela, no clique com o mouse (também com o `foot` noutra área de trabalho) e no checklist do HDMI, da tampa e da suspensão |
| `scripts/e2e-monitor.sh --autorizo` | o pet segue um monitor headless criado e removido no Hyprland: **só com o consentimento do Renan, a cada vez** (ele digita «sim» no terminal; sem terminal, recusa) |
| `cargo xtask globais [interface …]` | lista os globais do Wayland e confere os que o clique pede (só lê o registro) |
| `scripts/medir-custo.sh` | CPU do Hyprland, GPU e commits/s: escondido × parado, com e sem carga de repintura, e estresse |
| `cargo xtask skin-teste` / `nitidez` / `fantasma` / `carga` | gera a skin xadrez; compara captura e quadro esperado; acha pixel velho e fantasma; repintura invisível para medir custo |
| `bin/pet skin-instalar <zip\|pasta>` | o pack vira o Zeca em `skins-locais/` (com e sem contorno, `--estrito`), com lint, cobertura (`--nativos mvp`) e prévias em `tmp/previa-zeca-m2/` |
| `bin/pet skin-aprovar [id]` / `skin-revogar [id]` | aprova o conteúdo exato da skin da folha de contato vista (impressão digital conferida; cópia em `/state`) ou tira a aprovação |
| `cargo xtask zeca --pack <zip\|pasta> [--contorno] [--ancoras] [--estrito]` | monta o Zeca; `--ancoras` mostra o encaixe quadro a quadro; `--estrito` reprova aviso fora de `arte/zeca/avisos-aceitos.txt` |
| `cargo xtask skin-importar` / `lint-skin` / `cobertura` / `contato` | importa um pack; confere a skin; estados cobertos (`--nativos mvp`); folha de contato com a impressão digital e GIFs (escuro e claro lado a lado) |
| `scripts/verificar-ao-vivo.sh --personagem` / `scripts/medir-custo.sh --personagem` | a verificação e a medição do M1 com o personagem no lugar da `_teste` (aprovam só para o teste se faltar aprovação e revogam no fim) |

## Arquitetura em uma tela

- Crates (decisão 0040):
  - `crates/pet-core` é **puro** (só `std` e crates de dados): cérebro,
    animador, skin, raster, o **Motor** (`pet_core::motor`: personagem, pet e
    palco, mostrar/esconder, estresse, painel, prazos em ms num relógio
    injetado) e os contratos de cada sistema (`pet_core::plataforma`: os
    traits `Overlay` e `Desktop`, `Monitor`, `EventoPonteiro` e a `Caixa`).
    Tudo o que o Motor troca com a janela está no **palco**: pixels do
    dispositivo do monitor, origem no canto dele (decisão 0044); cada janela
    converte para as coordenadas dela. Os testes usam a
    `plataforma::falsa::JanelaFalsa` (feature `teste` fora do core).
    **Nunca** depende de crates Wayland ou de sistema, em nenhum alvo — os
    testes ficam rápidos e o `bin/pet verificar` confere;
  - `crates/pet-wayland`: a camada OVERLAY (a `Sessao` é o `Overlay` e o
    `Desktop` do Wayland, na mesma conexão e no mesmo `wl_seat`), os
    buffers, a descoberta e o foreign-toplevel genérico (`toplevel`); o que é
    só do Hyprland (`hyprland.lock`, o monitor FALLBACK, o leitor do socket
    de eventos em `hyprland::eventos`, o mapeamento dos toplevels para os
    endereços em `hyprland::mapeamento`) fica no adaptador `hyprland`;
  - `crates/pet-windows` e `crates/pet-macos`: esboços vazios que compilam
    (`cargo clippy --target` no `bin/pet verificar`, com os alvos do
    rustup instalados);
  - `crates/bichinho`: o daemon e o hook (binário estático no container `alpine`,
    uid 1000, rootfs somente leitura). O `nucleo` junta o Motor com as
    aprovações em disco e o `/v1/estado`; o `laco` é o do Linux (calloop
    com o Wayland); o `sem_janela` roda onde ainda não há janela (Windows e
    macOS). Threads: principal (o laço), ingress HTTP, vigia e o leitor do
    socket2 (só lê, traduz na hora e manda pela caixa do desktop).
  - No Motor (M4): `arraste` (a máquina do ponteiro), `posicoes`, `viagem`
    e `poof` (seguir o monitor), `balao` (o balão, o selo zZ e o coração),
    `janelas` (o anel de ativações e a janela de cada sessão) e o clique em
    ciclo sobre os avisos que o cérebro guarda (`Cerebro::pendencias`).
  - No Motor (M5): `intencoes` (o registro das decisões, decisão 0077),
    `escalada` (a máquina pura da L1–L4) e `tela` (a festa com a mesclagem, a
    base pela prioridade, os selos, o sono e a discrição); o executor dos
    cenários é o `pet_core::cenario` (atrás das features `teste` e
    `simulacao`), com o `de_eventos` que faz o cenário com pseudônimos.
- Uma camada OVERLAY do tamanho do monitor focado, criada com output NULL,
  nunca redimensionada, sem subsurfaces; o Zeca anda dentro do buffer.
  Cada pixel de arte vira um bloco D×D inteiro de pixels do monitor.
- Eventos do Claude Code chegam por `POST 127.0.0.1:27380/v1/evento`
  vindos do plugin `bichinho` (hooks async em exec form → `bichinho avisar
  <Evento>`, com a lista branca do `pet_core::aviso`; decisão 0041),
  são validados campo a campo (`pet_core::evento`, decisão 0019) e vão
  pela `Caixa` (um `mpsc` limitado que acorda o laço; no Linux, por um
  `Ping` do calloop) para o cérebro (`pet_core::cerebro`, decisões 0020 e
  0032), que mora no Motor, conta os prazos da chegada de cada evento e
  funciona mesmo sem compositor. Antes de vencer um prazo, o laço esvazia a
  caixa.
- Comandos chegam por `POST /v1/comando`, sempre `{"cmd", "arg"}`: as
  reações (`tocar`, 200 com a tag e se apareceu na tela; `esconder` e
  `mostrar`, 204), o clique (`clique`, 200 com o que ele fez; decisão 0057)
  e as aprovações (`aprovar_skin`, `revogar_skin`; 200
  depois de o laço trocar o personagem), com as mesmas checagens de `Host`,
  `X-Pet` e `Content-Type` (decisões 0030 e 0033). Só as aprovações passam
  pelo cadeado: uma reação nunca espera uma aprovação. Uma reação toca a
  tag do estado de mesmo nome no `skin.json`, ou a reserva do catálogo
  (`pet_core::estados`), nunca o repouso.

## Regras de ouro

- **Hooks:** sempre `async`, só metadados, nunca imprimem, sempre `exit 0`.
  O hook é o `bichinho avisar <Evento>` em exec form (decisão 0041): a lista
  branca do `pet_core::aviso`, com os validadores do fio v1, lida em fluxo
  (só os campos da lista ficam na memória; decisão 0045), sem log nenhum
  (nem com `PET_LOG=debug`), sem core dump, e TCP direto ao 127.0.0.1
  (nenhum proxy, nenhum curlrc). Conteúdo (prompt, código, resposta, título
  de janela) nunca sai do host nem vai para log. O
  `avisar.sh` fica de reserva até a troca: todo curl que fala com o pet leva
  `-q --noproxy '*'` e o jq dele roda sem `~/.jq` (decisão 0031). Os
  canários dos dois (`tests/hook.rs`, `tests/avisar.rs`) não podem cair.
- **Hyprland:** o daemon **nunca** abre o `.socket.sock` e nunca chama
  `hyprctl dispatch`/`keyword` (nem em `pet-wayland`). Só lê eventos do
  `.socket2.sock`. `hyprctl` só aparece em scripts de teste do host.
- **Títulos de janela** passam pela memória do daemon (as linhas do socket2,
  o título e o app id do foreign-toplevel) e são jogados fora na hora: nunca
  no log, no `/v1/estado`, no `/v1/debug/eventos` nem no disco. Por isso o
  daemon, como o hook, não deixa core dump (`privacidade::sem_core_dump` no
  começo do `rodar` e `ulimits: core: 0` no compose; decisão 0061).
- **Portável:** `unsafe` proibido no `pet-core` e no daemon do Linux; o que
  é de um sistema fica atrás de `cfg` e dos traits de
  `pet_core::plataforma`. Lógica nova do pet (arrastar, balões, voos) entra
  no Motor, com teste em relógio falso, não no `pet-wayland`.
- **Config do Hyprland** (`~/.config/hypr/*.lua`): só pela skill
  `omarchy` e com consentimento do Renan.
- **Arte:** o pack e tudo derivado dele (sheet, GIFs, folhas de contato,
  fotos) ficam em `skins-locais/` ou `tmp/` (gitignored), nunca no git nem
  em `docs/`; o xtask recusa gravar arte de pack em outra pasta do repo. É
  nossa e vai para o git: `arte/zeca/` (acessórios, âncoras, trajetórias do
  Zeca do pack) e o **Zeca original** em CC0 (`arte/zeca-livre/`, com o
  gerador `zeca.py`, e as skins `skins/zeca-livre` e `skins/zeca-livre-escuro`,
  refeitas por `bin/pet skin-livre` e conferidas por
  `cargo xtask zeca-livre --conferir`). Arte do Zeca original se muda no
  gerador, nunca à mão nos PNGs. A skin `_teste` nunca vira personagem.
- **Nome do personagem:** é o Zeca, um papagaio malandro com visual próprio.
  Nunca o chame nem o descreva como personagem de terceiros, e nunca cite
  nome, estúdio ou família de um, em código, docs, balões, commits ou PRs. O
  `bin/pet verificar` reprova se aparecer nos arquivos ou nas mensagens dos
  commits da branch (decisões 0035 e 0043); o corpo do PR se confere antes do
  `gh pr create` com o mesmo padrão (`NOMES_DE_TERCEIROS` no `bin/pet`).
- **Personagem só com aprovação:** o Zeca aparece só com a impressão
  digital aprovada pelo Renan (`bin/pet skin-aprovar`, decisões 0026 e 0029).
  Nunca aprove por ele: aprovação de teste se revoga no fim (os scripts ao
  vivo fazem isso sozinhos, até numa falha). **Nunca revogue uma aprovação
  que você não fez:** antes de mexer, leia `skin` no `/v1/estado` e o
  `/state/skins/<id>/aprovacao.json` (`docker compose exec -T bichinho cat …`);
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
- Plugin: **nunca** `claude plugin marketplace add` / `install` / `update`
  nem mexer na worktree estável (`~/.local/share/claude-pet/estavel`) antes
  do merge na `main` (sessões de outros projetos rodariam a branch). Para
  testar ao vivo, `claude --plugin-dir ~/Documents/claude-pet/plugin`. A
  troca depois do merge está no README (a ordem importa: `bin/pet
  instalar-host` antes de `claude plugin update`).
- O hook em exec form acha o `bichinho` pelo PATH do Claude Code: sem ele,
  os eventos não chegam e o `claude -p` fica calado (conferido). Confira com
  `command -v bichinho`, `bichinho versao` (o commit) e `bin/pet testar`.
- O `~/.local/bin/bichinho` é a lista branca de todas as sessões da máquina:
  só o binário da worktree estável vai para lá (o `bin/pet instalar-host`
  confere o commit gravado na imagem). Nunca instale o de uma branch no PATH
  (`--da-branch` é só para uma pasta de teste); para testar, o binário da
  branch vai só na sessão (`PATH="$PWD/target/debug:$PATH" claude
  --plugin-dir plugin`).
- `bichinho` sem subcomando não faz nada (no terminal, mostra a ajuda): o
  daemon é `bichinho rodar` (o `CMD` da imagem e os testes já passam).
  Um Claude Code que ignorasse o `args` do exec form não sobe daemon.
- O compose avisa que o volume `claude-pet_estado` "foi criado para o
  projeto claude-pet": é de propósito, o nome está preso a ele para a
  aprovação do Zeca sobreviver ao nome novo (decisão 0041). Nunca apague
  esse volume. O `bin/pet subir` (e `dev`, `reconstruir`) aposenta o
  container do projeto antigo, que seguraria a porta 27380.
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
  vem com o `prompt_id` do `/exit`. Cada notificação de tarefa e cada tique
  de um `/loop` é um turno com `prompt_id` novo: o hook do M5 manda o `orig`
  (a forma do prompt) e o `crn` (os agendamentos do Stop), e o cérebro separa
  o que a máquina começou (decisões 0072 e 0073). Sem o binário novo no
  PATH, os dois faltam e o pet degrada sem quebrar.
- `/v1/estado.tela` é o estado da aprovação (`ativa`, `sem_personagem`); a
  fotografia do M5 (a base, os selos, a escalada, a festa) é
  `/v1/estado.fotografia` (decisão 0080).
- Toda regra nova do cérebro ou da tela muda algum dourado de `cenarios/`:
  regere com `PET_ATUALIZAR_OURO=1` e leia o diff linha a linha antes do
  commit (é ele que diz o que o Zeca passou a fazer). O `bichinho simular`
  roda sem personagem, e um teste garante que as intenções não dependem da
  skin.
- Cenário gravado só de um pet de debug, pelo `bin/pet eventos --salvar`
  (pseudônimos, só os campos do fio v1); mesmo assim, leia antes de pôr no
  git.
- `bin/pet testar` precisa do pet de pé; as sessões de teste somem em 60 s
  e nunca se misturam com as reais.
- **Clique e sessão bloqueada:** bloqueado, o Hyprland recusa o `activate`
  do foreign-toplevel sem dizer nada; o clique espera 1,5 s o socket2 contar
  a troca (`/v1/estado.focando`) e só então marca o aviso como visto, ou diz
  no balão que não focou. Bloqueado ou com a tela apagada, o Hyprland ainda
  conta como ativa a última janela que teve o foco: por isso uma janela já
  ativa só conta na hora, e o pronto só sai pelo foco, com o Renan no
  teclado ou no mouse (`desktop.ocioso` falso, pelo `ext_idle_notifier_v1`
  com 5 s; decisão 0062). O aperto de verdade no pet conta como presença;
  o `bin/pet clique` não.
- `bin/pet clique` com avisos de sessões reais pendentes foca terminais de
  verdade: não rode com o Renan trabalhando (o `verificar-m4.sh` usa sessões
  de teste e recusa com avisos reais, que vêm antes no ciclo).
- `WAYLAND_DEBUG=1` (ou `client`) no ambiente do daemon desliga o
  foreign-toplevel: o wayland-client imprimiria os títulos das janelas no
  stderr. O clique cai no balão.
- O socket2 só conta trocas: logo depois de o pet subir, quem diz o
  `desktop.monitor_em_foco` é a camada (output NULL: ela cai no monitor em
  foco; decisão 0059), e o anel começa com a semente do foreign-toplevel (a
  janela ativa na conexão). Sem conexão Wayland o socket2 continua lido: todo
  prazo que o Motor anuncia tem de vencer também no
  `Motor::vencer_sem_conexao`, senão o laço gira a 100% de CPU.
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
  `🤖 Generated with [Claude Code](https://claude.com/claude-code)`. O merge
  é com merge commit, nunca squash: os hashes do PROGRESS continuam valendo
  (decisão 0037). Linha nova do PROGRESS nasce com "—" no commit.
- DECISIONS só cresce (decisões novas no fim, em ordem numérica); a única
  reescrita permitida foi a da decisão 0035.
- Nunca commitar `.env`, `config/bichinho.toml` (ou o `claude-pet.toml` de antes), `skins-locais/*`,
  `tmp/`, `target/`.
