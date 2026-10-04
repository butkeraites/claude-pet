# Decisões

Uma seção por decisão: o problema, a escolha, o porquê. Se mudar depois,
acrescente a mudança abaixo da original — não apague a história.

## 0001 — O personagem é o Zeca, feito com arte profissional (2026-10-02)

**Problema:** o primeiro rascunho de mascotes (capivara, gato, slime e
robô desenhados em ASCII) foi recusado como "muito feio". Um bichinho que
fica o dia inteiro na tela precisa de arte de verdade.
**Escolha:** a base é a arara do pack *Cute Parrots!* da exclusiveOlive
(itch.io, 48x48, ~20 animações). Ela vira o **Zeca**, um papagaio malandro
"estilo Zé Carioca, um pouco diferente": corpo verde, chapéu-palheta e
gravata-borboleta com faixa laranja. Corpo e animações são da artista; nós
só desenhamos os acessórios pequenos e a troca de paleta. Nada de nome ou
visual da Disney.
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
