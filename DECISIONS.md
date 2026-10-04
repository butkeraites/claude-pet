# Decisões

Uma seção por decisão: o problema, a escolha, o porquê. Se mudar depois,
acrescente a mudança abaixo da original — não apague a história.

## 0001 — O personagem é o Zeca, feito com arte profissional (2026-10-02)

**Problema:** o primeiro rascunho de mascotes (capivara, gato, slime e
robô desenhados em ASCII) foi recusado como "muito feio". Um bichinho que
fica o dia inteiro na tela precisa de arte de verdade.
**Escolha:** a base é a arara do pack *Cute Parrots!* da exclusiveOlive
(itch.io, 48x48, ~20 animações). Ela vira o **Zeca**, um papagaio malandro
com visual próprio: corpo verde, chapéu-palheta e gravata-borboleta com
faixa laranja. Corpo e animações são da artista; nós só desenhamos os
acessórios pequenos e a troca de paleta. Nada de nome ou visual copiado de
personagem de terceiros (decisão 0035).
**Por quê:** o pack já cobre praticamente todos os estados do pet
(comer, piar, decolar, planar, pousar, dormir, machucar). Desenhar o bicho
inteiro do zero cairia na "arte de programador" que já foi recusada.
A folha de contato com prévias passa pela aprovação do Renan antes de o
Zeca virar o personagem (M2).

## 0002 — Sem som (2026-10-02)

**Problema:** som chama atenção de longe, mas também cansa e invade.
**Escolha:** o pet não toca som nenhum. Não há dependência de áudio na
imagem nem bind do PipeWire.
**Por quê:** pedido explícito do Renan. Toda a atenção vem do movimento:
voo atravessando a tela, rajadas e balões (decisão 0010).

## 0003 — Festa proporcional ao trabalho, não ao relógio (2026-10-02)

**Problema:** o Claude termina toda resposta, até uma pergunta rápida.
Festa em toda resposta vira ruído; e com Opus em esforço máximo, uma
resposta simples já leva 20–90 s, então medir pelo relógio não separa
"respondeu" de "desenvolveu".
**Escolha:** a pontuação do turno soma o tempo de ferramenta (`duration_ms`
do PostToolUse, inclusive de subagentes), ferramentas de trabalho (Edit,
Write, MultiEdit, NotebookEdit, Bash), arquivos editados e subagentes.
Níveis: T0 aceno discreto (nenhum trabalho), T1 pulinho, T2 voo curto com
confete, T3 voo atravessando a tela (no máximo 1 a cada 10 min). Pesos e
limites ficam no config, e cada Stop registra os componentes para a semana
de calibração.
**Por quê:** é a opção "proporcional ao trabalho" escolhida pelo Renan,
medida pelo que o Claude fez e não pelo tempo que pensou.

## 0004 — Motor: um binário Rust com smithay-client-toolkit (2026-10-02)

**Problema:** o pet precisa de uma camada transparente por cima de tudo,
nítida na escala fracionária 1.5 dos dois monitores, leve num notebook de
7,4 GiB, e que sobreviva à queda do compositor dentro do Docker.
**Escolha:** um binário Rust estático (musl) com smithay-client-toolkit
0.21 e wayland-client puro (sem libwayland). Ele escreve os pixels
direto em pixels do monitor: cada pixel de arte vira um bloco D×D inteiro.
**Por quê:** nitidez garantida por construção em qualquer escala, imagem e
RSS pequenas, e reconexão ao compositor sem morrer.
Rejeitados: Quickshell (o caminho GL danifica a superfície inteira a cada
quadro e a imagem passa de 1 GiB) e GTK4+Python (nitidez a 1.5 não
comprovada, `_exit(1)` quando perde o compositor, LD_PRELOAD obrigatório
para o gtk4-layer-shell). O GTK fica só como saída de emergência com prazo,
se o encanamento Wayland não convergir no M1.

## 0005 — Uma camada OVERLAY do tamanho do monitor, com orçamento de commits (2026-10-02)

**Problema:** o bicho precisa ficar acima de tudo, inclusive de tela
cheia, ser arrastado, e às vezes voar pela tela inteira. No Hyprland
0.56.2, mudar margens recalcula o layout, redimensionar estica um buffer
velho, e subsurfaces são esmagadas (#10515).
**Escolha:** uma única camada OVERLAY transparente do tamanho do monitor
focado, criada com output NULL (o Hyprland escolhe o monitor focado), nunca
redimensionada e sem subsurfaces. O Zeca se move dentro do buffer; a
região de input fica só no corpo dele.
**Por quê:** é o mesmo padrão dos toasts do Omarchy e de outros pets em
Hyprland. Ressalva medida na revisão: cada commit de uma camada danifica a
área inteira dela, ou seja, repinta o monitor. Por isso há um orçamento:
média de até 2 commits/s parado, 0 dormindo, rajadas curtas de até 30 fps,
medido no Hyprland (CPU e GPU) no M1. Se estourar, plano B: uma superfície
pequena fixa para o repouso e um palco de tela cheia só durante arraste,
voo e confete.

**Atualização (2026-10-03): portão do M1 fechado com a tela acesa.** Com o
eDP-1 aceso (as decisões 0017 e 0018 tinham medido só com DPMS),
`scripts/verificar-ao-vivo.sh` passou inteiro:
- nitidez: 22 300 pixels opacos, nenhum fora de ±2, 892 blocos 5×5
  uniformes;
- sem pixel velho e sem fantasma (ao esconder e no SIGTERM);
- 0,80 commit/s parado;
- restart em 319 ms e volta de `kill -9` em 262 ms.

`scripts/medir-custo.sh` (3 rodadas de 20 s intercaladas) mediu:

| fase | CPU do Hyprland % | GPU ocupada % | commits/s do pet |
|---|---|---|---|
| escondido | 7,28 (6,50–7,89) | 11,03 (9,90–11,70) | 0 |
| parado | 7,86 (7,54–8,25) | 17,17 (16,70–17,80) | 0,70 |
| carga + escondido | 8,82 (8,39–9,24) | 17,47 (16,00–18,40) | 0 |
| carga + parado | 9,02 (8,94–9,14) | 19,07 (18,70–19,30) | 0,75 |
| estresse (40 confetes, 30 fps) | 9,66 | 19,4 | 30,8 |

A carga é uma repintura de tela cheia a 60 commits/s, como um vídeo. O
RSS do Hyprland vai de 61,0 para 69,8 MiB com a camada do pet mapeada.

- **Parado:** +0,58 ponto de CPU (orçamento ≤ 1) e 0,70 commit/s (≤ 2).
- **Custo estrutural sob repintura de tela cheia:** +0,20 ponto de CPU
  (≤ 1) e +1,6 de GPU (≤ 5).
- **Decisão:** **a camada única fica; o plano B não é necessário.**
- **Ponto de atenção para a bateria:** parado, a GPU sai mais vezes do
  RC6 (+6 pontos de ocupação). O sono profundo (0 commits) e rajadas de
  repouso mais espaçadas são as alavancas, se um dia pesar.
- **A carga tem de ser OVERLAY.** A primeira medição usou uma camada
  BACKGROUND, tapada pelo terminal em tela cheia, e o Hyprland não manda
  frame callback para superfície tapada: só 2,6 commits/s. A carga
  passou a ser OVERLAY transparente, sem área clicável.

## 0006 — Só eventos do Hyprland; nunca o socket de comandos (2026-10-02)

**Problema:** seguir o monitor ativo exige saber do Hyprland qual monitor
tem o foco.
**Escolha:** o daemon só lê o `.socket2.sock` (eventos: `focusedmonv2`,
`monitoraddedv2`/`monitorremovedv2`, `openwindow`/`closewindow`,
`activewindow`, `screencast`), numa thread dedicada que drena sempre, com
debounce de 300 ms. Nunca abre o `.socket.sock`.
**Por quê:** o socket de comandos executa programas no host
(`dispatch exec`) e congela o Hyprland por até 5 s se um cliente travar.
Eventos mais o próprio Wayland bastam. Drenar sempre evita a desconexão
que o Hyprland faz quando um cliente acumula 64 eventos.

## 0007 — Docker com bind de /run/user e descoberta em tempo de execução (2026-10-02)

**Problema:** o socket do Wayland e a assinatura do Hyprland mudam a cada
login; montar `/run/user/1000` ou sockets avulsos quebra no boot (o Docker
cria o caminho como root) e depois de recriar o socket.
**Escolha:** `restart: unless-stopped`, bind de `/run/user` inteiro em
`/host/run/user` (somente leitura, `propagation: rslave`,
`create_host_path: false`), e descoberta dentro do processo: a assinatura
mais nova em `hypr/`, o `hyprland.lock` para o nome do socket Wayland, e
`connect()` para provar que está viva. `XDG_RUNTIME_DIR` do container é
privado (`/tmp/xdg`).
**Por quê:** neste notebook o `docker.service` sobe no boot e o linger está
ligado, então o container sobe antes do login, espera o compositor e
sobrevive a logout/login, crash, suspensão e hotplug sem unit de systemd.

## 0008 — Entrada de eventos por HTTP no loopback (2026-10-02)

**Problema:** os hooks rodam no host e o pet no container.
**Escolha:** HTTP em `127.0.0.1:27380`, publicado só no loopback.
`POST /v1/evento` responde 204; checa Host, `Content-Type:
application/json` e `X-Pet: 1`, e limita o corpo a 8 KiB.
**Por quê:** o curl existe em qualquer máquina; as checagens de cabeçalho
barram requisições vindas de navegador (DNS rebinding, CSRF). Um socket
Unix exigiria um diretório do host existindo antes do boot.

## 0009 — Hooks assíncronos num plugin próprio, só com metadados (2026-10-02)

**Problema:** o hook não pode atrasar o Claude, não pode sujar o
transcript quando o pet estiver desligado, e o payload dos hooks carrega
prompts, código e respostas.
**Escolha:** plugin `bichinho` neste repo, com hooks de comando `async`
que chamam `plugin/scripts/avisar.sh`. O script monta, com uma lista
branca do jq, só metadados (evento, ids, ferramenta, tipo de notificação,
enum de erro, contagens, hash do arquivo editado, basename do projeto),
manda com `curl -m 2` e sempre sai 0. Instalação por marketplace local
(`bichinho-local`) apontando para uma worktree estável da `main`.
**Por quê:** async não segura o Claude; hooks do tipo http mostram erro no
transcript sempre que o pet está desligado; e nenhum conteúdo sai do host
(testes canário provam). Nomes de plugin que começam com `claude-` são
reservados, por isso `bichinho`.

## 0010 — Atenção só visual, com teto e consciente de presença (2026-10-02)

**Problema:** o pet tem que ser notado quando o Claude precisa do Renan,
sem virar um pisca-pisca que irrita, e sem som (decisão 0002).
**Escolha:** escalada L1 (0 s) → L2 (+30 s) → L3 (+90 s, voo até o alto da
tela) → L4 (teto, pulso discreto), com uma vaga de aviso por sessão.
L2 em diante só se o Renan não está olhando o terminal do Claude (título
começando com ✳, ◐ ou ◑) ou está sem mexer há 60 s. DND do Omarchy e
compartilhamento de tela deixam tudo discreto. Movimento grande só no
início de cada estado; trabalhando, o Zeca fica quase parado.
**Por quê:** o que chama atenção é o movimento que começa, não o movimento
constante; e com `--dangerously-skip-permissions` os "precisa de você"
reais são AskUserQuestion e ExitPlanMode, que aparecem quase sempre com o
Renan olhando o terminal.

## 0011 — Arte não redistribuível fica fora do git (2026-10-02)

**Problema:** a licença do pack permite editar e usar, mas proíbe
redistribuir, "mesmo editado". Um repo pode virar público um dia.
**Escolha:** o pack e tudo derivado dele (inclusive o Zeca recolorido)
ficam em `skins-locais/`, que é gitignored e entra só na imagem local
(nunca vai a registry). Acessórios, paleta e âncoras são arte nossa e vão
para o git em `arte/zeca/`. A skin xadrez `_teste` nunca é mostrada como
personagem: sem skin aprovada, o pet fica escondido.
**Por quê:** respeita a licença sem atrapalhar o uso pessoal, e um clone
novo só precisa de `bin/pet skin-instalar <zip>`.

## 0012 — Nomes (2026-10-02)

**Problema:** o personagem pode mudar; o código não deveria.
**Escolha:** repo, binário, compose e namespace da camada: `claude-pet`.
Plugin `bichinho`, marketplace `bichinho-local`. Personagem/skin `zeca`.
CLI `bin/pet`. Variáveis `PET_*`, cabeçalho `X-Pet: 1`.
**Por quê:** trocar o Zeca por outro bicho só mexe na skin e nos balões.

## 0013 — Fuso do host, nunca TZ fixo (2026-10-02)

**Problema:** o relógio da máquina está em America/New_York e os outros
projetos usam America/Sao_Paulo.
**Escolha:** bind de `/etc/localtime` somente leitura; nunca definir `TZ`.
**Por quê:** não dá para presumir onde o Renan está; o relógio do host é a
melhor aproximação de "agora" para sono e horários.

## 0014 — O container é empacotamento, não sandbox (2026-10-02)

**Problema:** acesso ao socket do Wayland (teclado virtual, captura de
tela) e ao `/run/user` já equivale a rodar código como o usuário.
**Escolha:** tratar o container como empacotamento e reduzir o resto:
uid 1000, `cap_drop: ALL`, `no-new-privileges`, rootfs somente leitura,
limites de pids/memória/CPU, sem `docker.sock`, sem rede do host, só
loopback na entrada, o daemon abre só `wayland-N` e `.socket2.sock`, bases
fixadas por digest, `cargo build --locked`.
**Por quê:** é honesto sobre o que o Docker protege aqui e ainda fecha as
portas baratas.

## 0015 — Toolchain stable no host; versão fixa no Docker (2026-10-02)

**Problema:** fixar `channel = "1.98.1"` no `rust-toolchain.toml` faria o
rustup baixar escondido um segundo toolchain (o instalado é `stable`).
**Escolha:** `rust-toolchain.toml` com `channel = "stable"` e os
componentes clippy e rustfmt; a versão exata do compilador fica fixada na
imagem de build `rust:1.98.1-alpine3.24`.
**Por quê:** nada muda no sistema sem o Renan pedir, e o binário que roda
continua reprodutível.

## 0016 — Parado barato, um quadro em voo e esconder sem fantasma (2026-10-02)

**Problema:** cada commit da camada repinta o monitor inteiro no Hyprland
0.56.2 (decisão 0005), então a tag `idle` em laço estouraria o orçamento de
2 commits/s. E, medido no M1: com a tela apagada (DPMS) o Hyprland não
desenha, não manda frame callback e o `grim` espera para sempre; camadas
destruídas ficam no `hyprctl -j layers` com `pid: -1` até a tela acender.
**Escolha:**
- parado = pose fixa (primeiro quadro da primeira tag de `idle`) e, a cada
  4 s, uma rajada que toca uma das tags de `idle` uma vez, alternando entre
  elas; com a skin de teste dá menos de 1 commit/s;
- um quadro em voo de cada vez, pelo frame callback; se ele não chegar em
  5 s, o próximo quadro segue mesmo assim (tela apagada: no máximo 1 commit
  a cada 5 s, sem congelar a animação por um callback perdido);
- esconder (debug ou SIGTERM) = região de input vazia, quadro transparente
  com commit, e a superfície só morre no frame callback seguinte ou 50 ms
  depois.
**Por quê:** cabe no orçamento sem perder a animação, não gasta nada com a
tela apagada e o fade de saída do Hyprland fotografa um quadro vazio, sem
fantasma.

## 0017 — Medições do M1 nesta máquina (2026-10-02)

**Problema:** o M1 é o portão do motor: o overlay tem de ser nítido,
barato no compositor, pequeno no container e aguentar reinício, crash e
falta de compositor (PLANO.md, verificação do M1).
**Escolha:** registrar o que `scripts/verificar-ao-vivo.sh` mediu (eDP-1,
1920x1200, escala 1.5) e o que ficou pendente:

| Item | Medido | Orçamento |
|---|---|---|
| Camada | `claude-pet` no nível 3 de eDP-1, 1280x800 lógicos em (640,0); `preferred_scale` 180/120; buffer 1920x1200; D=5, célula em (1706, 951) | nível 3, retângulo do monitor |
| Imagem | 4,68 MB (`docker image inspect`; 16,5 MB no `docker image ls` do containerd) | < 40 MB |
| RSS do container | 12,4 MiB com o pet parado (9 216 000 bytes são o SHM do buffer) | < 64 MiB |
| CPU do container parado | 0,02% (média de 5 amostras) | < 1% |
| Commits parado | 2 no último minuto | ≤ 120/min |
| Região de input | 94x107 lógicos em (1170, 677): só o corpo | só o corpo |
| `docker compose restart pet` | de volta em 659 ms, no mesmo lugar | ~3 s |
| `kill -9` pelo host | RestartCount 0 → 1, de volta em 489 ms | sobe |
| Sem compositor | "aguardando compositor: runtime do usuário ainda não existe" | loga e espera |
| Nitidez na tela | **pendente** | ±2 por canal, blocos D×D uniformes |
| Custo no Hyprland | **pendente** | parado ≤ +1 ponto de CPU, ≤ 2 commits/s |
| Clique ao lado do pet | **manual, pendente** | chega na janela de baixo |

A tela ficou apagada (DPMS) a sessão inteira, e assim o Hyprland não
desenha: não há captura para a nitidez nem repintura para medir. O
`cargo xtask nitidez` foi provado numa captura sintética (passa no lugar
certo; falha com 1 pixel de deslocamento e com um redimensionamento de
0,5%). Com a tela acesa: `scripts/verificar-ao-vivo.sh` (nitidez) e
`scripts/medir-custo.sh` (tabela escondido/parado/estresse), e os números
entram aqui como decisão nova.
**Por quê:** o container cabe com folga em todos os orçamentos e o ciclo de
vida funciona. O custo no compositor, que é o que decide entre a camada
única do tamanho do monitor e o plano B (superfície pequena de repouso +
palco temporário, decisão 0005), ainda não foi medido; até lá a camada única
continua sendo o desenho, e o M1 não fecha sem essas duas medições.

## 0018 — Revisão do M1: ritmo sem reserva, saída confirmada e custo sob carga (2026-10-02)

**Problema:** três revisões adversariais do M1 acharam, conferido no código:
- **mostrar logo depois de esconder se perdia:** a camada que estava saindo
  morria em seguida e o pet ficava escondido para sempre;
- **fantasma possível ao esconder:**
  - no SIGTERM o quadro transparente só ia com `flush` e o processo saía.
    O libwayland-server destrói um cliente que desligou sem ler o que ainda
    estava no socket;
  - logo depois de uma troca de escala o esconder pulava o quadro
    transparente.
- **a reserva de 5 s da decisão 0016 continuava fazendo commit com a tela
  apagada**, empilhando frame callbacks (uns 720 por hora). "Não gasta nada
  com a tela apagada" estava errado;
- **a linha "Commits parado: 2 no último minuto" da decisão 0017 não mede o
  orçamento:** foi tirada com DPMS, menos de um minuto depois da subida;
- **o `scripts/medir-custo.sh` não via o custo que decide entre a camada
  única e o plano B:** uma camada transparente do tamanho do monitor, sempre
  mapeada, entra em toda repintura que os outros causam.

**Escolha:**
- **Ritmo:**
  - no máximo um frame callback pendente, sem prazo de reserva. Callback não
    se perde: só não vem enquanto o monitor não desenha;
  - com a tela apagada, o pet faz o primeiro commit e depois nenhum. Medido ao
    vivo: 0 commits em 20 s e em 10 s.
- **Mostrar e esconder** viram função pura e testada:
  - mostrar durante a saída cancela a saída e força o quadro do pet;
  - esconder decide pelos pixels que o compositor guarda, e não pela cena
    guardada.
- **Saída do processo:** quadro transparente, destruição da camada e um
  `wl_display.sync` com prazo de 1 s. Ao vivo: "o compositor processou o
  quadro transparente e a destruição", parada em 377 ms.
- **Orçamento parado garantido pelo animador para qualquer skin:**
  - a pausa antes de cada rajada cresce até o trecho caber em 2 commits/s;
  - nenhum quadro dura menos de 34 ms (rajadas de até 30 fps).
  - Na skin `_teste` nada muda: 7 commits a cada 9,18 s, ≈ 0,76/s, calculado.
- **Descoberta:**
  - prova de vida com prazo de 3 s antes do registro, que não tem prazo;
  - nenhuma conexão na instância em backoff;
  - symlinks curtos só numa pasta própria;
  - rearmar o batimento depois de um erro não bate.
- **FALLBACK e 0x0:** um `enter` no FALLBACK (ou num monitor sem tamanho) e
  um `configure` 0x0 nunca viram casa do pet. A camada espera um monitor de
  verdade (`new_output`) e é recriada.
- **Custo (`scripts/medir-custo.sh`):**
  - escondido × parado intercalados em 3 rodadas, com média e faixa;
  - fase nova com carga de repintura invisível (`cargo xtask carga`): uma
    camada BACKGROUND transparente que faz commit a cada frame callback e
    repinta o monitor inteiro no ritmo dele, como um vídeo em tela cheia;
  - estresse;
  - cada transição conferida (estado, commits/s esperados, carga rodando,
    tela acesa).
- **Critério do custo estrutural** (provisório, até haver números): com a
  carga, o pet parado pode somar até +1 ponto de CPU do Hyprland e +5 pontos
  de GPU ocupada sobre o pet escondido. Estourou → plano B (decisão 0005).
- **Verificação ao vivo** ganhou:
  - esconder e mostrar seguidos;
  - pixel velho (onde o quadro é transparente tem de aparecer o fundo);
  - fantasma logo depois de esconder e durante o SIGTERM da troca para a
    produção (`cargo xtask fantasma`, comparando com capturas sem o pet);
  - commits numa janela fixa, só com a tela acesa.
- **`bin/pet foto`** em debug mostra só os pixels opacos do pet.

**Por quê:** cada correção fecha um caminho em que o pet some sem querer,
deixa um fantasma ou gasta com a tela apagada. Neste host o
`render:direct_scanout` é 0 (consulta de leitura), então a camada única
não está bloqueando scanout direto; se o Renan ligar, o custo de vídeo em
tela cheia muda e a medição sob carga tem de ser refeita.

A tela ficou apagada (DPMS) também nesta sessão. Continuam sem número:
- nitidez na tela;
- pixel velho e fantasma;
- ritmo parado com a tela acesa;
- custo no Hyprland, com e sem carga;
- clique manual;
- fotos do PR.

O portão do M1 segue aberto. Os números entram numa decisão nova quando
`scripts/verificar-ao-vivo.sh` e `scripts/medir-custo.sh` rodarem com a
tela acesa.

Ao vivo com a tela apagada passaram:
- camada no nível 3 de eDP-1;
- esconder e mostrar seguidos;
- imagem de 4 684 kB;
- RSS de 11,0 MiB;
- CPU parado de 0,01%;
- 0 commits em 10 s;
- região de input só no corpo;
- restart em 651 ms, no mesmo lugar;
- saída confirmada;
- `kill -9` com RestartCount 0 → 1, de volta em 471 ms;
- "aguardando compositor" sem compositor.

## 0019 — Fio v1: validação campo a campo e entrada dos eventos (2026-10-03)

**Problema:** o M3 congela o formato de fio dos hooks (revisão de produto:
tarefas em segundo plano, sessões sem terminal, `stop_hook_active`). A
entrada HTTP é a segunda barreira de privacidade, depois da lista branca do
`avisar.sh`. Um campo inesperado não pode vazar conteúdo nem deixar o pet
surdo quando o Claude Code muda um enum.
**Escolha:**
- `POST /v1/evento` exige `Host` de loopback na porta pública, `X-Pet: 1` e
  `Content-Type: application/json` (415), corpo de até 8 KiB (413) com
  `Content-Length` (411). Só `v = 1` e `e = [A-Za-z]{1,40}` são
  obrigatórios (400); aceito, 204.
- Cada campo opcional tem tipo, tamanho e classe de caracteres (tabela em
  `pet_core::evento`). O que não passa é descartado sozinho e só o nome do
  campo fica; campos desconhecidos são ignorados; nenhuma mensagem de erro
  cita o corpo (o erro do serde pode citar).
- `bgt` e `bgi` andam alinhados, até 16 tarefas; `bg` é a contagem total.
- O ingress carimba a hora de chegada (relógio do host) e manda o evento ao
  laço principal pelo canal do calloop (256 vagas; cheio, 503).
- `/v1/estado.eventos`: aceitos, recusados e idade do último. Em debug,
  `/v1/debug/eventos` guarda os últimos 200 eventos já validados, com a
  hora de chegada e os nomes descartados; sem debug nada é guardado. No log
  vão só o nome do evento e os 8 primeiros caracteres do `sid`.
- `POST /v1/comando`, subconjunto do M3: `{"cmd":"tocar","arg":"<reação>"}`,
  `esconder` e `mostrar`, sem persistir; campo a mais é recusado (um pedido
  como `esconder 30m` não pode virar `esconder` para sempre em silêncio).
- O animador toca uma reação uma vez, com as durações por quadro (mínimo de
  34 ms), e o repouso recomeça no fim dela com a pausa inteira. `nod` cai
  no estado `wave` quando a skin não tem `nod`; `done_small` também tem o
  `wave` de reserva; `bye` não tem reserva e, sem estado próprio, não anima.
**Por quê:** a lista branca do `avisar.sh` é a primeira barreira e os
canários a provam; esta protege contra um script velho, um bug ou outro
processo local. Descartar campo por campo, em vez de recusar o evento,
mantém o pet funcionando depois de uma atualização do Claude Code; `v` e
`e` são o mínimo para rotear.

## 0020 — Cérebro mínimo do M3: sessões, turnos e acomodação do Stop (2026-10-03)

**Problema:** o M3 liga o primeiro elo hook → reação sem pintar o M5 num
canto. Os hooks são async e chegam fora de ordem; um Stop pode ser seguido
de mais trabalho (Stop hook de outro plugin); `claude -p`, SDK e IDE criam
sessões sem terminal; o `bin/pet testar` roda ao lado da sessão real do
Claude que o chama; e sessões morrem sem `SessionEnd`.
**Escolha:**
- Core puro (`pet_core::cerebro`) com relógio injetado: parede (o mesmo do
  `ts` dos hooks) para ordem e duração, monotônico para os prazos. O `ts` só
  vale a até 6 h da hora de chegada carimbada no ingress; fora disso vale a
  chegada.
- Sessões por `sid`, só das origens em `sessoes.origens` (padrão `["cli"]`;
  evento sem `ent` não conta). Eventos de teste vivem num mundo à parte
  (chave `(teste, sid)`) e somem 60 s depois do último evento; sessão real
  sem eventos some em 12 h; no máximo 64 de cada tipo.
- Turnos por `prompt_id`. O `UserPromptSubmit` abre o turno (t0); evento de
  um turno que ninguém abriu (pet reiniciado, prompt atrasado) abre um
  turno implícito; um id novo encerra o turno aberto: com Stop pendente
  comemora na hora, sem Stop foi abandonado (Esc) e fecha sem festa.
- Componentes por turno: ferramentas de trabalho (Edit, Write, MultiEdit,
  NotebookEdit, Bash, também as que falharam), outras, arquivos únicos
  (`arq`), soma de `dur`, subagentes, falhas e ferramentas de subagentes.
  Ferramenta de subagente conta para o turno em que ele nasceu e não muda o
  estado da sessão. O `SubagentStart` chega com `agente` (o `agent_id` é o
  do subagente que nasce), mas vale como evento da thread principal.
- Stop: acomodação de 0,8 s, cancelada só por um evento de trabalho da
  thread principal (PreToolUse, PostToolUse, PostToolUseFailure,
  PermissionRequest, SubagentStart, PreCompact, PostCompact) da mesma
  sessão e do mesmo turno com `ts` posterior ao Stop. Dedupe por
  `(sid, turno)`, lembrando os últimos 32 turnos fechados de cada sessão.
- Fecham sem festa: ferramenta interrompida (`intr`), `idle_prompt` com
  turno aberto e sem Stop, `StopFailure` e `SessionEnd`.
- T0 (nenhuma ferramenta de trabalho, nenhum subagente, nenhum arquivo
  editado) → `nod`; o resto → T1, `done_small`. `celebracao.modo`:
  `desligada` não reage, `discreta` só acena, `sempre_grande` é igual ao
  proporcional até o M5 trazer níveis maiores.
- `SessionEnd` sempre larga a sessão com o turno e a acomodação; `bye` só
  quando não sobra sessão do mesmo tipo (a skin de teste não tem `bye`, e o
  tchau não anima no M3).
- `/v1/estado`: `sessoes` (8 caracteres do `sid`, projeto, origem, estado,
  contadores, hora do último evento), `ultima_reacao` (nome, `sid8`,
  projeto, hora, nível, teste), `turnos` (os últimos 20, com todos os
  componentes; `relogio_ms` só para calibrar, nunca pontua) e
  `cerebro.ignorados`, a contagem do que não contou, por motivo.
**Por quê:** a ordem dos hooks async é a do relógio do host no início do
script, não a de chegada; 0,8 s cobre o atraso de um PostToolUse sem
segurar a festa. O filtro de origem e o isolamento dos testes vêm da
revisão de produto. O M5 pontua T2/T3, correntes e escalada em cima dos
mesmos componentes, sem mudar o fio nem o registro.

## 0021 — Gate ao vivo do M3 e plugin só por `--plugin-dir` até o merge (2026-10-03)

**Problema:** o plano verifica o M3 com `claude plugin list` mostrando
`bichinho@bichinho-local` habilitado, mas instalar o marketplace antes do
merge faria toda sessão do Claude nesta máquina rodar o plugin de uma
branch (revisão de produto: acoplamento in-place). E o fio v1 foi escrito a
partir dos tipos do d.ts do 2.1.288: faltava ver o que o Claude Code manda
de verdade.
**Escolha:**
- Até o merge, o plugin só entra numa sessão por vez:
  `claude --plugin-dir ~/Documents/claude-pet/plugin`. A instalação
  (worktree estável destacada na `main`, `claude plugin marketplace add`,
  `claude plugin install bichinho@bichinho-local`) vem depois do merge.
  `claude plugin list` e `claude plugin marketplace list` terminaram o M3
  iguais aos de antes.
- Gate ao vivo (tela em DPMS; pilha de dev para ter o `/v1/debug/eventos`,
  produção refeita da branch no fim):
  - `bin/pet testar rapido` → `nod` e `bin/pet testar pequeno` →
    `done_small`, na produção;
  - sessão interativa no tmux em `~/Documents`, com as variáveis
    `CLAUDECODE` e `CLAUDE_CODE_*` do agente tiradas do ambiente:
    "responda só: ok" → `nod` (T0); "crie o arquivo …" → o Claude usou um
    Bash (`mkdir && echo`) → `done_small` (T1); "use a ferramenta Write …"
    → Read e Write → `done_small`, com o `arq` igual ao sha256 do caminho
    calculado à parte; `/exit` → `SessionEnd` → `bye`;
  - pet parado: `claude -p --plugin-dir …` imprimiu só `ok`, stderr vazio,
    saída 0; o `avisar.sh` levou ~30 ms com a porta recusando e 2,04 s com
    um servidor que aceita e nunca responde;
  - pet de pé: um `claude -p` mandou SessionStart, UserPromptSubmit, Stop e
    SessionEnd com `ent: sdk-cli`, e o cérebro ignorou os quatro.
- O que o 2.1.288 mandou (só os metadados que o `/v1/debug/eventos`
  guarda; nenhum campo descartado, nenhum evento recusado):
  - `CLAUDE_CODE_ENTRYPOINT` chega aos hooks posto pelo próprio Claude
    Code: `cli` no terminal, `sdk-cli` no `-p`;
  - `SessionStart` com `source: startup` e sem `prompt_id`;
  - `UserPromptSubmit` **sem** `source` (o d.ts avisa que o campo ainda
    está chegando): o turno fica com `src: null`;
  - `Stop` com `background_tasks: []` e `stop_hook_active: false`;
  - `PostToolUse` com `duration_ms` (Bash 151 ms, Read 33 ms, Write 84 ms);
  - `SessionEnd` chega mesmo com o processo saindo, com `reason:
    prompt_input_exit` (terminal) ou `other` (`-p`) e o `prompt_id` do
    próprio `/exit`;
  - do hook ao pet: 25 a 82 ms; a reação sai ~850 ms depois do `ts` do Stop.
- Não exercitados ao vivo (cobertos pelos canários e pelos testes do
  cérebro): PreToolUse de AskUserQuestion/ExitPlanMode, PermissionRequest,
  Notification, SubagentStart, PreCompact, PostCompact, StopFailure e
  PostToolUseFailure.
**Por quê:** sessões de outros projetos não podem rodar código de uma
branch; instalar a partir da `main` estável é o desenho do plano. Ver os
campos de verdade confirma o fio v1 e mostra que o `source` do
UserPromptSubmit ainda não vem: o M5 usa esse campo para continuação de
correntes e terá de tolerar a falta dele.

## 0022 — Revisão do M3: tchau só quando o processo sai e motivos com teto (2026-10-03)

**Problema:** relendo o cérebro antes de publicar a branch:
- um `/clear` (ou uma retomada) manda `SessionEnd` com `reason: clear`
  (`resume`) para o `sid` velho, e o processo segue com outro `sid`. Com
  uma sessão só, o pet daria tchau sem o Renan ter saído, contra o plano
  ("só o tchau depende do motivo");
- `cerebro.ignorados` ganhava uma chave por origem diferente: um processo
  local mandando `ent` sempre novo cresceria o mapa sem limite.
**Escolha:**
- `SessionEnd` continua largando a sessão, o turno e a acomodação sempre;
  o `bye` só vem com a última sessão **e** um motivo de saída do processo
  (`prompt_input_exit`, `logout`, `other`), nunca com `clear` ou `resume`;
- no máximo 32 motivos distintos em `ignorados`; os novos depois disso
  contam em `outros`.
**Por quê:** o tchau é a despedida de quem fechou o Claude; um `/clear` é
o mesmo terminal continuando. O teto custa uma linha e fecha o único mapa
do cérebro que ainda crescia com dado de fora.

## 0023 — O Zeca é o Parrot 2 no visual "Malandro rosa" (2026-10-03)

**Problema:** a decisão 0001 previa troca de paleta (bico amarelo, peito
creme) e acessórios laranja. Em 2026-10-03 o Renan viu a prévia com três
visuais sobre o pack comprado e escolheu.
**Escolha (do Renan):**
- o personagem se chama **Zeca** (nome confirmado) e nasce do **Parrot 2**,
  o papagaio verde do pack *Cute Parrots!*;
- **visual 1, "Malandro rosa":** o **bico rosa original fica** (`#E1536F` e
  `#CE3F6F`; a troca de paleta é vazia), **chapéu-palheta** com **faixa
  laranja** (`#FA9662`) e uma **gravata-borboleta rosa** pequena no peito
  (o rosa do bico, com o nó em `#CE3F6F`);
- papagaio malandro com visual próprio: nada de nome nem visual copiado de
  personagem de terceiros (sem paletó, sem charuto, sem guarda-chuva; faixa
  laranja e gravata rosa; decisão 0035).

Como o encaixe funciona (`cargo xtask zeca`, `arte/zeca/`):
- chapéu e gravata são grades de texto nossas (MIT), só com cores da paleta
  do próprio `.aseprite` (a rampa nova é a da palha), contorno na tinta do
  pack `#1D2427`;
- âncora: o olho branco do pack (a maior mancha branca, para as bolhas do
  sono e o risco da mordida não enganarem); chapéu em (olho − 4, olho − 6),
  gravata em (olho + 1, olho + 6);
- correções por tag e por quadro em `arte/zeca/ancoras.json`: no voo a
  gravata desce para a linha do queixo, atrás do bico (encostada no bico ela
  vira bico); a gravata some quando o peito está virado para longe (cabeça
  baixa comendo, decolagem e pouso agachados, mergulho de cabeça para
  baixo); comendo, o chapéu fica reto (as versões tortas comparadas lado a
  lado ficaram piores);
- quadros de clarão (silhueta branca do susto): sem olho, a âncora vem do
  quadro comum com a mesma silhueta, e chapéu e gravata também ficam
  brancos;
- a tag `Death` fica fora da skin: o Zeca não morre.

A fonte monogram, que estava na lista do M2, vai para o M6 junto com os
balões, que são os únicos que a usam.
**Por quê:** o bico rosa com gravata rosa amarra as cores, e o chapéu de
faixa laranja é a marca de malandro sem copiar personagem nenhum. O
encaixe por dados (regra do olho mais correções) deixa cada quadro conferido
e reproduzível sem redesenhar nada do pack.

## 0024 — Chapéu voa e volta (2026-10-03)

**Problema:** no mergulho a cabeça fica para baixo e a regra do olho põe o
chapéu no meio do corpo; o susto começa num quadro de silhueta branca.
**Escolha (pedido do Renan, "chapéu voa e volta"):** no mergulho e no susto
o chapéu sai da cabeça e cai de volta nela, como comédia física.
- **Variantes desenhadas à mão** (`arte/zeca/acessorios/`): a cambalhota no
  sentido horário em 0°, ~20°, 90°, 180°, 270° e ~340°, um torto de ~10° e
  o **amassado**, que aparece no quadro em que o chapéu cai na cabeça e
  desamassa no seguinte.
- **Trajetória quadro a quadro** em `arte/zeca/chapeu_voando.json`, que troca
  os quadros da tag: corpo do pack, duração e chapéu solto (variante e
  posição na célula) ou assentado.
- **Susto (`hurt`, `fly_hurt`):** o chapéu pula no clarão (branco como a
  silhueta), dá uma volta inteira enquanto o Zeca se encolhe e cai quando ele
  se endireita. O `hurt` passa de 400 para 840 ms (o corpo encolhido segura
  300 ms).
- **Mergulho:** a cabeça sai de baixo do chapéu, que fica no ar e começa a
  tombar (`dive_start`); no laço (`dive_loop`) o chapéu gira em cima dele, uma
  volta por ciclo de 4 quadros, para o laço emendar; no `dive_end` ele
  completa a volta e cai na cabeça quando o Zeca volta à horizontal (3
  quadros a mais, 680 ms).
- **Pouso (`landing`):** o chapéu se reassenta com o tranco: sobe 1 pixel e
  amassa no impacto.
**Por quê:** o chapéu que voa e volta transforma o quadro em que a regra
falhava na piada. A cambalhota usa poucas variantes legíveis a 48×48, e o
amassado vende o peso da queda.

## 0025 — Folha do Zeca: nomes normalizados, tags compostas e duas variantes (2026-10-03)

**Problema:** os nomes do pack têm espaço e parêntese (`Sit(End)`, `End
Dive`), e vários estados do pet são sequências de tags (levantar e olhar,
voo curto, voo com mergulho). O animador do M1 alterna as tags de `idle` e
o do M3 toca a primeira tag de cada reação. E o contorno creme ficou para o
Renan decidir vendo a folha.
**Escolha:**
- **Nomes:** o importador normaliza para ids (`Sit(End)` → `stand`, `End
  Dive` → `dive_end`, `Fly Bite` → `bite`) e guarda o original no campo
  `data` da tag, que a folha de contato mostra.
- **Tags compostas** (`arte/zeca/zeca.toml`), com quadros repetidos
  apontando para a mesma célula da folha (custam zero pixel):
  `stand_look_sit` (rajada do repouso: levanta, respira em pé e senta, sem
  pular do sentado para o em pé), `nod` (aceno T0), `short_flight` (T2) e
  `big_flight` (T3, com o mergulho e o chapéu voando). O caminho pela tela
  é do M6.
- **Estados:** `idle` = pose fixa do Sit Idle com rajadas de
  `stand_look_sit`; `working` comendo; `thinking` Sit Idle; `waiting`,
  `alert`, `wave`, `done_small`, `giggle`, `hello` e `bye` piando; `ready` em
  pé; `error` susto; `yawn`, `sleep` e `wake`; `dangle` voando; `land`
  pousando. `chao` lista as tags com os pés no chão, para o lint.
- **Tamanho:** `corpo_px` = 19, a altura da pose parada com chapéu → D = 8 no
  eDP-1 e 13 no 4K; `toque` = a caixa dessa pose; pés em (24, 32).
- **Duas variantes:** `zeca` e `zeca-contorno` (1 pixel de arte creme
  `#F7E7C5` por fora, nos 8 vizinhos), ambas em `skins-locais/` e com o mesmo
  toque e D. A escolha é `aparencia.skin`.
**Por quê:** ids sem espaço servem de chave estável; a sequência inteira numa
tag só funciona com os dois animadores sem mudar o core; e o contorno vira
uma escolha de config, não um rebuild.

## 0026 — Aprovação do personagem pela impressão digital, com cópia em /state (2026-10-03)

**Problema:** o PLANO exige que o Zeca só vire personagem depois de o Renan
aprovar a folha de contato e a demonstração ao vivo, que um snapshot
aprovado sirva de reserva se a skin quebrar e que, sem aprovação, o pet fique
escondido (nunca com a skin de teste). E a skin é reconstruída (arte nova,
pack reinstalado): a imagem pode passar a ter algo que o Renan não viu.
**Escolha:**
- **Impressão digital:** o sha256 da saída do `sha256sum` dos três arquivos,
  na ordem `skin.json`, dados, folha. No host é `sha256sum skin.json
  sheet.json sheet.png | sha256sum`.
- **`bin/pet skin-aprovar [id]`** (padrão: a skin configurada) calcula a
  impressão dos arquivos do host, os mesmos da folha de contato, e manda
  `POST /v1/comando {"cmd": "aprovar_skin", "arg": {"id": …, "sha256": …}}`
  (o formato do `/v1/comando` do M3). O daemon recusa se:
  - a skin da imagem tiver outra impressão (409: falta `bin/pet subir`);
  - não carregar (422);
  - for a `_teste` (403).
  Senão, grava `/state/skins/<id>/` com a cópia dos três arquivos e o
  `aprovacao.json` (id, sha256, hora), trocando a pasta inteira de uma vez. O
  laço principal escolhe o personagem de novo e a resposta espera isso (até
  2 s), com o personagem que ficou na tela.
- **Quem aparece:**
  1. a skin da imagem, se a impressão dela é a aprovada;
  2. senão, a cópia de `/state`, se a da imagem mudou depois da aprovação,
     sumiu ou não carrega (com o motivo em `/v1/estado.skin.avisos`);
  3. senão, ninguém (`tela: sem_personagem`).
  `/v1/estado.skin` mostra `origem` (`imagem` ou `snapshot`) e `sha256`.
- **Debug:** continua com a `_teste`. `PET_DEBUG_PERSONAGEM=1` (repassada
  pelo compose de dev) troca pelo personagem aprovado, com as mesmas regras
  e com as rotas de debug, para conferir o Zeca na tela (nitidez, foto
  mascarada).
- **`bin/pet skin-revogar [id]`** manda `{"cmd": "revogar_skin", "arg":
  "<id>"}`, que apaga a aprovação e a cópia: o pet some na hora.
- **Reaprovar depois de reconstruir a skin:** depois de `bin/pet
  skin-instalar` (ou de mexer em `arte/zeca/`) e de `bin/pet subir`, a
  imagem tem uma impressão nova, que não é a aprovada. O pet continua com a
  cópia aprovada antiga e avisa que a skin "mudou depois da aprovação". O
  Renan olha a folha de contato nova e roda `bin/pet skin-aprovar` de novo;
  a cópia em `/state` vira a nova. Sem a nova aprovação, nada muda na tela.
- **A aprovação é por id:** aprovar `zeca-contorno` não muda o personagem;
  quem escolhe é `aparencia.skin`, e o `bin/pet` avisa quando os dois
  divergem.

**Por quê:** aprova-se o que foi visto, não um nome. A cópia em `/state`
segura o Zeca quando a imagem muda ou quebra, e o caminho por HTTP no
loopback, com as checagens de Host e `X-Pet`, evita escrever no volume
Docker a partir do host.

## 0027 — O Zeca na tela: verificação ao vivo do M2 e quadros iguais sem commit (2026-10-03)

**Problema:** o M2 só fecha com a nitidez do M1 passando com o Zeca na tela
de verdade. E a folha do Zeca guarda cada pose repetida uma vez só (quadros
iguais apontam para a mesma célula, decisão 0025), o que expôs commits de
quadros idênticos: o animador comparava pelo índice do quadro.
**Escolha:**
- **Verificação ao vivo com o personagem:** `scripts/verificar-ao-vivo.sh
  --personagem` sobe a pilha de dev com `PET_DEBUG_PERSONAGEM=1` (o Zeca
  aprovado só para o teste, decisão 0026). Com a tela acesa e desbloqueada,
  tudo passou:
  - nitidez: 12 672 pixels opacos, nenhum fora de ±2 (maior desvio 0), 198
    blocos 8×8 uniformes;
  - sem pixel velho (67 968 pixels conferidos) e esconder sem fantasma
    (80 640);
  - parado: 1,50 commit/s (orçamento 2);
  - container: imagem de 4,77 MB, RSS de 11,9 MiB, CPU de 0,04%;
  - restart em 310 ms, no mesmo lugar; `kill -9` de volta em 261 ms;
  - região de input de 102×102 lógicos, só no corpo; D = 8 no eDP-1,
    célula em (1632, 920).
- **Produção:** aprovar faz o Zeca aparecer na hora no canto inferior direito
  do eDP-1 (camada no nível 3, `tela: ativa`); revogar o esconde na hora,
  sem fantasma no canto. As fotos mascaradas ficaram em `tmp/fotos/`: têm
  pixels do pack e nunca vão para o git nem para `docs/`.
- **A sessão bloqueia sozinha:** no meio da sessão o lock do Omarchy
  (quickshell) apagou e cobriu a tela. Bloqueado, o Hyprland não desenha
  camadas nem com a tela acesa. Os scripts ao vivo passaram a detectar isso
  (`LOCK` em `solitaryBlockedBy`) e dão NÃO VERIFICADO em vez de comparar a
  tela de senha. A verificação rodou depois que a sessão foi desbloqueada.
- **Quadros iguais não fazem commit:** o `Skin` ganha `canonico`, o primeiro
  quadro com o mesmo retângulo da folha no mesmo lugar da célula. O
  animador toca e conta trocas por ele: um passo que não muda a imagem não
  conta no orçamento nem gera commit. O Zeca parado foi de 1,52 commit/s
  calculado (1,50 medido) para 1,26 calculado e 1,20 medido ao vivo. A
  `_teste`, sem quadros repetidos, não muda (0,76).
- **Fim do M2 nesta máquina:** o Zeca está na imagem local, a aprovação de
  teste foi revogada e a produção ficou de pé com `tela: sem_personagem`. Quem
  aprova é o Renan, vendo a folha de contato (`bin/pet skin-aprovar zeca`).

**Por quê:** a nitidez depende só do caminho D×D, mas o portão pede a prova
com o personagem de verdade. Cada commit repinta o monitor (decisão 0005),
então pose repetida não pode custar repintura.

## 0028 — Revisão da arte do Zeca: gravata dentro do contorno, chapéu que gira no centro e cai depois do pouso (2026-10-03)

**Problema:** três revisões do M2 acharam, e a máquina confirmou, defeitos
na arte e no encaixe:
- a gravata pintava rosa por cima do contorno de 1 pixel do pack em 27 das
  70 células (16 tocadas por estados); no pack, toda cor que não é o branco
  fica cercada pela tinta, e no comendo/decolando a gravata emendava no
  contorno do bico (a "gravata que vira bico" da decisão 0023);
- uma correção de `ancoras.json` (a gravata do `dive_start[0]`) sumia calada,
  e índices como «01» passavam sem casar com quadro nenhum;
- o chapéu solto era posto pelo canto: trocando entre variantes 11x5 e 5x11
  o centro pulava 3 a 6 pixels (no laço do mergulho, um tremor de 10 Hz);
- no mergulho o chapéu voltava para a cabeça ainda no ar (`dive_end` com
  680 ms) e o pouso o amassava de novo, quando o pedido foi "cai de volta na
  cabeça no pouso";
- o Zeca parado fazia 1,20 commit/s medido (1,28 calculado): pela medida do
  M1 (+0,58 ponto de CPU do Hyprland a 0,70 commit/s, decisão 0005), a conta
  linear dá ~+1 ponto, no limite do orçamento.

**Escolha:**
- **Gravata no miolo do corpo:** ela só pinta pixel opaco com os 4 vizinhos
  opacos; pixel que cairia no contorno da silhueta ou no ar não é desenhado
  (o contorno do pack ganha) e o `cargo xtask zeca` avisa. As posições foram
  acertadas em `ancoras.json`: respiração do sentado e do sono 1 pixel à
  esquerda (a opção de subir 1 pixel encostava no bico), comendo/decolando
  1 pixel à esquerda, e no voo a gravata vai para o pescoço (canto em
  olho.x − 4, olho.y + 4), inteira dentro do contorno e longe do bico. O chapéu assentado aprovado não
  mudou; a ponta de palha da aba, sem tinta em cima, é do desenho aprovado e
  é a única abertura permitida.
- **Checagens novas no `zeca`:** cor de acessório encostada no transparente
  fora das aberturas do próprio desenho; chapéu assentado afundando no corpo
  ou sem encostar nele; chapéu solto a menos de 4 pixels do corpo ou 2 da
  borda; o centro do chapéu solto pulando mais de 3 pixels ou mudando de
  sentido mais de uma vez; o contorno creme juntando manchas soltas. Com
  `--estrito` (o `bin/pet skin-instalar` usa), aviso que não está em
  `arte/zeca/avisos-aceitos.txt` reprova antes de gravar. Aceito hoje, com o
  porquê no arquivo, só um: no `zeca-contorno`, o risco branco da mordida do
  pack (tag `bite`, que nenhum estado toca) emenda no bico.
- **Dado errado é erro:** num quadro em que a tag escondeu a peça, `x`/`y`
  criam de novo (com a variante da regra); deslocar o que não existe, índice
  fora da tag ou não canônico, `oculto` junto com posição, chapéu solto com
  canto e centro juntos, ou `#000000` nas cores nossas param o `zeca`. A
  conferência de paleta tira o índice transparente do `.aseprite` (no pack,
  `#000000`) e o lint avisa preto puro. Tags do pack com outra direção são
  expandidas na ordem certa.
- **Chapéu solto pelo centro** (`cx`, `cy` em `chapeu_voando.json`): a
  cambalhota gira em volta de um ponto fixo. Susto: sobe no clarão, gira
  parado em cima da cabeça e cai quando ele se endireita. Susto em voo: um
  arco (12 → 7 → 10). Mergulho: o chapéu fica para trás, gira num centro fixo
  durante o laço, desce enquanto ele sai do mergulho, e só cai na cabeça
  **depois do pouso** — tag nova `landing_mergulho` (corpos do `landing`, no
  fim do `big_flight`): ele pousa, se levanta, o chapéu cai em cheio
  (amassado) e desamassa. `dive_end` voltou aos 400 ms do pack. O pouso comum
  (`land`, voo curto) continua com o tranco de um amassado só.
- **Parado mais calmo:** `idle = sit_idle, sit_idle, sit_idle,
  stand_look_sit` — três respiradas e uma levantada a cada ~18 s, 0,88
  commit/s calculado, perto do 0,70–0,80 medido no M1; o `repouso.gif` mostra
  20 s, um ciclo inteiro.
- **Para o M6:** o chapéu solto mora em coordenadas da célula. Se o M6 mover
  a célula pela tela durante as tags do mergulho (o T3 atravessando a tela),
  o chapéu vai junto com o Zeca e a piada some: ou a célula fica parada na
  vertical enquanto `dive_start`/`dive_loop`/`dive_end`/`landing_mergulho`
  tocam, ou o chapéu vira uma trilha separada (variante e posição por quadro
  nos dados da skin) que o M6 põe na tela.

**Por quê:** a gravata cercada pela tinta é a regra de estilo do próprio
pack, e o contorno da silhueta ganhando garante isso mesmo com dado errado.
Girar em volta do centro é o que uma cambalhota faz. O chapéu caindo depois
do pouso é o pedido do Renan ("cai de volta na cabeça no pouso") e a piada
fica melhor com um instante de espera. O parado mais calmo cabe no custo
medido do M1 sem depender de uma conta linear.

## 0029 — Aprovação amarrada à folha de contato, config relida e troca com o pet na tela (2026-10-03)

**Problema:** a revisão do M2 achou, e o código confirmou:
- **pet congelado:** aprovar ou revogar com o pet na tela (inclusive a
  reaprovação da decisão 0026, cópia → imagem) apagava o palco e o
  `resolver` da camada não o refazia, porque só avisa quando monitor ou escala
  mudam: o último quadro da skin velha ficava parado na tela, com
  `/v1/estado.d` nulo;
- **`zeca-contorno` inalcançável:** o daemon lia `aparencia.skin` só na
  partida, e o `bin/pet subir` não recria o container quando só o config
  (montado de fora) muda;
- **aprovação sem a folha:** o `skin-aprovar` conferia host × imagem, mas nada
  ligava a skin à folha de contato que o Renan olhou;
- **impressão instável:** o `sheet.json` levava a versão do pet, e subir a
  versão pediria aprovar de novo a mesma arte;
- o `skin-instalar` descompactava com `unzip` em `/tmp` (sem teto) e não
  usava o leitor de zip conferido do xtask; prévias e folha podiam ser
  gravadas em pasta do git (`docs/`, `arte/`); o `verificar-ao-vivo.sh`
  deixava a aprovação de teste se fosse interrompido e, depois da aprovação de
  verdade, reprovaria a produção por não estar escondida.

**Escolha:**
- **Troca com o pet na tela:** com a camada pronta, o palco (D e posição) é
  refeito na hora e o quadro novo vai com a tela toda; uma escolha igual à da
  tela (mesmo id, impressão e origem: aprovar de novo, revogar outro id) não
  mexe em nada. Conferido com um daemon nativo ligado ao Hyprland de verdade
  (porta e `/state` de rascunho): aprovar, aprovar de novo, trocar para o
  `zeca-contorno` com o pet na tela (D = 8, célula refeita) e revogar.
- **Config relida a cada aprovação ou revogação:** para usar o contorno,
  `aparencia.skin = "zeca-contorno"` em `config/claude-pet.toml` e `bin/pet
  skin-aprovar zeca-contorno`, sem reiniciar.
- **Aprova-se o que foi visto:** o `cargo xtask contato` põe a impressão
  digital no título da folha e no `contato-<id>.sha256`; o `bin/pet
  skin-aprovar` recusa se a folha de `tmp/previa-zeca-m2/` não existe ou é de
  outra versão da skin. Depois de reconstruir a skin: `bin/pet skin-instalar`
  (gera as prévias de novo), olhar, `bin/pet subir`, `bin/pet skin-aprovar`.
- **`sheet.json` sem a versão do pet.**
- **`skin-instalar`** passa o zip direto ao `cargo xtask zeca` (lido em
  memória: CRC, teto de 64 MiB, sem extrair nada), monta com `--estrito` e
  exige os estados do MVP nativos (`cargo xtask cobertura --nativos mvp`, a
  tabela do PLANO, item 4: só com `idle` tudo caía em reserva e o portão nunca
  reprovava).
- **Arte de pack só fora do git:** importador, `zeca` e `contato` de uma skin
  não redistribuível só gravam, dentro do repo, em `skins-locais/` e `tmp/`.
  Foto do Zeca (`bin/pet foto`, verificação ao vivo) fica em `tmp/`.
- **Scripts ao vivo:** `verificar-ao-vivo.sh --personagem` e
  `medir-custo.sh --personagem` aprovam só para o teste quando falta aprovação
  e revogam no fim, até numa falha; o modo normal lê do volume se o Renan já
  aprovou e espera a produção de acordo. O `--personagem` também aprova de
  novo com o pet na tela e confere que ele continua desenhando.
- A GIF de prévia mostra o fundo escuro e o claro lado a lado, para escolher
  o contorno creme vendo os dois temas em movimento.

**Por quê:** a aprovação é o portão humano do personagem (decisão 0026): ela
precisa valer para a arte da folha vista, poder trocar o personagem sem
congelar a tela nem reiniciar, e nenhum teste pode terminar com o Zeca
aprovado no lugar do Renan.

## 0030 — Integração do M3 sobre o M2: um `/v1/comando`, reações pelos estados da skin e o config relido também no cérebro (2026-10-03)

**Problema:** o M3 (decisões 0019–0022) foi feito de madrugada, a partir da
`m1-overlay` de antes do portão do M1, e o M2 (0023–0029) à tarde, em cima
do portão fechado. Rebaseada a `m3-hooks` na `m2-zeca`, as duas metades se
encontram em quatro pontos:
- os dois escreveram um `POST /v1/comando` no mesmo formato `{"cmd",
  "arg"}`: o M3 com `tocar`, `esconder` e `mostrar` (204, sem esperar), o M2
  com `aprovar_skin` e `revogar_skin` (200, uma por vez, esperando o laço
  escolher o personagem de novo);
- o animador do M3 tinha a própria tabela de reservas (`nod` e `done_small`
  → `wave`), repetida no catálogo de estados do M2 (`pet_core::estados`),
  que a cobertura usa: duas fontes para a mesma regra;
- o `nod`, o aceno do T0 e a reação mais frequente, não estava entre os
  estados que o personagem tem de ter nativos: sem ele, o aceno cairia no
  `wave`, que no Zeca é o mesmo pio do pulinho do T1;
- o M2 relê o config a cada aprovação (decisão 0029), mas o cérebro ficava
  com as origens e o modo da partida, e o `/v1/estado.config` passaria a
  mostrar outra coisa que o `cerebro.origens`.

**Escolha:**
- **Um `/v1/comando` só**, com as mesmas checagens de `Host`, `X-Pet` e
  `Content-Type` e o mesmo corpo `{"cmd", "arg"}` sem campo a mais:
  `tocar`, `esconder` e `mostrar` vão ao laço e respondem 204 na hora;
  `aprovar_skin` e `revogar_skin` respondem 200 com o resultado depois de o
  laço escolher o personagem. Só as aprovações passam pelo cadeado: uma
  reação nunca espera uma aprovação. Um `cmd` desconhecido lista os cinco.
  O `bin/pet` fala com o `/v1/comando` por uma função só.
- **Um laço só** recebe pelo mesmo canal (256 vagas) eventos, reações,
  aprovações e debug; o `/v1/estado` traz o `skin` do M2 (`id`, `origem`,
  `sha256`, avisos) e o `sessoes`, `ultima_reacao`, `turnos`, `cerebro` e
  `eventos` do M3. O `Laco::novo` recebe a config da partida, de onde saem
  a skin configurada e a config do cérebro (com os argumentos dos dois
  marcos ele passava do limite do clippy; corrigido no próprio commit do
  T3.3 rebaseado).
- **Reações pelos estados do `skin.json`:** toca a primeira tag do estado de
  mesmo nome; senão a do primeiro estado de reserva que a skin tem, pelas
  reservas do catálogo (`estados::reserva`, a mesma caminhada da
  cobertura), nunca a pose parada (tocar o repouso não é reação); senão uma
  tag de mesmo nome. A tabela própria do animador saiu.
  - No Zeca: `nod` → a tag composta `nod` (levanta e senta, 600 ms);
    `done_small` → `chirp`; `bye` → `chirp`. O tchau, que na `_teste` não
    anima, no Zeca pia.
  - Na `_teste` (debug), como antes: `nod` → `wave`, `done_small` →
    `done_small`, `bye` não anima.
- **`nod` entra nos nativos do MVP** (`cargo xtask cobertura --nativos mvp`,
  que o `skin-instalar` exige): o Zeca passa com 18 de 18; um personagem sem
  aceno próprio não passa. A tabela do PLANO ("Zeca: arte e skin", item 4)
  ganha a linha do T0.
- **O config relido vale para o cérebro:** a cada aprovação ou revogação,
  `sessoes.origens` e `celebracao.modo` do arquivo novo passam ao cérebro
  (`Cerebro::reconfigurar`); uma sessão de origem que deixou de contar sai
  na hora, sem reação (os eventos dela seriam ignorados e ela só sumiria em
  12 h).
- **Rebase:** cada conflito foi resolvido mantendo os dois lados (lista na
  linha do T3.5 no PROGRESS). DECISIONS em ordem numérica (0019–0022 antes
  de 0023); PROGRESS em ordem cronológica: as linhas do M3, de madrugada,
  antes do portão do M1 (de manhã) e do M2 (à tarde). Todo commit
  rebaseado passa `cargo fmt --check`, `clippy -D warnings` e `cargo test`.

Ao vivo, com a produção refeita da `m3-hooks` (verificação pelo
`/v1/estado`, sem olhar pixels):
- `bin/pet skin-instalar` com o zip do pack deu as mesmas impressões das
  prévias do M2 (`zeca` 5b843b03…, `zeca-contorno` a0d5fbb1…), com os três
  arquivos, o `CREDITS.md`, as folhas de contato e os GIFs idênticos byte a
  byte; só o `cobertura.md` mudou (18 de 18 exigidos, era 17 de 17);
- o `zeca` estava aprovado: uma aprovação feita fora desta sessão em
  2026-10-03 às 23:28 UTC, depois do fim do M2, com a impressão da folha.
  Ela ficou como estava; com a mesma impressão na imagem nova, a produção
  mostra o Zeca (`tela: ativa`, D = 8) e as reações tocam nele;
- `bin/pet testar rapido` → `nod` (T0) e `pequeno` → `done_small` (T1,
  trabalho 2, 940 ms de ferramenta), isolados como teste; `bin/pet tocar`
  `nod`, `done_small` e `bye` aparecem em `/v1/estado.reacao`;
- gate interativo no tmux em `~/Documents`, com `--plugin-dir` e sem as
  variáveis `CLAUDECODE`/`CLAUDE_*` do agente: "responda só: ok" → `nod`
  (T0); "crie o arquivo …" → um Bash de 66 ms → `done_small` (T1); `/exit`
  → `bye`; as sessões de teste expiraram sozinhas;
- pet parado: `claude -p --plugin-dir …` imprimiu só `ok`, stderr vazio,
  saída 0; o `avisar.sh` levou 12 ms com a porta recusando; pet de pé: um
  `claude -p` mandou 4 eventos `sdk-cli`, todos ignorados;
- `claude plugin list`, `claude plugin marketplace list` e os arquivos de
  configuração do Claude Code terminaram iguais aos de antes.

**Por quê:** o formato `{"cmd", "arg"}` já era o mesmo dos dois lados, então
um endpoint só, com uma tabela de comandos, é menos superfície do que dois
caminhos; aprovações são lentas e raras e não podem segurar uma reação. Uma
fonte só para as reservas faz o `cobertura.md` dizer o que o pet toca. Com o
`nod` nativo, T0 e T1 continuam diferentes na tela. E o config relido tem de
valer para tudo o que o `/v1/estado.config` mostra.

## 0031 — Hooks só para o 127.0.0.1: sem curlrc, proxy nem `~/.jq`, e validadores iguais aos do pet (2026-10-03)

**Problema:** a revisão adversarial da integração (lente de hooks e
privacidade) achou o caminho que a lista branca não cobre: **para onde** os
metadados vão.
- O hook herda o ambiente do Claude Code (perfil do shell, bloco `env` do
  `settings.json`). Com `http_proxy`/`ALL_PROXY` no ambiente, ou um
  `~/.curlrc` com `proxy`, o curl do `avisar.sh` entregava cada evento ao
  proxy, que pode ser outra máquina: ids de sessão e de turno, nomes de
  ferramenta, hash de arquivo, nome da pasta do projeto, DND. E o pet
  ficava surdo sem erro nenhum. Os canários não viam: usam um curl falso e
  limpam o ambiente. Nada disso está ativo nesta máquina hoje (nenhuma
  variável de proxy nos processos do Claude, nenhum `~/.curlrc`, `env`
  vazio no `settings.json`).
- Achado nesta revisão: o jq lê sozinho o `~/.jq` antes do programa, e um
  `~/.jq` que redefine `test` ou `select` abria a lista branca (conferido no
  jq 1.8.2).
- Os validadores do jq não eram os do `pet_core::evento`, como o script
  dizia: o `$` do jq aceita um "\n" no fim do texto (id, enum, origem e
  pasta passavam com ele e o pet descartava), e a pasta aceitava todas as
  marcas Unicode (`\p{M}`), o pet só cinco faixas (uma pasta "1️⃣" saía e
  era descartada).
- `bgt` normalizava qualquer texto para `[a-z_]`: um campo de conteúdo que
  um dia passasse por ali sairia em minúsculas, e os canários só procuravam
  `SEGREDO` em maiúsculas.

**Escolha:**
- O curl do `avisar.sh`, do `bin/pet` e dos scripts de host sempre com `-q`
  (o primeiro argumento: nenhum curlrc) e `--noproxy '*'` (nenhum proxy do
  ambiente). O jq do `avisar.sh` roda com `HOME=/nonexistent` (sem
  `~/.jq`).
- Validadores com `\A…\z`; a pasta aceita letras, dígitos, espaço, `_.-` e
  só as marcas combinantes que o pet aceita. O que o pet descartaria nem
  sai do host.
- `bgt` por lista fechada, os rótulos do 2.1.288 normalizados (`shell`,
  `subagent`, `workflow`, `monitor`, `mcp_task`, `teammate`, `dream`,
  `auto_mode_scan`, `memory_import`, `cloud_session`); qualquer outro vira
  `outro`, nunca o texto dele.
- Canários (20, eram 16): todo vazamento é procurado sem caixa
  (`segredo`); novos: o curl de verdade com proxy em todas as variáveis e
  curlrc desviando (`proxy` e `connect-to`) em `CURL_HOME`,
  `XDG_CONFIG_HOME` e `HOME` ainda entrega ao daemon; um `~/.jq` que
  redefine `test`, `select` e `with_entries` não muda nada; ids, enums,
  origem e pasta com "\n" no fim e a pasta "1️⃣" saem sem nada que o pet
  descartaria; uma entrada de 8 MiB sai calada com 0. O `bin/pet testar`
  também passa com proxy e curlrc. Cada canário novo reprovou o script
  antigo antes da correção.

**Por quê:** a lista branca escolhe o que sai; isto garante que só sai para
o 127.0.0.1. Com o pet desligado continua custando uma conexão recusada. Um
tipo de tarefa desconhecido vale para o M5 o mesmo que o texto dele (quem
decide se é agente é a lista), e a lista não deixa conteúdo passar.

## 0032 — Revisão do cérebro: Stop depois do prompt seguinte, continuação de um Stop segurado e estado só por evento aplicado (2026-10-03)

**Problema:** a revisão adversarial da integração (lente do cérebro)
reproduziu num daemon de debug duas ordens reais de hooks async que o
cérebro errava. Para uma delas, a decisão 0020 dizia o contrário.
- **Stop depois do prompt seguinte.** Um prompt na fila, ou o aviso de uma
  tarefa em segundo plano entregue quando a sessão para, entra milissegundos
  depois do Stop, e os dois hooks chegam trocados (do hook ao pet são 25 a
  82 ms, decisão 0021). O `UserPromptSubmit` do p2 fechava o p1 na hora como
  `substituido`, sem festa, e o Stop do p1 caía como `stop_repetido`.
- **Stop segurado por outro plugin** (`stop_hook_active`; o ralph-loop, o
  hookify e outros do marketplace oficial ligam Stop hooks). A 0020 dizia
  que a acomodação de 0,8 s cobria esse caso, e não cobre. O PreToolUse só
  está ligado para AskUserQuestion e ExitPlanMode, então o primeiro evento
  da continuação é o PostToolUse da ferramenta, segundos depois. O primeiro
  Stop comemorava em 0,8 s, as ferramentas da continuação caíam como
  `turno_fechado` e o Stop final (`sha`) como `stop_repetido`. Um T0 que
  virava T1 nunca pulava, e o registro ficava errado. Nesta máquina nada
  segura o Stop hoje: dos plugins instalados, só o superpowers tem hooks, e
  só o SessionStart.
- Um evento ignorado mudava o estado da sessão: um prompt ou uma ferramenta
  atrasados deixavam a sessão `pensando` ou `trabalhando` sem turno aberto
  até o próximo evento, ou por 12 h.
- `Turno.arquivos` crescia sem teto com `arq` sempre novo. A 0022 dizia que
  `ignorados` era o último mapa que crescia com dado de fora, e não era.
- O `receber` vencia os prazos na hora do processamento. Com o laço parado
  (handshake do Wayland, troca de personagem), uma acomodação podia vencer
  antes do evento da fila que a cancelaria.

**Escolha:**
- **Turno trocado.** Um turno novo que começa sem o Stop do aberto não fecha
  o anterior na hora: o anterior espera 0,8 s (`ACOMODACAO_MS`). Se o Stop
  dele chega, comemora na hora, porque o turno novo já começou. Eventos
  atrasados dele contam nele, sem fechar o turno novo. Sem Stop, fecha
  `substituido` com a hora da troca, como antes. `StopFailure` e ferramenta
  interrompida fecham o turno do próprio `prompt_id`; antes fechavam o
  aberto, qualquer que fosse.
- **Turno comemorado.** O último turno fechado por um Stop guarda os
  contadores. Um evento de trabalho da thread principal com o mesmo
  `prompt_id` e `ts` depois do Stop reabre o turno (`continuacoes` + 1). O
  próximo Stop acomoda de novo e reclassifica, e só reage se o nível subir
  (T0 → T1: `done_small`). O turno tem um registro só, trocado pelo novo
  (com `sha`), e `reacao` passa a ser a última reação do turno. Um prompt
  novo encerra essa chance. É a regra "Stop com sha" do plano, no tamanho
  do M3; o M5 funde nos níveis T2 e T3.
- **Estado só por evento aplicado.** Evento ignorado (`turno_fechado`,
  `prompt_repetido`, `stop_repetido`), atrasado (antes do Stop pendente) ou
  do turno trocado não mexe no estado da sessão.
- **Teto de 1024 arquivos lembrados por turno.** Passado o teto, cada `arq`
  novo só soma, contando por cima. Os contadores somam saturando.
- **Relógio da chegada.** O ingress carimba também o `Instant` da chegada,
  e o laço passa ao cérebro a hora em que cada evento chegou. O calloop
  0.14 despacha o canal antes dos timers vencidos, então um laço parado não
  vence uma acomodação que um evento da fila cancela.
- **Testes:** 7 cenários novos na tabela (Stop depois do prompt seguinte,
  Stop do trocado fora da espera, eventos atrasados do trocado,
  stop-bloqueado subindo e sem subir de nível, evento atrasado depois da
  festa), 3 testes de registro e estado, um do teto de arquivos, um do
  relógio da chegada e um de ponta a ponta no daemon com as duas ordens. A
  tabela nova reprova o cérebro antigo.

**Por quê:** o turno é o `prompt_id` ("até o próximo prompt", diz o d.ts), e
o Stop e o prompt seguinte são dois hooks async correndo um contra o outro.
Esperar 0,8 s pelo Stop do trocado não custa nada: o Esc continua sem festa,
só 0,8 s depois. Reabrir em vez de ignorar mantém o registro certo para o M5
e põe o pulinho no fim de verdade; reagir só se o nível subir evita festa
dupla. Isto corrige a 0020: a acomodação cobre hooks fora de ordem dentro de
0,8 s, e o Stop segurado é coberto pela reabertura. Corrige também a 0022: o
teto dos arquivos fecha o último mapa do cérebro que crescia com dado de
fora.

## 0033 — O `tocar` responde o que o pet fez, e reação nunca espera o cadeado das aprovações (2026-10-03)

**Problema:** a revisão adversarial da integração (lentes do cérebro e do
processo) achou duas pontas soltas no `/v1/comando` unificado da 0030:
- O comando respondia de dois jeitos. As aprovações davam 200 com o
  resultado, depois de o laço trocar o personagem. O `tocar` dava 204
  sempre, com o nome só validado por `[a-z_]`. Sem personagem aprovado, sem
  compositor ou com um nome que a skin não sabe tocar, o `bin/pet tocar`
  saía 0 calado, e só o log do daemon avisava.
- A 0030 promete que só as aprovações passam pelo cadeado ("uma reação
  nunca espera uma aprovação"), mas nenhum teste segurava isso. O
  `receber_comando` do M2 pegava o cadeado no começo para qualquer comando,
  e voltar a esse formato passaria na suíte inteira.

**Escolha:**
- O `tocar` vai ao laço e espera a resposta por até 2 s (o mesmo prazo das
  aprovações e do quadro de debug), sem passar pelo cadeado. Respostas:
  - 200 com `{"reacao", "tocou", "tag", "motivo"}`: `tocou` diz se a
    reação apareceu na tela, e `motivo` diz por que não (sem compositor, ou
    o pet escondido; escondido, não toca);
  - 409 sem personagem aprovado;
  - 400 se a skin não tem estado, reserva nem tag com esse nome.
- `esconder` e `mostrar` continuam 204 na hora.
- O `bin/pet tocar` mostra a tag que a skin tocou e sai 1, com o motivo,
  quando a reação não aparece.
- Um teste prende o cadeado: `tocar`, `esconder` e `mostrar` respondem na
  hora, e a revogação espera até ele soltar. Com o cadeado posto no `tocar`
  (mutação), o teste reprova.

**Por quê:** um comando de depuração que sai 0 sem ter feito nada esconde
justamente o que se queria ver: o personagem sem aprovação, o compositor
fora. Esperar o laço custa milissegundos. O cadeado continua só onde há
disco e troca de personagem.

## 0034 — Revisão do gate do M3: `bin/pet testar` pelo registro do turno, plano sem instalação global, aprovação fora do CLAUDE.md e gate refeito com Write (2026-10-03)

**Problema:** a revisão adversarial da integração (lentes do processo e do
cérebro) achou:
- o `bin/pet testar` lia o resultado da `ultima_reacao`, que é de quem
  reagiu por último. Uma sessão real reagindo entre a reação do teste e a
  leitura seguinte (a cada 100 ms) fazia o teste esperar 5 s e falhar, com
  a reação feita;
- a verificação do M3 no PLANO ainda pedia `claude plugin list` com
  `bichinho@bichinho-local` habilitado e rodava o gate sem `--plugin-dir`,
  contra a 0021 e o CLAUDE.md: instalar antes do merge faria as sessões de
  outros projetos rodarem a branch;
- o CLAUDE.md versionado guardava estado desta máquina: a aprovação do
  `zeca` de 23:28 UTC "tratada como do Renan" e um "não a revogue". Estado
  de máquina envelhece num arquivo que vai para a `main`. E a origem, que o
  relatório da integração deixou como não confirmada, estava confirmada: o
  Renan respondeu "Aprovo, sem contorno" na sessão que orquestra os marcos
  às 23:27:37 UTC, e essa sessão rodou `bin/pet skin-aprovar zeca` às
  23:28:02;
- o gate refeito na integração (0030) não teve turno com Write ou Edit, e
  nada foi conferido na tela, embora a 0030 dissesse que as reações "tocam
  nele";
- um daemon de debug do M2 (o do T2.6, nativo, na porta 27399, conectado ao
  Hyprland) ficou rodando desde as 17:46.

**Escolha:**
- O `bin/pet testar` acha o turno da própria sessão de teste no
  `/v1/estado.turnos` (por `sid8` e `teste`) e lê a reação dele. O teste
  novo tem sessões reais reagindo a cada 50 ms; o `bin/pet` antigo
  reprovou 3 de 3 vezes.
- A verificação do M3 no PLANO segue a 0021: `--plugin-dir`, com
  `CLAUDECODE` e as `CLAUDE_CODE_*` fora do ambiente; `claude plugin list` e
  `claude plugin marketplace list` iguais antes e depois; um turno com Write
  conferido por `arquivos: 1`. A instalação e o `bichinho@bichinho-local`
  habilitado ficam para depois do merge (README).
- O CLAUDE.md perde o parágrafo da máquina e ganha a regra que dura: nunca
  revogue uma aprovação que você não fez; antes de mexer, leia o `skin` do
  `/v1/estado` e o `aprovacao.json` em `/state`.
- O daemon esquecido foi parado pelo PID, sem `pkill -f`.

Ao vivo. A tela ficou apagada e a sessão bloqueada o tempo todo: nada foi
conferido por pixel, só pelo `/v1/estado` e pelo log.
- Produção refeita da `m3-hooks` (`bin/pet subir`): tela ativa, `zeca` da
  imagem com a aprovação do Renan (sha 5b843b03…), D = 8. A aprovação não
  foi tocada.
- `bin/pet testar rapido` → `nod` (T0); `pequeno` → `done_small` (T1,
  trabalho 2, arquivos 1, 940 ms).
- `bin/pet tocar`: `nod` → tag `nod`, `done_small` → `chirp`, `bye` →
  `chirp`; `nada_disso` → 400. Com a tela apagada, `tocou` só diz que a
  reação tocou na camada, e o `commits_total` fica em 1. O pet não sabe se
  a tela está acesa, porque nunca pergunta ao Hyprland, e sem frame
  callback não faz commit (decisão 0018).
- Gate interativo no tmux, em `~/Documents`, com `--plugin-dir`:
  - "responda só: ok" → `nod` (T0);
  - "use a ferramenta Write para criar …/tmp/e2e/oi.txt" → um Write de 22
    ms → `done_small` (T1, trabalho 1, `arquivos: 1`);
  - `/exit` → `bye`.
  Foram 19 eventos aceitos e nenhum recusado. A primeira tentativa mandou o
  texto e o `Enter` no mesmo `tmux send-keys`: o Claude Code tratou como
  colagem e não enviou (a sessão saiu com `bye` quando o tmux foi
  derrubado). O texto vai com `-l`, e o `Enter` num `send-keys` à parte.
- Com o pet parado, `claude -p --plugin-dir …` imprimiu só `ok`, com stderr
  vazio e saída 0, e o `avisar.sh` levou 14 ms. Com o pet de pé, um
  `claude -p` mandou 4 eventos, todos ignorados como `origem:sdk-cli`.
- Log e `/v1/estado` sem nada dos prompts nem caminhos (0 ocorrências).
  `claude plugin list`, `claude plugin marketplace list`, `settings.json`,
  `installed_plugins.json` e `known_marketplaces.json` iguais antes e
  depois.

Fica pendente, porque pede o Renan: no Zeca, o aceno do T0 (`nod`, levanta
e senta, 600 ms) é um pedaço da rajada do repouso (`stand_look_sit`,
levanta, respira em pé e senta), que o parado toca sozinho a cada ~18 s.
Quem olha pode confundir os dois. Mudar o `nod` ou o repouso muda o
`skin.json`, a impressão digital e a aprovação: é uma folha de contato nova
e uma reaprovação dele, e depois a conferência na tela.

**Por quê:** o teste tem de achar o próprio resultado, não o do vizinho. O
plano não pode mandar quebrar uma regra de ouro. Estado de máquina vai para
o PROGRESS ou o corpo do PR; regra vai para o CLAUDE.md.

## 0035 — O personagem é o Zeca, nunca associado a personagem de terceiros (2026-10-03)

**Problema:** a pesquisa multiplataforma (`docs/pesquisa/09-multiplataforma.md`,
achado 4) mostrou que textos do DECISIONS e do PLANO citavam um personagem de
terceiros como referência de estilo. Papagaio verde, chapéu-palheta,
gravata-borboleta e um nome curto não são protegidos sozinhos; somados a um
texto que cita o personagem, facilitam uma reclamação (DMCA ou marca) contra
o projeto quando ele for público.
**Escolha (do Renan):**
- o personagem continua **Zeca**, também em público: um papagaio malandro com
  visual próprio;
- nada no projeto (código, docs, balões, commits, PRs) o chama ou o descreve
  como personagem de terceiros, nem cita nome, estúdio, título ou família de
  um;
- as passagens antigas (decisões 0001 e 0023, o PLANO) foram reescritas de
  forma neutra, a pedido do Renan. É a única exceção à regra de nunca
  reescrever o DECISIONS. A pesquisa nova já entrou reescrita;
- guarda: o `bin/pet verificar` reprova se uma busca sem caixa pelo nome do
  personagem e do estúdio achar algo nos arquivos do repositório (rastreados e
  novos, fora os ignorados). O padrão está escrito de um jeito que não casa
  consigo mesmo;
- a arte não muda: sem paletó, charuto ou guarda-chuva, cores próprias (faixa
  laranja, gravata rosa). O personagem padrão do app público continua em
  aberto (T9.0);
- o histórico do repositório privado ainda tem os textos antigos: o público
  será um repositório novo, sem o histórico (decisão 0036, T9.1).

**Por quê:** o risco está na combinação, e o texto era a parte que dependia só
de nós. O nome é escolha do Renan. A guarda impede que um texto novo traga a
referência de volta.

## 0036 — O app se chama bichinho (2026-10-03)

**Problema:** `claude-pet` é o nome do repositório, do binário, do compose e da
camada (decisão 0012). Já existe outro projeto com esse nome no GitHub (MIT,
desde 2026-02), e ele usa a marca de outra empresa. O nome do binário vai
entrar no `hooks.json` e no PATH de quem instalar (pesquisa, achado 4).
**Escolha (do Renan):**
- o produto e o binário se chamam **bichinho**, como o plugin; "para o Claude
  Code" fica só na descrição;
- o T8.1 troca o nome do crate e do binário, do namespace da camada, do
  projeto, do serviço e da imagem do compose, das mensagens da CLI e dos
  docs, preservando o volume com a aprovação do Zeca;
- ficam como estão: o repositório privado `butkeraites/claude-pet` (por ora),
  as variáveis `PET_*`, o cabeçalho `X-Pet`, a porta 27380 (o `avisar.sh` do
  plugin instalado continua falando com o pet até a troca), o `bin/pet`
  (atalho de desenvolvimento), a skin `zeca` e as strings de autoria gravadas
  na arte gerada (mudá-las mudaria a impressão digital aprovada);
- o repositório público será novo e limpo, sem o histórico (M9, T9.1).

Substitui a decisão 0012 no que ela dizia do repositório público, do binário,
do compose e da camada.
**Por quê:** um nome próprio evita a colisão e a marca alheia, e o binário
precisa do nome final antes de o hook nativo ir para o PATH.

## 0037 — Merge com merge commit, nunca squash (2026-10-03)

**Problema:** o PLANO mandava `gh pr merge --squash`. Com squash, os commits
de tarefa que o PROGRESS cita ficam fora da `main`.
**Escolha (do Renan):** os PRs entram na `main` com merge commit (`gh pr merge
--merge`): os commits da branch, um por tarefa, entram com os mesmos hashes.
As linhas novas do PROGRESS nascem com "—" na coluna Commit e ganham o hash
depois, num commit seguinte ou no PR.
**Por quê:** o PROGRESS é o índice das tarefas e precisa apontar para commits
que existem na `main`.

## 0038 — Lançamento multiplataforma e a nova ordem: T8.0 e T8.1 antes do M4 (2026-10-03)

**Problema:** o Renan quer, no fim, um lançamento open source para Linux,
macOS e Windows. A pesquisa (`docs/pesquisa/09-multiplataforma.md`) mostrou que
dá: o `pet-core` e o backend Wayland são reaproveitados, e por sistema mudam
só a janela do bicho e a ligação com o desktop. Mostrou também que arraste,
balões e voos (M4–M6) feitos dentro do `wl/` teriam de ser reescritos depois.
**Escolha (do Renan):**
- destino: open source; no macOS e no Windows o pet é um app nativo (AppKit,
  Win32). O Docker fica só no Linux: no Mac e no Windows ele é uma VM sem
  acesso à tela;
- marcos novos no PLANO: **M8** (multiplataforma, T8.0–T8.8) e **M9**
  (publicação, T9.0–T9.6), cada passo com a sua verificação;
- ordem: agora, antes do M4, só a costura de plataforma (T8.0) e o hook
  nativo com o nome novo (T8.1), mais o tamanho do Zeca no config (TP.2); depois
  M4–M7; depois T8.2–T8.8 e o M9. A T9.0 (arte, nome e licença) pode correr
  em paralelo, sem código. A T8.8 (GNOME) fica para a v1.1, salvo decisão em
  contrário;
- a conferência visual no macOS e no Windows precisa de uma máquina de verdade
  ou dos runners do GitHub Actions; sem isso, o item fica NÃO VERIFICADO;
- continuam abertas, com o Renan: a arte do personagem público, quanto gastar,
  GNOME e X11 na v1.0, a assinatura no Windows, a conta da Apple e o CI.

**Por quê:** as duas peças são pequenas (1 a 2 dias cada, pela pesquisa) e
fazem o M4–M6 nascer no código portável; feitas depois do M7, custariam o
dobro.

## 0039 — O clique no Zeca foca o terminal da sessão pelo foreign-toplevel, não pelo socket de comandos (2026-10-03)

**Problema:** o Renan quer que o clique no Zeca leve à janela do terminal da
sessão do Claude que terminou. No Hyprland o caminho óbvio é despachar
`focuswindow` pelo `hyprctl` (a pesquisa dos hooks sugeriu isso), mas isso é o
`.socket.sock`, que executa comandos no host e que o daemon nunca abre
(decisão 0006).
**Escolha (escopo do M4):**
- focar pelo Wayland: `zwlr_foreign_toplevel_manager_v1` com o
  `hyprland_toplevel_mapping_manager_v1`, que liga cada handle de toplevel ao
  endereço de janela do Hyprland (o mesmo do `activewindowv2`), e
  `zwlr_foreign_toplevel_handle_v1.activate(seat)`. A ideia de despachar
  `focuswindow` pelo socket de comandos fica descartada; a 0006 continua
  inteira. No M4, conferir na 0.56.2 que os dois protocolos estão no registro;
- identidade de janela por sessão: um anel com as últimas ativações (endereço
  e hora de chegada do `activewindowv2` no socket2, nunca o título), casado com
  o `ts` do hook (`UserPromptSubmit`, `SessionStart`) de cada sessão, e
  reforçado por dicas da cadeia de PIDs que o hook manda (`CLAUDE_PID` e pais,
  ids de terminal). As dicas entram como campo novo e opcional do fio v1, com
  decisão própria quando chegarem;
- pendências em ciclo, a mais urgente primeiro (a prioridade do cérebro); o
  clique sem pendência mostra um balão com as sessões abertas e o estado de
  cada uma. O balão mínimo e a fonte de pixel vêm do M6 para o M4;
- sem como focar (janela fechada, sessão sem identidade, protocolo ausente),
  o balão diz isso e mostra a lista;
- nos outros desktops (T8.7) o foco entra como `Desktop::focar` de cada
  adaptador.

**Por quê:** o foreign-toplevel é uma ação de alto nível sobre uma janela, sem
executar nada no host, e mantém o daemon longe do socket que roda comandos e
pode congelar o Hyprland. O anel casado com o `ts` acha o terminal sem ler
títulos de janela.

## 0040 — Costura de plataforma: o Motor no core, o Wayland num crate e a Caixa no lugar do canal do calloop (2026-10-03)

**Problema:** o M4–M6 (arraste, balões, voos) e o porte para Windows e macOS
(M8) precisam de um núcleo que não saiba em que sistema está. Até o M3, o
que o pet decide morava espalhado no laço do calloop (`laco.rs`: cérebro,
personagem, mostrar e esconder) e na sessão Wayland (`wl/mod.rs`: pet, palco,
estresse, painel, relógio da animação), e a entrada HTTP falava com o laço
pelo canal do calloop. A decisão 0038 antecipou esta costura para antes do
M4; ela não pode mudar nada do que o pet faz.
**Escolha (T8.0):**
- **`pet_core::motor`**: o `Motor` (cérebro, personagem, pet e palco,
  mostrar e esconder pela função pura `passo_de_visibilidade`, estresse,
  commits, painel do `/v1/estado`, quadro esperado da nitidez) e os prazos
  em milissegundos de um relógio monotônico que o laço injeta; o laço só
  acorda no `proximo_prazo` e entrega os eventos da janela. 16 testes novos
  com relógio falso e uma janela falsa (desenhar e marcar a próxima troca,
  quadro em voo que adia e o `Redesenhar` que retoma, esconder e cancelar a
  saída, trocar e revogar a skin, `tocar` com cada resposta, o cérebro
  acenando no prazo, janela fechada e recriada, painel, quadro esperado,
  estresse, encerrar, commit só de estado).
- **`pet_core::plataforma`**: os traits `Overlay` (a janela) e `Desktop` (a
  ligação com o ambiente) com capacidades (`CapOverlay`, `CapDesktop`), os
  tipos simples `Monitor`, `EventoPonteiro`, `Botao`, `Alca`, `Fase`, `Passo`,
  `Desenho` e `EventoOverlay`, e a **`Caixa`**: um `mpsc` limitado mais um
  `Despertador` do sistema (no Linux, o `Ping` do calloop), no lugar do
  `SyncSender` do calloop na entrada HTTP. Cheia, ela acorda o laço e a
  entrada responde 503, como antes.
- **`crates/pet-wayland`**: o `wl/` e a descoberta. A `Sessao` implementa o
  `Overlay`: põe na tela a cena que o Motor manda e conta o que aconteceu
  (camada pronta, quadro mostrado, camada fechada, ponteiro) como eventos; os
  prazos dela (destruir 50 ms depois de esconder, reservas de escala e de
  `enter`, recriar 250 ms depois de um `closed`) vencem no relógio do laço.
  O que é só do Hyprland (`hyprland.lock`, o monitor FALLBACK, e no M4 o
  socket de eventos e o foco) virou o adaptador `hyprland`, com um
  `Desktop` ainda sem capacidade; o resto (socket por caminho longo, backoff)
  ficou em `conexao`.
- **`crates/pet-windows` e `crates/pet-macos`**: vazios, compilando, com
  `#![cfg]` do próprio sistema. O daemon depende deles só no alvo deles.
- **O daemon**: o `nucleo` (igual em todo sistema) junta o Motor com as
  aprovações em disco e o `/v1/estado`; o `laco` do Linux só traz as fontes
  (sinais, caixa, batimento, descoberta, Wayland) e **um** prazo, o mais
  próximo entre o do Motor e o da janela. Antes de vencer um prazo o laço
  esvazia a caixa: a garantia da decisão 0032 (um evento que chegou antes
  cancela a acomodação) deixa de depender da ordem de despacho do calloop.
  Onde ainda não há janela (Windows e macOS), o `sem_janela` roda só com a
  `std`: o cérebro e o `/v1/estado` funcionam, como no Linux sem
  compositor; um teste o roda no Linux.
- **O log** (`registro`) foi para o core, com as mesmas mensagens.
- **Portão:** o `bin/pet verificar` procura o socket de comandos em todos os
  crates (provado com uma linha plantada no `pet-wayland`), barra crates de
  sistema no `pet-core` e, com os alvos `x86_64-pc-windows-msvc` e
  `aarch64-apple-darwin` instalados no rustup (instalados nesta máquina),
  passa o `cargo clippy --target … -D warnings` no core, nos esboços e no
  daemon. O check não linka: não pede o SDK da Microsoft nem o da Apple.

Ao vivo, com a tela apagada e a sessão bloqueada, um daemon nativo de
debug (porta 27399, `/state` de rascunho, skin `_teste`) ligado ao Hyprland
de verdade: camada criada, `configure`, `enter`, `preferred_scale` 180/120,
D = 5 com a célula em (1706, 951), o primeiro quadro, `tocar` respondendo
`tocou: true`, esconder com o quadro transparente, mostrar recriando a
camada, o quadro esperado do `/v1/debug/quadro` e a saída com "o compositor
processou o quadro transparente e a destruição". Ficam pendentes, porque
pedem a tela acesa e desbloqueada: `scripts/verificar-ao-vivo.sh
--personagem` e `scripts/medir-custo.sh` (nitidez, fantasma, ritmo parado e
custo no Hyprland iguais ao M1).
**Por quê:** a costura é pequena agora e cara depois do M4–M6. Com o Motor
puro, o que o pet faz é testado sem compositor, e o porte para outro sistema
é escrever um `Overlay`, um `Desktop` e um laço, sem tocar no resto.
