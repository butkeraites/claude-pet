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

## 0041 — Hook nativo (`bichinho avisar`) em exec form e o nome bichinho, com o volume da aprovação preso ao nome antigo (2026-10-03)

**Problema:** o hook era o `avisar.sh` (sh + jq + curl): no Windows, sem Git
Bash ele nem roda e, sem jq, só sai o evento mínimo; a lista branca vivia
num programa jq, duplicando os validadores do `pet_core::evento`. O nome
`claude-pet` ia para o binário, a camada e o compose (decisão 0036 trocou
para `bichinho`), e o volume com a aprovação do Zeca leva o nome do projeto
do compose: trocar o projeto criaria um volume vazio, sem a aprovação. E o
plugin instalado (0.1.0, decisão 0021) continua chamando o `avisar.sh` até a
troca depois do merge.
**Escolha (T8.1):**
- **`bichinho avisar <Evento>`**: lê o JSON do hook na entrada padrão (até
  64 MiB), monta o corpo pela lista branca de `pet_core::aviso` — a mesma do
  `avisar.sh`, campo a campo, com os validadores do fio v1, então o que o pet
  descartaria nem sai —, faz o hash do caminho editado (12 hexadecimais do
  sha256), lê o "não perturbe" do Omarchy (só o booleano), valida o
  `CLAUDE_CODE_ENTRYPOINT`, respeita `PET_TESTE=1` e manda por TCP direto ao
  127.0.0.1 (conexão em 300 ms, envio e resposta em 2 s): nenhum proxy,
  curlrc, jq ou shell. Espera a resposta do pet para o evento não se perder
  no fim do processo. Nunca imprime (um pânico sai 0, calado), sempre sai 0 e
  nunca passa de 4 s, nem com a entrada padrão aberta. Diferenças do
  `avisar.sh`, todas a favor do pet: a pasta passa pelo validador do pet; uma
  contagem `bg` acima de 10 000 não sai; uma porta 0 ou acima de 65 535 vira
  a padrão.
- **`hooks.json` em exec form**, igual nos três sistemas: `{"type":
  "command", "async": true, "command": "bichinho", "args": ["avisar",
  "<Evento>"]}` (`claude plugin validate --strict` aceita); plugin 0.2.0.
- **O binário no host:** o exec form acha o `bichinho` pelo PATH do Claude
  Code. `bin/pet instalar-host` copia o binário estático (musl, static-pie)
  da imagem para `~/.local/bin/bichinho` (troca de uma vez, confere que roda
  no host, avisa se a pasta não está no PATH ou se outro `bichinho` vem
  antes). Requisito documentado: `~/.local/bin` no PATH que o Claude Code vê
  (no Omarchy, está). Sem o binário, os hooks não chegam ao pet e o `claude
  -p` continua calado (conferido com um plugin de sondagem cujo comando não
  existe).
- **Reserva até a troca:** o `avisar.sh` fica no plugin, com os canários
  dele; o plugin 0.1.0 instalado continua chamando a cópia dele (no cache do
  Claude Code), que fala com o mesmo pet: a troca nunca deixa o pet surdo. O
  `bin/pet testar` vai pelo `bichinho` do PATH (ou `PET_BICHINHO`) e, sem
  ele, pelo `avisar.sh`, avisando. A troca depois do merge, na ordem (está no
  README): clone e worktree estável na `main`, `bin/pet subir`, `bin/pet
  instalar-host`, `claude plugin marketplace update bichinho-local`,
  `claude plugin update bichinho@bichinho-local` e `/reload-plugins`.
- **Canários portados** para o binário (`tests/hook.rs`, 18 testes, com os
  casos do `avisar.sh` num módulo comum): o binário de verdade, o ambiente
  limpo e um pet falso no 127.0.0.1 que guarda o pedido inteiro (linha,
  cabeçalhos e corpo) — nenhum segredo em evento nenhum, campos por evento,
  `PET_TESTE`, "não perturbe", entrada que não é JSON, tipos errados, nome de
  evento inválido, cabeçalhos, proxy e curlrc ignorados, origem, validadores
  iguais aos do pet, entrada de 8 MiB, pet desligado (menos de 0,5 s), pet
  travado (menos de 2,6 s), entrada que nunca fecha (sai no prazo) e o pet de
  verdade sem nada recusado nem descartado. Seis versões erradas de propósito
  reprovaram: a lista branca aberta (o prompt no `src`: 5 testes), o caminho
  editado em claro (2), imprimir no stderr (14), sair 1 (14), sem o prazo
  total (1) e a pasta inteira no `proj` (7).
- **O nome bichinho** (decisão 0036): crate e binário, namespace da camada,
  projeto, serviço e imagem do compose (`bichinho:local`), `/opt/bichinho` e
  `/etc/bichinho` no container, `config/bichinho.toml` (o `claude-pet.toml`
  ainda vale sozinho, com aviso no log), a pasta dos symlinks curtos, a carga
  de medição (`bichinho-carga`) e as mensagens. Ficam `claude-pet`: o
  repositório, a worktree estável e as strings de autoria gravadas na arte
  gerada (mudariam a impressão digital aprovada).
- **O volume:** preso ao nome `claude-pet_estado` no compose, sem copiar a
  aprovação; o compose avisa que ele foi criado para o projeto antigo, e é de
  propósito. `bin/pet subir`, `dev` e `reconstruir` aposentam o container do
  projeto antigo (ele seguraria a porta 27380); o volume nunca é tocado.

Ao vivo (tela apagada e sessão bloqueada; só `/v1/estado` e log): `bin/pet
subir` aposentou o `claude-pet-pet-1` e subiu o `bichinho-bichinho-1`
(healthy) com o mesmo volume: `tela: ativa`, `zeca` da imagem com a aprovação
do Renan (sha 5b843b03…), o `aprovacao.json` idêntico ao de antes, D = 8.
`bin/pet instalar-host` pôs o binário em `~/.local/bin/bichinho`;
`bin/pet testar rapido` → `nod` e `pequeno` → `done_small`, pelo hook nativo.
O `avisar.sh` do plugin instalado (mesmo sha256 antes e depois) entregou um
evento de teste ao daemon novo. Shellcheck limpo pela imagem oficial,
removida depois.

Fica para depois: a porta e o token por usuário no loopback (T9.3; o
`avisar.sh` instalado não mandaria o token) e a CLI de usuário no binário
(`estado`, `tocar`, `doutor`), que por enquanto é o `bin/pet`.
**Por quê:** um binário só, sem shell nem jq, é o mesmo hook nos três
sistemas, e a lista branca passa a ter uma fonte com os validadores do pet.
Prender o volume ao nome antigo não copia nem arrisca a aprovação, e vale
para todo jeito de subir o compose (produção, dev e scripts).

## 0042 — O tamanho do Zeca é config (`aparencia.tamanho`), sem mexer na skin (2026-10-03)

**Problema:** o Renan quer o Zeca menor na tela, ~10% da altura lógica do
monitor (hoje o corpo dá 12,7% no eDP-1). O tamanho vinha de uma fração fixa
(12%, entre 80 e 160 pixels lógicos) dividida pelo `corpo_px` da skin; mudar
a skin mudaria a impressão digital e pediria outra aprovação.
**Escolha (TP.2):**
- **`aparencia.tamanho`** = `pequeno` | `normal` | `grande` no config
  (`PET_APARENCIA_TAMANHO` no ambiente; padrão `normal`, o de antes), ~10%,
  ~12% e ~16% da altura lógica do monitor. O alvo do `normal` (12%, entre 80 e
  160 lógicos) é multiplicado por `fração ÷ 12%`, limites inclusive: os três
  continuam diferentes até no 4K, onde o `normal` bate no teto. O D continua
  inteiro e por monitor, e as posições em pixels inteiros. Com o Zeca (corpo
  de 19): D = 6, 8 e 10 no eDP-1 (o corpo dá 9,5%, 12,7% e 15,8%) e 11, 13 e
  17 no 4K. O `normal` dá exatamente o D de antes.
- **A skin não muda:** o tamanho entra no palco (`Motor`), não no
  `skin.json`; a impressão digital e a aprovação ficam como estão.
- Relido a cada aprovação (decisão 0029): com a janela pronta, o palco é
  refeito e o quadro novo redesenha a tela toda. No mais, vale quando o pet
  reinicia (`bin/pet parar && bin/pet subir`; o `bin/pet subir` sozinho não
  recria o container quando só o config muda).
- O `config/bichinho.toml` local do Renan (fora do git) pede `tamanho =
  "pequeno"`.

Ao vivo, com a tela apagada e a sessão bloqueada (só `/v1/estado`, log e o
`/state`; a conferência na tela fica pendente): produção refeita desta
branch, `tela: ativa`, D = 6 (era 8), `aparencia.tamanho` = `pequeno` vindo
do arquivo, célula em (1698, 984), região de toque de 76×76 lógicos (era
102×102), o `zeca` da imagem com a aprovação do Renan (sha 5b843b03…, o
`aprovacao.json` idêntico ao de antes). E o fim da branch, também ao vivo:
- `bin/pet testar rapido` → `nod` e `pequeno` → `done_small`, pelo hook
  nativo (`~/.local/bin/bichinho`);
- `claude --plugin-dir` com o plugin 0.2.0: só ele dispara, sem o 0.1.0
  instalado em dobro (medido com um invólucro que registra cada chamada do
  `bichinho`, só o nome do evento, num daemon de rascunho);
- gate interativo no tmux em `~/Documents`, sem `CLAUDECODE` e as
  `CLAUDE_*` do agente: "responda só: ok" → `nod` (T0); um Write → um turno
  de trabalho 1 e `arquivos: 1` → `done_small` (T1); `/exit` → `bye`. Sete
  chamadas do hook nativo, 15 eventos aceitos, nenhum recusado; nada dos
  prompts nem do caminho no log nem no `/v1/estado`;
- pet parado: o hook sai 0 em 1 ms; `claude -p --plugin-dir …` imprime só
  `ok`, com stderr vazio e saída 0 (quatro chamadas do hook nativo);
- o plugin instalado, o marketplace e a worktree estável ficaram como
  estavam (mesmo sha256 do `avisar.sh` e do `hooks.json` no cache).
**Por quê:** o tamanho é gosto de quem usa, não arte: fica no config, onde
trocar não pede aprovação. Multiplicar o alvo inteiro (limites junto) mantém
os três tamanhos distintos em qualquer monitor sem mexer no `normal`.

## 0043 — Revisão do plano do M4: ids de terminal em vez da cadeia de PIDs, aviso visto só com clique, os contratos do M4 na costura e a guarda de nomes nas mensagens (2026-10-04)

**Problema:** as três revisões da branch `m3b-portabilidade` acharam no plano
(TP.1):
- a decisão 0039 e o M4 diziam que a cadeia de PIDs que o hook manda
  (`CLAUDE_PID` e pais) desempataria o anel de ativações. No Docker o daemon
  roda em outro espaço de PIDs e não lê o `/proc` do host; o socket2, o
  foreign-toplevel e o `hyprland_toplevel_mapping` não trazem PID. Ligar um
  PID a uma janela pediria o `hyprctl clients`, que é o socket de comandos
  (decisão 0006) e só aparece em scripts de teste do host;
- o M4 dizia que o aviso de uma sessão some "quando a janela dela é focada",
  contra as regras do cérebro: o pronto some com ~10 s de foco, e o
  "esperando você" só sai com um evento da sessão ou um clique;
- a decisão 0040 punha o foco no adaptador `hyprland`, que não tem conexão
  Wayland. O `activate(seat)` do foreign-toplevel precisa da conexão e do
  `wl_seat`, que moram na `Sessao`;
- o PLANO ainda descrevia o hook antigo em alguns trechos: o `avisar.sh` como
  o hook, o curl de 2 s, a receita `novo-evento-hook` e o `testar`. Também
  citava o serviço `pet` do compose;
- a guarda de nomes (decisão 0035) olhava só os arquivos.

**Escolha:**
- **As dicas do hook** vão num campo novo e opcional do fio v1, com decisão
  própria no M4. São os ids de terminal do ambiente do hook (`TMUX_PANE`,
  `KITTY_WINDOW_ID`, `WEZTERM_PANE`), que só separam sessões dentro de um
  mesmo terminal. A identidade da janela é o anel de ativações casado com o
  `ts`; quando há dúvida, o clique cai no balão com a lista.
  - Se um dia a cadeia de PIDs for necessária, só uma decisão nova pode
    deixar o **hook** ler o `hyprctl -j clients`, jogando fora os títulos na
    memória. O daemon nunca. A regra do CLAUDE.md muda junto com essa
    decisão.
  - Corrige a 0039 nesse ponto.
- **O clique** que foca a janela de uma sessão marca o aviso dela como
  visto. O foco sem clique segue as regras de sempre.
- **A costura no M4** ganha:
  - o `EventoDesktop` e o `Motor::evento_desktop`;
  - o `Overlay::cursor`;
  - um punho por conexão no `Nucleo`, com a janela e o desktop juntos.

  O `Desktop` do Wayland usa a conexão e o `wl_seat` da `Sessao`, recriados a
  cada reconexão. O foreign-toplevel genérico fica no `pet-wayland`, e o
  mapeamento do Hyprland com o socket2 é a extensão do Hyprland. Corrige a
  0040 nesse ponto.
- **O PLANO** descreve o hook nativo, com o `avisar.sh` de reserva, o serviço
  `bichinho` e a receita nova do `novo-evento-hook`.
- **A guarda de nomes:** o `bin/pet verificar` procura os nomes também nas
  mensagens dos commits da branch (`main..HEAD`). O corpo do PR se confere
  antes do `gh pr create` (CLAUDE.md). A nota de memória do Claude sobre o
  projeto, fora do repositório, foi reescrita sem o nome.

**Por quê:** o plano não pode prometer o que as regras de ouro proíbem nem
contradizer o cérebro. E o M4 precisa encontrar a costura pronta para o foco.

## 0044 — Revisão da costura: o palco como sistema de coordenadas, a área útil do monitor, a janela pronta sem dado velho, o laço sem trabalho à toa e as guardas sem ponto cego (2026-10-04)

**Problema:** as revisões da costura (T8.0) acharam seis falhas.
- **Coordenadas misturadas no `Overlay`.** A cena e a célula estavam em
  pixels do monitor, a área de toque em coordenadas lógicas da janela, e o
  ponteiro "relativo à janela". Isso só funcionava porque a camada do Wayland
  cobre o monitor inteiro. Numa janela pequena (Win32, AppKit, X11), o clique
  e o arraste do M4 comparariam grandezas diferentes. Além disso:
  - o `Monitor` não tinha origem, área útil nem descrição;
  - a posição padrão usava o monitor inteiro, então o pet cairia na barra de
    tarefas ou no Dock;
  - o Motor nunca olhava as capacidades da janela.
- **Palco velho no buffer novo.** O `EventoOverlay::Pronta(monitor)` levava o
  monitor do momento em que entrou na fila. Quando dois chegavam na mesma
  leva (escala e `configure` juntos), o primeiro desenhava com o palco velho
  no buffer novo. Antes do T8.0, o código desenhava sempre com o estado de
  agora.
- **Nome vago.** O `EventoOverlay::Mudou` só significava "a janela terminou de
  sair".
- **Trabalho à toa no laço.** O laço do Linux rodava o `assentar` inteiro a
  cada leva do Wayland: publicava o painel e tirava e punha de novo o mesmo
  timer. A `Sessao` passava ao Motor cada movimento do ponteiro sobre o pet,
  e o Motor os ignora. Passar o mouse por cima custava trabalho na taxa do
  mouse, e recolocar o timer podia atrasar o prazo uma volta.
- **Guardas com ponto cego.**
  - A guarda do socket de comandos parava no primeiro `#[cfg(test)]` de cada
    arquivo. No `motor/mod.rs` ele vinha na linha 23 (`mod testes;`), e as
    574 linhas do Motor ficavam sem olhar. A decisão 0040 dizia que a guarda
    olhava todos os crates.
  - A guarda do core olhava só o alvo do host. No aarch64 (macOS e Linux
    arm), o core já puxa o `libc` pelo `cpufeatures` do `sha2`.
- **Testes faltando.** A agenda interna da camada não tinha testes, e o teste
  do laço sem janela dependia de um `sleep` com 400 ms de folga.

**Escolha (revisão do T8.0):**
- **O palco.** Tudo o que o Motor troca com a janela fica em pixels do
  dispositivo do monitor, com a origem no canto dele: a cena, a célula, a
  área de toque do `Overlay::desenhar` e o `EventoPonteiro`.
  - A camada do Wayland converte o toque para coordenadas lógicas. A conta é
    a mesma de antes, agora dentro dela.
  - Uma janela pequena subtrai a própria origem.
  - O `InfoOverlay.regiao` (o `regiao_entrada` do `/v1/estado`) continua nas
    coordenadas da janela.
- **O `Monitor`** ganhou três campos: `descricao` (para as posições salvas do
  M4), `origem` no desktop (para as janelas pequenas) e `area_util` no palco.
  A posição padrão do pet é o canto inferior direito da área útil. No Wayland
  a área útil é o monitor inteiro, porque a camada ignora as zonas
  exclusivas; no Hyprland, nada muda.
- **O Motor consulta as capacidades.** O estresse (confete pela tela inteira)
  só começa com uma janela do tamanho do monitor. O
  `Motor::acerta_o_pet(x, y)`, no palco, é por onde o clique do M4 vai
  começar.
- **Eventos da janela.** O `EventoOverlay::Pronta` não carrega mais o
  monitor: o Motor refaz o palco com o `Overlay::pronta()` de agora. O
  `Mudou` virou `Saiu`.
- **O laço.**
  - O prazo armado fica onde está quando não muda.
  - Depois de uma leva do Wayland, o painel só sai se um evento mudou o pet
    ou a janela; o batimento de 5 s republica de todo jeito.
  - Movimentos seguidos do ponteiro viram um só na fila.
- **A guarda do socket** pula só o item marcado com `#[cfg(test)]`: um
  `mod x;` ou um bloco até a `}` da coluna 0. Ela se prova a cada `verificar`
  com linhas plantadas. O `mod testes;` do Motor foi para o fim do arquivo. A
  afirmação da 0040 sobre a guarda passa a ser verdade.
- **A guarda do core** olha todos os alvos (`cargo tree --target all`). O
  `libc` só pode vir do `cpufeatures`.
- **Testes.**
  - A janela de mentira virou `plataforma::falsa::JanelaFalsa`, com uma
    versão pequena. Ela fica atrás da feature `teste`, que o binário nunca
    liga.
  - O Motor ganhou testes novos: duas `Pronta` na mesma leva, janela pequena
    com barra de tarefas e acerto no palco, e estresse recusado na janela
    pequena.
  - A agenda interna da camada é uma estrutura pura, com testes da ordem de
    vencimento e da geração.
  - O laço sem janela espera a reação aparecer, em vez de dormir um prazo
    fixo.

Ficaram para depois, de propósito:
- um teste do laço do calloop provando que a caixa entra antes do prazo
  (decisão 0032). O código são três linhas, e o teste pediria montar o laço
  com a conexão;
- um relógio injetado no `Nucleo`.

**Por quê:** a costura existe para o M4 e o M8 escreverem em cima dela. Um
contrato que só vale no Wayland seria pago depois, com o arraste e o clique
já escritos. E uma guarda com ponto cego dá uma garantia que não existe.

## 0045 — Revisão do hook nativo: o binário do PATH preso à worktree estável, leitura em fluxo só da lista branca, sem log, sem core dump e sem daemon por engano (2026-10-04)

**Problema:** as revisões do hook nativo (T8.1) acharam cinco problemas.
- **Isolamento perdido.** No exec form, o código do hook é o `bichinho` que
  estiver no PATH, e o `bin/pet instalar-host` o copiava da imagem montada
  da branch do clone. Depois da troca para o plugin 0.2.0, um
  `bin/pet subir && bin/pet instalar-host` numa branch trocaria a lista
  branca de **todas** as sessões do Claude Code da máquina. É exatamente o
  que a worktree estável existe para impedir (decisão 0021). O
  `--plugin-dir` testava o hooks.json da branch com o binário velho do
  PATH, e o `bichinho versao` não dizia de onde o binário saiu.
- **Leitura frágil e cara.** O hook lia o JSON inteiro como uma árvore de
  `Value`. Bastava um erro em qualquer lugar, mesmo num campo que o hook não
  lê, para ir só o mínimo `{"v":1,"e":…}`, que o pet ignora:
  - um substituto UTF-16 sozinho. O Claude Code corta um texto no meio de um
    emoji e o `JSON.stringify` manda `\ud83d`;
  - bytes que não são UTF-8;
  - mais de 128 níveis de aninhamento;
  - um número como `1e400`.

  O `avisar.sh` (jq) guardava os metadados em quase todos esses casos, então
  a 0041 errou ao dizer que as diferenças eram "todas a favor do pet". A
  mesma leitura gastava cerca de 17 vezes a entrada em memória: 172 MiB para
  um array numérico de 10 MB, cerca de 1 GiB para 60 MB.
- **Log no hook.** O hook herdava o log do daemon (`PET_LOG`) antes de
  começar. Os canários nunca rodaram com `PET_LOG=debug` nem com segredos no
  ambiente.
- **Daemon por engano.** O `bichinho` sem subcomando subia o daemon. Um
  Claude Code que ignorasse o `args` do exec form subiria um daemon a cada
  hook.
- **Windows e DND.** No Windows o `cwd` usa barra invertida, e o `proj` nunca
  saía. O "não perturbe" do Omarchy era lido em todo sistema.

**Escolha (revisão do T8.1):**
- **O binário do PATH é o da worktree estável.**
  - O `bin/pet subir` (e `dev`, `reconstruir`, e os scripts ao vivo) grava na
    imagem o commit de onde ela saiu: `BICHINHO_FONTE`, que é o
    `git rev-parse HEAD`, com `-sujo` se a árvore tem mudanças. O commit vai
    na etiqueta `bichinho.fonte`, no `bichinho versao` e no `/v1/estado`
    (`fonte`).
  - O `bin/pet instalar-host` recusa se esse commit não for o da worktree
    estável (numa máquina sem ela, o da `main`) ou se a árvore estava suja.
    Também confere que o binário diz a mesma fonte da etiqueta.
  - O `--da-branch` pula a conferência, com aviso. Serve só para uma pasta de
    teste (`PET_BIN_HOST`) fora do PATH.
  - Para testar uma branch, o binário vai só na sessão:
    `cargo build -p bichinho` e
    `PATH="$PWD/target/debug:$PATH" claude --plugin-dir plugin`. No
    `bin/pet testar`, o mesmo vale com `PET_BICHINHO`, e ele mostra o commit
    do binário.
- **Leitura em fluxo, só da lista branca** (`pet_core::aviso::Lido`).
  - As chaves são lidas como bytes. Os campos da lista ficam como texto cru e
    são interpretados um a um; um que não se lê cai sozinho.
  - Do `tool_input`, só o `file_path` e o `notebook_path`. Do
    `background_tasks`, a contagem e as 16 primeiras tarefas válidas.
  - O resto é pulado sem ser validado nem guardado.
  - A entrada é lida em fluxo e o hook segue quando o objeto fecha, sem
    esperar a entrada padrão fechar.
  - Medido: 12 MiB de pico com 10 MB ou com 60 MB de entrada (eram 172 MiB e
    cerca de 1 GiB).
  - Chave repetida: vale a última, como no jq.
  - Lixo depois do objeto é ignorado. Antes, mandava o mínimo.
  - O `avisar.sh` de reserva continua como era: com um substituto alto
    sozinho, ele também manda só o mínimo.
- **Sem log no hook.** O `main` chama `registro::desligar()` antes do
  `avisar`, sem olhar o `PET_LOG`. O `PET_LOG` ganhou o nível `off`.
- **Sem core dump.** No Linux o hook se marca como não despejável
  (`PR_SET_DUMPABLE` pelo invólucro seguro do `rustix`, sem `unsafe` nosso).
  Um aborto não leva a entrada para o `systemd-coredump`.
- **Sem daemon por engano.** O daemon é só `bichinho rodar`; a imagem e os
  testes já passavam o `rodar`. Sem subcomando, nada roda: sai 0 calado e,
  no terminal, mostra a ajuda.
- **Windows.** O `proj` corta nas duas barras. Uma barra invertida nunca
  passa no validador do pet, então no Linux só muda um nome de pasta com
  barra invertida, que agora manda o último pedaço. O DND do Omarchy só é
  lido no Linux.
- **Canários novos** em `tests/hook.rs`:
  - a matriz inteira com `PET_LOG=debug` e segredos no ambiente: chave da
    API, token, `TERM_PROGRAM`, `TMUX_PANE`, `KITTY_WINDOW_ID` e uma
    variável desconhecida;
  - conteúdo quebrado fora da lista: substitutos sozinhos, bytes que não são
    UTF-8, 200 níveis e `1e400`, no prompt, na saída da ferramenta, na
    resposta e na descrição de uma tarefa. Os metadados ficam;
  - objeto fechado com a entrada aberta;
  - sem subcomando, nada roda;
  - `versao` com a fonte.

  Cinco versões erradas de propósito reprovaram:
  - a leitura antiga por `Value`;
  - o log ligado no hook;
  - sem subcomando subindo o daemon;
  - esperar a entrada fechar;
  - um id de terminal no corpo.

**Por quê:** o hook é a fronteira da privacidade e roda em toda sessão da
máquina. O código dele tem de ser o revisado da `main`, tem de falhar para o
lado de mandar menos e nunca pode deixar o conteúdo em log, em core dump ou
em memória à toa.

## 0046 — Revisão do tamanho: o tamanho novo entra junto com a escolha do personagem, e o compose repassa as chaves do config que o ambiente pode trocar (2026-10-04)

**Problema:** a revisão do TP.2 achou dois problemas.
- **Quadro a mais ao reler o config.** Numa aprovação, o config relido
  aplicava o `aparencia.tamanho` novo na hora: refazia o palco e desenhava um
  quadro forçado com a skin de agora. Só depois escolhia o personagem.
  - Se a aprovação revogava a skin, ela era desenhada mais uma vez, no
    tamanho novo, antes de sair.
  - Se a aprovação trocava de skin, eram dois quadros forçados, e o primeiro
    ia para o lixo.

  Isso contrariava a regra de que, sem skin, o pet sai só com o quadro
  transparente.
- **Variável documentada que não fazia nada.** O `config/exemplo.toml` e a
  decisão 0042 diziam que `PET_APARENCIA_TAMANHO=pequeno` valia como o
  arquivo. O compose não passava a variável ao container: no Docker ela não
  fazia nada, sem aviso. O mesmo valia para `PET_SESSOES_ORIGENS` (M3),
  `PET_APARENCIA_SKIN` e `PET_CELEBRACAO_MODO`.

**Escolha (revisão do TP.2):**
- **O config relido só anota o tamanho novo.** A escolha do personagem que
  vem em seguida cuida da tela:
  - uma skin nova já nasce no palco novo;
  - uma skin revogada sai só com o quadro transparente;
  - a mesma skin ganha um quadro só, forçado, no palco novo
    (`Motor::redesenhar_palco`).

  Dois testes no núcleo do daemon reprovaram o código de antes. Eles usam a
  janela de mentira e uma aprovação de verdade em pastas temporárias.
- **O compose repassa as quatro variáveis sem valor:** `PET_APARENCIA_SKIN`,
  `PET_APARENCIA_TAMANHO`, `PET_CELEBRACAO_MODO` e `PET_SESSOES_ORIGENS`. Elas
  só existem no container se estiverem no ambiente do `bin/pet subir` ou no
  `.env`. Conferido num container avulso: sem a variável, ela nem aparece; com
  `PET_APARENCIA_TAMANHO=grande`, aparece. A precedência do config (o
  ambiente antes do arquivo) passa a valer também no Docker.

**Por quê:** uma aprovação é uma troca de personagem, e a tela tem de mudar
uma vez só, do jeito certo. Uma opção documentada que não faz nada é pior que
nenhuma.

## 0047 — Costura do M4: o punho por conexão, os eventos do desktop e o cursor (2026-10-04)

**Problema:** o clique do M4 foca a janela de uma sessão pelo desktop e toca a
risadinha na janela, na mesma volta do laço. O laço só entregava ao núcleo a
janela (`Option<&mut dyn Overlay>`); o `Desktop` do Hyprland era uma struct à
parte, sem a conexão Wayland nem o `wl_seat` que o
`zwlr_foreign_toplevel_handle_v1.activate(seat)` pede; e não havia como os
eventos do desktop (monitor em foco, janela ativa) chegarem ao Motor, nem
como o Motor pedir o cursor de agarrar (decisão 0043).
**Escolha (T4.1):**
- **`Punho`** (`pet_core::plataforma`): a janela e o desktop de uma conexão,
  juntos, pedidos um de cada vez (`janela()`, `desktop()`, e `ver_janela()` e
  `ver_desktop()` para só ler). O `Nucleo` recebe `Option<&mut dyn Punho>` em
  todo lugar onde recebia a janela; o Motor recebe o punho no
  `evento_overlay` (o clique vai focar uma janela) e a janela no resto.
- **`EventoDesktop`** e **`Motor::evento_desktop`**: a fonte dos eventos
  ligou ou caiu, o monitor em foco, a janela ativa (um id opaco e a hora de
  parede em que o desktop contou, o mesmo relógio do `ts` dos hooks), a
  semente da janela ativa na conexão nova, a presença (só o booleano do
  primeiro glifo do título), uma janela que abriu (com a proteção de tela
  como booleano) ou fechou, e os monitores. Nenhum evento carrega título,
  classe ou nome de área de trabalho.
- **`Desktop::eventos` e `Desktop::info`** (com padrão vazio): os eventos que
  a própria conexão conta (no Wayland, a semente pelo foreign-toplevel, na
  T4.9) entram no mesmo lote dos eventos da janela; o `/v1/estado.desktop`
  mostra a fonte dos eventos (`sem`, `ligado`, `caiu`), o monitor em foco, o
  endereço da janela ativa, a presença, se a conexão sabe focar janelas, os
  protocolos ligados para isso e quantas janelas ela conhece.
- **`Overlay::cursor`** (`Pegar`, `Agarrar`): no Wayland, `grab` e
  `grabbing` do cursor-shape-v1, com o serial do último `enter`.
- A **`Sessao`** do Wayland é o `Desktop` e o `Punho` da conexão; a struct
  `Hyprland` sem capacidade saiu. A janela de mentira ganhou o
  `DesktopFalso` e virou punho, para os testes do Motor e do núcleo.

Nada muda no que o pet faz: nenhum evento do desktop chega até o leitor do
socket2 (T4.4), e o cursor continua `grab` no `enter`.
**Por quê:** a janela e o desktop vivem na mesma conexão no Wayland e no
mesmo processo de interface no Windows e no macOS; entregá-los juntos, um de
cada vez, deixa o clique decidir e agir na mesma volta sem duas referências
mutáveis à mesma sessão. Os eventos do desktop entram no Motor como tipos
simples, sem nada de conteúdo, e o Motor continua testável com a janela e o
desktop de mentira.

## 0048 — Arrastar e clicar: limiares, a área de toque no palco inteiro só no arraste e o fail-safe (2026-10-04)

**Problema:** o Zeca tem de poder ser arrastado para qualquer lugar e
clicado (PLANO, "Seguir o monitor ativo, arrastar, clicar"), sem roubar
cliques do resto da tela, sem borrar a pixel art e sem ficar preso se o
compositor perder o ponteiro (numa área de trabalho vazia, a pegada
implícita do Hyprland pode sumir; pesquisa do Hyprland, seção 3).
**Escolha (T4.2):**
- **A máquina do ponteiro** (`pet_core::motor::arraste`, pura, no palco):
  botão esquerdo apertado no corpo vira arraste quando o ponteiro anda mais
  de 4 pixels lógicos (6 do dispositivo a 1,5) ou depois de 250 ms
  segurando; solto antes disso é clique. O direito só clica (e desiste se
  andar). O do meio não faz nada. Um `leave` com o botão apertado, antes de
  arrastar, desiste do clique; arrastando, quem decide é o soltar.
- **A posição:** a célula anda com o ponteiro em múltiplos de D a partir de
  onde estava quando o arraste começou (o deslocamento da pixel art, decisão
  0004), e o corpo fica sempre inteiro dentro da área útil (o `Palco` ganhou
  a `area`); ao soltar, ela vai até o ponto onde o botão subiu.
- **A área de toque** cresce para o palco inteiro só enquanto arrasta (o
  arraste segue até numa área de trabalho vazia) e volta ao corpo ao
  soltar, no fail-safe, ao esconder e quando a janela fecha.
- **Fail-safe:** 5 s sem evento do ponteiro com o botão apertado soltam o pet
  onde ele está.
- **Cursores:** `grabbing` ao apertar, `grab` ao soltar (o `grab` do `enter`
  continua na própria camada).
- **Animação:** enquanto arrasta, o estado `dangle` em laço (o voo do Zeca);
  ao soltar, `land` uma vez. O animador ganhou o estado segurado: uma reação
  no meio do arraste toca por cima e volta ao laço.
- **Clique esquerdo:** por enquanto, a risadinha (`giggle`); levar ao
  terminal da sessão chega na T4.10, e o direito (soneca) na T4.7.
- **Orçamento:** o arraste pode fazer commit no ritmo do ponteiro, sempre com
  um quadro em voo por vez (os movimentos com um quadro em voo esperam o
  frame callback, que desenha a posição de agora), e para ao soltar.
**Por quê:** os limiares separam clique de arraste do jeito que um
gerenciador de janelas faz; crescer a área de toque só durante o arraste
mantém o resto da tela clicável o tempo todo; andar em múltiplos de D mantém
o bicho nítido e com movimento de pixel art; e o fail-safe impede que uma
pegada perdida deixe a tela inteira presa ao pet.

## 0049 — Posições salvas por monitor, como fração do palco e pela descrição do monitor (2026-10-04)

**Problema:** o Renan arrasta o Zeca para onde quiser, em cada monitor, e o
lugar tem de sobreviver ao restart do pet (PLANO, verificação do M4). Numa
dock o conector muda (`DP-3` vira `DP-5`), a escala e o tamanho
(`aparencia.tamanho`) podem mudar, e a skin também.
**Escolha (T4.3):**
- **O que se guarda:** o ponto dos pés (a âncora `pe` da skin) como fração
  do palco (pixels do dispositivo do monitor), de 0 a 1 nos dois eixos. Ao
  voltar, a célula é posta com os pés na fração e o corpo é preso de novo na
  área útil: outra escala, outro D ou outra skin caem no mesmo lugar
  relativo.
- **A chave:** a descrição do monitor (fabricante, modelo e série), sem o
  conector que o Hyprland põe no fim da descrição do `wl_output`
  (`… (eDP-1)`); sem descrição, o nome do conector (`nome:eDP-1`). No
  máximo 32 monitores; o usado há mais tempo sai.
- **Quando:** guardada ao soltar um arraste (e no fail-safe); aplicada sempre
  que o palco é montado (a camada ficou pronta, mudou de escala ou de
  tamanho, trocou de skin). Sem posição salva, o canto inferior direito da
  área útil, como antes.
- **Onde:** `/state/posicoes.json` (o volume do pet, o mesmo da aprovação),
  gravado de uma vez (arquivo temporário e `rename`) pelo núcleo do daemon
  depois do lote que mudou a posição. Um arquivo que não se lê vira um
  aviso no log e o canto padrão; uma entrada ruim cai sozinha. O Motor só
  guarda e entrega o JSON: continua sem I/O.
**Por quê:** a fração dos pés é o que o olho percebe como "o mesmo lugar" em
monitores de tamanhos diferentes, e a descrição é o que identifica o
monitor físico. Gravar só ao soltar não custa nada parado.

## 0050 — O leitor do socket2: uma thread que só lê, só ids e booleanos, e o anel de ativações (2026-10-04)

**Problema:** seguir o monitor ativo, esconder durante a proteção de tela e
achar o terminal de uma sessão pedem os eventos do Hyprland
(`.socket2.sock`, decisão 0006). O Hyprland desconecta um cliente com 64
eventos acumulados, e o spinner do Claude Code no título do terminal gera
~4 eventos por segundo; as linhas trazem títulos de janela e classes, que
nunca podem chegar ao log, ao `/v1/estado`, ao `/v1/debug/eventos` nem ao
disco.
**Escolha (T4.4):**
- **Uma thread só para ler** (`pet_wayland::hyprland::eventos::Leitor`):
  drena sempre, traduz cada linha na hora e entrega ao laço por uma caixa
  própria (1024 vagas, acordando o laço por um `Ping`; uma caixa cheia
  conta o evento como perdido, nunca segura a leitura). Cai a ligação, ela
  conta (`Ligado(false)`), espera o backoff (1 s dobrando até 30 s; 60 s de
  ligação zeram) e liga de novo. Parar desbloqueia a leitura (`shutdown`) e
  a espera (condição), na hora.
- **O que sai de uma linha:** o nome do monitor em foco (`focusedmonv2`; o
  FALLBACK nunca), o endereço da janela ativa (`activewindowv2`, carimbado
  com a hora de parede em que a linha foi lida), o booleano "o título da
  janela em foco começa com ✳, ◐ ou ◑" (`activewindow`: só os 4 primeiros
  bytes do título são olhados, e só o booleano sai), o endereço de uma janela
  que abriu com o booleano "é a proteção de tela do Omarchy" (`openwindow`)
  e o de uma que fechou (`closewindow`), e "um monitor entrou ou saiu". Nomes
  e endereços passam por regras (`[A-Za-z0-9._-]`, hexadecimal); o resto
  (títulos, classes, áreas de trabalho, `windowtitle`, …) nem é
  interpretado. As repetições (o `activewindow` reenviado a cada troca de
  título) não acordam o laço. Linhas de mais de 4 KiB são puladas sem serem
  guardadas. Do log, só "ligado", "caiu" e o atraso.
- **Ciclo de vida:** o leitor nasce quando a descoberta acha a instância
  (antes do handshake do Wayland: com a conexão Wayland em backoff, os
  eventos já chegam) e morre quando não há mais Hyprland; uma instância nova
  troca o leitor. O laço esvazia a caixa do desktop antes da dos hooks: uma
  troca de janela que veio antes de um prompt entra no anel antes dele.
- **O anel de ativações** (`pet_core::motor::janelas::Anel`, no Motor): até
  128 trocas de janela ativa, só o endereço e a hora de parede; a fonte caiu,
  um buraco (até a próxima troca, nada se sabe); a semente do
  foreign-toplevel (T4.9) só entra com o anel vazio ou num buraco. A busca
  pela hora (`Anel::em`) diz a janela, ou dúvida se a troca para ela foi há
  menos de 1 s (a primeira troca do anel, ou a primeira depois de um buraco,
  não tem dúvida: não se sabe o que havia antes), ou nenhuma, ou
  desconhecida. O `/v1/estado.desktop.anel` mostra as 8 últimas trocas
  (endereço, hora e tipo).
- **O canário** (`tests/socket2.rs`): o daemon de verdade com uma instância
  de mentira do Hyprland (`hyprland.lock`, um socket2 que manda
  `activewindow>>firefox,SEGREDO-T`, um `openwindow` e um `windowtitlev2` com
  segredos depois das linhas úteis e uma sentinela no fim, e um Wayland que
  desliga na hora): nada com `segredo` nem `firefox` no `/v1/estado`, no
  `/v1/debug/eventos` nem no log com `PET_LOG=debug`, e o `.socket.sock` da
  instância nunca recebe conexão. Duas versões erradas de propósito
  reprovaram: a linha crua no log de debug e a classe do `openwindow` saindo
  como monitor em foco.
**Por quê:** ler numa thread que nunca espera é o que o Hyprland exige;
traduzir na hora e só para ids e booleanos é o que a regra de ouro exige; e
casar o prompt com a janela pela hora, num anel em que um buraco é
"não sei" e não "a de antes", evita mandar o Renan para a janela errada.

## 0051 — Seguir o monitor ativo: debounce, intervalo entre viagens, o poof procedural e o pouso entre monitores (2026-10-04)

**Problema:** o Zeca tem de ficar sempre no monitor em foco (PLANO, "Seguir o
monitor ativo"). A camada OVERLAY é de um monitor só e não pode atravessar
para o outro; o foco pisca em rajadas (a proteção de tela do Omarchy foca
cada monitor em sequência; o mouse cruzando a borda); e o pet pode ser
arrastado e solto em outro monitor.
**Escolha (T4.5):**
- **Quando ir** (`pet_core::motor::viagem`, puro): o `focusedmonv2` vira
  `MonitorEmFoco` (nunca o FALLBACK); só o último vale, depois de 300 ms
  parado. Não viaja arrastando (confere de novo ao soltar), nem com o pet
  escondido (ao voltar, a camada nasce no monitor em foco), nem sem a camada
  pronta (confere quando ela ficar), nem a menos de 1,5 s da viagem anterior
  (adia até lá). A quarta viagem em 20 s é rápida: sem poof. O
  `configreloaded` nem é lido; o FALLBACK e a camada sem casa continuam como
  no M1.
- **A viagem:** o poof de saída (4 passos de 60 ms; o pet some no segundo),
  o quadro transparente e a destruição da camada, a camada nova com output
  NULL (o compositor a põe no monitor em foco), o palco na posição salva
  daquele monitor e o poof de chegada (o pet aparece no segundo passo). A
  animação em curso continua no monitor novo. Esconder no meio desiste da
  viagem; a janela fechada pelo compositor no meio (o monitor saiu) espera o
  `Recriar` de sempre e chega do mesmo jeito.
- **O poof** é procedural (a receita do catálogo para `poof_in`/`poof_out`):
  oito bloquinhos de D em volta do corpo, branco gelo e cinza, abrindo na
  saída e fechando na chegada, ≈ 17 quadros por segundo por 240 ms (dentro
  da rajada de até 30). As partículas de verdade são do M6.
- **Soltar entre monitores:** o botão subindo fora do palco (a pegada
  implícita manda as coordenadas além da borda) vira um pouso: o ponto no
  desktop (a origem do monitor do `xdg_output` mais o ponto em pixels
  lógicos) e a pegada. A viagem começa na hora (sem debounce nem
  intervalo); na chegada, a célula fica com a pegada debaixo do ponto
  solto, presa à área útil, o pet toca o pouso e a posição fica guardada
  para o monitor novo. Sem a origem do monitor, cai na posição salva.
- **Mudança de layout:** um `wl_output`/`xdg_output` que muda faz a camada
  conferir de novo onde está (a origem nova vale para o próximo pouso).
- O `/v1/estado.viagem` mostra a fase (`poof`, `saindo`, `chegando`,
  `entrando`).
**Por quê:** o debounce e o intervalo seguram as rajadas sem deixar o pet
para trás; o poof esconde o salto entre camadas (uma camada não atravessa
monitores); e o pouso pelo ponto do desktop faz o arraste entre monitores
terminar onde o Renan soltou.

## 0052 — A fonte monogram (CC0) assada no core e o balão mínimo (2026-10-04)

**Problema:** o clique sem pendência mostra um balão com as sessões abertas
(decisão 0039), e o balão precisa de uma fonte de pixel com acento (o PLANO
escolheu a monogram, CC0, para o M6; o M4 a puxa). A fonte tem de ser livre
(CC0 ou OFL), com a licença no repositório, nítida como a arte (blocos
inteiros de pixels do dispositivo) e sem nada baixado em tempo de execução.
**Escolha (T4.6):**
- **A monogram**, de Vinícius Menézio (datagoblin), CC0 1.0: baixada do
  itch.io em 2026-10-04 (`monogram.zip`, sha256 81d05402…). O repositório
  guarda só o `monogram-bitmap.json` do pacote (390 glifos com o latim
  completo e os acentos do português; cada um em 12 linhas de bits, o bit 0
  na coluna da esquerda; avanço de 6), o `credits.txt` original, o texto da
  CC0 e um `LICENCA.md` com a origem e os sha256. O `NOTICE.md` dá o
  crédito.
- **Assada no core:** `cargo xtask fonte` gera
  `crates/pet-core/src/fonte/glifos.rs` (uma tabela em ordem de código, já no
  formato do `rustfmt`; `--conferir` só confere); um teste do core compara a
  tabela com o JSON e um do xtask, o arquivo com o gerado. A fonte vai
  dentro do binário. Sem o glifo, o do `?`.
- **O glifo na cena:** `Elemento::Glifo` (o caractere, o canto, o bloco e a
  cor), desenhado em blocos inteiros, um retângulo por trecho aceso de cada
  linha; o dano é o retângulo dele, como o de um sprite.
- **O balão mínimo** (`pet_core::motor::balao`): um quadro creme com borda
  de tinta e cantos de um pixel cortados, o rabinho de dois pixels
  apontando para o meio do corpo, o texto na monogram com um pixel da fonte
  valendo metade do D (para cima; 3 pixels do dispositivo com o Zeca
  pequeno no eDP-1). Fica em cima do corpo, preso dentro da área útil; sem
  espaço em cima, embaixo, com o rabinho virado. Até 6 linhas (a sexta vira
  "+ N") de até 36 caracteres (cortadas com "…"). Some sozinho: 5 s mais 1 s
  por linha, até 12 s (dois commits: aparecer e sumir). O arraste e a viagem
  para outro monitor o tiram. O `/v1/estado.balao` mostra as linhas.
- **O clique esquerdo**, por enquanto: a risadinha e o balão com as sessões
  abertas, uma por linha ("projeto: estado (há quanto tempo)", as reais
  antes das de teste, a mais recente primeiro; "nenhuma sessão do Claude
  aberta" sem nenhuma). O cérebro passou a guardar desde quando cada sessão
  está no estado (`estado_desde_ms` no `/v1/estado.sessoes`). Levar ao
  terminal de uma sessão pendente chega na T4.10, e o "pronto" com os avisos.
**Por quê:** a monogram cobre o português, é de domínio público e foi feita
para pixel art; assada no binário, não depende de arquivo nem de rede, e o
teste garante que a tabela é a do pacote. O balão em blocos inteiros fica
nítido como o Zeca; sumir sozinho mantém o orçamento de commits parado.

## 0053 — Esconder durante a proteção de tela e a soneca do botão direito (2026-10-04)

**Problema:** o PLANO pede que o Zeca se esconda durante a proteção de tela
do Omarchy (a janela `org.omarchy.screensaver`, uma por monitor, em tela
cheia; o OVERLAY ficaria por cima dela) e que o botão direito ponha o pet
para cochilar por 30 min, só com reações pequenas e um selo "zZ".
**Escolha (T4.7):**
- **Proteção de tela:** o `openwindow` do socket2 traz a classe só para virar
  o booleano "é a proteção de tela" (decisão 0050); o `EstadoDesktop` guarda
  os endereços dessas janelas e o `closewindow` os tira. Com alguma aberta,
  o `quer_mostrar` do Motor é falso: o pet sai com o quadro transparente e
  volta (camada nova, no monitor em foco) quando a última fecha. Se a fonte
  dos eventos cai e volta, a lista recomeça vazia: uma proteção que fechou
  no meio nunca diria que fechou. O `/v1/estado.desktop.protetor_de_tela`
  mostra o booleano.
- **Soneca:** o clique direito começa 30 min de soneca (o bocejo, `yawn`) ou,
  se o pet já cochila, acorda (o despertar, `wake`; a skin `_teste` não tem
  e fica na pose). Na soneca, as reações do cérebro ficam pequenas: o
  pulinho (e o que vier maior no M5) vira o aceno; o tchau continua. O selo
  "zZ" fica no alto, à direita do corpo, em letras da monogram creme com
  sombra de tinta (legível no tema escuro e no claro), parado: só um commit
  para aparecer e um para sumir. A soneca acaba sozinha no prazo. Não
  persiste num restart (os comandos persistidos `soneca` e `acordar` do
  `bin/pet` são do M7). O `/v1/estado.soneca_restante_s` mostra quanto
  falta.
**Por quê:** a proteção de tela é o Renan longe do computador, e o pet por
cima dela só gastaria GPU. A soneca é o "me deixa trabalhar" de um clique, e
o selo diz por que o pet está quieto sem pedir commit nenhum enquanto dura.

## 0054 — Os ids de terminal no fio v1: o campo `term`, só no começo da sessão e em cada prompt (2026-10-04)

**Problema:** a janela de uma sessão vem do anel de ativações (decisão 0043),
mas duas sessões no mesmo terminal (dois painéis do tmux, duas abas do
kitty ou do WezTerm) caem na mesma janela, e nada as separa. O PLANO pede um
campo novo e opcional do fio v1, com decisão própria, com os ids de
terminal que o hook vê no próprio ambiente — só ids, nunca títulos.
**Escolha (T4.8):**
- **O campo:** `term`, um objeto com até três ids: `tmux` (o
  `$TMUX_PANE`: `%` e de 1 a 10 dígitos), `kitty` (o `$KITTY_WINDOW_ID`) e
  `wezterm` (o `$WEZTERM_PANE`), esses dois de 1 a 10 dígitos. Opcional
  como todo campo do fio v1.
- **No hook** (`bichinho avisar`, `pet_core::aviso::terminal`): só no
  `SessionStart` e no `UserPromptSubmit` (é quando a janela é casada), e
  cada id só sai se passar no validador do pet; um ruim cai sozinho, os bons
  do mesmo evento saem. Nada mais do ambiente sai (`TERM_PROGRAM`, chaves,
  tokens; os canários já plantavam segredos nele).
- **No pet** (`pet_core::evento`): um objeto com os ids conhecidos, todos
  válidos; um ruim derruba o campo inteiro (só o nome `term` vai para os
  descartados), uma chave desconhecida é ignorada sem rastro (um hook mais
  novo não derruba o pet) e um objeto vazio é ausente.
- **O `avisar.sh`** de reserva não manda o `term`: o campo é opcional, e a
  reserva fica como estava.
- **Canários** (`tests/hook.rs`): com `TMUX_PANE`, `KITTY_WINDOW_ID` e
  `WEZTERM_PANE` válidos, `TERM_PROGRAM` e uma chave da API no ambiente, os
  ids saem no começo e no prompt, nenhum outro evento os leva, nada com
  `segredo` sai e o pet aceita o corpo inteiro; ids que não são ids (`%7;
  rm -rf …`, `SEGREDO-janela`) não saem. Duas versões erradas de propósito
  reprovaram: o `term` em todo evento e o painel do tmux sem validar.
- **A troca:** o plugin não muda (o `hooks.json` chama o mesmo `bichinho
  avisar`); o binário novo chega ao PATH pelo `bin/pet instalar-host` depois
  do merge, como na decisão 0045. Até lá, o hook instalado não manda o
  `term`, e o pet funciona sem ele.
**Por quê:** os ids de terminal são números que o terminal põe no ambiente
de todo processo filho; não dizem o que o Renan faz, só onde. Mandá-los só
quando a janela é casada mantém o fio pequeno, e o validador nos dois lados
mantém a regra de que o que o pet descartaria nem sai do host.

## 0055 — A janela de cada sessão: o anel casado com o `ts` do hook, pegajosa e com dúvida (2026-10-04)

**Problema:** o clique no Zeca leva ao terminal da sessão (decisão 0039). No
Docker o daemon não vê PIDs do host, e o socket2 e o foreign-toplevel não
trazem PID (decisão 0043): a janela da sessão tem de vir da hora em que o
Renan mandou o prompt.
**Escolha (T4.8):**
- **O casamento** (`pet_core::motor::janelas::Identidades`, no Motor): no
  `SessionStart` e no `UserPromptSubmit` de uma sessão que o cérebro
  acompanha, a hora do evento (o `ts` do hook, se plausível; senão a
  chegada) procura no anel de ativações (decisão 0050) a janela ativa
  naquela hora. A janela certa vira a da sessão.
- **Dúvida:** se a troca para aquela janela foi há menos de 1 s, há dúvida
  (o prompt pode ter saído da janela de antes); sem anel na hora (o pet
  subiu depois, a fonte caiu) não se sabe; numa área vazia, nenhuma janela.
- **Pegajosa:** uma janela certa fica até outra certa trocá-la (um
  `--resume` em outro terminal) ou ela fechar (`closewindow`). Um prompt com
  dúvida não apaga a janela certa de antes; só sem nenhuma a dúvida é
  guardada. Os ids de terminal do hook (decisão 0054) ficam junto.
- **O fim:** a identidade some com a sessão (o `SessionEnd`, ou a sessão que
  expirou).
- **No `/v1/estado.sessoes[].janela`:** o endereço (o do `activewindowv2`),
  a certeza (`certa`, `duvida`, `sem_anel`, `sem_janela`, `fechou`) e os ids
  de terminal. O clique (T4.10) usa a janela certa; sem ela, o balão diz o
  porquê.
- **Conferido no daemon de verdade** (`tests/janela.rs`), com linhas
  gravadas de um socket2 e eventos de hook com o `ts`: o prompt casa a
  janela ativa, a troca meio segundo antes deixa dúvida, passado o segundo o
  prompt seguinte casa, e o `closewindow` tira a janela.
**Por quê:** o terminal de uma sessão não muda de janela, e quase todo
prompt sai do teclado na janela ativa; casar pela hora acerta sem ler
títulos, e guardar a dúvida em vez de chutar evita mandar o Renan para a
janela errada.

## 0056 — Focar a janela pelo foreign-toplevel e o mapeamento do Hyprland, na conexão e no `wl_seat` da sessão (2026-10-04)

**Problema:** o clique no Zeca foca o terminal de uma sessão sem o socket
de comandos do Hyprland (decisões 0006 e 0039): pelo
`zwlr_foreign_toplevel_handle_v1.activate(seat)` do handle certo. O handle
vem do foreign-toplevel; o endereço da janela (o do `activewindowv2`, que o
anel guarda) só vem do `hyprland_toplevel_mapping_manager_v1`, que não tem
crate em Rust. E o `generate_interfaces!` do `wayland-scanner` gera a ponte
para a libwayland em C, com `unsafe` — proibido fora dos esboços.
**Escolha (T4.9):**
- **Conferido nesta máquina** (`cargo xtask globais`, que só lê o registro
  do Wayland): o Hyprland 0.56.2 anuncia `zwlr_foreign_toplevel_manager_v1`
  v3 e `hyprland_toplevel_mapping_manager_v1` v1. E o código do Hyprland
  confere o resto: o `activate` faz `activate(true)` (passa por cima das
  regras de foco, muda para a área de trabalho da janela e leva o ponteiro);
  o endereço do mapeamento é o ponteiro da janela, o mesmo
  `std::format("{:x}", …)` dos eventos do socket2.
- **O protocolo:** o XML do hyprland-protocols (BSD-3-Clause) vendorado sem
  mudança em `crates/pet-wayland/protocolos/`; o código de cliente pelo
  `wayland_scanner::generate_client_code!` (dependência direta do
  `pet-wayland`, já no `Cargo.lock`); as tabelas das duas interfaces
  escritas à mão (`c_ptr: None`: o pet usa só o backend em Rust puro), sem
  `unsafe`, com um teste que as confere contra o XML (pedidos, eventos,
  argumentos e destrutores).
- **O foreign-toplevel genérico** (`pet_wayland::toplevel`, serve aos outros
  wlroots no T8.7): as janelas anunciadas, cada uma com o endereço e se está
  ativa — nunca o título nem o app id, que chegam e são jogados fora. Cada
  toplevel novo é mapeado na hora (`get_window_for_toplevel_wlr`), e o
  handle do mapeamento é destruído depois da resposta. Com o
  `WAYLAND_DEBUG` de cliente ligado, o wayland-client imprime toda mensagem
  no stderr, títulos inclusive: aí o foreign-toplevel nem é ligado (aviso no
  log) e o clique cai no balão.
- **O desktop da sessão:** a `Sessao` liga os dois protocolos na própria
  conexão (opcionais: sem algum, o aviso no log e o clique cai no balão);
  `Desktop::focar(endereço)` acha o handle e faz `activate` no `wl_seat` da
  mesma conexão (`JanelaSumiu` se o endereço não está mais lá,
  `NaoSuportado` sem os protocolos ou sem seat). O `/v1/estado.desktop`
  mostra os protocolos ligados e quantas janelas têm endereço.
- **A semente:** a janela ativa pelo foreign-toplevel (com endereço) vira
  `JanelaInicial`; o anel a usa com ele vazio, num buraco ou depois de outra
  semente, até o socket2 contar uma troca (a primeira semente não tem
  dúvida; as seguintes são trocas que o foreign-toplevel viu).
- **Conferido ao vivo** com a sessão bloqueada e a tela apagada, num daemon
  nativo de rascunho (porta 27399, `/state` de rascunho, skin `_teste`,
  parado com SIGTERM em seguida): os dois protocolos ligados, a única janela
  aberta mapeada com o endereço que o `hyprctl -j clients` (só leitura, no
  host) dá para o `foot`, a semente no anel e o socket2 ligado. Focar de
  verdade fica pendente: bloqueado, o Hyprland recusa o foco a janelas.
**Por quê:** o foreign-toplevel é uma ação de alto nível sobre uma janela,
sem executar nada no host; o mapeamento é o que liga o handle ao endereço
que o anel conhece. Escrever as tabelas à mão é o preço de não ter `unsafe`,
e o teste contra o XML garante que elas são as do protocolo.

## 0057 — Os avisos das sessões e o clique que leva ao terminal, em ciclo (2026-10-04)

**Problema:** o clique esquerdo no Zeca leva à janela do terminal da sessão
que terminou ou que precisa do Renan (decisão 0039): com vários avisos, o
mais urgente primeiro (esperando você > erro > pronto) e cada clique ao
próximo; o clique que foca marca o aviso como visto; o pronto some com uns
10 s do terminal da sessão em foco, e o "esperando você" só com um evento da
própria sessão ou um clique. Sem pendência, o balão com as sessões; sem como
focar, o balão diz o porquê e mostra a lista.
**Escolha (T4.10):**
- **Os avisos moram no cérebro** (`pet_core::cerebro`): um espaço por
  sessão, com o tipo e desde quando.
  - Mudar de estado resolve o aviso de antes; entrar em "esperando você"
    (o `PermissionRequest`, a pergunta e o plano pelo `PreToolUse`, as
    notificações que pedem o Renan) ou em erro (o `StopFailure`) abre um.
  - O pronto abre quando a acomodação do Stop termina (o instante da festa,
    com ou sem festa: a celebração desligada não tira o aviso), desde a hora
    do Stop. A continuação de outro plugin, que reabre o turno, o tira; o
    Stop seguinte o devolve. Um prompt novo o resolve.
  - Um segundo gatilho do mesmo diálogo (a notificação depois do
    `PermissionRequest`) não abre outro, nem depois de visto. O
    `idle_prompt` nunca abre nem resolve: ele fecha o turno e para a
    sessão, mas a permissão continua na tela. Um evento atrasado, com `ts`
    mais velho que o estado de agora (a permissão que a ferramenta já
    usou), não mexe no aviso.
  - O pronto e o erro somem sozinhos em 2 h; o "esperando você" só com um
    evento da sessão, visto, ou com a sessão (o `SessionEnd`, as 12 h).
  - No `/v1/estado.sessoes[].aviso`, o tipo (`esperando`, `erro`, `pronto`)
    e desde quando; na lista do balão, a sessão parada com o pronto aparece
    como "pronto".
- **A ordem** (`Cerebro::pendencias`): o tipo mais urgente; no mesmo tipo,
  as sessões reais antes das de teste e a que espera há mais tempo primeiro.
- **O clique esquerdo** (`Motor::clicar`) vai ao aviso da vez: o mais
  urgente que esta volta do ciclo ainda não visitou (visitados todos,
  recomeça). Com a janela certa da sessão (decisão 0055), pede o foco ao
  desktop (`Desktop::focar`, o foreign-toplevel da decisão 0056), com a
  risadinha e um coração procedural (7 por 6 pixels da fonte, vermelho com
  borda de tinta, 1,2 s parado: dois commits). O aviso sai quando o desktop
  conta que a janela ficou ativa (o `activewindowv2` do socket2 ou o
  `activated` do foreign-toplevel), em até 1,5 s; já ativa, ou sem quem
  conte as trocas, sai na hora. Sem a confirmação no prazo (bloqueado, o
  Hyprland recusa o foco sem dizer nada), o aviso fica e o balão diz "não
  consegui focar a janela dela".
- **Sem como focar**, o balão traz a sessão com o aviso, o porquê e a lista
  das sessões, e o ciclo anda do mesmo jeito. Os porquês: da identidade
  ("não vi a janela dela", "trocou de janela perto do prompt", "nenhuma
  janela estava ativa", "a janela dela fechou") e do desktop ("aqui eu não
  sei focar janelas", "a janela dela sumiu", "o sistema recusou o foco").
  Sem aviso, a risadinha e a lista (decisão 0052).
- **O foco sem clique:** o pronto e o erro de uma sessão saem com 10 s do
  terminal dela em foco (contados do aviso ou de quando a janela ficou
  ativa, o que vier depois), só com a fonte das trocas ligada; o "esperando
  você" não sai assim.
- **`/v1/comando` `clique`** (`"esquerdo"`, o padrão, ou `"direito"`) e
  `bin/pet clique`: clicam no pet como o ponteiro e respondem o que ele fez
  (`focou`, com o endereço da janela e se o desktop já confirmou; `balao`,
  com o porquê; `lista`; `soneca`; sem compositor, `nada`). É por onde os
  scripts ao vivo conferem o clique, sem título nenhum.
- **Publicar:** um aviso visto fora de um evento do Claude (o clique, a
  confirmação, o foco) republica o `/v1/estado.sessoes`; apertar e soltar o
  botão republicam o painel, com o `focando` (o endereço que espera a
  confirmação). No log, só o endereço, o id curto da sessão e o tipo do
  aviso; o nome do projeto fica no balão e no `/v1/estado`, como antes.
**Por quê:** o aviso é o estado da sessão visto pelo lado do Renan, o que
ele ainda não viu, e o cérebro já sabe o estado. Esperar a confirmação do
desktop evita marcar como visto um foco que não aconteceu, e a memória da
volta evita que uma sessão sem janela prenda o clique nela.

## 0058 — A verificação ao vivo do M4, a produção na branch e a regra opcional só proposta (2026-10-04)

**Problema:** o M4 termina com a verificação na tela de verdade e a
produção refeita da branch (PLANO, M4). Parte dela pede a tela desbloqueada
ou o Renan com o mouse; o e2e de monitores mexe no Hyprland dele (cria e
remove um monitor); e a regra opcional do Hyprland só pode ser proposta,
nunca aplicada sem ele.
**Escolha (T4.11):**
- **`scripts/verificar-m4.sh`**, contra a produção que está de pé: o
  desktop no `/v1/estado` (o socket2 ligado, os dois protocolos do clique,
  janelas com endereço, o anel e as janelas das sessões só com endereços em
  hexadecimal), a janela ativa e o monitor em foco iguais aos do Hyprland
  (`hyprctl -j`, só leitura) e, com a sessão desbloqueada e a tela acesa, o
  clique de ponta a ponta: dois `foot` com um título-canário
  (`SEGREDO-M4-…`), uma sessão de teste casada com cada um pelo `ts` do
  prompt, A pedindo permissão e B pronta; o primeiro `bin/pet clique` tem de
  levar ao foot de A e o segundo ao de B (o `hyprctl -j activewindow`), e o
  terceiro mostra a lista; o título nunca aparece no log, no `/v1/estado`
  nem no `/v1/debug/eventos`. Os `foot` fecham num `trap`. Com a sessão
  bloqueada ou a tela apagada, sai NÃO VERIFICADO sem abrir janela; com
  avisos de sessões reais pendentes também (o clique iria a eles primeiro).
  `--manual` guia o Renan: arrastar numa área cheia e numa vazia, a posição
  depois de um restart, a proteção de tela e o clique com o mouse.
- **`scripts/e2e-monitor.sh --autorizo`**: cria um output headless, foca
  nele, confere o pet lá (a camada viva e o `/v1/estado.monitor`), volta,
  confere, remove e confere de novo, com um `trap` que remove o output e
  devolve o foco. É o único script que muda o Hyprland (`hyprctl output` e
  `dispatch focusmonitor`): escrito, nunca rodado sem o consentimento do
  Renan, a cada vez.
- **A regra opcional** (`order = 1` para ficar abaixo dos popups do
  Omarchy, `no_anim` para tirar o fade de ~180 ms de cada troca de
  monitor): proposta no README, a aplicar só pela skill `omarchy` e com o
  consentimento do Renan. Não foi aplicada; sem ela tudo funciona.
- **A linha da lista de sessões:** o nome do projeto encolhe (até 6
  caracteres) para o estado e o tempo caberem nos 36 do balão; ao vivo, a
  linha de uma sessão de teste saiu "m4-vivo (teste): esperando você (0 …".
- **Conferido ao vivo**, com a sessão bloqueada e a tela apagada, na
  produção refeita da branch (`bin/pet subir` do commit da T4.10): tela
  ativa, o Zeca aprovado pelo Renan intacto (sha256 5b843b03…), D=6 (o
  tamanho pequeno no eDP-1), o socket2 ligado, os dois protocolos ligados
  com a janela aberta mapeada, a semente no anel e nenhum título no log. O
  `scripts/verificar-m4.sh` passou no desktop e na janela ativa e deu o
  clique nos dois `foot` como NÃO VERIFICADO, sem abrir janela. Uma sessão
  de teste casou com a janela ativa (certa, pela semente), e o
  `bin/pet clique` focou a janela dela (já ativa: confirmado na hora) pelo
  `activate` do foreign-toplevel de verdade, sem erro de protocolo nem
  reconexão; o aviso saiu, o segundo clique mostrou a lista e o direito
  ligou e desligou a soneca.
- **Pendente** (pede a tela desbloqueada ou o Renan): o clique nos dois
  `foot`, o `--manual`, o e2e de monitores e a regra opcional.
- **A troca depois do merge:** o plugin não muda (0.2.0); na worktree
  estável, `bin/pet subir` e `bin/pet instalar-host` (o hook novo manda o
  `term`; o antigo continua funcionando com o pet novo, sem ele).
**Por quê:** o que dá para provar sem a tela fica provado no daemon de
verdade e na produção; o que pede a tela fica escrito para rodar com o
Renan, sem mexer no Hyprland dele nem focar janelas enquanto ele não está.

## 0059 — Revisão do seguir o foco: os prazos do Motor sem conexão e o alvo velho (2026-10-04)

**Problema:** as revisões do M4 acharam dois defeitos no seguir o monitor
ativo (T4.5).
- **O laço girava a 100% de CPU sem a conexão Wayland.** O M4 pôs no
  `Motor::proximo_prazo` prazos que só o `Motor::vencer(punho)` tirava: o
  debounce do foco, o fim do balão e o da soneca. Sem conexão (o compositor
  caiu, ou a conexão está no backoff) o `Nucleo::vencer` pulava o Motor, e o
  laço rearmava o mesmo prazo vencido sem fim. O leitor do socket2 vive
  desde a descoberta e sobrevive ao backoff (decisão 0050): bastava o mouse
  cruzar para o outro monitor. Conferido no daemon de verdade com o
  Hyprland de mentira: 200 tiques de CPU em 2 s (um núcleo inteiro).
- **O alvo velho fazia o pet viajar sem fim.** O `Seguir.alvo` nunca era
  corrigido. A camada com output NULL nasce no monitor em foco; se o alvo
  apontava para outro monitor (um `focusedmonv2` perdido enquanto o socket2
  reconectava, o Hyprland reiniciado com o eDP-1 em foco depois de uma
  sessão que terminou no HDMI, o HDMI intermitente), cada chegada decidia
  viajar de novo: a camada era destruída e recriada a cada 1,5 s, sem poof
  depois da terceira, até o Renan trocar de monitor. Com um monitor só,
  nunca.

**Escolha (revisão do T4.5):**
- **`Motor::vencer_sem_conexao`**: o `Nucleo::vencer` sem conexão vence os
  mesmos prazos sem desenhar (o arraste, o de conferir o foco, o balão, a
  soneca, o coração, o foco à espera de confirmação e o quadro). O
  `desconectou` tira o que só existe na janela (o balão, a viagem e o
  prazo de seguir); a soneca fica e acaba no prazo dela.
- **A camada revela o monitor em foco** (`Seguir::pousou`): o `Seguir` conta
  os focos que o desktop mandou e guarda a conta quando uma camada é pedida
  (`Motor::criar_janela`, o único lugar que pede). Quando ela fica pronta
  sem foco novo desde então, o monitor onde ela caiu é o monitor em foco:
  ele vira o alvo (o log diz quando corrigiu um velho) e o
  `/v1/estado.desktop.monitor_em_foco` (que antes ficava nulo até a
  primeira troca). Um foco que chegou depois de a camada ser pedida vale:
  o pet vai atrás dele.
- **A conexão nova esquece o alvo** (`Seguir::esquecer` no `conectou`): a
  camada nova nasce no monitor em foco. A fonte do foco que cai não apaga o
  alvo: o último foco contado é o melhor palpite (um `focusedmonv2` que
  chegou logo antes da queda ainda leva o pet), e a camada corrige se ele
  estiver velho. As revisões pediam apagar; não apagar não deixa laço, e
  apagar perderia uma viagem certa.
- **Testes:** no daemon de verdade, o socket2 com `focusedmonv2` e o Wayland
  no backoff gastam 0 tique em 2 s (eram 200; `tests/socket2.rs`); no Motor
  em relógio falso, nenhum prazo vencido fica armado sem conexão (o balão,
  a soneca e o foco do monitor, vencidos um a um como o laço faz), o alvo
  velho não viaja de novo em 30 s, e o foco que chega depois da camada
  pedida ainda leva o pet; no `Seguir`, o alvo corrigido e o esquecido.

**Por quê:** todo prazo que o Motor anuncia tem de ter quem o vença em
qualquer estado da conexão, senão o laço gira; e o compositor, que pôs a
camada no monitor em foco, sabe mais que um foco antigo que pode ter se
perdido.

## 0060 — Revisão da janela de cada sessão: o começo só preenche, a compactação nunca casa e o hook atrasado não desfaz (2026-10-04)

**Problema:** as revisões do M4 acharam que a janela de cada sessão (decisão
0055) podia grudar no lugar errado com certeza.
- **O `SessionStart` de qualquer origem casava como um prompt**, e uma
  janela certa sempre trocava a de antes. O Claude Code manda
  `SessionStart` com `source: compact` depois de uma compactação, que
  acontece no meio de um turno longo, com o Renan em qualquer janela (o
  `hooks.json` registra o evento sem filtro, e o hook manda o `src`). O
  navegador virava o "terminal" da sessão: 10 s com ele em foco davam o
  pronto como visto sem o Renan ter visto, e o clique "focava" o navegador,
  já ativo, e marcava o aviso. Só o próximo prompt no terminal consertava.
  O `SessionStart` de `startup` e `resume` também roda depois de o Claude
  Code subir (1–3 s), com o Renan podendo já estar noutra janela.
- **Hooks assíncronos chegam fora de ordem**, e o `observar` aplicava na
  ordem de chegada: um prompt atrasado desfazia o casamento de um mais novo.
- **Um prompt que não veio do teclado** (o `source` do `UserPromptSubmit`,
  que o cérebro já guarda: `user`, `system`) casaria do mesmo jeito.

**Escolha (revisão do T4.8):**
- **De onde vem a hora** (`motor::janelas::origem`): o `UserPromptSubmit`
  sem `src` ou com `user` é o prompt do teclado e troca a janela; o
  `SessionStart` sem `src` ou com `startup`, `resume`, `clear` ou `fork` é
  o começo e **só preenche** uma janela que ainda não é certa (sem
  identidade, com dúvida, sem anel, sem janela ou com a janela fechada); a
  compactação (`compact`), um prompt de sistema e qualquer origem
  desconhecida **não casam**.
- **O hook atrasado** (`Identidades::observar`): uma observação com a hora
  mais velha que a do último casamento da sessão é ignorada, ids de terminal
  inclusive.
- O hook continua mandando os ids de terminal no `SessionStart` de toda
  origem (decisão 0054); o pet só não casa a janela com o da compactação.
- **Testes:** no Motor, a compactação 30 s depois de o Renan ir ao navegador
  deixa a sessão no foot e o pronto pendente depois de 10 s com o navegador
  em foco; o prompt de sistema não casa e o do teclado troca; o prompt
  atrasado não desfaz o mais novo; o `resume` com o Renan noutra janela não
  troca a certa, e o começo de uma sessão nova preenche. Na função de origem
  e nas identidades, as mesmas regras, conferidas por três mutações (a
  compactação casando, sem a guarda do atrasado, o começo trocando a
  certa). No daemon de verdade (`tests/janela.rs`), a compactação com a
  terceira janela ativa há mais de 1 s não troca a janela da sessão.

**Por quê:** o prompt do teclado é o único momento em que a janela ativa
é, com quase certeza, o terminal da sessão; o resto ou preenche um vazio ou
fica de fora. Errar com certeza manda o Renan para a janela errada e apaga
um aviso que ele não viu.

## 0061 — Revisão do socket2 e do foreign-toplevel: o daemon sem core dump, a caixa cheia vira buraco e a semente volta com a fonte (2026-10-04)

**Problema:** as revisões do M4 acharam três falhas no que o daemon lê do
desktop (T4.4 e T4.9).
- **Títulos podiam ir ao disco num core dump.** Desde o M4 o daemon tem
  títulos de janela na memória: as linhas cruas do socket2
  (`activewindow>>CLASSE,TÍTULO`, `windowtitlev2>>…`) passam pelo buffer do
  leitor, e o título e o app id de cada janela do foreign-toplevel viram
  `String` e são jogados fora (os bytes ficam no heap liberado). O release
  usa `panic = "abort"` e o vigia aborta um laço travado; o `core_pattern`
  do host é o `systemd-coredump`, o container tinha o core ilimitado e o
  processo era `dumpable`. O hook já se protegia (decisão 0045); o daemon,
  não. Isso quebrava a regra de ouro de que título nenhum chega ao disco.
- **Eventos perdidos eram invisíveis.** Com a caixa do desktop cheia, o
  evento só era contado; a memória das repetições do leitor seguia com um
  valor que o laço nunca recebeu. Uma troca perdida para a janela B deixava
  o anel em A, a próxima `activewindowv2>>B` era filtrada como repetida, e
  um prompt mandado em B casava com A, com certeza.
- **Depois de o socket2 voltar, nada ressemeava o anel.** O anel ficava no
  buraco até o Renan trocar de janela, e os prompts do mesmo terminal davam
  "não vi a janela dela", embora o foreign-toplevel da conexão soubesse a
  janela ativa (a semente só saía numa troca de ativação).

**Escolha (revisão do T4.4):**
- **Sem core dump** (`bichinho::privacidade::sem_core_dump`, o mesmo do
  hook, agora num lugar só): o `daemon::rodar` tira o `dumpable` antes de
  tudo, antes do leitor do socket2 e da conexão Wayland. Sem ele o kernel
  não faz o core, e o `/proc/<pid>` passa a ser do root (proc(5)). Como
  segunda camada, o compose põe `ulimits: core: 0` (o `systemd-coredump`
  recusa um processo com o limite abaixo de uma página). Testes no binário
  de verdade: o `/proc/<pid>/status` do daemon e o do hook (com a entrada
  ainda aberta) são do root; os dois reprovaram sem a chamada.
- **A caixa cheia vira buraco** (`eventos::Entrega`): um evento que não
  coube zera a memória das repetições e marca a perda; antes do próximo
  evento o leitor manda `Ligado(false)` e `Ligado(true)` (o anel ganha um
  buraco no lugar das trocas perdidas, e a próxima ativação passa mesmo
  sendo a mesma janela), com um aviso no log com o total perdido. Um
  `Ligado(true)` que não coube na ligação conta como perda. Teste com uma
  caixa de 3 vagas que ninguém esvazia.
- **A semente volta com a fonte** (`Desktop::janela_ativa`, com padrão
  vazio): quando o socket2 conta `Ligado(true)`, o núcleo pede à conexão a
  janela ativa de agora (no Wayland, a janela com `activated` e endereço no
  foreign-toplevel) e a entrega como `JanelaInicial`, que entra no buraco.
  Teste no núcleo com o desktop de mentira.
- **O título do foreign-toplevel conferido** (`sessao::evento_do_toplevel`,
  a costura pura do `Dispatch`): um teste manda o `title` e o `app_id` com
  segredo e confere que nada deles fica nas janelas nem nos eventos. Antes,
  essa garantia era só leitura de código.
- O `perdidos` não foi para o `/v1/estado` (a revisão sugeriu): o buraco no
  anel e o aviso no log já mostram a perda, sem mais um caminho do leitor
  até o painel.

**Por quê:** a memória do daemon tem o que nunca pode ir ao disco, e um
core dump é disco; e o anel só serve se um "não sei" for marcado como não
sei, em vez de virar uma certeza velha.

## 0062 — Revisão dos avisos e do clique: o foco só conta com o Renan presente, a volta do ciclo recomeça e cada clique espera a sua confirmação (2026-10-04)

**Problema:** as revisões do M4 acharam três falhas nos avisos e no clique
(T4.10).
- **O pronto saía como visto com o Renan longe.** A regra "o pronto e o erro
  saem com 10 s do terminal em foco" (decisão 0057) não sabia se havia
  alguém olhando. Bloqueado, ou com a tela apagada, o Hyprland continua
  contando como ativa a última janela que teve o foco (o `activated` do
  foreign-toplevel e o `activewindowv2`): conferido na produção, com a
  sessão bloqueada e o terminal como `desktop.janela_ativa`. Uma tarefa
  longa que terminava com o Renan longe perdia o aviso 10 s depois. O mesmo
  deixava o clique do `/v1/comando` "focar" uma janela já ativa com a sessão
  bloqueada e dar o aviso como visto na hora; a verificação ao vivo do
  T4.11 passou só por isso.
- **A memória da volta do ciclo nunca expirava.** Uma sessão que o clique
  visitou sem conseguir focar ficava "visitada" até todas as outras serem
  visitadas: um clique horas depois pulava a mais urgente.
- **Um segundo clique antes da confirmação apagava a do primeiro.** O
  `focando` era um só: a ativação atrasada da primeira janela não marcava
  mais o aviso dela.

**Escolha (revisão do T4.10):**
- **O Renan presente** (`EventoDesktop::Ocioso`, o `desktop.ocioso` no
  `/v1/estado`): a `Sessao` liga o `ext_idle_notifier_v1` (o Hyprland 0.56.2
  anuncia a v2; genérico, serve aos outros wlroots no M8) no `wl_seat` da
  conexão e pede a notificação de entrada (`get_input_idle_notification`
  na v2, que ignora quem segura a tela acesa, como um vídeo) com
  `OCIOSO_MS` = 5 s; ela nasce "não ocioso", e o `idled` e o `resumed`
  viram `Ocioso(true)` e `Ocioso(false)`. A conexão que cai deixa o valor em
  "não se sabe".
  - O pronto e o erro só saem pelo foco com o Renan presente: os 10 s contam
    desde o mais tarde entre o aviso, a janela ficar ativa e o Renan voltar
    a mexer, e param enquanto ele está longe. Como 5 s é menos que 10 s,
    quem saiu logo antes do aviso já é dado como longe antes de o prazo
    vencer. Sem saber se ele está (um desktop sem o protocolo, a conexão
    caída), o pronto não sai pelo foco: fica até o clique ou o próximo
    prompt, que já o resolvem.
  - Uma janela já ativa só conta como vista na hora do clique com o Renan
    presente. O aperto de verdade no pet conta como presença (o `resumed`
    pode vir na mesma leva, depois); o clique do `/v1/comando` não, e com a
    sessão bloqueada espera a confirmação, que não vem, e o balão diz que não
    focou.
- **A volta do ciclo recomeça** (`VOLTA_DO_CICLO_MS` = 15 s, o tempo de ler
  o balão e clicar de novo): um clique mais tarde que isso depois do
  anterior começa outra volta, do mais urgente.
- **Cada clique espera a sua confirmação**: o `focando` virou uma lista (até
  4); uma ativação marca o aviso de todo clique que esperava aquela janela,
  e só o clique mais novo, sem outro esperando depois dele, mostra o balão
  quando vence. O `/v1/estado.focando` mostra a janela do mais novo.
- **Testes** no Motor em relógio falso: o pronto com o Renan longe fica e
  sai 10 s depois de ele voltar (pausando se ele sair de novo), sem saber se
  ele está o pronto não sai pelo foco, o clique de script com a sessão
  bloqueada não vê a janela já ativa e o do mouse vê, um clique um minuto
  depois volta ao mais urgente, e dois cliques seguidos têm os dois avisos
  vistos; cada regra conferida por mutação. Ao vivo, num daemon de rascunho
  sem personagem (porta 27399, parado com SIGTERM): o `ext_idle_notifier_v1
  v2` ligado, `ocioso` falso na partida e verdadeiro 5 s depois (a sessão
  bloqueada), e o `/proc/<pid>` do root (decisão 0061).
- O `hyprland_lock_notifier_v1` (o Hyprland também anuncia) ficou de fora:
  bloqueado, o Renan já está longe do teclado; ele pediria vendorar mais um
  XML e escrever mais tabelas à mão.

**Por quê:** "o terminal em foco" só quer dizer "o Renan viu" com ele ali;
errar para o lado de deixar o aviso custa um clique, e errar para o outro
apaga o que ele não viu. A volta do ciclo é para cliques seguidos, não para
a tarde inteira.

## 0063 — Revisão do arraste: a pegada perdida solta o pet e o cursor volta quando o aperto desiste (2026-10-04)

**Problema:** as revisões do M4 acharam duas falhas na máquina do ponteiro
(T4.2, decisão 0048).
- **Uma pegada perdida deixava um arraste fantasma.** Um `leave` no meio do
  arraste era ignorado, e o `enter` e o `motion` seguintes rearmavam o
  fail-safe e moviam o pet. Sem a pegada implícita (numa área de trabalho
  vazia, sem nenhuma superfície com o teclado; pesquisa do Hyprland, seção
  3), cruzar para o outro monitor perde o soltar: quando o ponteiro voltava,
  o pet grudava nele sem botão nenhum, o laço do voo seguia fazendo commits,
  a área de toque do palco inteiro capturava o monitor, e o próximo aperto
  era engolido. Com o mouse andando, o fail-safe nunca vencia — justo o que
  a decisão 0048 dizia evitar. O pouso entre monitores também não acontecia.
- **O cursor ficava "agarrando".** Um aperto que acabava sem gesto (o direito
  que andou e soltou, o fail-safe antes de arrastar) não devolvia o cursor
  de "pegar".

**Escolha (revisão do T4.2):**
- **O `leave` no meio do arraste é a pegada perdida**: com a pegada
  implícita o Hyprland só manda o `leave` depois de soltar, então um
  `leave` com o arraste de pé quer dizer que o soltar vai para outro lugar.
  O arraste acaba ali (`Gesto::Cancelou`): o pet pousa onde está, a área de
  toque volta ao corpo, a posição fica guardada e o foco é conferido (o
  ponteiro está no outro monitor: o pet vai atrás dele pelo seguir o foco,
  na posição salva de lá). O pet nunca anda sem um aperto novo.
- **`Gesto::Desistiu`**: o aperto que acaba sem clique nem arraste devolve o
  cursor de "pegar".
- **O cursor depois do `leave`**: a `Sessao` esquece o serial do `enter` no
  `leave`; um pedido de cursor fora da camada não vai ao compositor (o
  próximo `enter` põe o "pegar" de novo).
- **Testes:** na máquina, o `leave` arrastando cancela e o ponteiro que volta
  sem botão não arrasta; o direito que andou e o fail-safe do direito
  desistem. No Motor em relógio falso: arrastar, `leave`, `enter` e andar
  200 pixels deixam o pet onde pousou, sem o voo, com o toque no corpo e
  nada armado; o cursor volta a "pegar" nos dois casos. As duas mudanças
  conferidas por mutação.

**Por quê:** o pet só pode andar com o botão apertado em cima dele; um
"não sei onde o botão subiu" vira um pouso no lugar, e o seguir o foco leva
o pet ao monitor certo.

## 0064 — Revisão da verificação do M4: scripts que não mexem no que é do Renan, o orçamento das peças novas em teste e a troca depois do merge pelo clone (2026-10-04)

**Problema:** as revisões do M4 acharam falhas na verificação e nas docs
(T4.11).
- **`scripts/e2e-monitor.sh`:** o `trap restaurar EXIT INT TERM` não saía no
  Ctrl+C (o bash voltava ao meio do script e seguia trocando o foco); um
  output criado cujo nome só aparecesse depois dos 3 s da descoberta nunca
  era removido; o consentimento era só a flag `--autorizo`, que qualquer
  agente passa; e o dispatch era o antigo (`focusmonitor`), que o config em
  Lua do Hyprland 0.56 pode não aceitar (o Omarchy tenta o `hl.dsp` antes).
- **`scripts/verificar-m4.sh`:** os avisos de sessões reais eram conferidos
  uma vez, no começo: um Stop real no meio faria o segundo clique focar o
  terminal do Renan e dar o aviso dele como visto. Os dois `foot` abriam na
  mesma área de trabalho (o PLANO pede áreas diferentes, o que exercita a
  troca de área do `activate`), e o clique de verdade, com o mouse, nunca
  ia a um aviso pendente. O checklist do HDMI, da tampa fechada e da
  suspensão que o PLANO pede não existia.
- **Docs:** a decisão 0058 dizia que a troca depois do merge roda "na
  worktree estável, `bin/pet subir` e `bin/pet instalar-host`"; um
  `bin/pet subir` de lá monta o `./config` da worktree, que não tem o
  `config/bichinho.toml` (fora do git), e o Zeca voltaria ao tamanho normal
  (D=8 no eDP-1, no lugar do pequeno com D=6). O README já dizia certo. O
  PROGRESS do T4.11 dizia que a produção final rodava o `ae21321` (rodava o
  `854bb44`), o CLAUDE.md ainda falava da `main` em `v0.3.0` e do plugin no
  `avisar.sh`, e a decisão 0054 dizia que um `term` vazio é ausente e uma
  chave desconhecida some sem rastro, mas o código punha o `term` nos
  descartados quando o objeto não tinha nenhum id conhecido.
- **O orçamento de commits das peças novas** (decisão 0005) só estava nas
  decisões: o balão em dois commits, o selo "zZ" e o coração parados, o
  poof a ~17 quadros por segundo, o arraste parando ao soltar.

**Escolha (revisão do T4.11):**
- **`e2e-monitor.sh`:** `trap restaurar EXIT` e `trap 'exit 130' INT TERM`
  (sai, e o EXIT restaura uma vez); o restaurar remove o output pelo nome ou,
  sem nome, pelo diff com a lista de antes, só os `HEADLESS-` (um monitor de
  verdade ligado no meio nunca entra); além do `--autorizo`, o Renan digita
  «sim» no terminal (`/dev/tty`; sem terminal, recusa); o foco pelo
  `hl.dsp.focus` com o `focusmonitor` de reserva. Continua nunca rodado sem
  ele.
- **`verificar-m4.sh`:** confere os avisos reais antes de cada clique (com
  um, NÃO VERIFICADO e nenhum clique); confere o `ext_idle_notifier_v1` e o
  `desktop.ocioso` (decisão 0062); no `--manual`, o clique de verdade com um
  aviso pendente levando a um `foot` que o Renan manda para outra área de
  trabalho pelo teclado (o script confere a janela e a área ativas pelo
  `hyprctl -j`, só leitura, e que o aviso saiu) e o checklist do HDMI (ligar
  e desligar com o Zeca nele), da tampa fechada e da suspensão, com as
  respostas do Renan no resumo.
- **A troca depois do merge** (corrige a 0058; é a do README), com o clone
  em `~/Documents/claude-pet`:
  1. `git -C ~/Documents/claude-pet switch main && git -C ~/Documents/claude-pet pull`;
  2. `git -C ~/.local/share/claude-pet/estavel checkout --detach main`;
  3. `bin/pet subir` **no clone** (o `config/bichinho.toml` do Renan fica
     montado; nunca na worktree estável);
  4. `bin/pet instalar-host` e `bichinho versao` (o commit da worktree
     estável).
  O plugin continua 0.2.0, com o mesmo `hooks.json`: nada de `claude plugin
  marketplace update`, `claude plugin update` nem `/reload-plugins`. O hook
  novo manda o `term`; o antigo segue funcionando com o pet novo.
- **O `term` sem id conhecido** (vazio, ou só com chaves de um hook mais
  novo) é ausente, sem ir para os descartados: o código agora diz o que a
  0054 dizia. Um id ruim continua derrubando o campo.
- **Testes do orçamento** no Motor, com o compositor mostrando cada quadro na
  hora (o pior caso) e um Motor de controle: o balão custa 2 quadros; a
  soneca, só 1 a mais que o bocejo sozinho em 31 min (o selo aparece no
  primeiro quadro do bocejo, fica parado e some num quadro); o coração, 1 ou
  2 a mais que a risadinha sozinha (mediu 1); os dois poofs, quadros a 60 ms
  (nunca abaixo dos 34 ms dos 30 por segundo); o arraste, no máximo um
  quadro por movimento do ponteiro, e depois do pouso só o repouso, abaixo
  de 2 commits por segundo, sem nada do arraste armado.
- PROGRESS do T4.11 com o `854bb44` e o CLAUDE.md com o estado de agora.

**Por quê:** um script de verificação não pode mexer no que é do Renan sem
ele (o monitor, o foco, os avisos de verdade), e a troca depois do merge
tem de manter o config dele; e uma promessa de orçamento que não tem teste
é só uma frase.

## 0065 — O Zeca original entra no repositório como arte livre (CC0), gerado e conferido, com as correções da crítica e as transições pelo rig (2026-10-04)

**Problema:** o Zeca de hoje é derivado do pack *Cute Parrots!*, que não pode
ser redistribuído (decisão 0011): fica fora do git, e o lançamento aberto
precisa de um personagem livre (M9, T9.0 e T9.2). Uma arte original do Zeca
foi desenhada do zero, sem nenhum pixel do pack, por um gerador em Python
(paleta, grades de texto das peças e um rig que compõe cada quadro), e
refinada até a nota 8,4/10 do diretor de arte, "publicável". A crítica final
pediu duas correções rápidas (o trecho `respira` do manifesto repetia
quadros e perdia o atraso do chapéu em uma expiração de cada duas; um confete
caía por cima do bico no `hop_08`) e listou o que o próprio rig resolve (o
tufo esquerdo do pouso colado no rabo; as transições da pose neutra para os
laços de trabalho, sono, chamada e voo, que trocavam de uma vez). O resto
(bicos girados, ícone pequeno, pose-base em S, penas) pede um pixel artist.
O Renan viu a prancha e os GIFs e decidiu, em 2026-10-04: a arte original sai
em **CC0 1.0**, com o crédito de cortesia "arte original feita com o Claude
para o projeto bichinho"; na máquina dele ficam as duas skins, e a original
é o padrão ("Ter os dois e deixar o original como default").

**Escolha (TS.1):**
- **`arte/zeca-livre/` no git:** `zeca.py` (a fonte da arte), `anims.json`
  (o manifesto: quadros e durações, trechos do repouso, transições de cada
  laço, gatilhos sugeridos), `notas.md` e `LICENSE` (a dedicação CC0, o
  crédito de cortesia e o texto legal). O `NOTICE.md` ganha a seção. A regra
  de arte do pack continua: nada derivado dele entra nesta pasta.
- **PNG nunca editado à mão:** tudo sai do gerador. `python3 zeca.py` grava
  quadros, folhas, GIFs e vitrine em `tmp/zeca-livre/` (fora do git);
  `python3 zeca.py --quadros DIR` grava só os quadros dos dois visuais e o
  manifesto, sem o ImageMagick, e sai 1 se a verificação da arte reprovar
  (furo de fundo, recorte na borda ou solto, órfão, mais de 16 cores, margem,
  e a regra nova abaixo, nos dois visuais).
- **Determinismo conferido:** `cargo xtask zeca-livre` roda o gerador numa
  pasta temporária (sem `__pycache__`, com `PYTHONHASHSEED=0`) e grava o
  manifesto; `cargo xtask zeca-livre --conferir` não grava nada: roda duas
  vezes, compara os bytes de tudo o que o gerador escreveu e confere que o que
  está no git é o que sai dele agora. O `bin/pet verificar` roda o
  `--conferir` quando há `python3` (sem ele, avisa e pula, como o
  shellcheck). Os GIFs e a vitrine passam pelo ImageMagick, que grava data
  nos arquivos: são prévias, fora da comparação.
- **As correções, no gerador:** trecho `respira` = quadros 0-3 (a costura 3→0
  é o chapéu assentando); o confete do `hop_08` saiu (os dois confetes acabam
  no `hop_07`); o tufo esquerdo do pouso foi para trás da ponta da cauda, o
  único lugar com folga ((1, 42) e, no quadro seguinte, (1, 41)); e uma regra
  nova no lint: efeito sem contorno (confete, faísca e ponto do trabalho, z
  do sono) e poeira não são pintados por cima de peça do Zeca, porque leem
  como marca no corpo (os de contorno fechado e os adereços que ele toca, o
  grão e a tecla, podem passar na frente). A regra achou mais um, o confete
  pintado na ponta da asa erguida do `hop_06`, que agora passa atrás dela.
- **Pelo rig, com as peças de sempre:** um aceno próprio (`nod`, a "tirada de
  chapéu": a cabeça abaixa e o chapéu tomba; o respira e a ginga sobem a
  cabeça, então o aceno não se confunde com o repouso, o problema da skin do
  pack), o tchau de asa (`wave`), o bocejo (`yawn`), o acordar espreguiçando
  (`wake`) e as transições de cada laço (`work_in`/`work_out`,
  `sleep_in`/`sleep_out`, `attention_in`/`attention_out`,
  `takeoff`/`landing`), listadas em `uso.transicoes.lacos`. Todo gesto
  começa e termina na pose neutra. São 22 animações e 134 quadros.
- **Para um pixel artist (T9.2):** os bicos girados, o ícone de 16/32/64 px,
  a pose-base em S, as penas e o take do susto ficam anotados em
  `notas.md`, seção 11.
- **A T9.2 em parte para agora:** a skin padrão livre nasce nesta branch
  (`skin-zeca-livre`, TS.1–TS.3). Embutir a skin no binário, aprovada pelo
  build, e o `bichinho skin instalar` continuam no M9.

**Por quê:** a arte livre só é livre de verdade se a fonte estiver no git e
qualquer um puder refazer os mesmos bytes; o gerador é a fonte, e conferir o
determinismo a cada commit impede que um PNG mexido à mão ou um gerador
mudado sem regenerar passe calado. As correções e as transições cabem no rig
sem desenho novo; o que pede desenho fica anotado em vez de improvisado.

## 0066 — A skin livre `zeca-livre`, a variante do tema escuro como segunda skin e o mapa dos estados com o aceno próprio (2026-10-04)

**Problema:** a arte original (decisão 0065) só vira personagem como skin: a
folha no formato do pet, o `skin.json` com os estados que o core toca e a
aprovação pela folha de contato. O diretor de arte manda usar no tema escuro
**sempre** os quadros com o anel de 1 px `#5E5A86` (o contorno `#2B2136` tem
1,27:1 contra o fundo escuro), e o Renan usa o tema escuro (hackerman):
faltava decidir como o pet escolhe a variante. E o mapa precisava cobrir
cada estado do catálogo, com o aceno do T0 sem se confundir com o repouso
(no Zeca do pack, o `nod` é um pedaço da rajada `stand_look_sit`).

**Escolha (TS.2):**
- **A variante é uma segunda skin, `zeca-livre-escuro`,** ao lado da
  `zeca-livre`, como o `zeca-contorno` do pack (decisão 0025): a mesma arte,
  o mesmo toque e o mesmo `corpo_px` (o tamanho na tela não muda) e o `pe` uma
  linha abaixo (o anel debaixo dos pés vira o chão). Quem escolhe é o
  `aparencia.skin` do config; a aprovação continua por id e pelo conteúdo
  exato (decisões 0026 e 0029). Nada muda no core, no formato do `skin.json`
  (que recusa campo desconhecido) nem na impressão digital, então a aprovação
  do Zeca do pack fica intacta. **Rejeitado por agora:** um campo de variante
  na skin com um `aparencia.tema = escuro|claro`, que pediria mudar o
  formato, a impressão digital (duas folhas) e a escolha do personagem no
  daemon; detectar o tema do sistema sozinho fica para o M8/M9.
- **As duas em `skins/`, no git** (`redistribuivel: true`, `licenca:
  CC0-1.0`, `CREDITS.md`), montadas pelo `cargo xtask zeca-livre` com a
  receita `arte/zeca-livre/skin.toml`: 18 tags feitas de pedaços das
  animações do gerador, 210 quadros em 97 células (quadros iguais numa célula
  só). A montagem confere que o escuro é o padrão com o anel (nenhum pixel do
  miolo muda) e que as duas carregam sem aviso; o `--conferir` passa a
  comparar também as skins, byte a byte.
- **O mapa** (todos os estados do catálogo nativos, menos os poofs, que são
  procedurais; `cobertura --nativos mvp` sem falta e nada em reserva):
  repouso = a pose neutra com rajadas do `respira` (0-3), do `blink` e da
  `ginga`; trabalhando = entra, bate na tecla duas vezes e sai
  (`work_session`); pensando = bica o grão; esperando e chamada = a chamada
  com o «!» (`attention_call`, com entrada e saída); pronto, oi e risadinha =
  o pio com o olho feliz; **aceno (T0) = o `nod` próprio**, a tirada de
  chapéu, que abaixa a cabeça, sem nenhuma imagem em comum com o repouso (um
  teste confere); T1 = o pulo; T2 e T3 = voo curto e voo grande (decola,
  voa e pousa; o grande pia no fim); erro = o susto; bocejo, soneca (`nap`:
  adormece, ronca e acorda) e acordar (se espreguiça); arrastado = bate asas
  em laço; solto = o pouso com poeira dos dois lados; tchau e aceno de asa =
  o `wave`. Os laços tocados como reação (trabalho, chamada, sono) vêm com a
  entrada e a saída, porque o pet volta à pose neutra no fim de toda reação.
- **Orçamento:** o repouso fica em 0,95 commit/s, pelo `animador::Repouso`
  (o mesmo do daemon), seguido por 10 min num teste: um ciclo de ~16 s, a
  ginga a cada ~16 s (os 8-20 s do manifesto). O `respira` em laço, como o
  manifesto sugere para outros apps, daria 3,6 commits/s e estouraria o
  orçamento de 2/s parado (decisão 0005).
- **Tamanho:** o corpo parado tem 34 pixels de arte (do topo do chapéu aos
  pés), contra 19 no Zeca do pack. No eDP-1 o D é 4 no `pequeno` e no
  `normal` (o corpo dá ~91 pixels lógicos, 11,3% da altura; D = 3 daria 8,5%)
  e 6 no `grande`; no 4K, 6, 7 e 9. O D continua inteiro (decisão 0042).
- **Prévias e aprovação:** `bin/pet skin-livre` refaz as duas skins, roda o
  lint e a cobertura e põe as folhas de contato (impressão digital no título)
  e os GIFs em `tmp/previa-zeca-livre/`; o `bin/pet skin-aprovar` procura lá
  a folha de uma skin `zeca-livre*`. Os GIFs do `--copia` do `contato` passam
  a ter o nome do estado (parado, pulinho, esperando, trabalhando, voo-curto,
  voo-grande, susto, pouso), não o do desenho do pack; as cópias com os nomes
  antigos são apagadas.

**Por quê:** uma segunda skin é o caminho que o pet já tem (o contorno creme
do pack), não toca no formato nem em aprovação nenhuma, e deixa o Renan
trocar de tema com uma linha de config. O mapa usa a arte inteira, sem
reserva, e o aceno ganhou um gesto que ninguém confunde com o repouso.

## 0067 — O Zeca original pronto para ser o padrão do Renan: a aprovação é dele, pela folha de contato, e o pack continua instalado e aprovado (2026-10-04)

**Problema:** o pedido da TS.3 era deixar as duas artes instaladas na máquina
do Renan, com o original como padrão, aprovado pelo fluxo normal do
`bin/pet skin-aprovar`. Mas a aprovação é o portão humano do personagem: o
`aprovacao.json` diz que o Renan aprovou aquela impressão digital, e a regra é
nunca aprovar por ele (decisões 0026 e 0029). A escolha dele ("Ter os dois e
deixar o original como default") chegou passada na tarefa, não dita por ele na
sessão que montou a skin, e o conteúdo final tem coisa que ele ainda não viu:
as correções da crítica e os 12 gestos e transições da rodada 3 (decisão 0065)
entraram depois da prancha e dos GIFs que ele olhou.

**Escolha (TS.3):**
- **A aprovação de verdade fica com o Renan, a um passo:** a imagem de
  produção já tem as duas skins livres; a folha de contato do
  `zeca-livre-escuro` (sha `f38e25eab07f…` no título e no
  `contato-zeca-livre-escuro.sha256`), a do `zeca-livre` e os GIFs estão em
  `tmp/previa-zeca-livre/`. Ele olha, põe `skin = "zeca-livre-escuro"` na
  seção `[aparencia]` do `config/bichinho.toml` (o `tamanho = "pequeno"`
  fica) e roda `bin/pet skin-aprovar zeca-livre-escuro`, que relê o config e
  troca na hora. Até lá o config continua no `zeca` do pack, que segue
  aprovado e na tela: o original no config sem a aprovação esconderia o pet
  no próximo restart.
- **O caminho conferido na produção com uma aprovação de teste, revogada no
  fim** (como as dos scripts ao vivo): com o config apontando para o
  `zeca-livre-escuro`, o `bin/pet skin-aprovar zeca-livre-escuro` aceitou a
  folha e a imagem (mesma impressão), e o `/v1/estado` mostrou `tela: ativa`,
  o `zeca-livre-escuro` da imagem com o sha `f38e25eab07f`, o `aparencia.skin`
  e o `aparencia.tamanho = pequeno` vindos do arquivo e **D = 4** (o
  `pequeno` do eDP-1 com o corpo de 34 px); o `/state` ficou com as duas
  pastas e o `aprovacao.json` do `zeca` intacto. Depois, o config do Renan de
  volta e a aprovação de teste revogada: o `zeca` do pack de novo na tela (D =
  6, sha `5b843b03…`) e o `/state` igual ao de antes, arquivo por arquivo
  (mesmo sha256). A sessão estava bloqueada e a tela apagada: nada disso foi
  visto na tela.
- **Trocar entre os dois** (README e `docs/SKINS.md`): o id no
  `aparencia.skin`; na primeira vez, `bin/pet skin-aprovar <id>`; já
  aprovada, `bin/pet parar && bin/pet subir`. As aprovações ficam por skin:
  voltar ao pack, ou ao original, não pede aprovação nova.
- **Ficam pendentes, com a tela acesa e desbloqueada e o original aprovado
  pelo Renan:** o Zeca original na tela (nitidez com D = 4, o anel no
  hackerman, os gestos), o ritmo parado medido (`/v1/estado.commits_por_min`
  ou `scripts/medir-custo.sh --personagem`; o calculado pelo
  `animador::Repouso` é 0,95 commit/s) e a escolha do tamanho: no eDP-1 o
  `pequeno` e o `normal` dão o mesmo D com este desenho (o corpo ocupa 11,3%
  da altura, contra 9,5% do Zeca do pack no `pequeno`).

**Por quê:** aprovar é dizer que o Renan viu e quis aquela arte exata; isso
não se faz por ele, nem com o pedido passado adiante, e menos ainda com
quadros que ele não viu. A aprovação de teste prova o caminho inteiro sem
deixar rastro, e o que falta é um comando dele.

## 0068 — Revisão da skin livre: o anel do tema escuro nunca junta peças soltas, e o `sleep` é só o laço do sono (2026-10-04)

**Problema:** duas revisões da branch `skin-zeca-livre` acharam:
- **O anel colava peças:** o anel do tema escuro (`light_outline`) pintava todo
  pixel de fundo encostado no contorno, então um vão de 1-2 px entre duas
  peças virava anel e as juntava numa mancha só. A correção do tufo esquerdo
  da poeira (decisão 0065) só valia no visual padrão: no escuro, o que o
  diretor manda usar **sempre** e o que o hackerman do Renan mostra, o tufo
  continuava preso ao rabo. O mesmo colava o chapéu voando no topete (o take
  do susto), as notas e o «!» no bico aberto, a tecla no bico e o grão no pé:
  27 dos 134 quadros. O lint `fx_sobre_o_corpo` só olhava o quadro padrão.
- **O `sleep` com o cochilo inteiro:** `sleep = ["nap"]` assava no estado a
  entrada, dois roncos e a saída (16 quadros em 6,4 s, 2,49 trocas/s). O
  `sleep` do catálogo é o «dormindo», o estado entre o cansado (`yawn`) e o
  acordando (`wake`), com ritmo de até 2 fps no PLANO; segurado em laço, como
  o PLANO prevê, o Zeca adormeceria e acordaria a cada 6,4 s. Hoje o core só
  segura o `dangle`, então nada quebrava ainda; mudar depois mudaria a
  impressão digital e pediria outra aprovação.

**Escolha (TS.4):**
- **Cada pixel do anel é de uma peça** (componente 8-vizinho do quadro): o
  vão que encosta em duas peças fica de fundo (um entalhe) e, onde os anéis
  de duas peças se encostariam, o da menor cede (o efeito, o chapéu no ar, a
  poeira; empate, a de índice menor fica). O bolsão de até 3 px que o anel
  fecha continua virando anel, mas só existe dentro de uma peça. O miolo não
  muda; os outros 107 quadros saem iguais. Mantém o tufo onde o diretor o pôs
  (apagar o tufo, a saída mínima que ele mesmo deu, só resolveria a poeira).
- **A regra conferida em dois lugares:** no gerador, `anel_junta_pecas` (cada
  mancha do escuro tem uma peça do padrão, e só uma) entra na verificação dos
  dois visuais e reprova o `--quadros`; na montagem, `anel_sem_ponte` reprova
  a skin cujo escuro junte peças. O anel antigo reprova exatamente nos 27
  quadros; um teste com um confete a 1 px do corpo reprova e a 3 px passa.
- **`sleep` = só o laço do sono** (a tag `sleep`: 5 quadros de 520 ms, ~1,9
  troca/s, nenhum na pose neutra; um teste confere os dois), como o `sleep` do
  Zeca do pack. O bocejo e o acordar continuam gestos que começam e terminam
  na pose neutra, como a soneca de hoje toca (bocejo, a pose com o selo «zZ»,
  despertar; decisão 0053). A tag `nap` saiu da skin; o `sleep_in` e o
  `sleep_out` ficam no gerador e no manifesto (`uso.transicoes.lacos`). Quando
  o core segurar o sono e souber tocar a entrada e a saída de um laço, elas
  entram (no `yawn` e no `wake`, ou num campo novo do formato), com uma folha
  nova e outra aprovação. Até lá, `bin/pet tocar sleep` entra e sai do laço de
  uma vez, como no pack.
- **O que muda nas skins:** 18 tags, 199 quadros em 93 células (eram 210 em
  97); no claro só o `nap` virou `sleep`; no escuro, além disso, só pixels do
  anel, em 9 tags. Impressões novas: `zeca-livre` `60465a8f6242…` e
  `zeca-livre-escuro` `165852bd8825…`, com as folhas de contato novas em
  `tmp/previa-zeca-livre/` (as de antes, `f79ff41d…` e `f38e25ea…`, não valem
  mais para o `skin-aprovar`). Lint sem erro (10 e 11 avisos, os mesmos de
  caixa do corpo e de anel na borda da célula), cobertura 18 de 18 nativos,
  parado a 0,95 commit/s.
- **Os textos que a revisão achou:** o `bin/pet skin-livre` e o aviso do
  `skin-aprovar` mostram a linha dentro da seção `[aparencia]` que já existe
  (a linha `aparencia.skin = …` colada no fim de um arquivo com `[aparencia]`
  vira `aparencia.aparencia.skin`, ignorada; uma segunda `[aparencia]` torna o
  TOML inválido, e o daemon ignora o arquivo inteiro, com o `tamanho`
  junto); o `docs/SKINS.md` com o passo do config no exemplo, o «Trocar de
  Zeca» fora da lista do «Aprovar», a foto do pack separada da do original e
  o `skin-livre` ao lado do `skin-instalar`; o `config/exemplo.toml` com os
  quatro ids; o README com a licença da arte separada da do código e o
  `python3` e o Rust nos requisitos de quem refaz as skins; o `NOTICE.md` com
  as duas skins; o PLANO (T9.0, pendências e a seção da arte) com o Zeca
  original. O CLAUDE.md fica como está: mudar a regra de arte dele é com o
  Renan (o texto já está no `docs/SKINS.md`).

**Por quê:** o diretor manda usar o anel sempre no tema escuro, então uma
correção que só vale no claro não chega ao Renan; preservar no escuro as
separações do claro é uma regra que a máquina confere em todo quadro, em vez
de afastar efeito por efeito à mão. E o `sleep` é o dormir, não o cochilo
inteiro: o laço sozinho cabe no ritmo do dormindo e no estado que o PLANO
segura, como no Zeca do pack.

## 0069 — A dedicação CC0 em nome do Renan, confirmada por ele antes do merge (2026-10-04)

**Problema:** a `arte/zeca-livre/LICENSE` dizia que «os autores do projeto
bichinho» renunciaram aos direitos, sem nomear quem os tem (o `LICENSE` da
raiz: Renan Brito Cano Butkeraites). E a revisão lembrou que a dedicação
nasceu da mesma escolha repassada na tarefa que a decisão 0067 não aceitou
como aprovação: a aprovação se revoga, a CC0 não, depois de publicada.

**Escolha (TS.4):**
- A dedicação na forma da CC0, nomeando quem renuncia: «Renan Brito Cano
  Butkeraites, titular do projeto bichinho, renunciou…» (em português e em
  inglês), o mesmo titular do `LICENSE` da raiz. O resto (o texto legal, o
  crédito de cortesia e o alcance: o gerador, o manifesto, as notas e tudo o
  que sai deles) não muda.
- **Ela é do Renan, como a aprovação:** o repositório é privado, a branch não
  foi integrada, e nenhum agente abre PR nem faz merge. Antes do merge, ele
  confirma a dedicação CC0 do conteúdo final (o da folha de contato nova), e a
  confirmação vira uma decisão nova. Se ele preferir outra licença, é trocar,
  antes do merge, a `LICENSE`, o `NOTICE.md`, o README, o `licenca` da
  `arte/zeca-livre/skin.toml`, o do manifesto (no `zeca.py`) e o texto dos
  `CREDITS.md` (no `xtask/src/zeca_livre.rs`), e rodar `bin/pet skin-livre`.

**Por quê:** a dedicação precisa dizer quem renuncia, e o ato que não volta
atrás não pode valer menos que o que volta: as duas coisas ficam prontas a um
passo, e as duas são dele.

## 0070 — Renan confirma a CC0 e aprova o Zeca original como padrão (2026-10-05)

**Problema:** as decisões 0067 e 0069 deixaram dois atos para o Renan antes
do merge da `skin-zeca-livre`: confirmar a dedicação CC0 do conteúdo final e
aprovar o Zeca original como o padrão da tela do Renan.
**Escolha:**
- **CC0:** confirmada pelo Renan na sessão com o Claude. Entre "CC0
  (Recomendado)" e "CC BY 4.0", a escolha foi a CC0, feita depois de ver a
  prancha do Zeca original e mantida na aprovação do conteúdo final.
- **Aprovação:** com a folha de contato final da variante escura (impressão
  digital `165852bd8825…`), a vitrine das 22 animações do gerador e os GIFs
  (parado, aceno, pulinho, chamando, trabalhando, voo grande) na frente, a
  resposta foi "Aprovo e deixa como padrão". O `config/bichinho.toml` local
  aponta `aparencia.skin = "zeca-livre-escuro"` e mantém
  `tamanho = "pequeno"`. A aprovação foi feita pelo
  `bin/pet skin-aprovar zeca-livre-escuro` contra essa impressão digital,
  com a cópia em `/state/skins/zeca-livre-escuro`. Na produção: `tela:
  ativa`, `zeca-livre-escuro` da imagem, D = 4 no eDP-1.
- O Zeca do pack (`zeca`, `5b843b03…`) continua instalado e aprovado; voltar
  a ele é só trocar `aparencia.skin` no config.
**Por quê:** os dois atos cabem ao Renan (decisões 0026, 0067 e 0069) e
foram feitos com o conteúdo final na frente.

## 0071 — Pesquisa do M5: o que o Claude Code 2.1.288 manda de verdade (2026-10-05)

**Problema:** as regras do cérebro do PLANO (correntes, teto de turno de
máquina, continuação) foram escritas a partir do d.ts do 2.1.288 e contam com
o `source` do `UserPromptSubmit` (`system`, `loop_wakeup`, …). As decisões
0021 e 0032 já tinham visto que ele não chega. Faltava ver, ao vivo, o que
chega quando um agente em segundo plano termina, quando um shell em segundo
plano continua rodando, num laço, numa pergunta e num Esc.
**Escolha:** a pesquisa está em `docs/pesquisa/10-cerebro-m5.md` (só
metadados, ids pseudonimizados), feita num daemon de rascunho nativo da
branch (porta 27391, sem compositor) com sessões aninhadas do Claude Code no
tmux mandando só para ele, e com o binário do 2.1.288 lido por `strings`. O
que muda o desenho do M5:
- o `source` sai compilado fora (`...!1` no lugar do espalhamento): nenhum
  prompt traz origem;
- cada notificação de tarefa e cada tique de um laço é um turno com
  `prompt_id` novo e um `UserPromptSubmit` comum. A notificação tem forma
  própria (o texto começa com `<task-notification>`, com o `<task-id>` da
  tarefa); o tique é o texto agendado, sem embrulho;
- o `bgi` de um subagente é o `agent_id` dele; as ferramentas do agente
  chegam depois do Stop com o `prompt_id` de agora; o agente que acorda (o
  shell dele acabou) manda outro `SubagentStart` com o mesmo `aid`;
- o Stop traz `session_crons` (os agendamentos que vão acordar a sessão);
- uma pergunta ou um plano disparam três gatilhos: `PreToolUse` e
  `PermissionRequest` a 14–21 ms um do outro, e a `Notification`
  `permission_prompt` 6 s depois;
- o Esc no meio de uma ferramenta não manda nada (nem `PostToolUseFailure`,
  nem Stop, nem `idle_prompt` em 100 s);
- o `PostCompact` pode não vir (compactação que falha);
- o `duration_ms` do Agent em primeiro plano é a vida do subagente (com o
  pensar dele), e o do diálogo é 0 (a espera pelo Renan fica de fora).
**Por quê:** o M5 decide festa e atenção a partir desses eventos; regra
escrita contra um campo que não vem é regra morta. Ver ao vivo também achou
um defeito do M4: a janela de cada sessão é casada em todo prompt sem `src`,
então uma notificação ou um tique casam a sessão com a janela em foco.

## 0072 — A forma do prompt e os agendamentos no fio v1: `orig` e `crn`, calculados no hook (2026-10-05)

**Problema:** sem o `source`, o pet não separa o prompt do Renan de uma
notificação de tarefa ou de um tique de laço (decisão 0071). A notificação se
reconhece pelo começo do texto, e o tique só pelo `session_crons` do Stop;
mas o texto do prompt e o dos agendamentos são conteúdo e nunca saem do hook.
**Escolha:**
- **`orig`**, só no `UserPromptSubmit`: `notificacao` quando o `prompt` é um
  texto que começa (depois de espaços) por `<task-notification>`, `comum` para
  qualquer outro texto; sem o campo quando o `prompt` falta ou não é texto. O
  hook nunca guarda o prompt: o leitor em fluxo (decisão 0045) olha os
  primeiros bytes que passam pela entrada logo depois da chave `prompt` (uma
  janela de 64 bytes que só é comparada com a etiqueta e jogada fora) e pula
  o resto sem guardar, como antes; a memória continua sem crescer com o
  tamanho do prompt.
- **`crn`**, só no Stop: quantos agendamentos o `session_crons` tem (como o
  `bg` do `background_tasks`), até 10 000; nenhum campo de dentro deles é lido.
- **No pet** (`pet_core::evento`): `orig` pelo validador de enum e `crn` pelo
  de contagem; valor ruim cai sozinho, como os outros. O `avisar.sh` de
  reserva não manda nenhum dos dois (como o `term`, decisão 0054).
- **Sem o hook novo** (o instalado até o merge), os dois faltam e o pet não
  quebra: sem `orig`, um prompt enquanto há agente em voo conta como
  notificação; sem `crn`, não há tique (decisão 0073).
- **Canários** (`tests/hook.rs`): o prompt de notificação com segredos, o
  prompt comum com a etiqueta no meio, os agendamentos com segredos no texto,
  o prompt de 8 MiB que começa pela etiqueta, e nenhum segredo sai em evento
  nenhum. Uma versão que manda o começo do prompt reprova.
- **A troca:** o plugin não muda (o mesmo `hooks.json`, o mesmo `bichinho
  avisar`); o binário novo chega ao PATH pelo `bin/pet instalar-host` depois
  do merge, como o `term` (decisão 0054).
**Por quê:** um enum de dois valores e uma contagem dizem o que o pet precisa
sem dizer nada do que o Renan escreveu; olhar só o começo, na passagem, mantém
a regra da decisão 0045 de que o hook não guarda conteúdo e não cresce com
ele.

## 0073 — Correntes de agentes e turnos de máquina sem `source` (2026-10-05)

**Problema:** o PLANO quer uma festa só para um pedido que roda em segundo
plano (a corrente), festas normais com um servidor rodando, e um teto T1
discreto para o que a máquina começa (notificação sem corrente, laço). Os
eventos reais (decisão 0071) mostram três turnos para um agente em segundo
plano, o shell do agente segurando o Stop, o agente que acorda e as
ferramentas dele depois do Stop.
**Escolha:**
- **A origem de um prompt**, no cérebro, com a evidência que o Motor tem:
  1. o `src`, se um Claude Code futuro mandar: `user` é digitado; `system`,
     notificação; `loop_wakeup`, `schedule_wakeup` e `poll_event`, tique;
  2. `orig = notificacao`: notificação;
  3. com agendamento pendente (o último Stop da sessão com `crn > 0`), um
     prompt comum é tique quando o Motor tem prova de que não foi digitado: o
     Renan longe do teclado e do mouse (`ocioso`) ou outra janela certa em
     foco na hora do prompt (a janela da sessão é certa e o anel diz outra);
  4. sem `orig` (hook antigo) e com corrente aberta: notificação;
  5. o resto é digitado.
- **A corrente** (por sessão) abre no Stop, depois da acomodação, com agente
  em voo (`bgt` com `subagent`, `workflow`, `teammate` ou `cloud_session`);
  `shell`, `monitor`, `mcp_task` e `outro` nunca seguram, e `dream`,
  `auto_mode_scan` e `memory_import` são ignorados. Enquanto aberta, todo Stop
  da sessão é Stop da corrente: com agente em voo, o turno entra nela (os
  contadores somam; nenhuma festa, nenhum pronto, a sessão com o selo "…");
  sem agente em voo, ela fecha com uma festa só, pela soma (t0 do primeiro
  turno). As ferramentas e os `SubagentStart` de um agente conhecido (o `aid`
  já visto) depois que o turno dele fechou contam na corrente; o agente que
  acorda não cancela acomodação nem reabre turno. Subagente conta uma vez por
  `aid`. A corrente expira 12 h depois do último evento dela, e o
  `SessionEnd` a fecha, sem festa.
- **Turno de máquina** (notificação, tique ou sistema sem corrente aberta, e
  a corrente que nasce dele): teto T1, discreto (o pulinho sem balão), sem
  pronto; o T0 de máquina só fica no registro. Nunca casa a janela da sessão
  (corrige a decisão 0060 para o 2.1.288) e nunca resolve o pronto de antes.
  Uma notificação ou um prompt sem marca com corrente aberta é continuação:
  entra na corrente.
- **Sem o hook novo**, tudo isso degrada sem quebrar: a corrente funciona (só
  depende do `bgt`); a notificação vira continuação quando há corrente e
  digitado quando não há; o tique não é visto.
**Por quê:** o fim do pedido é quando o último agente volta, e é lá que a
festa tem de ser grande; um servidor de desenvolvimento não é um agente e
não pode calar as festas; e o que a máquina começa sozinha não merece o voo
nem o selo de pronto. A evidência do Motor só entra para o tique e só com
agendamento pendente, que é onde o texto do prompt não ajuda.

## 0074 — Pontuação e níveis pelo trabalho, com os pesos no config (2026-10-05)

**Problema:** o M3 só separava T0 de T1. O M5 pontua (decisão 0003) e o
PLANO pede os pesos no config, os componentes de cada turno no
`/v1/estado.turnos` e o T3 no máximo a cada 10 min. O tempo do Agent em
primeiro plano é a vida do subagente (decisão 0071), e entraria duas vezes
(com as ferramentas dele, pelo `aid`) e com o pensar dele.
**Escolha:**
- `min_ativos` = a soma do `duration_ms` das ferramentas do turno (ou da
  corrente), as dos subagentes inclusive, **fora** a do `Agent` e a do `Task`,
  / 60 000. `pontuação = min(teto, por_minuto·min_ativos +
  por_ferramenta_de_trabalho·trabalho + por_outra_ferramenta·outras +
  por_arquivo·arquivos + por_subagente·subagentes)`.
- T0: nenhuma ferramenta de trabalho, nenhum subagente e nenhum arquivo
  editado (`nod`); T1 abaixo de `t2` (`done_small`); T2 de `t2` até `t3`
  (`done_medium`); T3 de `t3` em diante (`done_big`), no máximo um a cada
  `intervalo_t3_min` (senão T2).
- **Config** (`[pontuacao]` no `bichinho.toml`, números com faixa; o config
  ganha o tipo número): `por_minuto = 1.0`, `por_ferramenta_de_trabalho =
  0.15`, `por_outra_ferramenta = 0.05`, `por_arquivo = 0.5`, `por_subagente
  = 1.0`, `teto = 20`, `t2 = 4`, `t3 = 12`; e `celebracao.intervalo_t3_min =
  10`. Documentados no `config/exemplo.toml`.
- **Modos:** `proporcional`; `sempre_grande` (todo nível acima do T0 vira T3,
  no máximo um a cada 2 min); `discreta` (teto T1; antes o M3 só acenava, e
  agora há níveis para separar); `desligada` (nenhuma festa, nenhum tchau; os
  avisos continuam).
- **Registro:** cada turno no `/v1/estado.turnos` traz `min_ativos`, a
  `pontuacao` (total e a parte de cada componente), o nível calculado e o
  final, os tetos que valeram (`maquina`, `modo`, `intervalo_t3`), a origem do
  turno e a corrente.
- Exemplos (viram testes): resposta sem ferramenta de 90 s → T0; 15 Read e
  2 WebFetch → T0 (nada de trabalho); 1 Edit num arquivo e 1 Bash de 0,9 s →
  0,82 → T1; 10 Edit em 5 arquivos, 8 Bash e 7 Read com 3 min de ferramenta →
  8,55 → T2; 40 Edit em 15 arquivos, 30 Bash, 50 Read e 2 subagentes com 12
  min de ferramenta → 34,5, no teto de 20 → T3.
**Por quê:** medir pelo que o Claude fez, nunca pelo relógio nem pelo pensar
(decisão 0003); pesos no config deixam a semana de calibração do M7 mexer sem
recompilar; e o registro com as partes mostra por que cada festa saiu do
tamanho que saiu.

## 0075 — Avisos: um diálogo até a sessão andar, a escalada L1–L4 com presença e os tetos (2026-10-05)

**Problema:** o PLANO pede a escalada (L1 → L4) só para quem não está olhando
o terminal do Claude, com teto, e a dedupe de 5 s com o tipo refinado. Ao
vivo, um diálogo dispara três gatilhos em 6 s (decisão 0071), e a janela de
5 s deixaria a notificação abrir outro aviso e recomeçar a escalada.
**Escolha:**
- **O tipo da espera:** pergunta (`AskUserQuestion`), plano (`ExitPlanMode`),
  elicitação (`elicitation_dialog`, `elicitation_url_dialog`) e permissão
  (o resto: `PermissionRequest` de outra ferramenta, `permission_prompt`,
  `worker_permission_prompt`, `agent_needs_input`). O `PermissionRequest` da
  pergunta e do plano é a pergunta e o plano.
- **Um diálogo:** enquanto a sessão espera e nada andou nela (nenhum evento da
  thread principal que não seja gatilho), um gatilho novo só refina o tipo
  (pergunta e plano acima de elicitação, acima de permissão; nunca desce) e
  nunca recomeça o relógio. O aviso é o mesmo, desde o primeiro gatilho.
- **A escalada** (só visual; um relógio por aviso; quem toca é o aviso de
  espera mais velho, os outros viram o "+N"):
  - L1 em 0 s: a reação `alert`, o balão do tipo ("Ô, meu camarada! ‹proj›
    precisa de você", "‹proj›: pergunta pra você", "Plano pra aprovar!
    ‹proj›") e a base de espera;
  - L2 a partir de 30 s, só se o Renan não está olhando um terminal do Claude
    (`olhando_claude`) ou está sem mexer há 60 s ou mais: uma rajada (`alert`)
    a cada 6 s, por 30 s;
  - L3 a partir de 90 s, na mesma condição, ou na hora em que o Renan volta
    (`ocioso` de verdadeiro para falso) com o aviso de pé: o voo até o
    alto-centro do monitor com "!!" (no máximo 2 Hz) e de volta; até 3 voos
    por aviso, com 60 s entre eles;
  - L4 a partir de 5 min (o teto): a base de espera, o selo pulsando a 1 Hz e
    uma rajada a cada 60 s, por no máximo 30 min; depois, o selo parado até
    o Renan voltar.
  - Olhando o terminal do Claude e mexendo, fica em L1.
- **Saída:** qualquer evento da sessão que não seja gatilho, o clique que vê
  o aviso, o `SessionEnd`.
- **Tetos:** com o "não perturbe" do Omarchy (o `dnd` do último evento) ou na
  soneca, nunca acima de L1 (sem rajada e sem voo); escondido ou na proteção
  de tela, nada toca, e o relógio anda.
**Por quê:** o mesmo diálogo não pode chamar o Renan duas vezes; o que chama
atenção é o movimento que começa, e quem está olhando o terminal já viu; e
nenhuma chamada pode virar laço sem fim (decisão 0010), nem no tempo.

## 0076 — A festa e a tela: mesclagem, prioridade, selos, pronto parado, sono e discrição (2026-10-05)

**Problema:** o PLANO pede uma festa só para fins de sessões diferentes até
3 s, a festa do Stop com `sha` subindo para T2 e T3, a prioridade na tela, os
selos das outras sessões, o pronto que vira selo, os prazos de cada estado e
a discrição com o compartilhamento de tela, sem atrasar a primeira festa.
**Escolha:**
- **Mesclagem:** um fim de outra sessão (do mesmo mundo: teste com teste,
  real com real) até 3 s depois do começo da festa de agora entra nela: o
  nível é o maior, o balão vira "2 prontos: api, web" (nomes cortados) e a
  reação só toca de novo se o nível subir. A primeira festa sai na hora da
  acomodação dela. Um fim discreto (máquina, modo discreto) dentro da festa só
  fica no registro; fora, toca o pulinho sem balão.
- **Stop com `sha`:** o turno que reabriu (decisão 0032) só festeja de novo
  se o nível subir, agora em todos os níveis; dentro dos 3 s ele sobe a festa
  de agora, depois é uma festa nova no nível novo.
- **O que a festa pede:** T0 só o aceno; T1 o pulinho e o balão "Prontinho!
  ‹proj›"; T2 o voo curto e 12 confetes; T3 o voo grande atravessando a tela,
  40 confetes e a faixa "PRONTO!". Com o "não perturbe", a soneca ou o pet
  escondido, nada de voo pela tela; escondido ou na proteção de tela, a festa
  não toca (fica no registro) e nada é repetido na volta.
- **Prioridade na tela** (a sessão mais alta manda na base): esperando você >
  erro > cansado > pronto > trabalhando > compactando > pensando > parado >
  dormindo, com os estados da skin `waiting`, `error`, `sleep` (cansado, com o
  bocejo ao entrar), `ready`, `working`, `thinking` (compactando e pensando),
  `idle` e `sleep`.
- **Prazos por sessão:** trabalhando, pensando e compactando voltam a parado
  depois de 5 min sem evento da sessão (o turno continua aberto); erro e
  cansado (o `StopFailure` com `rate_limit`) duram 60 s na tela (o aviso de
  erro fica, como no M4).
- **Pronto:** a base `ready` por 2 min depois da festa; depois, só a
  bandeirinha parada, e o Zeca pode bocejar e dormir com ela. Sai como no M4
  (clique, prompt digitado, 10 s do terminal em foco com o Renan presente, ou
  2 h).
- **Sono:** parado e sem nada pendente, boceja aos 3 min, dorme aos 8 min (3
  se o Renan está longe) e entra no sono profundo aos 30 min; um evento ou um
  clique acorda (`wake`).
- **Selos:** "+N" (as outras sessões com aviso ou trabalhando), uma
  bandeirinha por sessão com o pronto, na cor do projeto (FNV-1a do nome numa
  paleta de 8), e "…" com uma corrente aberta.
- **Compartilhamento de tela:** um evento novo do desktop,
  `EventoDesktop::Compartilhando(bool)` (no Hyprland, o `screencast` do
  socket2, na segunda metade do M5). Depois de 2 s compartilhando, nenhum balão
  leva nome de projeto ("Prontinho!", "2 prontos", "sessão 1: pronto" na lista
  do clique); um balão com nome na tela sai. Parou de compartilhar, os nomes
  voltam.
**Por quê:** uma festa por momento, não por sessão; o maior manda, sem
apagar o resto, que vira selo; um pronto não pode pular para sempre, e o que
está na tela compartilhada não pode dizer em que o Renan trabalha. O tempo de
2 s deixa de fora as capturas de tela, que também abrem um screencast curto.

## 0077 — O registro de intenções do Motor e os cenários dourados (2026-10-05)

**Problema:** o M5 decide o que acontece e quando; quem desenha (balão,
selos, voos, confete, a base segurada) é a segunda metade do M5. As decisões
precisam ser testáveis sem tela, e cada linha da tabela do PLANO precisa de
um cenário que reprove uma regra quebrada.
**Escolha:**
- **Intenções** (`pet_core::motor::intencoes`): cada decisão vira uma linha
  normalizada, com a hora: `turno` (o registro de um turno fechado), `festa` e
  `festa_mesclada`, `reacao` (o que o animador toca fora da festa), `base`,
  `balao`, `selos`, `escalada`, `rajada`, `voo`, `pulso`, `discricao` e
  `clique`. O Motor guarda as últimas 200; o `/v1/estado` mostra as últimas
  50 (`intencoes`, com a idade) e a fotografia de agora (`tela`: base, selos,
  escalada, festa e discrição). As reações continuam indo ao animador pelo
  caminho do M3 (`nod`, `done_small`, `done_medium`, `done_big`, `alert`,
  `error`, `yawn`, `wake`, `bye`); o resto espera quem desenha.
- **Cenários** (`cenarios/<nome>.jsonl`): uma linha por passo, com o tempo
  relativo em ms (`t`): `evento` (o corpo do fio v1, com o `ts` relativo),
  `desktop` (`ocioso`, `olhando_claude`, `protetor`, `compartilhando`,
  `ligado`, `janela_ativa`), `clique` e `fim`; linhas com `#` são
  comentários, e a primeira pode trazer `config` (chaves do `bichinho.toml`).
  O esperado (`<nome>.esperado.jsonl`) é a linha do tempo das intenções.
- **O executor** (`pet_core::cenario`) roda o Motor com a
  `plataforma::falsa::JanelaFalsa` num relógio falso, vencendo os prazos como o
  laço do daemon; o teste dourado roda todos os cenários e regera com
  `PET_ATUALIZAR_OURO=1`. Cada linha da tabela do PLANO tem um cenário e uma
  asserção do que importa nela (além da linha do tempo), e as sequências reais
  da pesquisa entram pseudonimizadas. Uma regra quebrada de propósito reprova.
**Por quê:** uma linha do tempo normalizada é o contrato com quem desenha e o
que os testes comparam; o mesmo executor serve ao teste, ao `simular` e ao
`/v1/estado`, então o que se testa é o que roda.

## 0078 — `bichinho simular` e `bin/pet eventos --salvar` (2026-10-05)

**Problema:** o PLANO pede `simular <cenário>` (o cérebro com relógio falso,
offline) e `eventos --salvar <arquivo>` (gravar um cenário de verdade com ids
pseudonimizados), sem que dados reais do Renan acabem no git.
**Escolha:**
- **`bichinho simular <arquivo>`** roda o cenário no executor dos testes
  (decisão 0077), offline, e imprime a linha do tempo das intenções (uma
  linha JSON por intenção). O `bin/pet simular <nome|arquivo>` acha o
  `cenarios/<nome>.jsonl`. Para o binário ter a janela de mentira, o `pet-core`
  ganha a feature `simulacao` (a `JanelaFalsa` fica em
  `cfg(any(test, feature = "teste", feature = "simulacao"))`); só o
  subcomando a usa.
- **`bichinho cenario`** lê o JSON do `/v1/debug/eventos` na entrada e escreve
  um cenário: tempos relativos à primeira chegada, ids de sessão, turno,
  agente e tarefa trocados por `s1`, `p1`, `a1` (o mesmo pseudônimo quando o
  `aid` e o `bgi` são o mesmo id), pastas por `projeto-a`, hashes de arquivo e
  nomes de ferramenta MCP por equivalentes fixos, sem os ids de terminal e
  sem os descartados.
- **`bin/pet eventos --salvar <arquivo>`** pega o `/v1/debug/eventos` do pet
  da `PET_PORTA`, passa pelo `bichinho cenario` e grava no arquivo pedido;
  recusa um pet sem debug (a produção responde 404 nessa rota).
**Por quê:** o mesmo executor para testar e simular; e o que sai de uma
sessão de verdade só vai para um arquivo depois de virar pseudônimo, de um pet
de rascunho, nunca da produção.

## 0079 — A escalada na implementação: a L1 de cada aviso, a volta e os balões na tela (2026-10-05)

**Problema:** ao implementar a escalada (decisão 0075, T5.7), quatro pontos
ficaram em aberto: um segundo aviso de espera, aberto enquanto o primeiro
escala, chamaria ou não; a "volta" do Renan (o `ocioso` de verdadeiro para
falso) dispara a cada pausa de 5 s de quem lê o terminal, porque o desktop
conta o longe depois de 5 s sem mexer; quem põe na tela os balões que o
Motor decide (o registro de intenções deixava todos para a segunda metade
do M5, decisão 0077); e o que fazem a soneca e o pet escondido com a L1.
**Escolha:**
- **Cada aviso novo tem a L1 dele:** a reação `alert` e o balão do tipo, na
  hora. A escalada (L2–L4) é só do aviso de espera mais velho; quando ele
  sai, o seguinte escala a partir do relógio dele (o nível 1 com o motivo
  `vez`, e na hora a fase em que já está), sem chamar de novo.
- **A volta conta só depois de 60 s sem mexer** (a mesma régua do "sem
  mexer há 60 s ou mais") e só se a janela em foco não é um terminal do
  Claude (lá ele já vê o diálogo); um aperto no pet também não conta.
- **Os balões vão para a tela pelo balão do M4**, na mesma chamada que anota
  a intenção `balao` (o erro, o cansado, a chamada, o tipo refinado); com o
  pet escondido ou a proteção de tela, nem a linha nem o balão. As reações
  continuam pelo animador do M3 (a `rajada` toca a `alert` de novo); o resto
  (a base, os selos, os voos, o pulso, o confete) espera quem desenha.
- **Na soneca**, a L1 toca o aceno no lugar da chamada (só reações pequenas,
  decisão 0053) e nada passa da L1; **escondido**, a L1 não toca e nada é
  repetido na volta: a escalada segue da fase em que o relógio dela está.
- **Os prazos** da escalada são recalculados na hora em que vencem (o Motor
  só sabe o "agora" do último lote); a máquina garante que um prazo olhado de
  novo no instante dele ainda vence, e o teste dela reprova o contrário.
**Por quê:** um aviso novo é uma notícia mesmo com outro escalando, mas duas
escaladas ao mesmo tempo seriam barulho; quem só leu o plano por 10 s não
"voltou" de lugar nenhum; e o balão já existe desde o M4 — deixá-lo para
depois faria a intenção dizer uma coisa e a tela outra.

## 0080 — A tela na implementação: a fotografia, a acomodação, o T0, o sono e a discrição (2026-10-05)

**Problema:** ao implementar a festa e a tela (decisão 0076, T5.8), o
`/v1/estado.tela` da decisão 0077 já existia com outro sentido (o estado da
aprovação: `ativa`, `sem_personagem`), e alguns casos ficaram em aberto: a
base no 0,8 s da acomodação, o T0 numa festa mesclada, a festa na soneca, o
que acorda o pet, e a discrição quando a fonte dos eventos do desktop cai.
**Escolha:**
- **`/v1/estado.fotografia`** é a fotografia de agora: a base (o estado da
  skin, a prioridade e a sessão que manda), o sono, os selos, a escalada
  (sessão, nível, tipo da espera e pulso), a festa dos últimos 3 s e a
  discrição. O `tela` continua sendo o da aprovação.
- **Na acomodação do Stop**, a sessão segura a prioridade que tinha: a base
  não pisca parada antes da festa.
- **O T0 é uma festa sem balão** (só o aceno). Um fim que entra numa festa
  (do mesmo mundo; uma festa por mundo) só toca se subir o nível, e o balão
  "N prontos: …" só sai com a festa no T1 ou acima.
- **Na soneca**, a festa é o aceno, sem confete, voo nem faixa (o balão
  fica); **com o "não perturbe"**, nenhum voo (o curto também); **escondido**,
  a intenção `festa` com `escondida` e nada mais.
- **O sono:** o relógio começa com o Motor (o pet nasce parado) e recomeça a
  cada evento de uma sessão que o cérebro acompanha (os dos agentes também:
  um workflow em segundo plano mantém o pet acordado) e a cada clique; um
  evento acorda com o despertar, o clique acorda sem ele (a risadinha toca
  por cima). O pronto não impede o sono; a espera e o erro, sim.
- **Os selos:** o "+N" conta as outras sessões com aviso de espera ou de erro
  ou ocupadas (pensando, trabalhando, compactando, esperando, no erro,
  cansadas); até 8 bandeirinhas, da mais velha para a mais nova, na cor do
  FNV-1a do nome do projeto (ou do `sid8`, sem pasta).
- **A discrição** continua ligada se a fonte dos eventos do desktop cai e
  volta (o fim do compartilhamento pode ter se perdido no meio); o balão da
  chamada diz "sessão N precisa de você" (a posição na lista do clique).
**Por quê:** o nome `tela` no `/v1/estado` já é lido pelos scripts e pelo
`bin/pet`; a base que pisca antes da festa seria um movimento sem
significado; um T0 é a resposta rápida e não merece balão nem quando chega
junto; e, na dúvida sobre o compartilhamento, o pet erra para o lado de não
mostrar o nome do projeto.

## 0081 — O compartilhamento de tela no Hyprland 0.56.2: o `screencast` que pisca com os quadros, as sessões contadas e a discrição que segura (2026-10-05)

**Problema:** a decisão 0076 deixou o compartilhamento de tela para o
socket2 (`EventoDesktop::Compartilhando`, a discrição 2 s depois e os nomes
de volta na hora em que ele para). Lido no código-fonte do 0.56.2
(`docs/pesquisa/11-tela-m5.md`), o `screencast>>ESTADO,TIPO` não diz se há
uma sessão de compartilhamento, e sim se há quadros sendo copiados: o `1` sai
num quadro copiado e o `0` meio segundo depois do último. Numa chamada com a
tela parada o sinal pisca, e o próximo desenho pode ser o do próprio pet (a
camada cobre o monitor): desligar a discrição no `0` mostraria o nome do
projeto justo no quadro que volta a ser compartilhado (a festa do Stop, logo
depois de o spinner do terminal parar). E há mais coisas no evento: várias
sessões ao mesmo tempo sem dizer qual, o `screencastv2` com o título da
janela compartilhada, o grim abrindo uma sessão de um quadro só, e o pet que
liga no meio sem saber das sessões de antes.
**Escolha:**
- **O adaptador do Hyprland** (`pet_wayland::hyprland::eventos`) lê só o
  `screencast>>ESTADO,TIPO`: o primeiro campo, `0` ou `1`; o tipo nem é
  guardado. O `screencastv2` é outro nome de evento e cai no "não uso", sem
  ser interpretado (como o `windowtitlev2`).
- **As sessões contadas na ligação** (a `Entrega` do leitor): um `1` soma, um
  `0` tira; só a primeira sessão manda `Compartilhando(true)`, e o fim da
  última, `Compartilhando(false)`. Um fim sem o começo visto (uma sessão de
  antes desta ligação) também vale como fim; uma ligação nova começa do
  zero; depois de uma perda na caixa, a tela compartilhada é contada de novo
  logo depois do `Ligado(false)`/`Ligado(true)` da decisão 0061. O socket2
  continua sempre drenado, na thread dele.
- **A discrição no Motor** (`motor::tela`): liga quando o sinal soma
  `DISCRICAO_APOS_MS` (2 s) num episódio (os piscares de meio segundo de uma
  tela parada somam; uma captura não chega lá); antes de ligar, sinais até
  `JUNTA_MS` (60 s) um do outro são o mesmo episódio; ligada, só desliga
  `SEGURA_MS` (5 min) depois do último sinal. A fonte dos eventos que cai
  desliga o sinal ali (o `desktop.compartilhando` também), e a discrição
  segura os 5 min: na dúvida, discreto (decisão 0080), mas nunca para sempre.
- **Testes:** a tradução (o `screencastv2` e um estado que não é 0 nem 1 não
  passam), as sessões contadas e a recontagem depois da perda no leitor; no
  Motor, a captura que não liga (e cinco seguidas que somam 2 s e ligam), a
  tela parada que pisca sem desligar com o nome de volta 5 min depois e a
  fonte que cai; o cenário `compartilhando-tela` refeito com a captura, a
  tela que pisca e os nomes que voltam; o canário do socket2 no daemon de
  verdade com segredos no `screencastv2` (nada no `/v1/estado`, no log nem no
  `/v1/debug/eventos`), duas sessões e a discrição ligada pelo laço. Duas
  mutações reprovaram: a discrição desligando no `0` (5 testes, o dourado
  entre eles) e o leitor sem contar as sessões (o teste dele e o canário).
**Por quê:** o que importa é não mostrar o nome do projeto numa tela que está
sendo vista por outros; o sinal do 0.56.2 diz "quadros saindo agora", e o
quadro que denuncia é justamente o que o pet desenha depois de uma pausa. Os
5 min cobrem as pausas de uma tela parada (os ritmos da base nunca passam de
30 s sem desenhar, fora o sono profundo), e o custo do erro é pequeno: o balão
diz "Prontinho!" sem o nome por mais alguns minutos depois da chamada.

## 0082 — A base segurada no animador: os ritmos de cada estado e o sorteio com a semente injetada (2026-10-05)

**Problema:** o cérebro anuncia a base (decisão 0076: o estado da skin da
sessão mais alta na prioridade), mas o animador só sabia o repouso do `idle`:
o Zeca ficava parado do mesmo jeito trabalhando, esperando ou dormindo, e o
sono profundo não parava os commits. Cada estado pede um movimento próprio
dentro do orçamento (decisão 0005): trabalhando e pensando quase parados, até
4 fps, com micro-ações sorteadas a cada 10–30 s (PLANO, "Movimento"; revisão
de produto em `docs/pesquisa/08-revisoes.md`), o sono até 2 fps, o sono
profundo sem commit nenhum, a espera no teto da L4 só na pose (o selo pulsa e
a rajada vem a cada minuto, decisão 0075). E o núcleo é puro: o sorteio não
pode ler o relógio nem o sistema.
**Escolha:**
- **`animador::Base`** (o estado da skin e o `Ritmo`), que o Motor passa ao
  pet a cada mudança da base anunciada (no fim de cada `observar_tela`, na
  escalada e em todo pet novo): `Pet::definir_base`, que só faz algo se a
  base mudou. A pose é o primeiro quadro da primeira tag do estado; a pose
  nova entra no próximo quadro, já, ou no fim da reação que estiver tocando;
  o arraste continua por cima até ser largado.
- **Os ritmos** (`animador::Ritmo`):
  - `repouso` (parado, pronto, erro e esperando até a L3): a pose e rajadas
    das tags do estado com a pausa do M1 (pelo menos 4 s e até 2 commits/s);
  - `quieto` (trabalhando, pensando, compactando): quadros de pelo menos 250
    ms e uma rajada a cada 10–30 s, sorteada (8 trechos de cada vez, que se
    repetem);
  - `laco` (dormindo e cansado): a primeira tag do estado em laço, com quadros
    de pelo menos 500 ms;
  - `parado` (o sono profundo e a espera na L4): só a pose, e o pet não marca
    prazo nenhum (o `animador::NUNCA` vira `None` no `Pet::sprite`, e o
    `Motor::desenhar` passa a deixar o prazo vazio quando nada muda sozinho).
- **Um estado que a skin não tem** cai nas reservas do catálogo
  (`estados::reserva`, agora aceitando o `idle`), e no fim no próprio `idle`.
- **O sorteio** (`pet_core::sorteio::Sorteio`, xorshift64*): o Motor guarda
  um e dá a cada pet novo uma semente tirada dele; o daemon semeia na partida
  (`Motor::semear`, com a hora e o processo), e quem não semeia (os testes, o
  `bichinho simular`) usa a fixa. As intenções não dependem dele.
- **Medido** em 30 min com o animador (sem reações): no Zeca original, o
  parado 0,95 commit/s, o pronto 1,31, o erro e a espera 2,0 (o teto do
  repouso: 23 e 12 quadros por rajada), trabalhando 0,77 e pensando 0,55
  (nenhum quadro abaixo de 250 ms), dormindo 1,92 (520 ms por quadro) e o sono
  profundo 0; no Zeca do pack, todos abaixo de 2,0 (dormindo 2,0, com o piso
  de 500 ms nos quadros de 100 ms).
- O laço do daemon passou a recusar um prazo absurdo (`checked_add`) em vez de
  estourar o `Instant`: o `u64::MAX` do estado segurado de um quadro só nunca
  chegava lá, mas a pose parada chegaria.
**Por quê:** a base é o que o Renan vê de canto de olho por horas; o
movimento grande fica no começo de cada estado (as reações) e o resto respira
dentro do orçamento, até parar de vez no sono profundo. Sortear as
micro-ações tira a cara de relógio do trabalho; a semente injetada mantém o
núcleo puro e os testes repetíveis.

## 0083 — Os selos ao lado do corpo e o selo do aviso que pulsa uma vez por segundo (2026-10-05)

**Problema:** o Motor anuncia os selos das outras sessões (decisão 0076: o
"+N", as bandeirinhas dos prontos na cor do projeto, o "…" da corrente) e o
pulso da L4 (decisão 0075: "pose de espera + selo pulsando a 1 Hz"), mas
nada disso ia para a tela. Os selos têm de ser nítidos (blocos inteiros, a
fonte dos balões), parados (o orçamento, decisão 0005), sem cobrir a área de
toque nem sair do monitor, e legíveis no tema escuro e no claro. A espera do
Zeca original já tem um «!» desenhado na arte, à direita da cabeça; a do Zeca
do pack, não (`docs/pesquisa/11-tela-m5.md`). E um pulso de 1 Hz de ciclo
inteiro (aceso e apagado a cada meio segundo) seria 2 commits/s só ele, mais
a rajada de cada minuto: acima dos 2/s da espera.
**Escolha:**
- **A fileira** (`motor::selos`): ao lado do corpo, na altura da cabeça, à
  esquerda (o pet mora no canto inferior direito) ou à direita sem espaço;
  sem caber nos dois lados, o lado maior, e as bandeirinhas mais novas saem;
  presa na área útil. Do corpo para fora: o selo do aviso, o "+N" (até
  "+99"), o "…" e as bandeirinhas (até 8, da mais velha para a mais nova). Um
  pixel do selo vale `dt` (a metade do D, a mesma dos balões e do "zZ"), em
  blocos inteiros e opacos; nada na área de toque (só o pet recebe o clique,
  e a área não muda).
- **Os desenhos:** o "+N" e o "…" na monogram, creme com a sombra de tinta,
  como o "zZ"; a bandeirinha com o mastro creme, o pano na cor do projeto e a
  sombra de tinta; o selo do aviso, um «!» de 6 por 14 pixels com a borda de
  tinta e o recheio amarelo (#FFD23F), fora da cabeça para não cair em cima
  do «!» da arte e para existir no pack.
- **A paleta dos projetos** (as 8 cores de `tela::cor`): vermelho #E5394B,
  laranja #F28C28, amarelo #FFD23F, verde #4CC35A, turquesa #2EC4B6, azul
  #3A86FF, roxo #9B5DE5 e rosa #FF7AB6 (vivas no tema escuro; a sombra de
  tinta dá a borda no claro).
- **O pulso da L4:** o selo do aviso troca de cor (amarelo, vermelho) uma vez
  por segundo, contado do começo do pulso: um commit por segundo. Com a pose
  parada da L4 (decisão 0082) e a rajada de cada minuto, a espera no teto
  fica em ~1,4 commit/s.
- **Quando:** o selo do aviso aparece com o aviso que o pet chama (a L1) e sai
  com ele; a fileira some no poof da viagem. O Motor pede o próximo quadro a
  cada mudança da tela (`redesenhar_ja` no fim de cada `observar_tela` e da
  escalada); sem mudança, a janela não faz commit.
**Por quê:** os selos dizem o que as outras sessões estão fazendo sem tirar a
base de quem manda, e ao lado do corpo eles não brigam com o balão (em cima)
nem com o "zZ" e o coração (nos cantos de cima). O selo do aviso fora da
cabeça funciona nas duas skins, e a troca a cada segundo ainda é um
batimento que o olho pega de canto, dentro do orçamento da espera.

## 0084 — O voo da escalada: até o alto-centro com o "!!", na grade de 34 ms, e de volta para a casa (2026-10-05)

**Problema:** a L3 (decisão 0075) e a volta do Renan (decisão 0079) pedem o
voo até o alto-centro do monitor com "!!" piscando a no máximo 2 Hz, e de
volta, até 3 vezes; o Motor anotava a intenção `voo`, e nada voava. O voo
mexe na célula do pet: tem de andar em múltiplos de D (nitidez), a até 30
quadros por segundo e curto (orçamento), com a área de toque seguindo o pet,
sem nunca gravar posição (a do Renan fica), e sem brigar com o arraste, a
viagem entre monitores, o poof, o pet escondido, a proteção de tela, a soneca
e o "não perturbe".
**Escolha:**
- **`motor::voo::Voo`**, puro: da casa ao alvo em 30 passos de 34 ms (1,02
  s, suave no começo e no fim), pairando 2,18 s com o "!!" trocando a cada
  272 ms (8 passos: 1,84 Hz, abaixo dos 2 Hz) e de volta em 30 passos; ~4,2 s
  ao todo. Toda duração é múltiplo do passo a partir do começo: nenhum quadro
  do voo sai a menos de 34 ms do anterior. A célula é sempre a casa mais um
  múltiplo de D em cada eixo.
- **O alvo:** o meio do corpo no meio da área útil, com o topo do corpo logo
  abaixo do "!!" (a margem de 16 px lógicos da borda, o "!!" e dois pixels de
  arte), preso na área e trazido para a grade de D da casa.
- **No Motor:** a intenção `voo` da escalada começa o voo (um de cada vez), se
  o pet está na tela, solto, fora de uma viagem (o poof inclusive), fora da
  soneca e sem o "não perturbe"; senão a intenção fica e o voo não sai. O pet
  segura o `dangle` (o voo da skin) e, no fim, larga e pousa (`land`). O
  desenho move a célula (`andar_voo`); a área de toque é a do corpo onde ele
  está; enquanto voa, os prazos do desenho (as asas, o balão, o pulso) entram
  no passo seguinte da grade do voo, e a fileira de selos dá lugar ao "!!",
  em blocos de D, em cima da cabeça. Nada grava posição: a casa volta no fim.
- **Interrupções:** o fim da escalada (a resposta, o clique que vê o aviso),
  a soneca e o "não perturbe" mandam de volta já (da célula de agora até a
  casa, em 30 passos); esconder e viajar acabam o voo na casa; um palco novo
  (outro monitor, outra escala) acaba o voo na posição salva; o arraste que
  pega o pet no ar acaba o voo onde ele está, e o arraste segue dali (soltar
  grava, como todo arraste).
- **Testes:** o voo puro (múltiplos de D, as fases, o fim na casa, os passos
  de pelo menos 34 ms e o pisca abaixo de 2 Hz, a volta antes da hora e a
  grade dela); no Motor, a L3 voando ao alto-centro com o "!!" em blocos de D
  e a área de toque junto, nenhum quadro a menos de 34 ms, a volta à casa sem
  nada para gravar e menos de 140 quadros no voo; a resposta no meio
  mandando de volta e o arraste pegando no ar; arrastando, a intenção fica e o
  voo não sai; a soneca manda de volta. Duas mutações reprovaram (voar
  arrastando; os prazos fora da grade do voo).
**Por quê:** o voo é o movimento grande da escalada, o que o olho pega de
longe; num monitor que repinta inteiro a cada commit (decisão 0005), ele tem
de ser uma rajada curta e regular, e nunca pode deixar o pet longe de onde o
Renan o pôs.

**Atualização (2026-10-05, T5.20): o voo que acaba sem quadros.** Com a
sessão bloqueada ou a tela apagada, o compositor não pede quadros (decisão
0018): o voo da L3 começava, o primeiro quadro ficava em voo e a máquina só
andava no desenho seguinte, que não vinha. O voo velho continuava "no ar" e
segurava o próximo: a volta do Renan (o voo na hora em que ele volta) não
saía, e o `/v1/estado.desenho` escondia a fileira de selos. Agora o voo que
já devia ter acabado termina pelo relógio antes de outro começar (o pouso,
como no desenho, só toca até 1 s depois do fim, senão o pet pousaria do nada
quando a tela acendesse), e o `desenho` mostra o voo e a fileira pelo
relógio, como o próximo quadro vai desenhar. Um teste com o quadro preso em
voo e a volta 80 s depois reprova sem cada uma das duas correções.

## 0085 — A festa na tela: a fonte de 12 confetes do T2, a chuva de 40 do T3 e o passo de 34 ms (2026-10-05)

**Problema:** a festa (decisão 0076) pede, além da reação e do balão que já
iam para a tela, 12 confetes no T2 e a chuva de confete no T3 (até 60
partículas, PLANO "Efeitos e balão"; a intenção diz 40), e a festa mesclada
que sobe de nível pede os efeitos do nível novo. O confete do M1 só servia ao
estresse (pedaços que quicam pela tela). O voo atravessando a tela e o
holofote "PRONTO!" são do M6; a reação do nível (`done_medium`,
`done_big`) já é um voo dentro da célula nas duas skins. Rajadas têm de
ficar em até 30 quadros por segundo e curtas (decisão 0005), e a reação
anima ao mesmo tempo que o confete.
**Escolha:**
- **`confete::Festa`:** pedaços que caem na grade de arte do pet (múltiplos
  de D a partir da célula, como o confete do estresse), nas cores do
  confete: a **fonte** (o T2: 12 pedaços de 2 pixels de arte saindo da
  cabeça, para cima, que a gravidade puxa de volta) e a **chuva** (o T3: 40
  pedaços de 3 pixels caindo do alto da tela inteira, em alturas
  diferentes). O que sai pelos lados ou por baixo some; o que ainda está
  acima da borda não aparece; a festa acaba quando o último pedaço sai.
  Determinístico pela semente (a do sorteio do Motor).
- **No Motor:** a festa com confete (a nova, ou a mesclada que subiu de
  nível, no lugar da de antes) começa o efeito; os passos andam um a cada 34
  ms contados do começo (os que venceram andam todos de uma vez), no fim da
  cena (para os índices dos outros elementos não mudarem), por no máximo 4,5
  s. Enquanto ele anda, os prazos do desenho entram no passo seguinte da
  grade dele (a reação, o balão): nunca dois quadros a menos de 34 ms. O voo
  da escalada, se houver, manda na grade. Esconder, viajar, um palco novo e o
  fim da conexão tiram o confete; uma janela que não cobre o monitor (o M8)
  não mostra confete nenhum.
- **O que não muda:** a soneca dá o aceno sem confete, o "não perturbe" só
  tira o voo (decisão 0080), o fim de máquina e o modo discreto continuam
  pequenos (o pulinho sem balão, sem confete), e escondido nada toca.
- **Testes:** a fonte (sobe, cai, acaba entre 15 e 80 passos, na grade e na
  tela) e a chuva (pela tela inteira, mais de 20 pedaços juntos, acaba em
  até 130 passos); no Motor, com as reações tocando como no laço, o T2 com
  até 12 pedaços na grade de D e o T3 com a chuva, os dois acabando em até
  4,5 s e sem quadros a menos de 34 ms; sem confete na soneca e numa janela
  pequena; a festa mesclada do T2 ao T3 trocando a fonte pela chuva. Uma
  mutação (os prazos fora da grade do confete) reprovou.
**Por quê:** o confete é o que diz "foi grande" de canto de olho, e a chuva
pela tela é a parte do T3 que cabe no M5 sem arte nova; no relógio de 34 ms,
o pior caso (a reação mais o confete) continua uma rajada de até 30 quadros
por segundo, que acaba sozinha.

## 0086 — O `/v1/estado.desenho`: o que as intenções viraram na janela (2026-10-05)

**Problema:** o `/v1/estado` tinha as intenções (decisão 0077) e a
fotografia do que o cérebro decidiu (decisão 0080), mas não o que a janela
está desenhando: com a sessão do Renan quase sempre bloqueada, a
conferência ao vivo da segunda metade do M5 (a base no animador, os selos, o
voo, o confete) não teria como ser feita sem olhar a tela. E o que vai para
o `/v1/estado` só pode ser metadado.
**Escolha:** `Painel::desenho` (o `/v1/estado.desenho`), montado do estado
do Motor na hora do painel: a `base` e o `ritmo` do animador, a fileira de
`selos` (o `aviso` `normal` ou `aceso`, se `pulso`, o `mais`, a `corrente` e
quantas `bandeiras`; vazia no voo e na viagem), o `voo` (a `fase` e o
`motivo`) e o `confete` (quantos pedaços na tela). Vazio sem o pet desenhado
(sem conexão, escondido). Só enums e contagens: nem o nome do projeto, que o
balão e a fotografia já mostram, entra aqui. A seção da tela no
`docs/CENARIOS.md` diz o que cada intenção desenha. Testes no Motor (a
espera, o voo, a L4 com o pulso aceso, a festa com confete, o JSON sem o
nome, nada sem janela) e no núcleo do daemon (o `desenho` no
`/v1/estado`).
**Por quê:** a intenção diz o que o cérebro quer, a fotografia diz o que ele
decidiu agora, e o desenho diz o que a janela está fazendo com isso: com a
tela fora do alcance, é a terceira linha que fecha a conta (e o
`commits_total`, que só anda com a tela acesa).

## 0087 — As demonstrações do `bin/pet testar` para a tela do M5 (2026-10-05)

**Problema:** o `bin/pet testar` só tinha o `rapido` (o aceno) e o `pequeno`
(o pulinho). A segunda metade do M5 precisa de demonstrações ao vivo do que
a tela faz (o voo curto com o confete, a chuva do T3, a chamada da pergunta,
a festa de duas sessões), conferidas pelo `/v1/estado` com a sessão
bloqueada, sem nunca mexer nas sessões reais do Renan nem nas aprovações.
**Escolha:**
- **Cenários novos**, todos com `PET_TESTE=1` pelo hook de verdade (o mundo
  de teste, que some em 60 s e nunca se mistura com o real; o T3 do mundo de
  teste não segura o do real): `medio` (uma edição e 4 min de Bash: 4,8
  pontos, `done_medium` e a festa com 12 confetes), `grande` (12 min de Bash:
  12,8 pontos, `done_big` e a chuva de 40; um segundo `grande` em 10 min vira
  T2, e o `testar` diz isso e espera o `done_medium`), `pergunta` (o
  `AskUserQuestion`: o aviso de espera do tipo pergunta, a chamada `alert` e a
  L1; depois a resposta e o Stop, que acabam a escalada e dão o aceno) e
  `dois-prontos` (duas sessões, `demo-api` e `demo-web`, terminando uma logo
  depois da outra: uma festa só, a mesclada, e o balão "2 prontos: demo-api,
  demo-web").
- **O que mostra:** o turno (a reação, o nível, a pontuação, os tetos e os
  contadores), as intenções da sessão de teste (a festa, a escalada, a
  chamada) e o `/v1/estado.desenho`, só metadados; o veredito no fim. Cada
  demonstração espera a festa de antes passar (até 4 s, pela
  `fotografia.festa`): um fim até 3 s depois de outra festa entraria nela.
- **O desenho pelo relógio:** o `desenho.confete` conta os pedaços na hora do
  painel (os passos andam numa cópia), e o voo que acabou com a tela apagada
  não pousa do nada quando ela acende: o `/v1/estado` diz o que a janela vai
  mostrar mesmo sem quadros.
- **Testes** (`tests/testar.rs`, com o daemon de verdade e o hook nativo):
  `medio`, `grande` e o segundo `grande` virando T2; `pergunta` e
  `dois-prontos`, só com sessões de teste e sem o texto do prompt no
  `/v1/estado`.
**Por quê:** com a tela do Renan quase sempre bloqueada, a demonstração que
vale é a que se confere sozinha pelo `/v1/estado`; e um teste no mundo de
teste pode rodar na produção a qualquer hora sem tocar no que é dele.

## 0088 — A prova do orçamento e da nitidez da tela do M5 em relógio falso (2026-10-05)

**Problema:** a segunda metade do M5 pôs a base, os selos, o pulso, o voo e o
confete na tela; o orçamento de commits (decisão 0005: parado e esperando em
média até 2/s, 0 no sono profundo, rajadas até 30 por segundo e curtas) e a
nitidez (D inteiro, pixels inteiros, movimento em múltiplos de D) precisam de
prova em teste, com o personagem de produção, e não só nas contas de cada
decisão. Com a sessão do Renan bloqueada, a medição na tela fica pendente.
**Escolha:** testes do Motor em relógio falso, com o compositor mostrando
cada quadro na hora (o pior caso) e as reações tocando como no laço do daemon
(`quadros_como_o_laco`), contando os commits (os quadros novos e os só de
estado), com o Zeca original de produção (`zeca-livre-escuro`, D = 4 no
`pequeno` do eDP-1) e com a skin de teste:
- **trabalhando por 20 min** (um Bash a cada 30 s): 0,68 commit/s no Zeca
  original (0,18 na de teste), nenhum quadro a menos de 250 ms (até 4 fps);
  o teste exige até 1/s;
- **a espera na L4 por 10 min** (a pose parada, o pulso a uma troca por
  segundo, a rajada a cada minuto): 1,38 commit/s (1,03); antes dela, a
  chamada, as rajadas da L2 e os três voos da L3 sem quadro a menos de 34
  ms; depois dos 30 min da L4, o selo parado sem commit nenhum em 10 min;
- **parado por 30 min** (o repouso, o bocejo, o laço do sono e o quadro do
  sono profundo): 1,66 commit/s (1,67); depois, **nenhum commit** em 60 min
  de sono profundo;
- **a rajada do T3** (o `done_big` e a chuva): 118 commits em 5 s (99),
  nenhum a menos de 34 ms, o confete fora da tela antes dos 5 s, e o minuto
  seguinte (o pronto no repouso) até 2/s;
- **a nitidez**, com o Zeca original no D = 4: em cada quadro da festa T3 com
  os selos, da espera com o «!», do voo com o "!!" e do pulso, o sprite no D
  do palco e na grade de D da casa, cada bloco em múltiplos da metade do D
  (os selos, o balão) ou do D (o "!!", o confete), os glifos na metade do D,
  tudo opaco e dentro do monitor.
Uma mutação (o pulso com um ciclo inteiro por segundo, duas trocas) reprovou
o orçamento da L4 e o teste do pulso, como a decisão 0083 previa.
**Por quê:** o orçamento é a condição de a camada do tamanho do monitor
continuar valendo (decisão 0005); com o pior caso em teste, uma mudança que o
estoure reprova o `bin/pet verificar` antes de chegar à tela do Renan. A
medição na tela (CPU e GPU do Hyprland, `scripts/medir-custo.sh`) continua a
ser feita com a tela acesa e desbloqueada.

## 0089 — Revisão das correntes: os agentes que nascem depois do Stop, o pedido digitado com a corrente aberta e a continuação depois do fim dela (2026-10-05)

**Problema:** a revisão adversarial do cérebro do M5 (lente das correntes e
dos cenários) achou três buracos, reproduzidos no `bichinho simular`:
- **O agente que nasce depois do Stop virava a thread principal.** Os
  agentes de um workflow nascem quando o workflow quer, e o `bgi` do Stop é o
  id do workflow, não o deles. Um `SubagentStart` de um `aid` nunca visto
  dentro da acomodação cancelava o Stop (a corrente nem abria, e o workflow
  acabava num T0 de máquina); depois dela, reabria o turno comemorado, que
  engolia o trabalho do agente e fechava `substituido` no prompt seguinte (13
  min de ferramenta viraram um T1 de 1,05).
- **Todo Stop com a corrente aberta era dela** (decisão 0073): um pedido que
  o Renan digita enquanto um agente em segundo plano trabalha acabava sem
  festa e sem pronto, somado na festa do agente ou perdido se a corrente
  expirasse. Era um retrocesso do M4 e o contrário da festa proporcional ao
  trabalho de cada pedido.
- **A continuação de um Stop hook depois do fim da corrente** pontuava só o
  turno, com o teto de máquina, e trocava o registro da corrente pelo dele.
**Escolha:**
- **O agente novo em segundo plano:** um `SubagentStart` de um `aid` nunca
  visto não é a thread principal (1) na acomodação de um Stop que listou
  agentes em voo: conta como subagente desse turno, que vai para a corrente,
  e não cancela a acomodação; (2) com a corrente aberta e nenhum turno aberto
  daquele prompt: conta na corrente e o `aid` passa a ser dela, então o que
  ele faz conta nela até com um prompt digitado aberto. Com o turno daquele
  prompt aberto, é dele (o Agent em primeiro plano), como no M3; depois de um
  Stop sem agente em voo, continua sendo a thread principal (a continuação
  de um Stop hook).
- **A ferramenta de um agente** que é da corrente (em voo no último Stop ou
  nascido para ela) conta na corrente; a de um agente que sobreviveu ao turno
  dele, com a corrente aberta, também; sem corrente, fica ignorada como antes.
- **O pedido digitado com a corrente aberta:** um turno digitado cujo Stop
  só lista agentes que já são da corrente (não começou nada novo em segundo
  plano) festeja sozinho, pelo trabalho dele, com o pronto; a corrente segue
  (o "…") e fecha no Stop sem agente em voo. Um pedido digitado que começa
  trabalho novo em segundo plano entra na corrente, como o que a abriu; um
  Stop digitado sem nada em voo fecha a corrente com a soma, como antes. Os
  turnos de máquina (notificação, tique) continuam entrando nela. Com o hook
  antigo (sem `orig`), um prompt com a corrente aberta continua sendo
  notificação (a degradação da decisão 0073).
- **A continuação depois do fim da corrente:** o turno em que a corrente
  fechou guarda a corrente; se ele reabre (o `sha`), o Stop seguinte o põe de
  volta nela e pontua a soma de novo, com o t0, a origem e o resumo dela, e
  só festeja se o nível subir.
- **Testes:** no cérebro, os agentes de um workflow nascendo na acomodação e
  depois do turno comemorado, o pedido digitado (sozinho, com o pronto; o que
  lança agente novo entra; o agente do workflow que nasce no meio dele e
  sobrevive) e a continuação depois do fim da corrente; os dourados
  `workflow-agentes-depois-do-stop` e `digitado-durante-a-corrente`, e o
  `workflow-longo` com o terceiro agente nascendo 12 s depois do Stop e a
  última notificação com `orig: notificacao` (o 2.1.288 nunca manda o
  `src`). Quatro mutações reprovaram (o agente novo como thread principal, o
  pedido digitado dentro da corrente, a continuação sem a corrente, a
  ferramenta do agente que sobreviveu ao turno ignorada).
- **Limite:** um agente de workflow que nasce com um pedido digitado aberto
  conta nesse pedido (o ponto do subagente e o que ele faz enquanto o pedido
  está aberto): nada separa o nascimento dele do de um Agent em primeiro
  plano. O que ele faz depois que o pedido fecha é da corrente.
**Por quê:** o fim de um pedido em segundo plano é quando o último agente
volta, e um workflow é o pedido mais longo que existe: perder o trabalho
dele ou fechá-lo num T0 é o contrário da festa proporcional ao trabalho
(decisão 0003). E um pedido que o Renan digita é dele, com o fim dele: não
pode sumir dentro da corrente de outro pedido.

## 0090 — Revisão da espera e da escalada: a pose pelo aviso, o diálogo visto no terminal da sessão, a volta com a conta dela e o voo que ninguém viu (2026-10-05)

**Problema:** as revisões adversariais do M5 (lentes do cérebro e da tela)
acharam, e uma sessão aninhada do 2.1.288 conferiu (pesquisa em
`docs/pesquisa/10-cerebro-m5.md`, seção da revisão):
- **O diálogo dispensado nunca saía.** O Esc numa pergunta e um plano
  recusado sem comentário não mandam evento nenhum (nem `PostToolUse`, nem
  `PostToolUseFailure`, nem Stop, nem `idle_prompt` em 80–94 s). A sessão
  ficava "esperando" até o próximo prompt, ou 12 h: a escalada inteira (L2 a
  L4) por um diálogo que não existia mais e o sono bloqueado. O clique que
  levava ao terminal tirava o aviso, mas não a pose: a prioridade vinha do
  estado da sessão, e a espera em repouso ficava a 2 commits/s por até 12 h.
- **Olhar qualquer terminal do Claude calava a escalada de outra sessão:**
  com o Renan trabalhando no terminal de A, a pergunta de B nunca passava da
  L1 (a decisão 0079 dizia "lá ele já vê o diálogo", e no terminal de A não
  vê).
- **A volta quase nunca aparecia** com a proteção de tela aos 150 s e o
  bloqueio aos 900 s (a configuração do Renan): os voos da L3 saíam com ele
  longe e gastavam a conta da volta; o primeiro toque chegava com a proteção
  de tela ainda aberta, e o voo não saía nem tentava de novo; bloqueado, o
  voo saía embaixo da tela de senha e acabava pelo relógio antes do
  desbloqueio.
- **A janela que fechava no meio do voo** (o HDMI desplugado com o pet nele)
  deixava o pet segurando o voo da skin em laço, a uns 12 commits/s: o voo
  que acabava sem palco não largava.
**Escolha:**
- **A pose pelo aviso:** a prioridade "esperando você" vem do aviso de
  espera, e só enquanto ele segura a base: até o teto da escalada (o fim da
  L4, 35 min depois do aviso) ou, com o diálogo visto no terminal da sessão,
  até `ESPERA_VISTA_NA_BASE_MS` (2 min) depois. Depois, o selo "!" fica
  parado, o pet volta ao repouso e pode dormir com ele; o aviso continua para
  o clique e a lista. O clique que vê o aviso solta a pose na hora. Uma sessão
  "esperando" sem aviso (vista, ou um gatilho atrasado) conta como parada.
- **O diálogo visto no terminal da sessão:** com a janela certa da sessão em
  foco e o Renan presente por `ESPERA_VISTA_MS` (5 s), contados do mais tarde
  entre o aviso, a janela ficar ativa e ele voltar a mexer, o diálogo conta
  como visto (a intenção `escalada` com o motivo `vista`, e `vista` na
  `fotografia.escalada`): daí em diante nada passa da L1 (nem rajada, nem
  voo, nem pulso, nem o voo da volta), e um voo no ar volta para a casa. O
  Esc e o "não" são feitos no terminal da sessão: o diálogo dispensado conta
  como visto e não escala com o Renan indo embora. Sem a janela certa da
  sessão, nada conta como visto, e o teto solta a pose.
- **Olhar a espera** é ter em foco o terminal da sessão que espera, quando a
  janela dela é certa; sem ela, um terminal do Claude qualquer, como antes.
  Vale para a escalada e para a volta (corrige a decisão 0079).
- **A volta:** os voos da volta têm a conta deles (`VOLTAS_MAX`, até 3 por
  aviso), fora dos 3 da L3, e o próximo da L3 espera os 60 s dele depois de
  um. A volta com o pet fora da tela (a proteção de tela ainda aberta) espera
  até `VOLTA_VALE_MS` (2 min) o pet poder aparecer e voa então, com a L3 dela
  no motivo `voltou`. A volta acorda o pet (o teto pode ter soltado a pose e
  deixado ele dormindo com o selo). No desenho, que não muda as intenções: o
  voo da volta que não pôde começar (sem palco: a camada ainda voltando da
  proteção de tela) ou que acabou sem a janela mostrar quadro nenhum (a
  sessão bloqueada: o compositor segura o primeiro) sai de novo quando a
  janela mostrar quadros (o palco pronto, ou o quadro preso mostrado no
  desbloqueio), por até `VOLTA_POR_MOSTRAR_MS` (2 min).
- **A janela que fecha no meio do voo:** o `Sumiu` acaba o voo e o confete,
  como o esconder; o voo que acaba sem palco larga o voo da skin.
- **Testes:** na escalada, os voos da volta com a conta deles e a volta que
  espera o pet aparecer (e sai no prazo, ou com o "não perturbe"); no Motor,
  o clique que vê a espera solta a pose e o pet dorme, a volta que ninguém
  viu com a sessão bloqueada sai no desbloqueio (uma intenção só), a janela
  que fecha no meio do voo larga o voo, e o orçamento depois do teto (o
  repouso e o sono profundo sem commit, com o selo); os dourados novos
  `pergunta-noutro-terminal`, `pergunta-dispensada` e
  `pergunta-com-protetor-de-tela`, e o `pergunta-ausente` (o voo da volta e
  o pet dormindo depois do teto) e o `pergunta-com-volta` (o terceiro voo da
  L3) refeitos. Sete mutações reprovaram (olhar qualquer terminal, sem a
  espera vista, a pose pelo estado, a volta gastando os voos da L3, a volta
  escondida perdida, sem mostrar a volta de novo, o voo sem palco sem
  largar).
**Por quê:** chamar por um diálogo que já não existe é o pior erro da
escalada: ensina o Renan a ignorar o pet. O diálogo dispensado não manda
nada, mas é dispensado no terminal da sessão, e isso o desktop vê; quem viu o
diálogo e foi embora sabe dele, e o selo parado basta. A volta é o momento em
que a chamada mais vale, e só vale se aparecer na tela.

## 0091 — Revisão da tela: o orçamento da espera, o "+N" na acomodação, o erro escondido e o sono, a tela compartilhada como o "não perturbe" e a medição ao vivo com o pet acordado (2026-10-05)

**Problema:** as revisões adversariais do M5 (lentes da tela e das regras)
acharam:
- **A espera passava do orçamento.** A base de espera ficava no repouso, no
  teto de 2 commits/s (as tags de espera do Zeca original são densas,
  decisão 0082), e a escalada tocava por cima: medido em relógio falso com o
  Zeca original (D = 4), 2,85 commits/s nos 5 primeiros minutos de um aviso
  com o Renan longe e 2,0 com ele lendo no terminal. A prova da decisão 0088
  só tinha medido a L4.
- **O "+N" piscava na acomodação do Stop:** ele contava as sessões pelo
  estado cru, e a sessão que parava saía dele 0,8 s antes de a bandeirinha
  aparecer (a base já segurava a prioridade de antes, decisão 0080).
- **O susto do erro e o bocejo do cansado tocavam escondidos** (na proteção
  de tela, com o pet escondido), com a intenção anotada, contra a decisão
  0079.
- **O erro segurava o sono por 2 h:** ele fica 60 s na tela, mas o aviso dele
  (2 h) bloqueava o sono, com o pet acordado no repouso sem nada que dissesse
  por quê.
- **A tela compartilhada só tirava os nomes:** a decisão 0010 diz que o "não
  perturbe" e o compartilhamento de tela "deixam tudo discreto", e o PLANO
  estreitou isso sem decisão; as rajadas, os voos ao alto-centro e o voo da
  festa apareciam para quem assistia.
- **O `scripts/medir-custo.sh` media o sono como "parado":** ele sobe uma
  pilha de dev nova e pede para ninguém mexer; aos 3 min o pet do M5 boceja e
  dorme (decisão 0076), e as fases "parado" depois disso mediam o laço do
  sono (no limite dos 2,0 da faixa). E não havia fase do M5.
**Escolha:**
- **O ritmo atento** (`animador::Ritmo::Atento`): o repouso com até
  `COMMITS_POR_S_ATENTO` (1) commit/s, para a espera na L1 e o erro, que têm a
  chamada, as rajadas e o balão por cima. Da L2 em diante, a espera fica só
  na pose: quem anda são as rajadas, os voos e o pulso do selo. Medido em
  relógio falso com o Zeca original: a espera chamando a 1,42 commit/s nos 5
  primeiros minutos e 1,39 em 15; lida no terminal da sessão, 0,95 em 10
  min; o erro, 1,02 nos 60 s dele (a skin de teste, menos). O teste novo do
  orçamento reprova as duas metades (a pose parada só na L4; a L1 no repouso
  de sempre).
- **O "+N"** conta a sessão pela prioridade que ela segura: na acomodação, a
  de antes, e ele muda só quando a bandeirinha aparece. Uma espera sem aviso
  (vista) não conta.
- **Escondido**, a entrada no erro e no cansado não toca nem anota, e fica
  vista: nada se repete na volta.
- **O erro não segura o sono:** depois dos 60 s dele na tela, como o pronto
  (corrige a decisão 0080); o aviso continua para o clique e o "+N".
- **A tela compartilhada é discreta como o "não perturbe"** (decisão 0010):
  além dos balões sem nome, nada passa da L1, nada voa (nem a escalada nem a
  festa), e um voo no ar volta para a casa quando a discrição liga; o
  confete fica, como no "não perturbe" (decisão 0080).
- **A medição ao vivo** (`scripts/medir-custo.sh`): antes de cada fase
  "parado", uma sessão de teste (`PET_TESTE=1`) manda um `SessionStart` pelo
  hook de dentro da imagem, com o pet escondido (o despertar não toca), e a
  fase só vale com `fotografia.sono` acordado e `desenho.base` `idle` no
  começo e no fim (uma sessão real do Renan trabalhando no meio para a
  medida). Duas fases novas: `trabalhando` (uma sessão de teste, até 1
  commit/s) e `dormindo` (o laço do sono depois de o pet dormir sozinho, de
  0,3 a 2 por segundo). O sono profundo (aos 30 min) fica com o teste em
  relógio falso. Não rodado aqui: a sessão estava bloqueada.
**Por quê:** a camada do tamanho do monitor só vale com o orçamento (decisão
0005), e o que pega o olho na espera é o movimento que começa (a chamada, a
rajada, o voo), não a base que respira por baixo dele. O "+N" que pisca é
movimento sem significado; o que toca escondido ou dorme acordado engana; e
quem compartilha a tela numa chamada não quer o pet voando pela tela dos
outros.

## 0092 — O que ficou da revisão do M5: o "✳" fora, a presença no casamento da janela, o T3 que não tocou, os limites conhecidos e as docs reconciliadas (2026-10-05)

**Problema:** as revisões adversariais do M5 deixaram pontos menores:
- a regra do PLANO "o turno fecha quando o título do terminal focado vira ✳
  com uma só sessão trabalhando" não foi feita, e nenhuma decisão a tirou;
- um prompt que chega com o Renan longe do teclado e do mouse (`ocioso`)
  nunca casa a janela da sessão, o que muda a regra das decisões 0055 e 0060,
  e só o PROGRESS do T5.4 e um comentário diziam isso (a decisão 0073 usa o
  longe só como prova do tique, com `crn > 0`);
- o intervalo de 10 min do T3 era gasto mesmo quando o T3 não aparecia (o pet
  escondido, a proteção de tela, a soneca que o troca pelo aceno);
- limites sem registro: o tique de um `/loop` com o Renan no teclado e a
  janela da própria sessão em foco, as mensagens de teammate e de canal, e o
  "não perturbe" que só chega com os eventos;
- as docs desencontradas (o PLANO com o pulso "a 1 Hz", a faixa do T3 em
  "2,5–4 s" e a lista do M6 com o que o M5 já fez; o CLAUDE.md com o rumo e as
  pendências velhos e sem como voltar a produção para a `main`; o README com
  a regra do M3 e sem a notificação do shell no hook antigo), o comentário
  partido do `bin/pet`, o log "reação yawn da sessão " sem sessão, e
  pronomes para o Renan em textos novos;
- as sobras das sessões aninhadas das pesquisas em `~/.claude` e em
  `/tmp/claude-1000`.
**Escolha:**
- **O "✳" fica de fora:** no 2.1.288 o título do terminal fica ✳ o tempo todo
  (pensando, num Bash de 20 s, com um diálogo na tela e depois do Esc;
  conferido no tmux, decisão 0090), então ele diz "terminal do Claude", não
  "parado". O turno que o Esc deixa aberto fecha no prompt seguinte, no
  `idle_prompt` ou no `SessionEnd`, e o estado volta a parado pelo prazo de 5
  min (decisão 0076); a espera que ele deixa tem as regras da decisão 0090.
  O PLANO ganha a nota.
- **A presença no casamento da janela** (atualiza a decisão 0073): um prompt
  que chega com o Renan longe do teclado e do mouse nunca casa a janela da
  sessão: sem entrada nenhuma, a janela ativa não é a de onde o prompt saiu
  (o Remote Control, um prompt colado por outro programa, o tique sem `crn`). O
  próximo prompt digitado casa como antes; até lá, o clique cai no balão.
- **O T3 que não tocou** (escondido, na proteção de tela, ou o aceno da
  soneca) devolve o intervalo: o próximo T3 conta do T3 de antes. A sessão
  bloqueada não entra (o Motor não a vê).
- **Os limites conhecidos**, escritos:
  - o tique de um `/loop` com o Renan no teclado e a janela da própria sessão
    em foco (outro painel do tmux, outra aba do mesmo terminal) conta como
    digitado (o pronto e a janela casada de novo): o aviso de ocioso do
    Wayland é do teclado inteiro e não diz para qual janela vão as teclas, e
    uma segunda notificação de 1–2 s não separaria o painel do lado;
  - as mensagens de teammate (`<teammate-message>`) e de canal (`<channel
    source=…>`) existem no binário do 2.1.288, e não foi visto ao vivo se
    passam pelo `UserPromptSubmit`; se passarem, contam como digitadas
    (`orig` comum). O conserto é um valor novo e fechado do `orig` no hook
    (com canários e decisão própria), para quando o Renan usar times de
    agentes ou canais;
  - o "não perturbe" vem no `dnd` de cada evento do Claude (o pet nunca lê a
    pasta do Omarchy, que guarda o histórico da área de transferência):
    ligado no meio de uma espera, vale no próximo evento de qualquer sessão;
    o clique direito (a soneca) cala na hora;
  - a permissão de um Bash demorado segura a espera até o `PostToolUse` (só
    a pergunta e o plano têm `PreToolUse`); com a decisão 0090, 5 s no
    terminal da sessão param a escalada.
- **As docs reconciliadas:** as notas de correção do PLANO (o "✳", o pulso, o
  T3 de até 4,5 s, a lista do M6); o CLAUDE.md (o estado com a revisão, o
  rumo, as pendências com a conferência na tela do M5, o PR e as sobras, a
  volta da produção para a `main` e as pegadinhas novas); o README (a festa
  pelo trabalho no lugar da regra do M3, o hook antigo com a notificação do
  shell e o prompt digitado com a corrente aberta, o que mudou na espera); o
  `docs/CENARIOS.md`; o comentário do `bin/pet`; o log "reação yawn do pet";
  e as descrições dos cenários sem pronome para o Renan. As decisões 0079,
  0084 (a atualização) e 0087 ainda dizem "ele" para o Renan em alguns
  lugares: o DECISIONS só cresce (a única reescrita permitida foi a da 0035),
  então ficam; o texto novo evita.
- **As sobras:** as da sessão aninhada desta revisão (a pasta do projeto em
  `~/.claude/projects`, a de `/tmp/claude-1000` e o plano em
  `~/.claude/plans`) foram apagadas; as das pesquisas do T5.1 e do T5.2 e as
  linhas no `~/.claude/history.jsonl` (um arquivo que as sessões do Renan
  escrevem o tempo todo) ficam listadas no CLAUDE.md, para apagar só com o OK
  do Renan.
**Por quê:** uma regra do plano que nenhum código segue é uma promessa que
ninguém cumpre; melhor dizer por quê e o que a substitui. Os limites são o que
a semana de calibração do M7 vai encontrar: escritos, não são defeitos a
descobrir de novo.

## 0093 — A memória das sessões: o pet que reinicia não esquece as sessões abertas do Claude (2026-10-05)

**Problema:** o cérebro guarda as sessões só na memória do processo. Cada
reinício do pet (toda atualização refaz a produção, um crash, um `docker
restart`) esquecia todas as sessões abertas, e uma sessão parada não manda
nada (o `idle_prompt` sai uma vez por turno, ~60 s depois do Stop): o clique
no Zeca dizia "nenhuma sessão do Claude aberta" com o Renan cheio de sessões
abertas, até cada uma ser usada de novo (relatado pelo Renan em 2026-10-05).
O Zeca tem de acompanhar todas as sessões abertas (decisão do Renan). E os
prazos do cérebro contam num relógio monotônico que recomeça do zero com o
processo: guardar os instantes dele não serviria.
**Escolha:**
- **O arquivo** `/state/sessoes.json` (`pet_core::memoria`, versão 1, no
  máximo 64 sessões, `MAX_SESSOES`, em até 256 KiB): só as sessões reais,
  nunca as de teste, e só metadados: o `sid` inteiro (opaco, para casar os
  próximos eventos; no `/v1/estado` continua só o `sid8`), o nome da pasta do
  projeto, a origem, o estado e desde quando, a hora do último evento, os
  agendamentos do último Stop (o `crn`, que separa o tique do prompt
  digitado), o aviso pendente (o tipo, a espera, desde quando e, no aviso que
  o pet chamava, o nível da escalada e a hora em que o Renan viu o diálogo),
  a janela do terminal (o endereço, a instância do compositor a que ele
  pertence, a certeza e a hora do casamento) e os ids de terminal. Nada de
  prompt, título de janela ou caminho; os tempos são de parede. O boot id da
  máquina vai junto.
- **A gravação** é só do laço principal, nunca da entrada HTTP nem do hook:
  no batimento de 5 s, se a memória mudou desde a última gravação (no máximo
  a cada 4 s), e no SIGTERM, antes do adeus ao compositor (o
  `stop_grace_period` é de 5 s). De uma vez: um arquivo temporário na mesma
  pasta, só do dono (0600), e o `rename`. Um `kill -9` perde no máximo os
  últimos segundos.
- **A leitura** é na partida, antes de achar o compositor (no boot o pet sobe
  antes dele; nada espera por ele). A conferência é a do fio v1, campo a
  campo: uma sessão com um campo ruim fica de fora; o arquivo que não é este
  JSON, de outra versão ou grande demais é ignorado inteiro, com um aviso que
  não cita o conteúdo, e o pet começa vazio. A primeira gravação troca o
  arquivo ruim, e um que falta é só a primeira partida.
- **Os relógios:** a hora de parede gravada vira o instante do relógio novo
  (`Agora::no_laco`), negativo quando é de antes da partida. O cérebro passa a
  guardar o último evento, a entrada no estado e o aviso como `Instante` (ms
  com sinal), e a escalada o começo do aviso; todo prazo sai deles
  (`cerebro::depois`), e um prazo de antes da partida já venceu. Assim os
  prazos restaurados são os de antes, contados no relógio novo, e nada muda
  para o pet que não reiniciou (os testes e os dourados de antes passaram sem
  mexer). Um teste restaura com o relógio do laço em 0 e a parede 30 s
  adiante e confere cada prazo ao milissegundo.
- **O que volta** (`Cerebro::restaurar`) é o que ainda vale pelas regras de
  sempre, contadas das horas de antes: a sessão com o último evento há menos
  de 12 h (`VIDA_SESSAO_MS`) e de uma origem que o config aceita, as mais
  novas primeiro; o estado com o prazo dele; o pronto e o erro até 2 h; o
  "esperando você" enquanto a sessão vive. A sessão guarda a hora do último
  evento de antes (a vida de 12 h e o `SessionEnd` a tiram como antes) e vai
  marcada `restaurada` no `/v1/estado.sessoes` até o próximo evento dela.
- **O turno aberto não volta, e a sessão volta no estado dela só pelo prazo
  dele.** O Stop pode ter se perdido com o pet fora (o hook é assíncrono, não
  acha o pet e não tenta de novo). Nada de turno, corrente ou festa: a sessão
  que trabalhava (pensando, trabalhando, compactando) volta assim até 5 min
  depois do último evento dela, o prazo de sempre de um turno cujo Stop não
  vem (o do Esc, decisões 0076 e 0092), ou até o `idle_prompt`, que a deixa
  parada sem festa. O próximo evento do turno abre um turno implícito, como no
  M3, e o Stop festeja o que veio depois da volta, com o pronto. Voltar
  parada de cara mostraria "parado" com o Claude trabalhando até o próximo
  evento; o prazo curto erra menos, e para o mesmo lado de um Stop perdido
  sem reinício nenhum.
- **A volta é quieta:** nenhuma reação, festa, olá, balão ou chamada de novo.
  O erro e o cansado entram como já vistos (sem o susto) e ficam o resto dos
  60 s deles; os avisos de espera contam como já chamados (sem a L1 de
  novo). A escalada do aviso de espera mais velho **segue do tempo que
  passou** (`Escalada::retomada`): o nível de antes; as rajadas e os voos
  cuja hora já passou contam como gastos; o próximo de cada um sai um
  intervalo inteiro depois da volta; a fase de agora sobe o nível nas horas
  dela (a L4 com o pulso aos 5 min do aviso, o teto aos 35, contados do aviso
  de antes); o diálogo que o Renan já tinha visto continua visto. Recomeçar
  da base quieta repetiria a L2 e a L3 de um aviso que já passou por elas, e
  ensinaria o Renan a ignorar o pet (decisão 0090). A intenção `restauracao`
  diz quantas sessões e avisos voltaram e quantas ficaram de fora (ou por que
  nada voltou), e a `escalada` com o motivo `restaurada` diz de onde a
  escalada segue.
- **A máquina que reiniciou:** depois de um boot todo Claude de antes morreu,
  e as sessões seriam fantasmas por 12 h. O daemon lê o boot id do Linux
  (`/proc/sys/kernel/random/boot_id`, o do host também dentro do container),
  atrás da costura de plataforma; o núcleo só compara. Outro boot id, ou um
  que falta na gravação ou agora: nada volta (uma sessão fantasma é pior que
  esquecer). Sem o boot id (Windows e macOS até o M8), nada é guardado.
- **Outro compositor** (um logout e um login sem reiniciar a máquina; as
  sessões do tmux sobrevivem): a janela de cada sessão leva a instância do
  compositor em que foi vista (no Hyprland, a assinatura que a descoberta
  acha). Quando a descoberta acha uma instância, as janelas vistas noutra
  saem (a certeza vira "fechou", e o clique diz que a janela dela fechou): a
  sessão fica, e o próximo prompt digitado casa a janela nova. Vale também
  com o pet de pé (antes, o endereço velho ficava, e noutra instância poderia
  ser outra janela), e o anel e a janela ativa da instância de antes saem
  junto. Na partida, a janela restaurada espera a instância ser achada.
- **Testes:** no cérebro, a volta com o relógio do laço do zero e a parede
  adiante (o trabalhando, o pronto e as duas vidas de 12 h ao milissegundo),
  o próximo evento da restaurada, o que vale e o que não vale de cada regra e
  o teto; no formato, a ida e a volta, os campos ruins e os arquivos
  recusados; na escalada, a retomada (a L4 sem rajada na volta, o teto
  contado do aviso de antes, a L3 e a L2 com o que já passou); no Motor, só
  metadados, a volta quieta com o diálogo visto, outra partida da máquina e
  outro compositor (ao vivo e na memória); os dourados
  `reinicio-sessao-parada` (com uma sessão de teste), `reinicio-no-meio-do-turno`
  (com o Stop perdido), `reinicio-com-pergunta`, `reinicio-com-pronto-e-erro`,
  `reinicio-depois-de-13-h`, `reinicio-da-maquina`,
  `reinicio-com-outro-compositor` e `reinicio-com-arquivo-corrompido` (o
  executor ganha o passo `reinicio`: o Motor novo com o relógio do laço do
  zero, a parede adiante e os eventos com o pet fora perdidos); no daemon, a
  gravação de uma vez e só do dono, o ritmo, os arquivos ruins trocados e o
  boot id; com o binário de verdade, o SIGTERM que grava e a partida seguinte
  que devolve; o canário do hook não chega ao arquivo. Dez mutações
  reprovaram (sem semear os avisos chamados, sem semear os estados vistos, o
  último evento e o aviso sem ir ao relógio novo, a sessão de teste na
  memória, sem conferir o boot id, a janela de outro compositor que fica, a
  escalada que recomeça da L1, o trabalhando sem o prazo dele, a sessão de 13
  h que volta). Ao vivo, num daemon de rascunho da branch (a 27391) com uma
  sessão aninhada do 2.1.288: um turno curto, a sessão parada, o SIGTERM, e a
  partida seguinte com a sessão na lista, restaurada, sem evento nenhum; com o
  boot id trocado no arquivo, a lista vazia.
- **Limites:** o T3 de 10 min não é lembrado (o intervalo recomeça na volta);
  a corrente de agentes aberta não volta (o "…" some, e o fim dela festeja
  só o turno que a fecha); o turno que estava aberto festeja só o trabalho
  depois da volta, e um turno de máquina aberto na parada volta como um turno
  implícito digitado; um evento que chega com o pet fora se perde; os voos da
  volta do Renan recomeçam a conta. Uma sessão aberta antes de o plugin ser
  instalado não manda nada até o `/reload-plugins` (a sessão principal do
  Renan só chegou ao pet hoje, depois dele), e depois dele só aparece no
  próximo evento: a memória não ajuda aí.
**Por quê:** o trabalho do Zeca é ficar de olho nas sessões do Claude do
Renan, e a produção reinicia a cada atualização: esquecer as sessões a cada
reinício fazia a lista do clique mentir. Guardar só os metadados que a
lista, os selos e o clique pedem, e refazer cada prazo pela hora de parede,
mantém as regras de sempre; voltar quieto deixa o reinício como coisa do pet,
não do Claude (nada de festa nem de chamada por algo que já passou). Depois
de um boot, esquecer é o certo; depois de um logout, a sessão continua viva
no tmux, mas o endereço de uma janela de outro compositor nunca é de
confiança: focar a janela errada é pior que o balão.

## 0094 — Revisão da memória das sessões: o `idle_prompt` tira a espera que sobrou e as horas do futuro ficam de fora (2026-10-05)

**Problema:** relendo a decisão 0093 antes de levar a memória para a
produção, dois buracos. Uma pergunta que o Renan responde com o pet fora,
num turno que também acaba com o pet fora (a resposta e o Stop se perdem),
voltava com a memória e seguia escalando: o `idle_prompt`, o único evento que
a sessão ainda manda depois disso, nunca mexia em aviso nenhum (decisão
0057), e o pet chamaria por um diálogo que não existe mais até o próximo
prompt, o pior erro da escalada (decisão 0090). E um arquivo com horas muito
adiante (o relógio que voltou entre as partidas, o arquivo mexido)
restaurava sessões com prazos fora do relógio do laço.
**Escolha:**
- **O `idle_prompt` tira a espera que sobrou** (corrige a decisão 0057, que o
  deixava sem mexer em aviso nenhum): ele nunca sai com um diálogo na tela
  (lido no binário do 2.1.288, no T5.7), então um aviso de espera de antes
  dele é de um diálogo que acabou sem o evento chegar. A espera sai no
  primeiro `idle_prompt` da sessão; um atrasado, de antes do aviso, não mexe
  nela; o pronto e o erro continuam com ele (ele se repete, e quem os tira é
  o Renan). Sem reinício nenhum, o caso não acontece: nenhum dourado mudou.
  O dourado novo `reinicio-com-pergunta-respondida-fora` mostra a espera que
  volta quieta, segue escalando (a L2 ainda sai: o pet não tem como saber) e
  sai no `idle_prompt`, sem festa.
- **As horas do futuro:** uma sessão com alguma hora (o último evento, o
  estado, o aviso, o diálogo visto, o casamento da janela) mais de 6 h
  adiante da partida (`JANELA_TS_MS`, a mesma janela do `ts` dos hooks)
  fica de fora.
- **Testes:** o dos avisos no cérebro (o `idle_prompt` tira a espera que
  sobrou, não a de um atrasado, nem o pronto), o da memória com as horas do
  futuro e o dourado com a asserção dele. Três mutações reprovaram (a espera
  que fica, o atrasado que a tira, o pronto que sai junto).
- **Limites:** a espera respondida com o pet fora segue escalando até o
  `idle_prompt`, uns 60 s depois do fim do turno. Uma janela que fechou com o
  pet fora continua na sessão até o próximo prompt casar outra ou o clique
  dizer que ela sumiu (no mesmo compositor, o endereço de uma janela fechada
  poderia, em tese, ser o de uma nova; a parada de uma atualização dura
  segundos).
**Por quê:** a memória não pode trazer de volta uma chamada por algo que já
passou, e o `idle_prompt` é a prova, mandada pela própria sessão, de que não
há diálogo na tela: o primeiro evento depois de um turno perdido. E nada lido
do disco pode pôr um prazo fora do relógio do laço.

## 0100 — macOS: o build, os testes e o `bin/pet verificar` verdes no Mac, sem mexer no Linux (2026-10-05)

**Problema:** o M8 pede o Zeca nativo no Mac (Apple Silicon), antes do M6
(pedido do Renan em 2026-10-05). No clone do macOS, `cargo build`, `cargo
test` e `bin/pet verificar` não passavam, por diferenças de sistema, não de
lógica:
- o `xtask` puxava o `smithay-client-toolkit` (cliente Wayland do host, dos
  comandos `carga` e `globais`) sem `cfg`, e ele nem compila no macOS (o
  `rustix` esconde o `pipe_with` em `apple`): a compilação do workspace
  inteiro quebrava;
- a memória das sessões (decisão 0093) ficava desligada sem um boot id, que
  só vinha do `/proc` do Linux;
- o laço sem janela (`sem_janela`, Windows e macOS) não tratava SIGTERM: o
  processo morria sem gravar a memória das sessões na saída;
- a entrada HTTP respondia 413 a um corpo grande sem drenar o resto, e o
  fechamento com dados por ler virava um RST que engolia a resposta no
  macOS;
- vários testes eram do Linux/Hyprland (socket2, `hyprland.lock`, o "não
  perturbe" do Omarchy) ou usavam o `sha256sum`, que o macOS não tem, e o
  `AF_UNIX` do macOS tem o caminho mais curto (`SUN_LEN` 104, não 108);
- o `bin/pet` roda no bash 3.2 do macOS, que, sem locale UTF-8, lia um byte
  do `»` logo depois de `$id` como parte do nome da variável (`set -u` →
  "unbound variable"); e o `verificar` validava o compose, que só vale no
  Linux.

**Escolha (parte macOS da T8.2, o build e o CI):**
- **Build.** O `smithay-client-toolkit` e o `rustix` do `xtask` ficam sob
  `cfg(target_os = "linux")`, e `carga`/`globais` (clientes Wayland do host)
  avisam que só rodam no Linux fora dele. O `pet-wayland` já era vazio no
  macOS (decisão 0040); agora o workspace inteiro compila no Mac.
- **Boot id do macOS.** `memoria::boot_id()` lê o `kern.bootsessionuuid` pelo
  `sysctl` (só a `std`, sem `unsafe` no daemon): um UUID que muda a cada
  partida da máquina, como o `/proc/sys/kernel/random/boot_id` do Linux.
  Assim a memória das sessões funciona no Mac. Windows continua sem boot id
  (a memória fica desligada) até ter o seu.
- **Encerrar sem janela.** O `sem_janela` trata SIGTERM/SIGINT numa thread
  (`signal-hook`, agora em todo Unix): o sinal manda `Comando::Encerrar` pela
  caixa, o laço sai e grava a memória das sessões, como o `encerrar` do laço
  do Linux (decisão 0093). E publica o estado inicial antes da primeira
  espera, para a restauração aparecer no `/v1/estado` na hora, não só no
  batimento.
- **Fechamento gracioso da entrada.** Fora do Linux, depois de responder a
  entrada HTTP meia-fecha a escrita e drena um pouco do corpo por ler, para o
  413 (e os outros erros) chegarem ao cliente no macOS sem um RST no meio. No
  Linux o fechamento de antes já entrega a resposta: lá nada muda.
- **Testes.** Os testes do socket2 e da janela de cada sessão (instância de
  mentira do Hyprland) e o canário do "não perturbe" do Omarchy ficam só no
  Linux (`cfg`); o canário do `avisar.sh` usa o `shasum` no macOS (o próximo
  da ordem do script, que já caía nele). Nada do comportamento do daemon
  muda: o que saiu são provas de peças que só existem no Linux.
- **`bin/pet verificar`.** As referências `$id»`/`$pedida»` viraram
  `${id}»`/`${pedida}»` (o bash 3.2 em locale C parava nelas); o `compose` e
  o `compose dev` são pulados com aviso fora do Linux (no Mac o bichinho é
  app nativo, o container é uma VM sem tela). O resto (fmt, clippy, testes,
  clippy dos alvos Windows e macOS, lint-skin, arte livre, marketplace,
  plugin, shellcheck) roda igual.
- **Clippy.** O `rust-toolchain.toml` é `stable` flutuante (decisão 0015); o
  stable deste Mac (1.96) acusou um `nonminimal_bool` num `!…is_some_and` do
  M5 (`motor::ver_a_espera`), trocado pela sugestão do próprio clippy
  (`is_none_or`), que é estável desde a 1.82 (a MSRV é 1.85) e não muda o
  comportamento. Toca código do M5, que corre em paralelo no Linux: a troca é
  de uma linha e idêntica no efeito.

**Por quê:** o porte não pode mudar o que o pet faz; estas são todas
diferenças de sistema (ferramenta ausente, sinal, locale, limite de socket,
lint de uma versão de clippy mais nova) resolvidas atrás de `cfg`, de uma
fonte equivalente ou de uma escrita portável, com o Linux byte a byte igual.

## 0101 — Spike do NSPanel no macOS: a tabela «funciona / plano B / não dá» antes do backend (2026-10-06)

**Problema:** antes de escrever o `pet-macos` (T8.5), a pesquisa
(`docs/pesquisa/09-multiplataforma.md`) deixou incertezas que só uma máquina
resolve (parte macOS da T8.2): o painel por cima de tudo e de app em tela
cheia, em todos os Spaces; o clique fora do corpo atravessar pelo alfa ou só
pelo plano B; o App Nap atrasar os prazos; um painel não ativador conseguir
trazer outro app para a frente no macOS 15+; e como achar o monitor ativo.

**O spike (descartável):** um binário em Rust com a mesma pilha do backend
(`objc2` 0.6; `objc2-app-kit`, `objc2-quartz-core`, `objc2-core-graphics`
0.3) abre um `NSPanel` não ativador com uma skin de teste (corpo opaco
magenta sobre fundo transparente). Rodado no Mac do Renan (macOS 26.6.2,
Apple Silicon, 3 telas: o notebook 1512×982 @2× e dois monitores externos
1200×1920 @1×). O clique-através foi medido sem sintetizar clique, por
`+[NSWindow windowNumberAtPoint:belowWindowWithWindowNumber:]` (diz qual
janela pegaria um clique num ponto da tela), calibrado: com
`ignoresMouseEvents=true` ele devolve a janela de baixo, logo é ciente do
roteamento, não só da geometria. Prints pelo `screencapture` (com a Gravação
de Tela que o Renan autorizou) guardados fora do git.

**A tabela:**

| Item | Veredito | Como |
|---|---|---|
| Por cima de tudo (janelas normais) | **funciona** | `NSPanel` borderless não ativador, nível alto, `CanJoinAllSpaces\|FullScreenAuxiliary\|Stationary\|IgnoresCycle`. Prints sobre outras janelas e no monitor externo |
| Sobre app em tela cheia | **funciona** | o mesmo painel aparece na tela cheia (Space à parte); o Renan confirmou na tela |
| Em todos os Spaces | **funciona** | `CanJoinAllSpaces`; a tela cheia é um Space à parte e o painel entra nela |
| Clique fora do corpo atravessa pelo alfa | **não** → **plano B** | com o conteúdo num `CALayer`, `ignoresMouseEvents=false` faz a janela pegar o retângulo inteiro, inclusive a margem transparente (`windowNumberAtPoint` devolve o painel em todo ponto). O alfa visual não vira região de clique. Plano B: alternar `ignoresMouseEvents` pela posição do ponteiro |
| App Nap atrasa os prazos | **não atrapalhou** | deriva ≤ ~17 ms em 80+ s com o app em segundo plano e o painel visível. Mesmo assim, segurar `beginActivity` enquanto houver sessão, por garantia |
| Painel não ativador ativa outro app (macOS 15+) | **funciona** | `activateWithOptions` no Finder devolveu `true` e o Finder veio para a frente (assíncrono, ~1–2 s); `yieldActivationToApplication` existe e foi chamado sem efeito ruim. O clique-leva-ao-terminal é viável |
| Monitor ativo: foco do app ou ponteiro | **ambos** → NSScreen.main | `NSScreen.main` acompanhou o app ativo (x = −1200, −2400, 0 conforme a janela em foco mudava de tela); o ponteiro também dá a tela. Usar `NSScreen.main` reavaliado em `NSWorkspaceDidActivateApplicationNotification`, com o ponteiro de reserva |
| Desenho nítido (CALayer + CGImage) | **funciona** | `CGImage` BGRA pré-multiplicado, `CALayer` com filtro nearest: bloco nítido no print |
| A pilha objc2 0.6 / 0.3 | **compila e roda** | prova o stack do `pet-macos` (inclui `define_class!` para o content view e `CGEvent`) |

**O que isto manda para o backend (T8.5):**
- `NSPanel` borderless não ativador, `CanJoinAllSpaces|FullScreenAuxiliary|
  Stationary|IgnoresCycle`, nível alto, transparente, sem sombra,
  `becomesKeyOnlyIfNeeded`, `hidesOnDeactivate=false`;
- `CALayer` com `CGImage` BGRA pré-multiplicado, `contentsScale =
  backingScaleFactor`, filtro nearest;
- **click-through pelo plano B:** alternar `ignoresMouseEvents` pela posição
  do ponteiro — dentro da caixa de toque do pet, `false` (o clique e o
  arraste chegam); fora, `true` (atravessa). Como com `ignoresMouseEvents=
  true` a janela não recebe `mouseMoved`, a posição vem de um monitor global
  do mouse (`NSEvent` global monitor) ou de um timer lento lendo
  `NSEvent.mouseLocation`;
- monitor ativo por `NSScreen.main` em `NSWorkspaceDidActivateApplication
  Notification` + timer lento;
- `beginActivity` enquanto houver sessão (App Nap por garantia);
- focar o terminal (T8.7) por `NSRunningApplication.activate` (opcional
  `yieldActivation` antes); a janela exata por `AXUIElement` com a permissão
  de Acessibilidade, sem guardar título.

**Nota de método:** o clique sintético de verdade não saiu autônomo — o
binário do spike, ad-hoc, não tem trust de Acessibilidade (o `CGEvent` é
descartado), e o `orca computer` devolveu `permission_denied` no clique. A
prova do click-through veio da consulta `windowNumberAtPoint` (sem evento),
calibrada, que é suficiente e não depende de permissão.

**Por quê:** é o passo que decide o backend. O achado principal — o alfa não
atravessa com `CALayer`, então o click-through é o plano B — muda como o
`Overlay` do macOS trata o ponteiro, e é melhor saber agora.

## 0102 — Backend do macOS: o NSPanel que anda, o laço do AppKit e o click-through pelo plano B (2026-10-06)

**Problema:** o T8.5 pede o `pet-macos` como [`Overlay`] e [`Desktop`] de
verdade, com o laço do AppKit no lugar do `sem_janela`, a partir do que o
spike decidiu (decisão 0101).

**Escolha (T8.5, a janela e o laço; o foco do terminal fica para o T8.7):**
- **A janela** (`pet_macos::Painel`, o `Overlay`): um `NSPanel` não ativador,
  borderless, nível alto, `CanJoinAllSpaces | FullScreenAuxiliary | Stationary
  | IgnoresCycle`, transparente e sem sombra. É uma **janela pequena que
  anda** (`tela_inteira: false`): a cada quadro o `desenhar` acha a caixa da
  cena no palco, rasteriza um `CGImage` BGRA pré-multiplicado (o mesmo raster
  do `pet-core`), põe no `CALayer` (filtro nearest, `contentsScale =
  backingScaleFactor`) e reposiciona a janela no monitor. O confete da tela
  inteira fica para o palco transitório (o Motor já o barra com
  `!tela_inteira`).
- **As coordenadas** (`pixels::Tela`): a conversão entre o palco (device px,
  origem no topo do monitor) e o AppKit (pontos, origem embaixo), pelos frames
  do `NSScreen`. O ponteiro volta do `locationInWindow` para o palco.
- **O click-through é o plano B** (decisão 0101): o painel alterna
  `ignoresMouseEvents` pela posição do ponteiro — dentro da caixa de toque do
  pet, pega (o clique e o arraste chegam); fora, atravessa. Como com
  `ignoresMouseEvents` ligado a janela não recebe `mouseMoved`, a posição vem
  do `NSEvent.mouseLocation`, lido a cada volta do laço (voltas curtas só com
  o pet na tela; sem desenhar nada, o orçamento de commits não muda). Com um
  botão apertado, o toggle congela (o arraste continua).
- **O ponteiro**: uma `NSView` própria (`define_class!`) loga
  `mouseDown`/`Dragged`/`Up` e o botão direito, vira [`EventoPonteiro`] no
  palco, e aceita o primeiro clique sem ser a janela chave (`acceptsFirstMouse`).
- **O monitor ativo**: o painel segue o `NSScreen.main` (`seguir_monitor` no
  batimento); quando muda, refaz a tela e anuncia `Pronta` para o Motor
  refazer o palco no monitor novo. (O seguir fino com poof do M4 e os eventos
  do `NSWorkspace` ficam para o T8.7.)
- **O laço** (`crate::laco_macos`): a thread principal roda o run loop do
  AppKit em fatias e, entre elas, faz o que o `sem_janela` faz (esvazia a
  caixa, vence os prazos, publica, grava a memória), com o `Punho` de verdade.
  A entrada HTTP acorda a thread principal pelo `CFRunLoop::wake_up`
  (`pet_macos::Despertador`). SIGTERM/SIGINT mandam `Encerrar` pela caixa e a
  memória das sessões vai para o disco na saída. O app é Accessory e segura
  um `beginActivity` contra o App Nap, por garantia.
- **Config e estado** ficam em `~/Library/Application Support/bichinho`
  (decisão 0100).
- **`unsafe` só no `pet-macos`**, com `// SAFETY:` em cada bloco (quase tudo
  do AppKit no objc2 0.6 já é seguro; sobra o `msg_send!` e o
  `CGBitmapContextCreate` com ponteiro cru). O crate tem o `[lints]` próprio
  (não herda o `unsafe_code = "forbid"` do workspace) e enforça o `// SAFETY`.
- **Testes sem janela**: o daemon dos testes roda com `PET_SEM_JANELA=1` (o
  `sem_janela`, sem abrir NSPanel na tela): os testes do cérebro não dependem
  da janela, como no Linux sem compositor. No Linux a variável é ignorada.

Ao vivo neste Mac (macOS 26.6.2, Apple Silicon): o daemon com `PET_DEBUG=1`
desenhou o pet `_teste` (xadrez de QA) no canto do monitor ativo, nítido
(blocos D×D) e **por cima de tudo** (sobre o IDE e o navegador), com 44
commits de desenho e `tela: ativa`; a foto está fora do git. **Pendentes,
com a tela e o Renan (T8.7 e a conferência do M8):** o arraste, o
click-through de verdade atravessando, o clique direito (soneca), o seguir o
monitor com a viagem, a reação aos eventos reais, e o foco do terminal
(T8.7).

**Por quê:** é o backend que o spike desenhou. A janela pequena que anda
mantém os commits baratos (o `CALayer` só troca a imagem da célula), e o
plano B dá o click-through que o alfa não deu.

## 0103 — macOS: o clique leva ao app do terminal pelo NSWorkspace, sem permissão (parte do T8.7) (2026-10-06)

**Problema:** o Renan quer que o clique no Zeca leve ao terminal da sessão do
Claude. No macOS o caminho sem permissão é trazer o app do terminal para a
frente (decisão do Renan e pesquisa 09); a aba exata pede Acessibilidade.

**Escolha (a parte macOS do T8.7, o foco no nível do app):**
- O `DesktopMac` conta o app em foco pelo `NSWorkspace` (o
  `frontmostApplication`, lido a cada volta do laço): a cada troca, anota uma
  `JanelaAtiva` no relógio de parede, e na primeira leitura liga a fonte
  (`Ligado(true)`) e semeia o anel com a `JanelaInicial`. A "janela" é o
  **bundle id do app** (`company.thebrowser.dia`, `com.googlecode.iterm2`…),
  nunca o título nem o nome da janela — o canário dos títulos continua valendo.
- O anel de ativações e o casamento com o `ts` do hook são os mesmos do M4/M5
  (decisões 0055 e 0057), sem código novo no Motor: a sessão casa com o app
  que estava em foco quando o prompt foi digitado.
- `Desktop::focar(bundle)` acha o `NSRunningApplication` daquele bundle, cede a
  ativação (`yieldActivationToApplication`, macOS 14+ cooperativo) e o traz
  para a frente (`activateWithOptions`). O spike provou que funciona (decisão
  0101). Capacidades: `janela_ativa` e `foca_janela` ligadas; `segue_foco`
  não (o painel segue o monitor sozinho).

Ao vivo (smoke): o daemon mostrou `protocolos: ["NSWorkspace"]`,
`foca_janelas: true`, o anel semeado com o app em foco e o relógio de parede,
e o clique sem sessão caindo na lista. **Pendentes (segunda parte do T8.7):**
o campo novo do fio v1 com o app da sessão (o `__CFBundleIdentifier` do
ambiente e a árvore de processos, para tmux e restauração), com decisão e
canários; e a aba/janela exata por `AXUIElement` com a permissão de
Acessibilidade (opcional, pedida uma vez). A conferência com dois terminais é
do M8 na tela (step 6).

**Por quê:** reusa todo o anel e o ciclo do clique do M4/M5; só muda a fonte
das ativações (o `NSWorkspace` no lugar do socket2) e o `focar` (ativar o app
no lugar do foreign-toplevel). Dá o clique-leva-ao-terminal no nível do app
sem pedir nenhuma permissão.

## 0104 — macOS: o Bichinho.app, a instalação e o `bichinho diagnostico` (parte do T9.3) (2026-10-06)

**Problema:** para o dia a dia no Mac, o pet precisa de um app nativo com
identidade estável (para as permissões não se perderem), de subir no login,
de ir e voltar do PATH e do plugin, e de um relatório que o Renan possa colar.

**Escolha (a parte macOS do T9.3):**
- **`bichinho diagnostico`**: um relatório só com metadados — a versão e o
  commit, o binário que o Claude vê no PATH (de onde o hook chama), se o
  daemon responde, o backend, as pastas de estado e config, e, no macOS, o
  LaunchAgent e o estado das permissões (Acessibilidade e Gravação de Tela,
  por `AXIsProcessTrusted` e `CGPreflightScreenCaptureAccess`, declaradas à
  mão no `pet-macos`). Nenhum conteúdo, nenhum título.
- **`scripts/mac-empacotar.sh`**: monta o `Bichinho.app` — o binário de
  release em `Contents/MacOS/bichinho`, `Info.plist` com `LSUIElement` (sem
  Dock), bundle id neutro `dev.bichinho.pet`, e **assinatura ad-hoc**
  (`codesign -s -`), para a Acessibilidade e a Gravação de Tela ficarem presas
  ao app e sobreviverem às atualizações.
- **`scripts/mac-instalar.sh`**: monta o `.app` (padrão `~/Applications`),
  liga o hook no PATH (`~/.local/bin/bichinho` → o binário do `.app`, a mesma
  assinatura, então o TCC vale para os dois), cria o config em
  `~/Library/Application Support/bichinho`, e, com os pedidos, instala o
  **LaunchAgent** (`--launchagent`, `RunAtLoad` + `KeepAlive` com
  `SuccessfulExit=false`) e o **plugin** (`--plugin`, de uma worktree estável
  da `main`, nunca da branch, com confirmação, porque vale para todas as
  sessões do Claude).
- **`scripts/mac-desinstalar.sh`**: tira o LaunchAgent, o link do PATH e o
  `.app`; deixa os dados (a aprovação do Zeca) salvo `--tudo`, e o plugin
  (que se tira pelo `claude plugin uninstall`).
- O daemon do `.app` é `…/MacOS/bichinho rodar` (o LaunchAgent passa o
  `rodar`): o `bichinho` sem subcomando continua inerte (a regra de ouro), e
  o hook no PATH chama `bichinho avisar`.

Ao vivo: o `.app` montado e assinado ad-hoc (`Identifier=dev.bichinho.pet`,
`Signature=adhoc`, `LSUIElement` true), o binário dele roda, e o
`bichinho diagnostico` mostra o backend nativo, as pastas, o LaunchAgent e as
permissões (as duas concedidas neste terminal). **Não rodei a instalação de
verdade** (PATH, LaunchAgent, plugin são fora do repositório e pedem o
consentimento do Renan).

**Por quê:** a assinatura ad-hoc com bundle id fixo dá a identidade estável
que o TCC precisa; o LaunchAgent e o plugin ficam opcionais e avisados porque
mexem fora do repositório.
