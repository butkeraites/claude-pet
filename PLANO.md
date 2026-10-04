# Plano: claude-pet — o Zeca, papagaio pixel art que comemora quando o Claude Code termina

> Plano aprovado em 2026-10-02. Mudanças de rumo entram em DECISIONS.md; este arquivo só ganha correções e IDs de tarefa.

## Contexto

O Renan quer um bichinho de pixel art que more na tela e reaja ao Claude Code rodando no terminal. Ele deve comemorar de um jeito fofo, engraçado e **chamativo** quando o Claude termina de desenvolver algo, e chamar o usuário quando o Claude precisa dele. Requisitos:

- **Docker:** roda 100% local, em Docker.
- **Por cima de tudo:** "ficar no topo da tela" foi lido como ficar acima de todas as janelas. A posição inicial é o canto inferior direito; se ele quiser o bicho no alto, basta arrastar, e a posição fica gravada por monitor.
- **Arrastável:** pode ser levado com o mouse para qualquer lugar.
- **Monitor ativo:** fica sempre no monitor/área de trabalho que está em foco.
- **Terminal:** funciona com o Claude Code CLI.
- **GitHub:** o repositório fica no GitHub dele e evolui aos poucos.

**Escolhas já feitas com ele:**

- **Personagem.** É a arara do pack *Cute Parrots!* da exclusiveOlive (itch.io, US$ 0,50), transformada num papagaio malandro "estilo Zé Carioca, um pouco diferente":
  - verde, com chapéu-palheta e gravata-borboleta em cores próprias (laranja);
  - nome próprio, proposto **Zeca**. Não é o personagem da Disney nem copia o visual dele.
  - O Renan recusou rascunhos em ASCII ("muito feias"). Por isso o corpo e as animações vêm de um pixel artist profissional; nós só desenhamos acessórios pequenos.
- **Sem som.** Toda a atenção vem da animação.
- **Festa proporcional ao trabalho** do turno.
- **Repositório privado** `butkeraites/claude-pet`, clonado em `~/Documents/claude-pet`.

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
- **Clique:** ele dá uma risadinha com coração e marca o aviso exibido como visto.
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
| Hooks | Plugin `bichinho`, no próprio repo, com 13 hooks `async` de comando que chamam `avisar.sh` (jq + `curl -m 2`, sempre exit 0). | Async não atrasa o Claude. Um hook do tipo http mostraria erro no transcript sempre que o pet estivesse desligado. Só **metadados** saem do host. |
| Festa | Calculada pelo **trabalho real**: ferramentas de trabalho, arquivos editados, subagentes e tempo de ferramenta. Não usa tempo de relógio. | Com Opus em esforço máximo, uma resposta simples leva 20–90 s. Medir por relógio daria festa em toda resposta. |
| Atenção | Escalada só visual, com teto, e que sabe se você está presente: olhando o terminal do Claude, fica no nível 1. | Não há som. Movimento que começa chama atenção; movimento constante cansa. |
| Arte | O pack comprado fica em `skins-locais/` (gitignored) e entra só na imagem local, que nunca vai a registry. Chapéu e gravata são arte nossa, commitada. | A licença permite editar e proíbe redistribuir. |
| Nomes | - repo, binário, compose e namespace da camada: `claude-pet`;<br>- plugin e marketplace: `bichinho` (nomes de plugin que começam com `claude-` são reservados);<br>- personagem/skin: `zeca`;<br>- CLI: `bin/pet`;<br>- variáveis de ambiente: `PET_*`. | Trocar o personagem não exige renomear o código. |
| Fuso | Bind de `/etc/localtime`, nunca `TZ`. | O host está em America/New_York e os outros projetos usam São Paulo; não dá para assumir nenhum dos dois. |

## Arquitetura

```
HOST (Hyprland, uid 1000)                                  CONTAINER claude-pet (alpine 3.24.2, uid 1000, rootfs ro)
claude (foot) + plugin bichinho                            PID1 docker-init (init: true)
  hook async: sh avisar.sh <Evento>                         └ claude-pet rodar (um processo)
    jq (lista branca) -> curl -m 2 POST ----------------->     main (calloop): cérebro, animador, cena, sessão Wayland, timers, estado
      127.0.0.1:27380/v1/evento                                thread ingress: HTTP 0.0.0.0:27380
/run/user (bind ro, rslave) ------------------------------>    thread hypr: leitor do .socket2.sock (só eventos)
  1000/wayland-1          <- 1 camada OVERLAY                  thread watchdog: batimento > 60 s -> abort() -> Docker reinicia
  1000/hypr/<HIS>/.socket2.sock
/etc/localtime (ro), ./config -> /etc/claude-pet (ro), volume estado -> /state
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
- Ciclo nativo, o mais rápido: `docker compose stop pet` e depois `cargo run -p claude-pet -- rodar`, com `PET_HOST_RUNTIME=/run/user` e caminhos locais.

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
| Pet desligado enquanto o Claude roda | O hook async falha calado: o curl expira em 2 s e o script sai com 0. |

## Superfície, renderização e nitidez

**Superfície:**
- camada OVERLAY, namespace `claude-pet`, ancorada nas 4 bordas;
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
  - dá para sobrescrever no config.
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

**Tela cheia e proteção de tela:**
- O padrão é mostrar por cima da tela cheia, como foi pedido; `aparencia.tela_cheia = esconder` é opcional.
- Durante a proteção de tela do Omarchy (janela `org.omarchy.screensaver`), o Zeca se esconde.

**Posições salvas:**
- São guardadas como fração da tela, por monitor.
- A chave é a descrição do output (fabricante/modelo/série), com o nome do conector como reserva, porque em docks o nome DP-N muda.

**Arrastar:**
- O arraste começa depois de 4 px ou de 250 ms segurando. Só então a região de input cresce para a superfície inteira, para o arraste funcionar até numa área de trabalho vazia.
- A região volta ao tamanho do corpo ao soltar, depois de 5 s sem eventos de ponteiro, em `closed`, ao esconder e antes de qualquer re-home.
- Solto fora do monitor: re-home imediato para o monitor sob o ponteiro.
- O cursor fica `grab` ao passar por cima e `grabbing` enquanto segura (cursor-shape-v1).

**Clicar:**
- Botão esquerdo: risadinha + coração. Marca como visto **só o aviso exibido**; as outras sessões continuam.
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

**Stop:**
- Todo Stop é candidato a fim de turno.
- Acomodação de 0,8 s: o fim é cancelado só por um evento da thread principal com `ts` posterior ao Stop.
- Dedupe por `(sid, turno)`.
- Um Stop com `sha=true` de um turno que já comemorou recalcula a pontuação e funde na festa só se o nível subir.

**Pontuação** (todos os pesos ficam no config; cada Stop registra os componentes em `/v1/estado.turnos`):

```
min_ativos = soma do dur de todas as ferramentas do turno (inclusive as dos subagentes, via aid) / 60000   # nunca o tempo de pensar
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
   - Frases cariocas em PT-BR, editáveis em `assets/frases.toml`: "Prontinho!", "Tá pronto, parceiro!", "Ô, meu camarada! ‹proj› precisa de você", "Plano pra aprovar!", "Deu ruim...", "Cansei...", "Oi! Me arrasta pra onde quiser".
8. **Créditos:** em `NOTICE.md` e no `CREDITS.md` de cada skin (exclusiveOlive e monogram/datagoblin).

## Repositório e convenções

```
claude-pet/
  CLAUDE.md README.md DECISIONS.md PROGRESS.md PLANO.md LICENSE(MIT) NOTICE.md
  Cargo.toml Cargo.lock rust-toolchain.toml(channel "stable" + clippy + rustfmt) rustfmt.toml .editorconfig
  crates/pet-core/src/{config,event,brain,score,attention,animator,recipes,skin,aseprite,raster,scene,particles,text,geometry,scenario}.rs
  crates/claude-pet/src/{main,daemon,discovery,hypr,ingress,store,watchdog}.rs  crates/claude-pet/src/wl/{surface,fractional,input,shm,idle}.rs
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
- novas entradas são acrescentadas, nunca reescritas;
- as decisões deste plano entram como 0001+.

**PROGRESS.md:** uma linha por tarefa, `| Data | Tarefa | O quê | Commit |`.

**Commits e PRs:**
- Commit: uma frase em português citando `(T2.3)` ou `(decisão 0007)`, terminando com `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Um commit por tarefa; uma branch e um PR por marco, revisado por você; `gh pr merge --squash` e tag `v0.N.0`.
- O corpo do PR termina com a linha do Claude Code.

**Nunca commitar:** `.env`, `config/claude-pet.toml`, `skins-locais/*`, `tmp/`, `target/`.

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
| `novo-evento-hook` | lista branca do `avisar.sh` → canário → `hooks.json` → tabela do cérebro → `/reload-plugins` |
| `conferir-na-tela` | ciclo dev, `foto`, `nitidez`, consulta de camadas |

## CLI `bin/pet` (bash + curl + jq)

| Comando | O que faz |
|---|---|
| `estado` | resumo de `/v1/estado` |
| `doutor` | saúde do container, conexões Wayland/hypr, `bichinho@bichinho-local` habilitado em `claude plugin list --json`, idade do último evento por sessão, DND, skin aprovada |
| `soneca [30m]`, `acordar`, `esconder [30m]`, `mostrar`, `posicao-padrao`, `recarregar` | controles, persistidos |
| `tocar <reação>` | toca uma reação direto |
| `testar <cenário>` | eventos sintéticos **pelo `avisar.sh` real**, com `teste:true` e TTL de 60 s; nunca se misturam com sessões reais |
| `simular <cenário>` | roda o cenário no core com relógio falso; o daemon só exibe as intenções |
| `eventos --salvar <arquivo>` | grava um cenário com ids pseudonimizados |
| `foto` | captura com grim para o Claude ler |
| `skin-instalar <zip\|dir>` | importa um pack |
| `plugin-atualizar` | atualiza a worktree estável do plugin |
| `subir`, `parar`, `logs`, `reconstruir`, `dev` | atalhos do compose |
| `verificar` | portão antes de commit |

**`verificar` roda:**
- `cargo fmt --check`;
- `cargo clippy --all-targets -- -D warnings`;
- `cargo test`;
- `lint-skin` em todas as skins;
- `docker compose config -q`;
- `claude plugin validate` (`--strict`) no repo e em `plugin/`;
- shellcheck, se estiver instalado.

**Config `config/claude-pet.toml`** (gitignored; modelo em `exemplo.toml`; relido a quente):
- Precedência: comandos persistidos em `/state`, depois `PET_*`, depois o arquivo, depois os padrões.
- `/v1/estado.config` mostra o valor de cada chave e de onde veio.

## Marcos (cada um: branch + PR; cada tarefa: commit + linha no PROGRESS)

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
1. **Posição:** `hyprctl -j layers` mostra `claude-pet` no nível 3 do monitor focado, com o retângulo lógico do monitor.
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
6. **Reinício:** `docker compose restart pet` volta em até 3 s, no mesmo lugar.
7. **Crash:** `kill -9` do processo pelo host faz o RestartCount subir. `docker kill` **não** serve, porque cancela a política de restart.
8. **Sem compositor:** `docker compose run --rm --no-deps -e PET_HOST_RUNTIME=/tmp/nada pet` registra "aguardando compositor".

**Se falhar:**
- Nitidez, clique ou reconexão falhando são bugs a corrigir em `wl/`.
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
- `SEGREDO-n` plantado em todo campo de conteúdo; nada disso pode sair do script;
- o script sempre sai 0;
- não imprime nada;
- o curl é falso.

**Verificação:**
- `claude plugin list` mostra `bichinho@bichinho-local` habilitado.
- Gate **interativo** num diretório já confiável:
  1. `tmux new-session -d -s e2e -c <dir> 'claude --dangerously-skip-permissions "responda só: ok"'` → `/v1/estado` mostra a sessão e o aceno T0;
  2. um segundo prompt editando um arquivo temporário → pulinho T1;
  3. `/exit`.
- `claude -p` só serve para conferir que, com o pet parado, não aparece erro de hook.
- `time (printf '{}' | sh plugin/scripts/avisar.sh Stop)` leva no máximo 2,1 s com o pet parado.

### M4 — Arrastar e seguir o monitor ativo

**Tarefas:**
- máquina de arrastar/clicar (limiares, região de input com fail-safe, cursores);
- leitor do socket2 com reconexão:
  - guarda só nome do evento, monitor, endereço da janela e o primeiro glifo do título;
  - nunca registra o payload em log;
- seguir o foco (debounce, congelar no arraste, intervalo entre viagens, poof, `closed`, mudança de layout, FALLBACK);
- soltar entre monitores;
- posições salvas por descrição do monitor;
- esconder durante a proteção de tela;
- botão direito para soneca.
- Propor a regra opcional do Hyprland **pela skill omarchy e com seu consentimento**:
  ```lua
  hl.layer_rule({ name = "claude-pet", match = { namespace = "^claude-pet$" }, order = 1, no_anim = true })
  ```
  - `order = 1` deixa o Zeca abaixo dos popups do Omarchy (polkit, menus, toasts);
  - `no_anim` tira o fade de ~180 ms em cada troca de monitor.

**Verificação:**
- arrastar numa área de trabalho cheia e numa vazia;
- a posição sobrevive ao restart;
- `omarchy-launch-screensaver force` esconde o Zeca e ele volta depois;
- canário do socket2: `activewindow>>firefox,SEGREDO-T` nunca aparece em logs, `/v1/estado` ou `/v1/debug/eventos`;
- `scripts/e2e-monitor.sh --autorizo`, só com seu consentimento a cada vez:
  - `hyprctl output create headless` e descobre o nome novo por diff em `hyprctl -j monitors`;
  - foca o monitor novo e confere o Zeca lá; volta e confere de novo, com prazos de ≥2,5 s;
  - remove o output;
  - um `trap` sempre restaura o estado.
- HDMI, clamshell e suspensão ficam no checklist manual, registrado no PROGRESS.

### M5 — Cérebro completo

**Tarefas:**
- turnos, correntes e tarefas de fundo;
- teto de turno de máquina;
- interrupção e `idle_prompt` fechando turno;
- SessionEnd;
- pontuação e níveis por trabalho;
- mesclagem;
- prioridade;
- selos;
- escalada L1–L4 com presença;
- dedupe de aviso;
- Stop com `sha`;
- DND;
- modo discreto;
- cenários dourados com relógio falso;
- `simular` e `eventos --salvar`.

**Verificação:** `cargo test -p pet-core` verde. Tabelas, exemplos de pontuação e cenários:

| Cenário | Esperado |
|---|---|
| `rapido` | aceno T0 |
| `resposta-longa-sem-ferramenta` | T0 |
| `pequeno`, `medio`, `grande` | T1, T2, T3 |
| `dois-prontos` | uma festa só, "2 prontos" |
| `pergunta`, `pergunta-dupla` | um aviso só |
| `plano-lido-no-terminal` | fica em L1 |
| `pergunta-ausente` | L1 → L2 → L3 → teto |
| `idle-prompt-repetido` | — |
| `servidor-em-segundo-plano` | festas normais com um dev server rodando |
| `workflow-longo` | T3 no Stop final do turno `system` |
| `stop-bloqueado` | — |
| `interrompido` | — |
| `erro-limite` | — |
| `protetor-de-tela` | — |
| `compartilhando-tela` | balão sem nome de projeto |

### M6 — Encanto e atenção (portão de "sensação")

**Tarefas:**
- partículas e movimento reduzido;
- balões (9-slice, monogram, pop, datilografia, frases cariocas, modo discreto);
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
- **Nome Zeca:** confirmar ou trocar. Só muda a skin e os balões.
- **Consentimentos:**
  - a regra do Hyprland no M4;
  - cada execução do e2e com monitor virtual;
  - CI no GitHub Actions (opcional).
- **Testes manuais que exigem hardware:** HDMI, tampa fechada, suspensão, logout/login, reboot.
