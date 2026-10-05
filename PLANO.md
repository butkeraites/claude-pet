# Plano: bichinho — o Zeca, papagaio pixel art que comemora quando o Claude Code termina

> Plano aprovado em 2026-10-02. Mudanças de rumo entram em DECISIONS.md; este arquivo só ganha correções e IDs de tarefa.
>
> **Ampliado em 2026-10-03, a pedido do Renan** (decisões 0035–0039): o destino é um lançamento open source para Linux, macOS e Windows, com o app **bichinho**. Entraram os marcos M8 (multiplataforma) e M9 (publicação), a nova ordem (T8.0 e T8.1 antes do M4) e, no M4, o clique que leva ao terminal da sessão. A pesquisa está em `docs/pesquisa/09-multiplataforma.md`.

## Contexto

O Renan quer um bichinho de pixel art que more na tela e reaja ao Claude Code rodando no terminal. Ele deve comemorar de um jeito fofo, engraçado e **chamativo** quando o Claude termina de desenvolver algo, e chamar o usuário quando o Claude precisa dele. Requisitos:

- **Docker:** roda 100% local, em Docker.
- **Por cima de tudo:** "ficar no topo da tela" foi lido como ficar acima de todas as janelas. A posição inicial é o canto inferior direito; se ele quiser o bicho no alto, basta arrastar, e a posição fica gravada por monitor.
- **Arrastável:** pode ser levado com o mouse para qualquer lugar.
- **Monitor ativo:** fica sempre no monitor/área de trabalho que está em foco.
- **Terminal:** funciona com o Claude Code CLI.
- **GitHub:** o repositório fica no GitHub dele e evolui aos poucos.

**Escolhas já feitas com ele:**

- **Personagem.** É a arara do pack *Cute Parrots!* da exclusiveOlive (itch.io, US$ 0,50), transformada num papagaio malandro com visual próprio:
  - verde, com chapéu-palheta e gravata-borboleta em cores próprias (laranja);
  - nome próprio, proposto **Zeca** (confirmado em 2026-10-03, também em público). Não copia nome nem visual de personagem de terceiros, e nenhum texto do projeto o associa a um (decisão 0035).
  - O Renan recusou rascunhos em ASCII ("muito feias"). Por isso o corpo e as animações vêm de um pixel artist profissional; nós só desenhamos acessórios pequenos.
- **Sem som.** Toda a atenção vem da animação.
- **Festa proporcional ao trabalho** do turno.
- **Repositório privado** `butkeraites/claude-pet`, clonado em `~/Documents/claude-pet`.
- **Lançamento aberto (2026-10-03):** no fim, o app **bichinho** sai open source para Linux, macOS e Windows; no macOS e no Windows como app nativo, e o Docker só no Linux. O repositório público será novo e limpo (marcos M8 e M9; decisões 0036 e 0038).

**Ambiente (verificado):**

| Item | Situação |
|---|---|
| Sistema | Omarchy/Arch |
| Compositor | Hyprland 0.56.2, com config em Lua |
| Monitores | eDP-1 (1280x800 lógicos, escala 1.5) e HDMI-A-1 4K acima dele (2560x1440 lógicos, escala 1.5), conectado só às vezes |
| Docker | 29.7, rootful; `docker.service` habilitado e linger ligado |
| Claude Code | 2.1.288, roda com `--dangerously-skip-permissions` |
| GitHub | `gh` autenticado como butkeraites |
| Rust | rustc 1.98.1 em `~/.cargo/bin`, que fica fora do PATH |
| Memória | 7,4 GiB |

**Como o plano foi feito:**
- pesquisa técnica sobre hooks, Hyprland, Docker+Wayland, motor gráfico e arte;
- busca de referências de arte, com links e licenças conferidos um a um;
- três propostas de implementação, julgadas;
- a síntese foi atacada por dois revisores adversariais, um de viabilidade e um de produto. As correções deles já estão aqui.

## O que você vai ver

**Presença na tela:**
- O Zeca fica pousado no canto inferior direito do monitor ativo, por cima de tudo.
- Só o corpo dele é clicável; o resto da tela recebe os cliques normalmente.
- **Arrastar:** ele fica pendurado batendo as asas e, ao soltar, pousa com uma poeirinha.
- **Clique:** com um aviso pendente (pronto ou precisa de você), ele dá uma risadinha com coração e leva você à janela do terminal daquela sessão do Claude, marcando o aviso como visto; com vários, o mais urgente primeiro, e cada clique passa para o próximo. Sem nada pendente, um balão mostra as sessões abertas e o estado de cada uma (M4, decisão 0039).
- **Troca de monitor:** ele some num "poof" e reaparece no monitor que ganhou o foco.

**Reações ao Claude:**
- **Trabalhando:** fica quase parado, bicando sementes de vez em quando.
- **Terminou:** a reação depende do tamanho do trabalho.
  - Resposta sem trabalho de verdade: um aceno discreto.
  - Trabalho pequeno: um pulinho e o balão "Prontinho! ‹projeto›".
  - Trabalho médio: um voo curto e confete.
  - Trabalho grande: decola, atravessa a tela voando, solta chuva de confete e a faixa "PRONTO!", depois pousa de volta.
- **Precisa de você** (pergunta, plano para aprovar ou permissão):
  - ele pia com um "!" e o balão "Ô, meu camarada! ‹projeto› precisa de você";
  - se você não está olhando o terminal do Claude, a chamada cresce em rajadas e num voo até o alto da tela;
  - a escalada tem teto, nunca vira loop infinito.
- **Erro de API:** "Deu ruim...".
- **Limite de uso:** um cochilo com "Cansei...".
- **Ninguém mexendo:** boceja, dorme (zZ) e para de desenhar.

## Decisões principais (com o porquê)

| Tema | Escolha | Por quê |
|---|---|---|
| Motor | Um binário Rust estático com smithay-client-toolkit 0.21 (wayland-client puro, sem libwayland). | Escreve pixels direto em pixels do monitor, então a nitidez fica garantida na escala 1.5. A imagem fica pequena e a RSS abaixo de 64 MiB. Reconecta ao compositor sem morrer. **Rejeitados:**<br>- Quickshell: o caminho GL danifica a superfície inteira a cada quadro, e a imagem tem ~1 GiB.<br>- GTK4+Python: nitidez a 1.5 não comprovada, `_exit` quando perde o compositor, e exige LD_PRELOAD. |
| Superfície | Uma única camada OVERLAY transparente, do tamanho do monitor:<br>- criada com output NULL, e o Hyprland a coloca no monitor focado;<br>- nunca redimensionada e sem subsurfaces;<br>- o Zeca se move dentro do buffer. | O OVERLAY fica acima até de janelas em tela cheia. Mover o conteúdo dentro do buffer evita recalcular o layout. Redimensionar estica um buffer velho. Subsurfaces são esmagadas no Hyprland (#10515). |
| Custo no compositor | Orçamento de commits: média de até 2/s parado, 0 dormindo, rajadas curtas de até 30 fps. Medido no Hyprland, não só no container. | No Hyprland 0.56.2, cada commit de uma camada danifica a área inteira dela, ou seja, repinta o monitor. Se o M1 estourar o orçamento, vem o **plano B**: uma superfície pequena fixa para o repouso e um "palco" de tela cheia mapeado só durante arraste, voo e confete. |
| Seguir o monitor | Só eventos do `.socket2.sock`, com debounce de 300 ms. | O `.socket.sock` executa comandos no host e congela o Hyprland se um cliente travar. O daemon nunca o abre. |
| Docker | `alpine:3.24.2`, `restart: unless-stopped`, bind de `/run/user` (read_only, `rslave`, `create_host_path: false`). WAYLAND_DISPLAY e a assinatura do Hyprland são descobertos em tempo de execução. | Sobe no boot (docker.service + linger) e sobrevive a logout/login, crash, suspensão e hotplug sem unit de systemd. Não sofre a corrida de boot nem fica preso a sockets velhos. |
| Entrada de eventos | HTTP em `127.0.0.1:27380`, só loopback. `POST /v1/evento` responde 204. | O curl sempre existe. Checar Host, Content-Type e `X-Pet: 1` barra requisições vindas do navegador. |
| Hooks | Plugin `bichinho`, no próprio repo, com 13 hooks `async` em exec form que chamam o próprio binário, `bichinho avisar <Evento>`: a lista branca do `pet_core::aviso`, TCP direto ao 127.0.0.1 com prazo curto, sempre exit 0 (decisão 0041). Até a troca do plugin instalado, o `avisar.sh` (jq + `curl -m 2`) fica de reserva. | Async não atrasa o Claude. Um hook do tipo http mostraria erro no transcript sempre que o pet estivesse desligado. Só **metadados** saem do host. |
| Festa | Calculada pelo **trabalho real**: ferramentas de trabalho, arquivos editados, subagentes e tempo de ferramenta. Não usa tempo de relógio. | Com Opus em esforço máximo, uma resposta simples leva 20–90 s. Medir por relógio daria festa em toda resposta. |
| Atenção | Escalada só visual, com teto, e que sabe se você está presente: olhando o terminal do Claude, fica no nível 1. | Não há som. Movimento que começa chama atenção; movimento constante cansa. |
| Arte | O pack comprado fica em `skins-locais/` (gitignored) e entra só na imagem local, que nunca vai a registry. Chapéu e gravata são arte nossa, commitada. | A licença permite editar e proíbe redistribuir. |
| Nomes | - app, binário, crate, compose (projeto, serviço, imagem) e namespace da camada: `bichinho` (decisão 0036; até o T8.1, `claude-pet`); o repositório de desenvolvimento continua `claude-pet`;<br>- plugin e marketplace: `bichinho` e `bichinho-local` (nomes de plugin que começam com `claude-` são reservados);<br>- personagem/skin: `zeca`;<br>- CLI de desenvolvimento: `bin/pet`;<br>- variáveis de ambiente: `PET_*`; cabeçalho `X-Pet: 1`. | Trocar o personagem não exige renomear o código; um nome próprio evita colisão e marca alheia. |
| Fuso | Bind de `/etc/localtime`, nunca `TZ`. | O host está em America/New_York e os outros projetos usam São Paulo; não dá para assumir nenhum dos dois. |

## Arquitetura

```
HOST (Hyprland, uid 1000)                                  CONTAINER bichinho (alpine 3.24.2, uid 1000, rootfs ro)
claude (foot) + plugin bichinho                            PID1 docker-init (init: true)
  hook async: bichinho avisar <Evento>                      └ bichinho rodar (um processo)
    lista branca (pet_core::aviso) -> POST -------------->     main (calloop): Motor (cérebro, animador, cena), sessão Wayland, prazos, estado
      127.0.0.1:27380/v1/evento                                thread ingress: HTTP 0.0.0.0:27380
/run/user (bind ro, rslave) ------------------------------>    thread hypr: leitor do .socket2.sock (só eventos)
  1000/wayland-1          <- 1 camada OVERLAY                  thread watchdog: batimento > 60 s -> abort() -> Docker reinicia
  1000/hypr/<HIS>/.socket2.sock
/etc/localtime (ro), ./config -> /etc/bichinho (ro), volume claude-pet_estado -> /state
```

**Falhas esperadas, tratadas dentro do processo:**
- compositor some ou cai o EOF do socket2: espera e reconecta;
- skin com defeito: usa o último snapshot aprovado ou se esconde, **nunca** a skin de teste.

**Bugs:**
- usam `panic = "abort"`, e o `restart: unless-stopped` do Docker traz o processo de volta;
- um timer de batimento de 5 s roda sempre, independente da renderização, para o watchdog não abortar à toa durante o sono profundo ou com a tela apagada.

**Latência alvo:** abaixo de 150 ms entre o hook e o primeiro quadro da reação.

## Docker

**`Dockerfile`, em dois estágios:**
1. Build em `rust:1.98.1-alpine3.24` com `cargo build --release --locked` e cache de registry/target.
2. Runtime em `alpine:3.24.2`, sem pacotes extras: sem Mesa, sem `/dev/dri` e sem áudio. Os pixels vão por SHM e quem compõe é a GPU do Hyprland.

**Conteúdo da imagem:**
- o binário;
- `assets/`, `skins/` e `skins-locais/`;
- `USER ${APP_UID}:${APP_GID}`;
- `HOME=/tmp` e `XDG_RUNTIME_DIR=/tmp/xdg` (privado, 0700).

**Bases e `.dockerignore`:**
- as duas bases ficam fixadas por `@sha256`, registradas em DECISIONS;
- o `.dockerignore` **não** pode excluir `skins-locais/`.

**`docker-compose.yml` (pontos essenciais):**

Desde o T8.1 (decisões 0036 e 0041) o projeto, o serviço e a imagem se chamam `bichinho`, e o volume do estado fica preso ao nome antigo (`claude-pet_estado`), com a aprovação do Zeca. O esboço original:

```yaml
name: claude-pet
x-log: &log
  logging: {driver: json-file, options: {max-size: "10m", max-file: "5"}}
services:
  pet:
    <<: *log
    image: claude-pet:local
    build: {context: ., args: {APP_UID: "${APP_UID:-1000}", APP_GID: "${APP_GID:-1000}"}}
    restart: unless-stopped
    init: true
    user: "${APP_UID:-1000}:${APP_GID:-1000}"   # tem que ser o dono de wayland-1 e hypr/ (0700)
    read_only: true
    cap_drop: [ALL]
    security_opt: ["no-new-privileges:true"]
    pids_limit: 64
    mem_limit: 128m
    cpus: 0.5
    stop_grace_period: 5s
    ports: ["127.0.0.1:${PET_PORTA:-27380}:27380"]
    environment: {PET_LOG: "${PET_LOG:-info}", PET_PORTA_PUBLICA: "${PET_PORTA:-27380}"}
    tmpfs: ["/tmp:size=16m,mode=1777"]
    volumes:
      - {type: bind, source: /run/user, target: /host/run/user, read_only: true,
         bind: {propagation: rslave, create_host_path: false}}   # NUNCA /run/user/1000 nem sockets avulsos
      - {type: bind, source: /etc/localtime, target: /etc/localtime, read_only: true, bind: {create_host_path: false}}
      - {type: bind, source: ./config, target: /etc/claude-pet, read_only: true, bind: {create_host_path: false}}
      - estado:/state
    healthcheck: {test: ["CMD", "/usr/local/bin/claude-pet", "saude"], interval: 60s, timeout: 5s, retries: 3, start_period: 10s}
volumes: {estado: {}}
```

**Desenvolvimento:**
- `docker-compose.dev.yml` traz `restart: "no"`, `PET_LOG=debug` e `PET_DEBUG=1`, e monta `skins/` e `assets/` somente-leitura.
  - Ele **não** pode se chamar `compose.override.yml`, porque esse nome é mesclado automaticamente.
- Ciclo nativo, o mais rápido: `docker compose stop bichinho` e depois `cargo run -p bichinho -- rodar`, com `PET_HOST_RUNTIME=/run/user` e caminhos locais.

## Descoberta e ciclo de vida

**Descoberta (a cada 2 s enquanto espera):**
1. Lista `/host/run/user/<uid>/hypr/*`, da mais nova para a mais velha pelo epoch em `<hash>_<epoch>_<rand>`.
2. Lê o `hyprland.lock`; a linha 2 é o nome do socket Wayland.
3. Só aceita a instância se `connect()` funcionar tanto no `.socket2.sock` quanto no `wayland-N`.
4. Conecta por caminho absoluto, por um symlink curto em `/tmp/xdg`, por causa do limite de 108 bytes do AF_UNIX.

**Backoff:** vale só para nova tentativa na *mesma* assinatura. Se aparecer uma assinatura nova, a conexão é imediata.

| Evento | Comportamento |
|---|---|
| Boot antes do login | O container sobe; `/saude` responde 200 com `tela: aguardando`; o Zeca aparece até 2 s depois do Hyprland. |
| Logout ou crash do Hyprland | EOF → desmonta tudo e volta a esperar. |
| Suspensão | As conexões continuam. |
| HDMI desplugado ou clamshell do Omarchy | A camada recebe `closed` → espera 250 ms → recria no monitor focado. |
| Monitor muda de posição no layout | Mudança de posição/tamanho do xdg_output → re-home depois de 200 ms. |
| Só sobrou o output FALLBACK, ou um output 0x0 | Esconde e espera `focusedmonv2`/`monitoraddedv2`. |
| Na partida, nenhum foco conhecido | Usa o primeiro `wl_output` que não seja FALLBACK e tenha tamanho maior que zero. |
| Pet desligado enquanto o Claude roda | O hook async falha calado e sai com 0: a conexão é recusada na hora (com um pet travado, desiste em 2 s). |

## Superfície, renderização e nitidez

**Superfície:**
- camada OVERLAY, namespace `bichinho` (até o T8.1, `claude-pet`), ancorada nas 4 bordas;
- `exclusive_zone -1` e teclado NONE, então nunca rouba o foco.

**Primeiro mapeamento** (com output NULL, a escala só chega quando a camada é mapeada):
1. `configure`;
2. viewport no tamanho `(w,h)`, buffer transparente de 1x1 e região de input vazia;
3. commit;
4. espera `enter` + `preferred_scale` e só então desenha.

**Buffer e escala:**
- `wl_shm` ARGB8888 premultiplicado em `round(w*s) x round(h*s)`, com viewport de destino `(w,h)`.
- Cada superfície ganha um **SlotPool novo**, que nunca é reaproveitado entre monitores. Um slot reaproveitado é limpo antes de usar.
- Cada pixel de arte vira um bloco **D×D** de pixels do monitor, com D inteiro calculado por monitor:
  - o alvo é o corpo do Zeca ocupar cerca de 12% da altura lógica do monitor (entre 80 e 160 px);
  - dá para mudar no config: `aparencia.tamanho` = `pequeno` (~10%), `normal` (~12%) ou `grande` (~16%), com os limites acompanhando, D sempre inteiro e sem mexer na skin (TP.2, decisão 0042).
- Toda posição é um pixel inteiro do monitor, e os deslocamentos andam em múltiplos de D.

**Dano e ritmo:**
- O dano vai como **lista** de retângulos, ou agrupado numa grade de 64x64. **Nunca** vira um único retângulo envolvente, porque o confete espalhado viraria upload da tela toda.
- Só desenha quando algo muda, com um quadro em voo de cada vez.
- Ritmo por estado: confete até 30 fps; parado com pose fixa e piscadas (média de até 2 commits/s); dormindo até 2 fps; sono profundo com 0 commits.

**Para esconder:**
1. desenha transparente por cima;
2. faz commit;
3. destrói a superfície.

Assim o fade de saída do Hyprland não deixa um quadro fantasma.

**Memória:**
- cerca de 9 MB de SHM no eDP-1 e 33 MB no 4K;
- o Hyprland guarda uma textura do mesmo tamanho, o que entra no orçamento.

## Seguir o monitor ativo, arrastar, clicar

**Seguir o monitor:**
- Eventos usados: `focusedmonv2`, com debounce de 300 ms e só o último valor.
- Congela durante o arraste.
- Intervalo mínimo de 1,5 s entre viagens; mais de 3 viagens em 20 s viram "poof" rápido.
- `configreloaded` nunca move o pet.
- O re-home segue esta ordem:
  1. `poof_out`;
  2. commit transparente e destruição;
  3. criação com output NULL;
  4. ao receber `enter`, a posição salva daquele monitor ou o padrão (canto inferior direito, com 8–16 px de margem);
  5. `poof_in`.
- A animação em curso continua no monitor novo.
- Uma camada com output NULL cai no monitor em foco: se nenhum foco novo chegou desde que ela foi pedida, o monitor onde caiu **é** o monitor em foco e vira o alvo (um foco que se perdeu nunca faz o pet viajar sem fim; decisão 0059).
- Sem a conexão Wayland o socket2 continua lido: todo prazo do Motor vence também sem janela (decisão 0059).

**Tela cheia e proteção de tela:**
- O padrão é mostrar por cima da tela cheia, como foi pedido; `aparencia.tela_cheia = esconder` é opcional.
- Durante a proteção de tela do Omarchy (janela `org.omarchy.screensaver`), o Zeca se esconde.

**Posições salvas:**
- São guardadas como fração da tela, por monitor.
- A chave é a descrição do output (fabricante/modelo/série), com o nome do conector como reserva, porque em docks o nome DP-N muda.

**Arrastar:**
- O arraste começa depois de 4 px ou de 250 ms segurando. Só então a região de input cresce para a superfície inteira, para o arraste funcionar até numa área de trabalho vazia.
- A região volta ao tamanho do corpo ao soltar, depois de 5 s sem eventos de ponteiro, num `leave` no meio do arraste (a pegada perdida: o pet pousa onde está; decisão 0063), em `closed`, ao esconder e antes de qualquer re-home.
- Solto fora do monitor: re-home imediato para o monitor sob o ponteiro.
- O cursor fica `grab` ao passar por cima e `grabbing` enquanto segura (cursor-shape-v1).

**Clicar** (atualizado em 2026-10-03, decisão 0039; detalhes no M4):
- Botão esquerdo com pendência: risadinha + coração e **foco na janela do terminal da sessão** do aviso exibido, que fica marcado como visto; as outras sessões continuam, e o próximo clique vai ao próximo aviso, do mais urgente para o menos urgente.
- Botão esquerdo sem pendência: um balão com as sessões abertas e o estado de cada uma.
- Botão direito: soneca de 30 min, só com reações pequenas e um selo "zZ".

## Hooks do Claude Code (plugin `bichinho`)

**Eventos usados (13):**

| Evento | Filtro |
|---|---|
| SessionStart | — |
| UserPromptSubmit | — |
| PreToolUse | matcher `AskUserQuestion\|ExitPlanMode`: são os "precisa de você" reais com `--dangerously-skip-permissions` |
| PostToolUse | nenhum |
| PostToolUseFailure | — |
| PermissionRequest | — |
| Notification | matcher `permission_prompt\|worker_permission_prompt\|elicitation_dialog\|elicitation_url_dialog\|agent_needs_input\|idle_prompt` |
| SubagentStart | — |
| PreCompact / PostCompact | — |
| Stop / StopFailure | — |
| SessionEnd | — |

**SubagentStop não é usado:** assim, subagente nunca dispara festa.

**Formato**, igual aos plugins oficiais em disco:

```json
"Stop": [{"hooks": [{"type": "command", "async": true,
  "command": "sh \"${CLAUDE_PLUGIN_ROOT}/scripts/avisar.sh\" Stop || true"}]}]
```

**O hook.** Desde o T8.1 (decisão 0041) é o próprio binário, `bichinho avisar <Evento>`, em exec form (`"command": "bichinho", "args": ["avisar", "<Evento>"]`): a mesma lista branca em Rust (`pet_core::aviso`, com os validadores do fio v1), TCP direto ao 127.0.0.1, calado, sempre 0, com prazo. O `avisar.sh` abaixo fica de reserva até a troca do plugin instalado.

**`plugin/scripts/avisar.sh`** (POSIX sh + jq + curl):
- lê o JSON do hook pela entrada padrão;
- monta, com uma lista branca do jq, só metadados;
- manda com `curl -q --noproxy '*' -sS -m 2 -H 'Content-Type: application/json' -H 'X-Pet: 1' --data-binary @- http://127.0.0.1:${PET_PORTA:-27380}/v1/evento` (`-q` primeiro: nenhum curlrc; `--noproxy '*'`: nenhum proxy do ambiente), com o jq sem `~/.jq` (decisão 0031);
- não imprime nada e **sempre sai 0**. Um Stop hook que saísse com 2 impediria o Claude de parar.

**Formato de fio v1** (todos os campos são opcionais, exceto `v` e `e`; strings com tamanho e caracteres validados):

```json
{"v":1,"e":"Stop","ts":1790020208123,"sid":"…","turno":"<prompt_id>","agente":true,"tool":"Edit",
 "nt":"permission_prompt","err":"rate_limit","src":"user","reason":"logout","intr":true,"sha":true,
 "aid":"…","bg":2,"bgt":["subagent","shell"],"bgi":["…"],"dur":1830,"arq":"3fa2b19c04de",
 "proj":"agenda-presidencial","ent":"cli","dnd":false}
```

**Campos que precisam de explicação:**

| Campo | Origem |
|---|---|
| `err` | enum de `StopFailure.error`. O campo `error_type` não existe. |
| `aid` | `agent_id`, validado como token; só existe dentro de subagentes e serve para atribuir as ferramentas deles ao turno certo |
| `bgt` / `bgi` | tipos (lista fechada dos rótulos do 2.1.288 normalizados para `[a-z_]`; outro tipo vira `outro`) e ids de `background_tasks` |
| `dur` | `duration_ms` do PostToolUse |
| `arq` | sha256 truncado do caminho editado, calculado no host |
| `ent` | `$CLAUDE_CODE_ENTRYPOINT` |
| `dnd` | lido no host em `~/.local/state/omarchy/notifications.json` |

Essa pasta do Omarchy **nunca** é montada no container, porque guarda o histórico da área de transferência.

**Nunca sai do host:**
- `prompt`, `tool_input` (só o hash do caminho), `tool_response`;
- o texto de erro, `message`, `title`, `session_title`, `last_assistant_message`;
- `transcript_path` e caminhos completos.

**Manifests:**
- `plugin/.claude-plugin/plugin.json` com `name: bichinho`; a versão fica só aqui.
- `.claude-plugin/marketplace.json` com `name: bichinho-local` e `source: ./plugin`.

**Instalação nesta máquina:**
- O marketplace local aponta para uma **worktree estável, destacada na `main`**: `git worktree add --detach ~/.local/share/claude-pet/estavel main`. O `--detach` existe porque a `main` pode estar em checkout no clone principal. Assim, um WIP de outra branch não chega às sessões do Claude de outros projetos.
- Comandos: `claude plugin validate … --strict`, depois `claude plugin marketplace add ~/.local/share/claude-pet/estavel`, depois `claude plugin install bichinho@bichinho-local`.
- `bin/pet plugin-atualizar` atualiza a worktree depois de cada merge (`git -C <wt> checkout --detach main`) e lembra de rodar `/reload-plugins`.

## Cérebro (crate pura, com relógio e RNG injetados)

**Sessões:**
- Contam só sessões com `ent = cli`. Execuções `claude -p`, SDK e IDE são ignoradas por padrão (config `sessoes.origens`).
- **SessionEnd sempre** limpa os avisos e o turno daquela sessão; só o "tchau" depende do motivo.

**Turnos:**
- A chave é o `prompt_id`.
- Um turno também fecha, sem festa:
  - quando chega `idle_prompt` daquela sessão;
  - quando `PostToolUseFailure` vem com `intr`;
  - quando o título do terminal focado vira "✳" com uma só sessão trabalhando.

**Tarefas em segundo plano:**
- Só trabalho de agente (`subagent`, `workflow`, `teammate`, `cloud_session`) abre ou estende uma corrente.
- Shells e monitores nunca seguram uma festa.
- `dream`, `auto_mode_scan` e `memory_import` são ignorados.
- Um `UserPromptSubmit` com `src=system` enquanto a corrente está aberta é **continuação**: mantém o t0 e soma os contadores.
- O teto de "turno de máquina" (T1 discreto) vale só para `loop_wakeup`, `schedule_wakeup` e `poll_event`, ou para `system` sem corrente aberta.
- Uma corrente de agentes expira em 12 h.
- *Correção (2026-10-05, decisões 0071–0073):* o 2.1.288 nunca manda o `source`. A notificação de tarefa vem com `orig = notificacao` (o hook olha só o começo do prompt) e o tique é um prompt comum numa sessão com agendamento pendente (`crn`) que o Renan não digitou (longe do teclado, ou outra janela certa em foco). A corrente abre e fecha pelo `bgt` de cada Stop, e todo Stop com a corrente aberta é Stop dela.

**Stop:**
- Todo Stop é candidato a fim de turno.
- Acomodação de 0,8 s: o fim é cancelado só por um evento da thread principal com `ts` posterior ao Stop.
- Dedupe por `(sid, turno)`.
- Um Stop com `sha=true` de um turno que já comemorou recalcula a pontuação e funde na festa só se o nível subir.

**Pontuação** (todos os pesos ficam no config; cada Stop registra os componentes em `/v1/estado.turnos`):

```
min_ativos = soma do dur de todas as ferramentas do turno (inclusive as dos subagentes, via aid), fora a do Agent/Task, / 60000   # nunca o tempo de pensar (decisão 0074)
score = min(20, 1.0*min_ativos + 0.15*ferramentas_trabalho(Edit,Write,MultiEdit,NotebookEdit,Bash)
                + 0.05*outras_ferramentas + 0.5*arquivos_unicos + 1.0*subagentes)
T0  sem ferramenta de trabalho, sem subagente e sem arquivo editado -> aceno discreto
T1  score < 4                            -> pulinho + balão
T2  4 <= score < 12                      -> voo curto + 12 confetes + balão
T3  score >= 12 (no máx. 1 a cada 10 min) -> voo atravessando a tela + chuva de confete + faixa "PRONTO!" (2,5–4 s)
```

**Modos e mesclagem:**
- `celebracao.modo`: `proporcional` (padrão), `sempre_grande`, `discreta` ou `desligada`.
- Fins de sessões diferentes até 3 s depois se fundem numa festa só: o nível é o maior dos dois e o balão diz "2 prontos: api, web".

**Prioridade na tela** (o mais alto ganha):

esperando você > erro > cansado > pronto > trabalhando > compactando > pensando > parado > dormindo

As outras sessões aparecem como selos: um contador "+N", uma bandeirinha com a cor do projeto e "…" para trabalho em segundo plano.

**Pronto:**
- vira um selo estático depois de cerca de 2 min, e o Zeca pode bocejar e dormir com o selo;
- some quando é visto (clique, novo prompt ou cerca de 10 s com o terminal daquela sessão em foco) ou depois de 2 h.

**Escalada de "precisa de você"** (só visual):
- Há um espaço de aviso por sessão.
- Um segundo gatilho em até 5 s só refina o tipo, e pergunta/plano tem prioridade sobre permissão. Evita aviso duplicado para o mesmo diálogo.

| Nível | Quando | O que o Zeca faz |
|---|---|---|
| L1 | 0 s | antecipação → pio + "!" + balão; fica no loop de espera |
| L2 | +30 s, só se você **não** está olhando o terminal do Claude ou está sem mexer há 60 s ou mais | rajadas a cada 6 s, por 30 s |
| L3 | +90 s, ou quando você volta | voa até o alto-centro do monitor, bate as asas com "!!" (pisca a 2 Hz no máximo) e volta; até 3 vezes |
| L4 | 5 min ou mais (teto) | pose de espera + selo pulsando a 1 Hz; uma rajada a cada 60 s |

**Saída da escalada:** qualquer evento da própria sessão ou um clique no aviso exibido.

**Presença:**
- `olhando_claude` = o título da janela focada começa com ✳, ◐ ou ◑ (vem do `activewindow` do socket2).
  - Fica guardado só esse booleano, nunca o título.
- `ext_idle_notifier_v1` diz se você está mexendo.
- Ao voltar depois de 2 min ou mais com algo pronto ou pendente: um "voltou!" com resumo, uma vez só.

**Discrição:**
- DND do Omarchy ligado: sem voo pela tela e sem escalada acima de L1.
- Compartilhamento de tela ativo há mais de 2 s: balões sem nome de projeto.
  - **Nunca** usar a regra `no_screen_share` nesta camada: ela pinta de preto o monitor inteiro compartilhado.

**Timers:**

| Situação | Tempo |
|---|---|
| Trabalhando/pensando sem eventos volta a parado | 5 min |
| Erro | 60 s |
| Bocejo | 3 min |
| Sono | 8 min, ou 3 min se você saiu |
| Sono profundo (0 commits) | 30 min |

**Movimento:**
- Trabalhando e pensando: poses quase estáticas, no máximo 4 fps e 1 pixel de arte de movimento, com micro-ações sorteadas a cada 10–30 s.
- Movimento grande fica reservado para o **início** de cada estado.

## Zeca: arte e skin

1. **Compra (ação sua):** o pack *Cute Parrots!* em https://exclusiveolive.itch.io/cute-parrots-pixel-art-asset-pack.
   - Coloque o zip em qualquer pasta e rode `bin/pet skin-instalar <zip>`.
   - Licença: "can be used in commercial and non-commercial projects; can be edited; cannot be redistributed or resold, even if edited". O pack tem quadros de 48x48, 3 cores e cerca de 20 animações (Idle, Walk, Sit, Sit Idle, Stand, Sleep Start/Sleep/Awake, Eating, Chirp, Take Off, Fly, Glide, Dive Start/Loop/End, Bite, Hurt, Landing, Death).
2. **Importação:** `cargo xtask skin-importar` lê o `.aseprite` direto (crate `asefile`) ou tiras PNG e gera:
   - `sheet.png`;
   - `sheet.json` (Aseprite json-array, com tags e duração por quadro);
   - o esqueleto do `skin.json`.
3. **Virar Zeca:** `cargo xtask zeca` produz `skins-locais/zeca/` (detalhes em `docs/SKINS.md`).
   - **Base e cores (decisão 0023, escolha do Renan):** o Parrot 2 verde no visual "Malandro rosa": o bico rosa original fica (a troca de paleta em `arte/zeca/paleta.toml` é vazia).
   - **Acessórios:** chapéu-palheta de faixa laranja e gravata-borboleta rosa, desenhados por nós em grades de texto (`arte/zeca/acessorios/`, commitados).
   - **Regras de estilo dos acessórios:** contorno de 1px na tinta do pack (nunca `#000000`), toda cor cercada pela tinta como no pack, paleta travada no pack e no máximo uma rampa nova (a da palha).
   - **Encaixe quadro a quadro:**
     - âncora da cabeça detectada pelo olho branco do pack, com correções manuais em `arte/zeca/ancoras.json`;
     - a gravata só no miolo do peito, dentro do contorno do pack; some quando o peito está virado para longe (decisão 0028);
     - **chapéu voa e volta** (decisões 0024 e 0028): no susto e no mergulho o chapéu sai da cabeça, dá uma cambalhota em volta de um centro fixo e cai de volta nela (no fim do susto; no mergulho, depois do pouso). Dormindo, o chapéu fica.
   - **Contorno creme externo (opcional):** 1 pixel de arte em volta do bicho, para ler no tema escuro hackerman. Duas skins, `zeca` e `zeca-contorno`; você decide vendo a folha de contato e os GIFs (fundo escuro e claro lado a lado). A licença permite editar.
4. **Mapa de estados** (fica em `skin.json`; as *receitas* do core criam o que faltar a partir de poses-chave):

| Estado | Tags do pack |
|---|---|
| parado | Sit Idle (pose fixa), com Idle em rajadas |
| trabalhando | Eating em micro-rajadas |
| pensando | Sit Idle + "…" |
| esperando você | Chirp em rajadas + "!" |
| pronto | Stand + bandeirinha |
| T0 (aceno) | Stand → Sit |
| T1 | Chirp + receita pulo |
| T2 | Take Off → Fly em arco curto → Landing |
| T3 | Take Off → Fly/Glide atravessando a tela → Dive → Landing |
| erro | Hurt |
| cansado | Sleep Start |
| dormir | Sleep Start → Sleep → Awake |
| arrastado | Fly |
| solto | Landing + poeira |
| clique | Chirp + coração |
| oi / tchau | Chirp + aceno / poof |
| (nunca usado) | Death |

5. **Aprovação (portão humano, decisões 0026 e 0029):**
   - `cargo xtask contato` gera a folha de contato (todas as tags, sobre fundo escuro e claro, ×4, com a impressão digital da skin) e GIFs; o `bin/pet skin-instalar` põe as cópias em `tmp/previa-zeca-m2/`.
   - Você aprova a folha (`bin/pet skin-aprovar`, que só aceita a skin da folha que você viu) antes de o Zeca virar o personagem.
   - A demonstração ao vivo é a própria aprovação: aprovar põe o Zeca na tela na hora; se não gostar, `bin/pet skin-revogar` o esconde de novo, também na hora.
   - Um snapshot aprovado vai para `/state` e serve de reserva se a skin quebrar.
6. **Sem personagem aprovado, o pet fica escondido** (estado `sem_personagem`, visível em `bin/pet doutor`).
   - A skin xadrez `_teste` só aparece com `PET_DEBUG=1` ou numa demonstração explícita que se esconde sozinha depois de cerca de 10 s.
7. **Efeitos e balão:**
   - Efeitos procedurais na paleta FX da skin: confete, brilhos, "!", "?", zZ, coração, poeira, poof e raios.
   - No máximo 40 partículas, ou 60 em T3.
   - O balão é um 9-slice com a fonte **monogram** (CC0, tem acentos), entra com pop e texto datilografado, e é truncado em 18 caracteres.
   - Frases de malandro em PT-BR, editáveis em `assets/frases.toml`: "Prontinho!", "Tá pronto, parceiro!", "Ô, meu camarada! ‹proj› precisa de você", "Plano pra aprovar!", "Deu ruim...", "Cansei...", "Oi! Me arrasta pra onde quiser".
8. **Créditos:** em `NOTICE.md` e no `CREDITS.md` de cada skin (exclusiveOlive e monogram/datagoblin).
9. **O Zeca original (2026-10-04, decisões 0065–0069):** um segundo Zeca, desenhado do zero (nenhum pixel do pack) e dedicado ao domínio público pela CC0 1.0, sai do gerador `arte/zeca-livre/zeca.py` como as skins `zeca-livre` (tema claro) e `zeca-livre-escuro` (tema escuro, com o anel de 1 px que nunca junta peças soltas), em `skins/` e no git. Passa pelo mesmo portão (folha de contato e `skin-aprovar`); o mapa dos estados e a troca entre os dois Zecas estão no `docs/SKINS.md`.

## Repositório e convenções

```
claude-pet/
  CLAUDE.md README.md DECISIONS.md PROGRESS.md PLANO.md LICENSE(MIT) NOTICE.md
  Cargo.toml Cargo.lock rust-toolchain.toml(channel "stable" + clippy + rustfmt) rustfmt.toml .editorconfig
  crates/pet-core/src/{config,event,brain,score,attention,animator,recipes,skin,aseprite,raster,scene,particles,text,geometry,scenario}.rs
  crates/claude-pet/src/{main,daemon,discovery,hypr,ingress,store,watchdog}.rs  crates/claude-pet/src/wl/{surface,fractional,input,shm,idle}.rs
  # desde o T8.0/T8.1: crates/{pet-core (com motor e plataforma), pet-wayland, pet-windows, pet-macos, bichinho}
  xtask/   # skin-importar | zeca | lint-skin | cobertura | contato | skin-teste | fonte | nitidez
  arte/zeca/{paleta.toml,ancoras.json,acessorios/}   assets/{fonte/monogram/,frases.toml}
  skins/_teste/   skins-locais/.gitkeep   cenarios/*.jsonl + *.esperado.jsonl   config/exemplo.toml
  .claude-plugin/marketplace.json   plugin/{.claude-plugin/plugin.json,hooks/hooks.json,scripts/avisar.sh}
  bin/pet   scripts/{verificar-ao-vivo.sh,e2e-monitor.sh}   docs/{ARQUITETURA,TESTES,SKINS,HOOKS,SEGURANCA}.md  docs/pesquisa/
  .claude/skills/{nova-animacao,novo-evento-hook,conferir-na-tela}/SKILL.md
  Dockerfile docker-compose.yml docker-compose.dev.yml .env.example .dockerignore .gitignore
```

**Código:**
- `pet-core` **nunca** depende de crates Wayland; os testes ficam rápidos.
- Dependências:
  - core: `serde`, `serde_json`, `toml`, `png`;
  - binário: `smithay-client-toolkit` (`default-features=false`, `features=["calloop"]`), `wayland-client 0.31`, `wayland-protocols 0.32` (client+staging), `signal-hook`;
  - xtask: `png`, `gif`, `asefile`.
- Sem tokio e sem clap. Perfil release com `lto="thin"`, `strip`, `panic="abort"`.

**Idioma:**
- Português: docs, comentários, commits, CLI, config, balões.
- Inglês: identificadores Rust e chaves semânticas da skin.

**DECISIONS.md:**
- entradas `## NNNN — Título (AAAA-MM-DD)` com **Problema / Escolha / Por quê**;
- novas entradas são acrescentadas, nunca reescritas (a única exceção, pedida pelo Renan, foi tirar as menções a um personagem de terceiros; decisão 0035);
- as decisões deste plano entram como 0001+.

**PROGRESS.md:** uma linha por tarefa, `| Data | Tarefa | O quê | Commit |`.

**Commits e PRs:**
- Commit: uma frase em português citando `(T2.3)` ou `(decisão 0007)`, terminando com `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Um commit por tarefa; uma branch e um PR por marco, revisado por você; merge com **merge commit** (`gh pr merge --merge`), nunca squash, para os hashes citados no PROGRESS continuarem valendo (decisão 0037); tag `v0.N.0`.
- O corpo do PR termina com a linha do Claude Code.

**Nunca commitar:** `.env`, `config/bichinho.toml` (e o `config/claude-pet.toml` de antes), `skins-locais/*`, `tmp/`, `target/`.

**CLAUDE.md:**
- Estado do projeto e comandos. O `~/.cargo/bin` fica fora do PATH, mas `bin/pet` acrescenta.
- Regras de ouro:
  - hooks async, só metadados, sempre exit 0;
  - nunca abrir `.socket.sock` nem chamar `hyprctl dispatch`/`keyword` no daemon;
  - mudanças no Hyprland só pela skill omarchy e com consentimento;
  - a skin de teste nunca vira personagem.
- Pegadinhas:
  - estouro de 64 eventos no socket2;
  - `idle_prompt` repete a cada ~60 s;
  - Stop não vem depois de Esc;
  - `hyprctl output create` não aceita nome;
  - nunca usar `compose.override.yml`;
  - nunca passar `GDK_SCALE` ao container.

**Skills do projeto:**

| Skill | Receita |
|---|---|
| `nova-animacao` | catálogo → skin → gatilho no cérebro → cenário → `tocar` + `foto` |
| `novo-evento-hook` | lista branca do `pet_core::aviso` (e do `avisar.sh`, enquanto a reserva existir) → canários em `tests/hook.rs` (e `tests/avisar.rs`) → `hooks.json` em exec form → tabela do cérebro → teste ao vivo só numa sessão, com o binário da branch (`cargo build -p bichinho` e `PATH="$PWD/target/debug:$PATH" claude --plugin-dir plugin`; decisão 0045) → `/reload-plugins` depois do merge |
| `conferir-na-tela` | ciclo dev, `foto`, `nitidez`, consulta de camadas |

## CLI `bin/pet` (bash + curl + jq)

| Comando | O que faz |
|---|---|
| `estado` | resumo de `/v1/estado` |
| `doutor` | saúde do container, conexões Wayland/hypr, `bichinho@bichinho-local` habilitado em `claude plugin list --json`, idade do último evento por sessão, DND, skin aprovada |
| `soneca [30m]`, `acordar`, `esconder [30m]`, `mostrar`, `posicao-padrao`, `recarregar` | controles, persistidos |
| `tocar <reação>` | toca uma reação direto |
| `testar <cenário>` | eventos sintéticos **pelo hook de verdade** (o `bichinho avisar` do PATH ou de `PET_BICHINHO`, que diz de que commit veio; sem ele, o `avisar.sh` de reserva), com `teste:true` e TTL de 60 s; nunca se misturam com sessões reais |
| `simular <cenário>` | roda o cenário no core com relógio falso; o daemon só exibe as intenções |
| `eventos --salvar <arquivo>` | grava um cenário com ids pseudonimizados |
| `foto` | captura com grim para o Claude ler |
| `skin-instalar <zip\|dir>` | importa um pack |
| `plugin-atualizar` | atualiza a worktree estável do plugin |
| `subir`, `parar`, `logs`, `reconstruir`, `dev` | atalhos do compose |
| `verificar` | portão antes de commit |

**`verificar` roda:**
- `cargo fmt --check`;
- `pet-core` sem crate de Wayland, de laço de eventos ou de sistema, em todos os alvos (`cargo tree --target all`; o `libc` só pela detecção de CPU do `sha2`, decisão 0044);
- o socket de comandos do Hyprland e o `hyprctl` fora do código de produção de todos os crates (só o item marcado com `#[cfg(test)]` fica de fora), com uma prova da própria guarda a cada rodada;
- nenhum nome de personagem de terceiros nos arquivos nem nas mensagens de commit da branch (decisões 0035 e 0043);
- `cargo clippy --all-targets -- -D warnings`;
- `cargo test`;
- `cargo clippy --target` para Windows e macOS, com os alvos do rustup instalados;
- `lint-skin` em todas as skins;
- `docker compose config -q` (produção e dev);
- `claude plugin validate` (`--strict`) no repo e em `plugin/`;
- shellcheck, se estiver instalado.

**Config `config/bichinho.toml`** (gitignored; modelo em `exemplo.toml`; relido a quente; o nome de antes, `claude-pet.toml`, ainda vale sozinho):
- Precedência: comandos persistidos em `/state`, depois `PET_*`, depois o arquivo, depois os padrões.
- `/v1/estado.config` mostra o valor de cada chave e de onde veio.

## Marcos (cada um: branch + PR; cada tarefa: commit + linha no PROGRESS)

### Ordem (decisão 0038, 2026-10-03)

M0–M3 estão na `main` (tags `v0.1.0`–`v0.3.0`). Daqui em diante:

1. **Agora, antes do M4** (branch `m3b-portabilidade`), duas peças pequenas do M8 e uma opção de config:
   - **TP.1** plano e docs (M8, M9, esta ordem, o M4 novo, a pesquisa e a regra do nome);
   - **T8.0** costura de plataforma, sem mudar o comportamento;
   - **T8.1** hook nativo (`bichinho avisar`) e o nome **bichinho** no binário, na camada e no compose;
   - **TP.2** tamanho do Zeca no config (`aparencia.tamanho`).

   Assim arraste, balões e voos (M4–M6) já nascem no Motor portável, e não dentro do backend Wayland.
2. **M4 → M7**, como planejado.
3. **Depois do M7:** T8.2 → T8.3 → T8.4 e T8.5 → T8.6 e T8.7 → T9.1 a T9.6. A T8.8 (GNOME) fica para a v1.1, salvo decisão em contrário.
4. **Em paralelo, sem código:** T9.0 (personagem público, licença da arte e o pedido à exclusiveOlive).
5. **Skin livre (2026-10-04, branch `skin-zeca-livre`, decisão 0065 e seguintes):** uma arte original do Zeca, sem nenhum pixel do pack, em CC0 1.0. Puxa para agora parte da T9.2 (a skin padrão livre); embutir no binário e aprovar pelo build continuam no M9.
   - **TS.1** a arte no repositório: o gerador `arte/zeca-livre/zeca.py` com o manifesto, as notas e a licença CC0, as correções da crítica final e as transições pelo rig, e o `cargo xtask zeca-livre --conferir` (mesmos bytes a cada execução) no `bin/pet verificar`;
   - **TS.2** a skin `zeca-livre` (redistribuível, em `skins/`) e a variante do tema escuro, com o mapa de todos os estados do core, `lint-skin` sem erro, `cobertura --nativos mvp` sem falta e as folhas de contato;
   - **TS.3** o original como padrão do Renan (o pack instalado e aprovado, para voltar pelo config), aprovado pela folha de contato com `bin/pet skin-aprovar`, a produção refeita e a troca entre os dois no README e no `docs/SKINS.md`. *A aprovação é dele: a TS.3 deixa tudo a um passo e confere o caminho com uma aprovação de teste revogada (decisão 0067).*
   - **TS.4** a revisão: o anel do escuro sem juntar peças soltas (a poeira no rabo, o chapéu voando no topete, as notas no bico; o gerador e a montagem reprovam se juntar), o `sleep` só com o laço do sono (dentro dos 2 fps do dormindo) e a dedicação CC0 em nome do Renan, que confirma antes do merge (decisões 0068 e 0069); folhas de contato novas para a aprovação dele.

A beta pública mínima é T8.0–T8.5 mais T9.0–T9.4.

### M0 — Fundação

**Tarefas:**
- **T0.1 Docs:** CLAUDE.md, README, DECISIONS (0001+ a partir deste plano), PROGRESS, PLANO (este plano), LICENSE, NOTICE, ignores, `.env.example`, `config/exemplo.toml`.
- **T0.2 Pesquisa:** extraída para `docs/pesquisa/`.
- **T0.3 Workspace:** `Cargo.toml` do workspace, `rust-toolchain.toml`, crates `pet-core` e `claude-pet`, `xtask`.
- **T0.4 Daemon:** config com precedência, `/saude`, `/v1/estado` provisório, watchdog, SIGTERM, subcomando `saude`.
- **T0.5 Docker e CLI:** Dockerfile, os dois compose e `bin/pet` (`subir`, `parar`, `logs`, `estado`, `verificar`).
- **T0.6 GitHub:** `gh repo create butkeraites/claude-pet --private --source=. --remote=origin --push`. CI no GitHub Actions só se você quiser.
- **Fora do repo:** notas de memória do Claude (feedback de arte e resumo do projeto).
- **Ação sua:** comprar e baixar o pack da arara.

**Verificação:**
- `bin/pet verificar` verde;
- `docker compose up -d --build` com o serviço healthy;
- `curl -fsS 127.0.0.1:27380/saude`;
- `gh repo view butkeraites/claude-pet --json visibility -q .visibility` → `PRIVATE`.

### M1 — Overlay nítido e barato (portão)

**Tarefas:**
- **T1.1** descoberta e reconexão (assinatura mais nova, `hyprland.lock`, `connect()` de prova, backoff só na mesma assinatura);
- **T1.2** sessão Wayland (camada NULL, primeiro mapeamento 1x1, escala fracionária + viewporter, batimento de 5 s);
- **T1.3** core: skin (Aseprite json-array), raster D×D, animador mínimo; `cargo xtask skin-teste`; quadros dourados;
- **T1.4** renderização SHM com SlotPool novo por superfície, dano em lista, ritmo por estado e esconder sem fantasma;
- **T1.5** região de input, cursor, `/v1/estado.sprite_disp` (retângulo em pixels do monitor), `/v1/debug/quadro`, `bin/pet foto`; skin `_teste` só em modo debug;
- **T1.6** `scripts/verificar-ao-vivo.sh`, `cargo xtask nitidez`, medições de custo registradas em DECISIONS.

**Verificação** (`scripts/verificar-ao-vivo.sh`):
1. **Posição:** `hyprctl -j layers` mostra `bichinho` (até o T8.1, `claude-pet`) no nível 3 do monitor focado, com o retângulo lógico do monitor.
2. **Nitidez:**
   - `grim -o <monitor>` captura o monitor inteiro, com transformação identidade;
   - recorte em pixels do monitor por `sprite_disp`;
   - comparação só nos pixels opacos com o quadro esperado de `/v1/debug/quadro`, com tolerância de ±2 por canal (por causa de gestão de cor ou luz noturna);
   - confere que cada bloco D×D é uniforme.
3. **Custo no compositor:**
   - 30 s com o pet escondido, parado, trabalhando e em T3; arrastando, se você topar arrastar durante a medição;
   - CPU do Hyprland pelo delta de utime+stime em `/proc/$(pgrep -x Hyprland)/stat`;
   - GPU por `1 − Δrc6_residency_ms/Δt` em `/sys/class/drm/card1/gt/gt0/rc6_residency_ms`;
   - orçamento: parado com no máximo +1% de CPU do Hyprland e média de até 2 commits/s.
4. **Limites do container:** RSS abaixo de 64 MiB (96 no 4K) e CPU parado abaixo de 1%.
5. **Clique:** clicar ao lado do bicho chega na janela de baixo.
6. **Reinício:** `docker compose restart bichinho` volta em até 3 s, no mesmo lugar.
7. **Crash:** `kill -9` do processo pelo host faz o RestartCount subir. `docker kill` **não** serve, porque cancela a política de restart.
8. **Sem compositor:** `docker compose run --rm --no-deps -e PET_HOST_RUNTIME=/tmp/nada bichinho` registra "aguardando compositor".

**Se falhar:**
- Nitidez, clique ou reconexão falhando são bugs a corrigir no backend Wayland (`crates/pet-wayland`, desde o T8.0).
- Custo acima do orçamento → plano B de duas superfícies (repouso pequeno + palco temporário).
- O GTK fica só como saída de emergência com prazo, se o encanamento Wayland não convergir.

**Portão:** PR com as fotos para você ver.

### M2 — Zeca (portão de arte)

**Tarefas:**
- **T2.1** `cargo xtask skin-importar`: lê o `.aseprite` pelo `asefile` (tags, duração por quadro, camadas achatadas) ou, se ele não ler, tiras PNG com uma tabela linha → tag documentada; escreve `sheet.png`, `sheet.json` (json-array do Aseprite) e o esqueleto do `skin.json`, com nomes de tag normalizados e o nome original guardado;
- **T2.2** `cargo xtask zeca --pack <zip|pasta>`: o Parrot 2 com o visual "Malandro rosa" (bico original), chapéu-palheta e gravata-borboleta encaixados quadro a quadro (âncora no olho, correções em `arte/zeca/ancoras.json`, regras por tag, acessórios brancos nos quadros de clarão), "chapéu voa e volta" no mergulho e no susto (`arte/zeca/chapeu_voando.json`) e contorno creme opcional; saída em `skins-locais/zeca/` com `CREDITS.md`;
- **T2.3** `cargo xtask lint-skin` e `cargo xtask cobertura` (`cobertura.md`: nativo, receita, reserva ou faltando);
- **T2.4** `cargo xtask contato`: folha de contato (todas as tags, índice e duração, fundo escuro e claro, ×4) e um GIF por tag, em `tmp/`;
- **T2.5** `bin/pet skin-instalar <zip|pasta>`: descompacta fora do repo, roda o `zeca`, mostra lint, cobertura e prévias; idempotente;
- **T2.6** aprovação: `bin/pet skin-aprovar` e `skin-revogar` pelo `/v1/comando`, com hash do conteúdo e snapshot aprovado em `/state` como reserva; sem aprovação o pet fica escondido (`sem_personagem`), nunca com a skin de teste;
- **T2.7** ao vivo: imagem com a skin, nitidez do M1 passando com o Zeca, fotos mascaradas em `tmp/` e aprovação revogada no fim, para o Renan aprovar vendo a folha de contato.
- A fonte monogram vai para o M6, junto com os balões (decisão 0023).

**Se o pack ainda não tiver sido comprado:** o M3 vem antes. O pet só aparece em debug ou demonstração.

**Verificação:**
- `lint-skin` sem erros e `cargo xtask zeca --estrito` sem aviso novo;
- `cobertura.md` com os estados do MVP (a tabela do item 4) nativos: `cargo xtask cobertura --nativos mvp`;
- a nitidez do M1 continua passando com o Zeca;
- **você aprova a folha de contato e a demonstração ao vivo.**

### M3 — Esqueleto andante: hook → reação

**Tarefas:**
- **T3.1** fio v1 com validação: `/v1/evento` (Host, Content-Type, `X-Pet`, até 8 KiB; 204, 400, 415), `/v1/comando` (`tocar`, `esconder`, `mostrar`), `/v1/debug/eventos` (só debug); o evento chega ao laço principal pelo canal do calloop; o animador toca uma reação uma vez e volta à pose;
- **T3.2** plugin (manifests, 13 hooks, `avisar.sh`) e testes canário;
- **T3.3** cérebro mínimo: sessões (só `ent = cli` por padrão), turnos por `prompt_id`, acomodação e dedupe do Stop, T0 aceno contra T1 pulinho, eventos de teste com TTL de 60 s; `/v1/estado.sessoes`, `.ultima_reacao` e `.turnos`;
- **T3.4** `bin/pet testar`, gate interativo ao vivo com `--plugin-dir`; o marketplace local pela worktree estável entra só depois do merge na `main`;
- **T3.5** integração sobre o M2 (a `m3-hooks` rebaseada na `m2-zeca`, decisão 0030): um `/v1/comando` para as reações e as aprovações, reações pelos estados da skin com as reservas do catálogo, `nod` nativo no MVP, config relida também no cérebro; `skin-instalar`, `bin/pet testar` e gate interativo de novo com o Zeca.

**Testes canário:**
- `SEGREDO-n` plantado em todo campo de conteúdo; nada disso pode sair do script (procurado sem caixa);
- o script sempre sai 0;
- não imprime nada;
- o curl é falso; com o curl de verdade, proxy no ambiente e curlrc não desviam o evento do 127.0.0.1, e um `~/.jq` não muda a lista branca (decisão 0031).

**Verificação** (até o merge na `main`, o plugin só entra por sessão, com `--plugin-dir`; decisão 0021):
- `claude plugin list` e `claude plugin marketplace list` iguais antes e depois do gate: nada instalado globalmente.
- Gate **interativo** num diretório já confiável, com `CLAUDECODE` e as `CLAUDE_CODE_*` tiradas do ambiente:
  1. `tmux new-session -d -s e2e -c <dir> 'claude --plugin-dir ~/Documents/claude-pet/plugin --dangerously-skip-permissions "responda só: ok"'` → `/v1/estado` mostra a sessão e o aceno T0;
  2. um segundo prompt que cria um arquivo temporário com a ferramenta Write → pulinho T1, com `arquivos: 1` no turno (`/v1/estado.turnos`; o `arq` exato, sha256 do caminho, só aparece na pilha de dev, em `/v1/debug/eventos`, e os canários o conferem);
  3. `/exit` → tchau.
- `claude -p --plugin-dir …` só serve para conferir que, com o pet parado, não aparece erro de hook.
- `time (printf '{}' | sh plugin/scripts/avisar.sh Stop)` leva no máximo 2,1 s com o pet parado (desde o T8.1, o hook é o `bichinho avisar`: conexão recusada na hora, menos de 0,5 s nos canários).
- Na tela (acesa e desbloqueada), o `commits_total` do `/v1/estado` sobe durante as reações; com a tela apagada ou bloqueada, as reações só são conferidas no `/v1/estado`.
- Depois do merge: a instalação pela worktree estável (README) e `claude plugin list` mostrando `bichinho@bichinho-local` habilitado.

### M4 — Arrastar, seguir o monitor ativo e levar ao terminal

Atualizado em 2026-10-03 (decisões 0038 e 0039). O M4 nasce em cima da costura do T8.0: a máquina de arrastar e clicar, as pendências e o balão moram no Motor (`pet_core::motor`), com relógio falso nos testes; o backend Wayland só traduz ponteiro, monitores e janelas (`pet_core::plataforma`).

**Tarefas** (IDs na ordem dos commits: T4.1 → T4.11; a costura primeiro, a verificação ao vivo e a produção refeita no fim):
- **T4.2** máquina de arrastar/clicar (limiares, região de input com fail-safe, cursores);
- **T4.4** leitor do socket2 com reconexão:
  - guarda só nome do evento, monitor, endereço da janela e o primeiro glifo do título;
  - nunca registra o payload em log;
- **T4.5** seguir o foco (debounce, congelar no arraste, intervalo entre viagens, poof, `closed`, mudança de layout, FALLBACK);
- **T4.5** soltar entre monitores;
- **T4.3** posições salvas por descrição do monitor;
- **T4.7** esconder durante a proteção de tela;
- **T4.7** botão direito para soneca;
- **T4.8–T4.10** **clicar no Zeca leva à janela do terminal da sessão do Claude que terminou ou que precisa de você** (decisão 0039):
  - **T4.8** **identidade de janela por sessão:** o leitor do socket2 guarda um anel com as últimas ativações (`activewindowv2`: endereço da janela e a hora em que o evento chegou, nunca o título). O `ts` do `UserPromptSubmit` (e do `SessionStart`) de cada sessão escolhe no anel a janela que estava ativa quando o Renan mandou o prompt: é o terminal daquela sessão (o `SessionStart` só preenche uma janela que ainda não é certa, o de compactação e o prompt de sistema nunca casam, e um hook atrasado não desfaz um casamento mais novo; decisão 0060). O hook pode mandar também, num campo novo e opcional do fio v1 (validado no `pet_core::evento` e com decisão própria), os ids de terminal que ele vê no próprio ambiente (`TMUX_PANE`, `KITTY_WINDOW_ID`, `WEZTERM_PANE`; só ids, nunca títulos). Eles só separam sessões dentro de um mesmo terminal (painéis do tmux, abas): no Docker o daemon roda em outro espaço de PIDs, e nem o socket2 nem o foreign-toplevel trazem PID, então uma cadeia de PIDs não leva a uma janela sem o `hyprctl clients`, que é o socket de comandos (decisão 0043). Quando o anel tem dúvida (dois terminais trocados em menos de 1 s), o clique cai no balão com a lista;
  - **T4.9** **focar sem o socket de comandos:** `zwlr_foreign_toplevel_manager_v1` + `hyprland_toplevel_mapping_manager_v1` (que liga cada handle de toplevel ao endereço de janela do Hyprland, o mesmo do `activewindowv2`) e `zwlr_foreign_toplevel_handle_v1.activate(seat)`. O daemon continua sem abrir o `.socket.sock` e sem chamar `hyprctl` (decisão 0006). Conferir na 0.56.2 que os dois protocolos aparecem no registro; se faltar algum, o clique cai no balão;
  - **T4.10** **pendências em ciclo:** com vários avisos (precisa de você, erro, pronto), o primeiro clique vai ao mais urgente, pela prioridade do cérebro (esperando você > erro > pronto), e cada clique seguinte vai ao próximo. O clique que foca a janela de uma sessão marca o aviso dela como visto; o foco sem clique segue as regras de sempre (o pronto some depois de ~10 s com o terminal da sessão em foco e o Renan no teclado ou no mouse, pelo `ext_idle_notifier_v1`; bloqueado ou longe, fica; decisão 0062; o "esperando você" só sai com um evento da própria sessão ou um clique); um clique mais de 15 s depois do anterior recomeça do mais urgente;
  - **T4.10** **clique sem pendência** (o balão mínimo e a fonte na **T4.6**): um balão com a lista das sessões abertas (nome da pasta do projeto, estado — pensando, trabalhando, esperando você, pronto, parado — e há quanto tempo), que some sozinho. Pede o **balão mínimo e a fonte de pixel** (monogram, CC0), puxados do M6; o M6 só acrescenta pop, datilografia e as frases;
  - **T4.10** **sem como focar** (a janela fechou, a sessão não tem identidade, o compositor não oferece os protocolos): o balão diz isso e mostra a lista;
- **tamanho:** `aparencia.tamanho` (`pequeno`, `normal`, `grande`) chega antes, no TP.2; no M4 o arraste, as posições salvas e o balão usam o D que o tamanho escolhido dá em cada monitor.
- **T4.1** **o que a costura ganha no M4** (decisão 0043): o `EventoDesktop` e o `Motor::evento_desktop` (monitor em foco, anel de ativações, "não perturbe"); o `Overlay::cursor` (pegar e agarrar no arraste); um punho por conexão (janela e desktop juntos) no `Nucleo`, no lugar do `Option<&mut dyn Overlay>`. O `Desktop` do Wayland usa a mesma conexão e o mesmo `wl_seat` da `Sessao`, recriados a cada reconexão: o `zwlr_foreign_toplevel_manager_v1` (genérico, serve ao Sway, ao labwc e aos outros wlroots) fica no `pet-wayland`, e o `hyprland_toplevel_mapping_manager_v1` com o socket2 é a extensão do Hyprland.
- **T4.11** a verificação ao vivo (abaixo), com `scripts/e2e-monitor.sh --autorizo` escrito e rodado só com seu consentimento, e a produção refeita; propor a regra opcional do Hyprland **pela skill omarchy e com seu consentimento**:
  ```lua
  hl.layer_rule({ name = "bichinho", match = { namespace = "^bichinho$" }, order = 1, no_anim = true })
  ```
  - `order = 1` deixa o Zeca abaixo dos popups do Omarchy (polkit, menus, toasts);
  - `no_anim` tira o fade de ~180 ms em cada troca de monitor.

**Verificação:**
- arrastar numa área de trabalho cheia e numa vazia;
- a posição sobrevive ao restart;
- `omarchy-launch-screensaver force` esconde o Zeca e ele volta depois;
- canário do socket2: `activewindow>>firefox,SEGREDO-T` nunca aparece em logs, `/v1/estado` ou `/v1/debug/eventos`;
- testes do Motor em relógio falso: anel de ativações × `ts` do hook (janela certa, troca no mesmo segundo, sessão sem identidade), ciclo das pendências do mais urgente ao menos urgente, balão de sessões sem pendência;
- ao vivo, com a tela acesa e desbloqueada: duas sessões do Claude em dois `foot`, em áreas de trabalho diferentes; com a sessão A pronta, o clique foca o `foot` de A (`hyprctl -j activewindow` só no script de teste do host); com A e B pendentes, cliques seguidos vão do mais urgente ao outro; sem pendência, o balão lista as duas com o estado;
- o daemon segue sem `.socket.sock` e sem `hyprctl` (`bin/pet verificar`), e nenhum título de janela aparece no log, no `/v1/estado` nem no `/v1/debug/eventos`;
- `scripts/e2e-monitor.sh --autorizo`, só com seu consentimento a cada vez:
  - `hyprctl output create headless` e descobre o nome novo por diff em `hyprctl -j monitors`;
  - foca o monitor novo e confere o Zeca lá; volta e confere de novo, com prazos de ≥2,5 s;
  - remove o output;
  - um `trap` sempre restaura o estado.
- HDMI, clamshell e suspensão ficam no checklist manual, registrado no PROGRESS.

### M5 — Cérebro completo

Atualizado em 2026-10-05 com a pesquisa do M5 (`docs/pesquisa/10-cerebro-m5.md`, decisões 0071–0078). No 2.1.288 o `UserPromptSubmit` nunca traz o `source`: a notificação de tarefa se reconhece pelo começo do prompt, que o hook olha e resume num enum (`orig`), e o tique de laço pelos agendamentos do Stop (`crn`), com a evidência de presença do Motor (decisões 0072 e 0073). O M5 é dividido em duas metades: o cérebro decide **o que** acontece e **quando**, num registro de intenções testável (esta lista); quem desenha as intenções com o que já existe (estados da skin, balão mínimo, confete, selos no estilo do zZ) vem depois, na mesma branch. O polimento (balões 9-slice, física das partículas, o voo T3 atravessando a tela com o holofote, a variedade parada, o "voltou!") continua no M6.

**Tarefas** (IDs na ordem dos commits):
- **T5.1** pesquisa e plano: `docs/pesquisa/10-cerebro-m5.md`, estas tarefas e as decisões 0071–0078;
- **T5.2** hook: `orig` (a forma do prompt: `notificacao` ou `comum`) no `UserPromptSubmit` e `crn` (quantos agendamentos) no Stop, calculados no `bichinho avisar` sem o texto sair dele; validadores do fio v1, canários, e o hook da branch conferido ao vivo numa sessão aninhada (decisão 0072);
- **T5.3** registro de intenções e cenários: `pet_core::motor::intencoes`, `pet_core::cenario` (formato, executor com a `JanelaFalsa` em relógio falso, `PET_ATUALIZAR_OURO=1`), o `/v1/estado.intencoes` e os primeiros cenários com o comportamento de hoje (decisão 0077);
- **T5.4** correntes e turnos de máquina: a origem do prompt, a corrente que abre, estende e fecha pelo `bgt`, o trabalho dos agentes depois do Stop (pelo `aid`), o agente que acorda, a expiração de 12 h, o teto T1 discreto sem pronto, a janela casada só pelo prompt digitado e o hook antigo degradando sem quebrar (decisão 0073);
- **T5.5** pontuação e níveis: o config com números (`[pontuacao]`, `celebracao.intervalo_t3_min`, documentados no `config/exemplo.toml`), T0–T3, o T3 no máximo a cada 10 min, os modos e os componentes de cada turno no `/v1/estado.turnos` (decisão 0074);
- **T5.6** fechamentos e prazos: interrupção (com e sem `PostToolUseFailure`) e `idle_prompt` fechando sem festa, `SessionEnd` limpando tudo da sessão, os 5 min de trabalhando/pensando/compactando, os 60 s de erro e o cansado do `rate_limit` (decisões 0073 e 0076);
- **T5.7** avisos e escalada: os tipos de espera, um diálogo até a sessão andar, L1–L4 com presença (`olhando_claude`, sem mexer há 60 s, a volta), saída, tetos do "não perturbe" e da soneca (decisões 0075 e 0079);
- **T5.8** a festa e a tela: mesclagem de 3 s, o `sha` que sobe de nível, prioridade e base, selos, pronto parado depois de 2 min, sono, proteção de tela, o compartilhamento de tela (`EventoDesktop::Compartilhando`, 2 s) e a fotografia de agora no `/v1/estado.fotografia` (decisões 0076 e 0080);
- **T5.9** cenários reais pseudonimizados, as asserções de cada linha da tabela e a prova de que o executor pega uma regra quebrada (decisão 0077);
- **T5.10** `bichinho simular` e `bichinho cenario`, `bin/pet simular` e `bin/pet eventos --salvar`, CLAUDE.md, README e docs (decisão 0078).

**Verificação:** `bin/pet verificar` verde a cada commit e `cargo test -p pet-core` com os cenários. Tabelas, exemplos de pontuação (decisão 0074) e cenários:

| Cenário | Esperado |
|---|---|
| `rapido` | aceno T0 |
| `resposta-longa-sem-ferramenta` | T0 |
| `pequeno`, `medio`, `grande` | T1, T2, T3 (o segundo T3 em 10 min vira T2) |
| `dois-prontos` | uma festa só, "2 prontos" |
| `pergunta`, `pergunta-dupla` | um aviso só |
| `plano-lido-no-terminal` | fica em L1 |
| `pergunta-ausente` | L1 → L2 → L3 → teto |
| `idle-prompt-repetido` | — |
| `servidor-em-segundo-plano` | festas normais com um dev server rodando |
| `workflow-longo` | T3 no Stop final da corrente (a notificação que a fecha) |
| `stop-bloqueado` | a continuação só festeja se subir de nível |
| `interrompido` | sem festa, com e sem `PostToolUseFailure` |
| `erro-limite` | cansado, sem festa |
| `protetor-de-tela` | a festa não toca escondida; o pronto fica |
| `compartilhando-tela` | balão sem nome de projeto |
| `real-*` (da pesquisa, pseudonimizados) | o agente em segundo plano numa festa só; o servidor e a notificação do shell; o laço; a pergunta e o plano; o Esc; o `/compact` |

### M6 — Encanto e atenção (portão de "sensação")

**Tarefas:**
- partículas e movimento reduzido;
- balões (9-slice, monogram, pop, datilografia, frases de malandro, modo discreto; o balão mínimo e a fonte chegam antes, no M4);
- variedade parada: piscar, olhar em volta, micro-ações sorteadas, bocejo, sono, sono profundo;
- física do arraste: pêndulo, pouso com quique, poeira;
- momentos T3 (voo atravessando a tela, chuva de confete, holofote "PRONTO!");
- "voltou!" via `ext_idle_notifier_v1`;
- primeira aparição com "Oi! Me arrasta pra onde quiser";
- dica "sem sinal do Claude Code — rode `bin/pet doutor`" se nenhum hook chegar em 5 min.

**Verificação:**
- Cada reação conferida com `tocar` + `foto` e revisada por você. Critério:
  - nota de canto de olho em até 2 s;
  - ainda agradável na 10ª vez;
  - legível em tema escuro e claro;
  - nítida.
- T3 no 4K usa menos de 30% de CPU por no máximo 4 s.
- Sono profundo com `commits_por_min == 0`.
- Trabalhando por 20 min fica dentro do orçamento de commits.

### M7 — Polimento e v0.1.0

**Tarefas:**
- CLI completo e `doutor`;
- recarga a quente do config;
- README com GIF;
- `docs/SEGURANCA.md` e `docs/TESTES.md` (checklist manual);
- medições registradas no PROGRESS;
- `git tag -a v0.1.0` + `gh release create`;
- semana de calibração dos pesos usando `/v1/estado.turnos`, registrada em DECISIONS.

**Verificação:**
- **Clone novo** num diretório temporário: `docker compose up -d --build` → `bin/pet skin-instalar <zip>` → instalar o plugin → sessão interativa via tmux → festa acontece.
- A desinstalação deixa `claude plugin list`, `claude plugin marketplace list` e `docker images` limpos.

### M8 — Multiplataforma: Linux amplo, Windows e macOS

Acrescentado em 2026-10-03 (decisão 0038), a partir de `docs/pesquisa/09-multiplataforma.md`. O núcleo (`pet-core`) e o backend Wayland ficam; por sistema mudam só a janela do bicho (`Overlay`) e a ligação com o desktop (`Desktop`). No macOS e no Windows o pet é um app nativo; o Docker fica só no Linux.

> **Conferência visual no macOS e no Windows:** precisa de uma máquina de verdade (Mac alugado por dia, VM de avaliação do Windows, voluntários) ou dos runners macOS e Windows do GitHub Actions (prints como artefato). Este notebook só compila (`cargo check --target …`) e roda o que não precisa de tela; sem máquina nem runner, os itens visuais ficam **NÃO VERIFICADO**.

**Tarefas e verificação:**
- **T8.0 Costura de plataforma, sem mudar o comportamento (antes do M4).** `pet_core::motor` (cérebro, pet e palco, mostrar/esconder, estresse, painel, prazos em ms) e `pet_core::plataforma` (traits `Overlay` e `Desktop` com capacidades; `Monitor`, `EventoPonteiro`, `Alca`; a `Caixa`, canal neutro com despertador do SO, no lugar do canal do calloop); o `wl/` e a descoberta vão para `crates/pet-wayland`, com o Hyprland como adaptador; `pet-windows` e `pet-macos` vazios, compilando.
  - *Verificação:* `bin/pet verificar` verde, com testes novos do Motor em relógio falso; `cargo check` para `x86_64-pc-windows-msvc` e `aarch64-apple-darwin`; com a tela acesa e desbloqueada, `scripts/verificar-ao-vivo.sh --personagem` e `scripts/medir-custo.sh` iguais ao M1 dentro do ruído.
  - *Contrato de coordenadas* (revisão, decisão 0044): tudo o que o Motor troca com a janela está no **palco** (pixels do dispositivo do monitor, origem no canto dele): a cena, a célula, a área de toque e o ponteiro. A camada do Wayland converte o toque para coordenadas lógicas; uma janela pequena (T8.4–T8.6) subtrai a própria origem. O `Monitor` traz a descrição, a origem no desktop e a área útil (sem barra de tarefas, Dock ou painel), e a posição padrão do pet é o canto da área útil.
- **T8.1 Hook nativo e o nome bichinho (antes do M4).** `bichinho avisar <Evento>` no lugar do `avisar.sh` (a mesma lista branca, com os validadores do `pet_core::evento`, hash do caminho editado, DND do Omarchy, `CLAUDE_CODE_ENTRYPOINT`, `PET_TESTE`, POST no 127.0.0.1 com prazo curto, sem proxy, calado, sempre 0); `hooks.json` em exec form; binário no host por `bin/pet instalar-host`; o `avisar.sh` fica de reserva até a troca. Binário, camada, compose e imagem passam a se chamar `bichinho`, com o volume da aprovação preservado.
  - *Verificação:* os canários portados para testes do subcomando (reprovam uma versão vazando de propósito); `bin/pet testar` e o gate interativo do M3 pelo hook novo; `claude -p` calado com o pet parado; aprovação do Zeca intacta depois de refazer a produção. O token por usuário no loopback vem com a instalação nativa (T9.3): o `avisar.sh` instalado não o mandaria.
- **T8.2 Spikes de risco e CI nos três SOs.** Código descartável que abre a skin `_teste` e mede o incerto: no Windows, janela *layered* com clique pelo alfa, todas as áreas de trabalho virtuais, DPI por monitor, firewall no loopback; no macOS, `NSPanel` não ativador em todos os Spaces e sobre tela cheia, clique pelo alfa (plano B: alternar `ignoresMouseEvents`), App Nap. CI com matriz `ubuntu-24.04`, `macos-15` e `windows-2025`.
  - *Verificação:* uma tabela «funciona / plano B / não dá» por item, registrada como decisão antes de codar os backends; Mac alugado por um dia e VM do Windows (ou os prints dos runners; captura preta conta como NÃO VERIFICADO).
- **T8.3 Linux: qualquer Wayland com layer-shell** (KDE Plasma 6, Sway, niri, COSMIC, labwc, Wayfire, river). Descoberta pelo `WAYLAND_DISPLAY` exigindo só `zwlr_layer_shell_v1` (o `hyprland.lock` fica para o modo Docker); "convocar" (recriar a camada com output NULL) como base universal; adaptadores Hyprland, Sway/i3, niri, KWin e wlr-foreign-toplevel; reservas sem escala fracionária ou cursor-shape; autostart XDG.
  - *Verificação:* laboratório em Docker com Sway, labwc e niri aninhados (monitor, escala, D e região no `/v1/estado`); CI com Sway e labwc headless e a checagem de nitidez do M1; KDE e COSMIC por VM ou voluntários, com checklist.
- **T8.4 Windows (Win32 nativo, `pet-windows`).** Janela pequena *layered* que anda (DIB pré-multiplicado, alfa 1/255 na área de toque), `WS_EX_TOOLWINDOW | NOACTIVATE | TOPMOST`, monitor ativo por `SetWinEventHook`, DPI por monitor, palco transitório para voo e confete, laço `MsgWaitForMultipleObjectsEx`, arquivos em `%APPDATA%`/`%LOCALAPPDATA%`, autostart pela chave Run. O `unsafe` fica só neste crate, com `// SAFETY:`.
  - *Verificação:* CI `windows-2025` a cada PR (build, testes, canários do hook); smoke gráfico com captura comparada ao `/v1/debug/quadro` (±2) e clique ao lado chegando na janela de baixo; VM ou PC com checklist (áreas virtuais, vídeo em tela cheia, dois monitores com DPI diferente, Windows Terminal e VS Code, custo parado).
- **T8.5 macOS (AppKit nativo, `pet-macos`).** App `Accessory` com `NSPanel` não ativador, `CALayer` com `CGImage` BGRA pré-multiplicado e filtro nearest, monitor ativo por `NSScreen.main`, laço `NSApplication.run`, LaunchAgent. O `unsafe` fica só neste crate.
  - *Verificação:* CI `macos-15` a cada PR (build arm64 e x86_64, testes, canários); smoke gráfico com `screencapture`; Mac alugado com checklist (Spaces, tela cheia, Stage Manager, monitor 1x com Retina 2x, custo parado).
- **T8.6 Linux X11** (Mint Cinnamon, XFCE, MATE, i3, KDE X11) **e GNOME experimental pelo XWayland.** `x11rb` em Rust puro (o binário musl continua estático): janela ARGB pequena, SHAPE de entrada, EWMH (ABOVE, STICKY, SKIP_TASKBAR), monitor ativo por `_NET_ACTIVE_WINDOW` e RandR.
  - *Verificação:* Xephyr ou Xvfb em Docker com i3, Openbox e xfwm4, com e sem compositor (captura com `xwd`, nitidez, clique com `xdotool`); CI com Xvfb; Mint, XFCE e GNOME por VM (com consentimento) ou voluntários.
- **T8.7 Focar o terminal em todos os SOs e a bolha de sessões.** O que o M4 faz no Hyprland vira `Desktop::focar` por ambiente: Sway (`[con_id] focus`), niri (`FocusWindow`), KWin (script), foreign-toplevel genérico, X11 (`_NET_ACTIVE_WINDOW` com source 2), Windows (`SetForegroundWindow` dentro do clique; a janela do Windows Terminal pelo dono da pseudo-janela), macOS (ativar o app do terminal; a aba exata por AppleScript pede permissão de Automação). A bolha reconcilia com `claude agents --json`.
  - *Verificação:* duas sessões em dois terminais, em áreas diferentes, no Hyprland e no laboratório (Sway, niri, Xephyr); VM do Windows (duas janelas do Windows Terminal e o VS Code); Mac alugado (Terminal.app, iTerm2, Ghostty); canário: título de janela nunca no log, no `/v1/estado` nem no `/v1/debug/eventos`.
- **T8.8 GNOME (Ubuntu, Fedora): extensão «overlay remoto».** Extensão GJS mínima (`addTopChrome`, clique e arraste, focar janela) falando D-Bus com o daemon (`zbus`, Rust puro), publicada no extensions.gnome.org. Fica para a v1.1, salvo decisão em contrário.
  - *Verificação:* VM com GNOME 49/50; revisão aprovada no EGO; checklist (por cima de tudo, clique fora atravessa, foca a janela, logout/login documentado).

### M9 — Publicação open source

Acrescentado em 2026-10-03 (decisões 0036 e 0038). O repositório público é novo e limpo; o privado continua `claude-pet` por ora.

**Tarefas e verificação:**
- **T9.0 Personagem público, nome e licença (paralela, sem código, pode começar já).** Arte do personagem padrão (encomenda de um papagaio original em CC BY 4.0 ou CC0, ou licença por escrito da exclusiveOlive para embutir o Zeca, ou só «traga seu pack» com uma skin provisória livre), busca do nome no INPI e no USPTO, o comentário pedindo permissão na página do pack.
  - *Verificação:* permissão ou licença por escrito arquivada; decisão no DECISIONS; a folha de contato do personagem novo passa pelo portão do M2 (`skin-aprovar`; `cobertura --nativos mvp` sem faltas).
  - *Andamento (2026-10-04, decisões 0065–0069):* a arte já existe: o Zeca original, desenhado do zero e dedicado ao domínio público pela CC0 1.0 (`arte/zeca-livre/`, skins `zeca-livre` e `zeca-livre-escuro`). Faltam a confirmação do Renan da dedicação CC0 do conteúdo final (antes do merge da `skin-zeca-livre`), a aprovação dele pela folha de contato, os itens de pixel artist de `arte/zeca-livre/notas.md` (seção 11) e a busca do nome.
- **T9.1 Repositório público limpo.** Repositório novo, exportado sem o histórico; NOTICE e CREDITS em ordem; guarda no CI contra skin rastreada com `redistribuivel: false` e qualquer arquivo em `skins-locais/`; Dockerfile publicável; SECURITY.md, página de privacidade (só metadados), CONTRIBUTING e modelos de issue com a saída do `doutor`.
  - *Verificação:* script no CI sem arquivo derivado do pack em `git log --all --name-only`; a guarda de nomes do `bin/pet verificar` (decisão 0035) verde; uma skin plantada com `redistribuivel: false` faz o CI falhar.
- **T9.2 Skin padrão livre embutida e «traga seu pack» no binário.** A skin original embutida e aprovada pelo build; o montador do `xtask` vira o crate `pet-arte` e o subcomando `bichinho skin instalar <zip>` (grava na pasta de dados do usuário, nunca baixa nada do itch.io). *Em parte puxada para a skin livre (TS.1–TS.4, decisões 0065–0069): a arte original em CC0 com o gerador (TS.1) e a skin `zeca-livre` (TS.2) nascem na branch `skin-zeca-livre`; o que pede um pixel artist (bicos girados, ícone pequeno, pose-base em S, penas) está em `arte/zeca-livre/notas.md`.*
  - *Verificação:* CI nos três SOs com um pack sintético dá a mesma impressão sha256 (determinismo); o pack de verdade só no notebook, nunca no CI.
- **T9.3 Instalar, iniciar com o sistema, desinstalar e `doutor`, por SO.** `bichinho configurar` (autostart e o plugin, sempre com confirmação), supervisor, instância única por usuário, menu no botão direito, porta e token por usuário no loopback, `doutor` (binário no PATH que o Claude vê, porta, token, backend e capacidades, permissões no macOS).
  - *Verificação:* contêineres Linux limpos (Arch, Ubuntu, Fedora); runners macOS e Windows instalam, `doutor` verde, desinstalam sem sobras; Mac alugado e VM sobem o pet sozinhos depois do reboot.
- **T9.4 Release e canais.** `dist` (Linux musl x86_64 e aarch64, macOS arm64 e x86_64, Windows x86_64; shell, PowerShell, Homebrew, MSI; atestados e SHA256), mais `.deb`, `.rpm`, AUR `-bin` e winget à parte; Rust fixo no CI, `--locked` e `--remap-path-prefix`; marketplace do plugin no próprio repositório.
  - *Verificação:* tag de ensaio num repositório de teste com todos os jobs verdes e `gh attestation verify`; instalação por canal nos runners; dois builds Linux com o mesmo SHA256.
- **T9.5 Assinatura.** Windows por SignPath Foundation (grátis, exige tudo OSI e nenhuma arte proprietária no binário) ou Certum Open Source (€ 69+); macOS sem notarização no começo (fórmula do Homebrew), Developer ID quando houver `.dmg` ou o foco na aba exata.
  - *Verificação:* `signtool verify /pa` no runner e o nome do editor numa VM limpa; depois `spctl -a -vv` e `codesign -dv`.
- **T9.6 Beta pública e v1.0.0.** README com tabela de suporte por SO e desktop, GIF só da skin livre, instalação por canal, plugin e privacidade; checklist manual por SO a cada release; beta de 2 a 4 semanas.
  - *Verificação:* instalação nova e completa em cada SO (Hyprland no notebook, laboratório Docker, VM KDE, Mac alugado, VM do Windows): instalar, abrir sessão do Claude, aceno, pulinho e chamada, clicar e focar o terminal, desinstalar sem sobras; matriz da beta preenchida.

## Segurança e privacidade (vai para DECISIONS e docs/SEGURANCA.md)

**O container é empacotamento, não sandbox.** Quem acessa o `wayland-1` (teclado virtual, captura de tela) já roda código como o usuário. O bind de `/run/user` expõe também o D-Bus e o gpg-agent.

**O que reduz o risco:**
- uid 1000, `cap_drop ALL`, `no-new-privileges`, rootfs somente-leitura, limites de pids, memória e CPU;
- nada de `docker.sock` e nada de rede do host;
- entrada só por loopback, com checagem de Host, Content-Type e `X-Pet`, até 8 KiB;
- o daemon abre só o `wayland-N` e o `.socket2.sock`;
- bases fixadas por digest e `--locked`;
- hooks passam só metadados, com testes canário;
- o socket2 nunca guarda títulos de janela;
- cenários gravados são pseudonimizados.

## Riscos principais

| Risco | Mitigação |
|---|---|
| Custo de repintura do Hyprland | orçamento de commits medido no M1; plano B de duas superfícies |
| Bugs no encanamento Wayland feito à mão | o SCTK cobre a maior parte; M1 é portão |
| O Zeca ficar feio | arte profissional; só acessórios pequenos são nossos; folha de contato e portão humano antes de virar personagem |
| Incomodar | escalada com teto e consciente de presença; mesclagem; soneca; DND; movimento só no início dos estados |
| Festa errada (fundo, máquina, Esc, Stop bloqueado) | regras de corrente/continuação; timeouts; cenários dourados |
| Vazar conteúdo | lista branca + canários; títulos descartados; DND lido no host |
| Mudanças no Claude Code ou no Hyprland | parsers tolerantes; cenários; `validate --strict` depois de cada atualização do Claude Code |

## Pendências que dependem de você

- **Comprar e baixar** o pack da arara (US$ 0,50+): https://exclusiveolive.itch.io/cute-parrots-pixel-art-asset-pack
- **Nome Zeca:** confirmado em 2026-10-03, também em público (decisão 0035).
- **Consentimentos:**
  - a regra do Hyprland no M4;
  - cada execução do e2e com monitor virtual;
  - CI no GitHub Actions (opcional até o M8; a T8.2 pede os runners de macOS e Windows).
- **Testes manuais que exigem hardware:** HDMI, tampa fechada, suspensão, logout/login, reboot.
- **O Zeca original (branch `skin-zeca-livre`, decisões 0065–0069):** confirmar a dedicação CC0 do conteúdo final antes do merge; aprovar o `zeca-livre-escuro` pela folha de contato em `tmp/previa-zeca-livre/` (`skin = "zeca-livre-escuro"` na seção `[aparencia]` e `bin/pet skin-aprovar zeca-livre-escuro`); a regra da arte livre no CLAUDE.md (o texto está no `docs/SKINS.md`).
- **Abertas pela pesquisa multiplataforma** (`docs/pesquisa/09-multiplataforma.md`, T9.0): a arte do personagem público (o Zeca original em CC0 já existe, decisão 0065; falta o que pede pixel artist), quanto gastar com arte e assinatura, GNOME e X11 na v1.0 ou na v1.1, a rota de assinatura no Windows, a conta da Apple, um Mac e um Windows para a conferência visual (aluguel, VM ou voluntários) e o tempo para issues e contribuições.
