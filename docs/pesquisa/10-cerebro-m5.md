# Cérebro do M5: o que o Claude Code 2.1.288 manda de verdade

> Pesquisa de 2026-10-05, para o M5 (cérebro completo). **O que vale é o
> `PLANO.md` (M5) e o `DECISIONS.md` (0071 em diante)**; isto é a evidência.
> Só metadados: nenhum prompt, título de janela ou caminho aparece aqui. Os
> ids estão trocados por pseudônimos (`s1` sessão, `p1` turno, `x1` agente ou
> tarefa em segundo plano). Pode envelhecer: confira antes de confiar num
> detalhe, e refaça depois de cada atualização do Claude Code.

## Como foi feito

- **Daemon de rascunho**, nativo, da branch `m5-cerebro` (igual à `main`
  nessa hora): `PET_DEBUG=1`, `PET_ESCUTA=127.0.0.1:27391`,
  `PET_PORTA_PUBLICA=27391`, estado e config numa pasta de `tmp/`,
  `PET_HOST_RUNTIME=/nao/existe` (nunca conectou ao Hyprland). Parado pelo
  PID no fim. A produção (porta 27380) não recebeu nada.
- **Sessões aninhadas** do Claude Code 2.1.288 no tmux, em pastas de
  rascunho em `tmp/`, com `CLAUDECODE` e todas as `CLAUDE_*` fora do
  ambiente, `--dangerously-skip-permissions --model haiku` e `PET_PORTA=27391`:
  o hook do plugin instalado (o `bichinho avisar` do PATH, commit `a174f23`)
  mandou só para o daemon de rascunho. O texto foi com `tmux send-keys -l` e o
  `Enter` num `send-keys` à parte.
- **O que chegou** foi lido do `/v1/debug/eventos` (só o que passou na
  validação do fio v1). Para entender a origem de cada turno, os metadados do
  transcript das próprias sessões de rascunho (`origin`, `turnOrigin`,
  `isMeta`, `promptId`; nunca o texto) foram conferidos à parte.
- **O binário** (`~/.local/share/mise/installs/claude/2.1.288/claude`) foi lido
  com `strings` (o JavaScript do Claude Code vai dentro dele, minificado).

Experimentos: uma resposta simples; um turno com Write e Edit; um agente em
segundo plano (a ferramenta Agent com `run_in_background`) que terminou e
voltou; um agente que o modelo pôs em segundo plano sem pedirem; um shell em
segundo plano que fica rodando (um servidor de mentira) e turnos normais com
ele; um shell curto em segundo plano que acaba; um `/loop` de 1 minuto
(CronCreate) com dois tiques; um AskUserQuestion respondido pelo tmux; um
ExitPlanMode no modo plano, aprovado; um Esc no meio de uma ferramenta; um
`/compact`; e o `/exit`, com o diálogo do trabalho em segundo plano.

## O que chegou

Tempo em segundos desde o primeiro evento; `atraso` (o `ts` do hook até a
chegada) ficou entre 1 e 22 ms em todos.

### Resposta simples e turno com edição (s1)

```
  0.000 s1 SessionStart       -    src=startup
  8.719 s1 UserPromptSubmit   p1
  9.922 s1 Stop               p1   bg=0
 25.020 s1 UserPromptSubmit   p2
 33.981 s1 PostToolUse        p2   tool=Write dur=23 arq=…
 34.416 s1 PostToolUse        p2   tool=Read dur=21
 36.127 s1 PostToolUse        p2   tool=Edit dur=22 arq=… (o mesmo)
 39.283 s1 Stop               p2   bg=0
```

O `SessionStart` vem sem `prompt_id` (ele só existe depois do primeiro
prompt). Todo Stop traz `bg` (0 sem nada em segundo plano). O
`UserPromptSubmit` nunca traz `src`.

### Agente em segundo plano: três turnos para um pedido

```
 51.264 s1 UserPromptSubmit   p3                         ← o Renan pede
 53.463 s1 SubagentStart      p3   agente aid=x1
 53.469 s1 PostToolUse        p3   tool=Agent dur=6       ← volta na hora
 54.888 s1 Stop               p3   bg=1 bgt=subagent bgi=x1
 58.705 s1 PostToolUse        p3   tool=Bash dur=41 agente aid=x1
 59.944 s1 UserPromptSubmit   p4                         ← notificação: o agente "terminou"
 62.851 s1 Stop               p4   bg=1 bgt=shell bgi=x2  ← o shell que o agente deixou rodando
 83.798 s1 SubagentStart      p4   agente aid=x1          ← o MESMO agente acorda
 84.611 s1 UserPromptSubmit   p5                         ← notificação: terminou de novo
 85.946 s1 Stop               p5   bg=0
145.949 s1 Notification       p5   nt=idle_prompt         ← 60 s depois do Stop
```

- O id da tarefa em segundo plano de um subagente (`bgi`) **é** o `agent_id`
  dele (`aid`).
- Depois do Stop de p3, as ferramentas do agente chegam com `agente`, `aid` e
  o `prompt_id` de p3 (o prompt de agora). O cérebro do M3 as ignora
  (`ferramenta_de_agente_fora_do_turno`): o turno já fechou.
- O agente terminou com um shell dele ainda rodando; o Stop seguinte do
  turno principal lista esse shell (`bgt=shell`). Quando o shell acabou, o
  **agente** acordou (`SubagentStart` de novo, com o mesmo `aid` e o
  `prompt_id` de agora), terminou, e só então o turno principal foi
  acordado. O cérebro do M3 tomou esse `SubagentStart` por uma continuação
  da thread principal e reabriu p4.
- Cada notificação abre um turno novo, com `prompt_id` novo e um
  `UserPromptSubmit` sem marca nenhuma de que não foi o Renan.
- No transcript, os turnos p4 e p5 têm `origin.kind: "task-notification"` e
  `turnOrigin: "task_notification"`, e o texto do prompt **começa** com
  `<task-notification>` seguido de `<task-id>` com o id da tarefa (o mesmo do
  `bgi`).

### Agente que o modelo pôs em segundo plano (s1)

```
901.201 s1 UserPromptSubmit   p20
903.736 s1 SubagentStart      p20  agente aid=x5
903.740 s1 PostToolUse        p20  tool=Agent dur=5
905.528 s1 PostToolUse        p20  tool=Bash dur=75 agente aid=x5
906.116 s1 PostToolUse        p20  tool=Read dur=3 agente aid=x5
907.464 s1 Stop               p20  bg=2 bgt=shell,subagent bgi=x3,x5
908.194 s1 UserPromptSubmit   p21                        ← notificação, 0,7 s depois do Stop
909.492 s1 Stop               p21  bg=1 bgt=shell bgi=x3
```

O pedido era de um agente em primeiro plano; o `PostToolUse` do Agent voltou
em 5 ms e o Stop listou o agente em voo. As ferramentas do agente vieram
antes do Stop, e o agente ainda escrevia a resposta. A notificação chegou
**dentro** da acomodação de 0,8 s do Stop anterior.

### Shell em segundo plano: o servidor que fica e o que acaba (s1)

```
163.763 s1 UserPromptSubmit   p6                         ← sobe o "servidor"
165.301 s1 PostToolUse        p6   tool=Bash dur=14
166.312 s1 Stop               p6   bg=1 bgt=shell bgi=x3
171.305 s1 UserPromptSubmit   p7                         ← um turno normal
171.964 s1 Stop               p7   bg=1 bgt=shell bgi=x3  ← o servidor continua
177.884 s1 UserPromptSubmit   p8                         ← um shell curto em segundo plano
179.777 s1 PostToolUse        p8   tool=Bash dur=17
180.533 s1 Stop               p8   bg=2 bgt=shell,shell bgi=x3,x4
185.797 s1 UserPromptSubmit   p9                         ← notificação: o shell x4 acabou
186.745 s1 Stop               p9   bg=1 bgt=shell bgi=x3
246.748 s1 Notification       p9   nt=idle_prompt
```

Com o servidor rodando, **todo** Stop da sessão vem com `bg ≥ 1`. Um shell
que acaba também acorda a sessão com um turno de notificação (no transcript,
`<task-notification>` com o `<task-id>` x4).

### `/loop` de 1 minuto (s2)

```
282.669 s2 SessionStart       -    src=startup
298.324 s2 UserPromptSubmit   p10                        ← "/loop 1m …" (a skill vira prompt)
303.558 s2 PostToolUse        p10  tool=ToolSearch dur=2
305.408 s2 PostToolUse        p10  tool=CronCreate dur=1
307.270 s2 Stop               p10  bg=0
321.462 s2 UserPromptSubmit   p11                        ← tique
322.358 s2 Stop               p11  bg=0
381.475 s2 UserPromptSubmit   p12                        ← tique
382.210 s2 Stop               p12  bg=0
441.487 s2 UserPromptSubmit   p13                        ← tique
442.249 s2 Stop               p13  bg=0
538.985 s2 SessionEnd         p14  reason=prompt_input_exit
```

- Cron não é tarefa em segundo plano: `bg=0` em todos os Stops.
- Cada tique é um turno com `prompt_id` novo e `UserPromptSubmit` sem marca.
  No transcript: `isMeta: true`, `turnOrigin: "scheduled"` e o texto do
  prompt **igual** ao que foi agendado. Nada no texto separa um tique de um
  prompt digitado.
- O Stop do 2.1.288 traz `session_crons` (os agendamentos que vão acordar a
  sessão), que o hook de hoje não manda.
- O `/exit` com trabalho em segundo plano abre um diálogo ("parar as tarefas
  e sair?"); o `SessionEnd` veio com o `prompt_id` do `/exit`. Enquanto o
  diálogo estava aberto, um tique não entrou.

### Pergunta e plano: três gatilhos para um diálogo (s1)

```
545.775 s1 UserPromptSubmit   p15
547.980 s1 PreToolUse         p15  tool=AskUserQuestion
547.994 s1 PermissionRequest  p15  tool=AskUserQuestion   ← +14 ms
553.992 s1 Notification       p15  nt=permission_prompt   ← +6 s
564.739 s1 PostToolUse        p15  tool=AskUserQuestion dur=0  ← respondida
565.506 s1 Stop               p15  bg=1 bgt=shell bgi=x3

865.906 s1 UserPromptSubmit   p19                        (modo plano)
871.839 s1 PreToolUse         p19  tool=ExitPlanMode
871.860 s1 PermissionRequest  p19  tool=ExitPlanMode      ← +21 ms
877.857 s1 Notification       p19  nt=permission_prompt   ← +6 s
884.481 s1 PostToolUse        p19  tool=ExitPlanMode dur=1 ← aprovado
886.567 s1 PostToolUse        p19  tool=Write dur=30 arq=…
888.329 s1 PostToolUse        p19  tool=Read dur=9
889.488 s1 Stop               p19  bg=1 bgt=shell bgi=x3
```

- A pergunta e o plano disparam `PreToolUse` **e** `PermissionRequest` da
  mesma ferramenta, a 14–21 ms um do outro, e a `Notification` 6 s depois:
  **três gatilhos de um diálogo só**. A janela de 5 s do plano não cobre a
  notificação.
- O `duration_ms` da ferramenta de diálogo é 0 ou 1: a espera pelo Renan não
  entra no tempo de ferramenta (o schema diz "Excludes permission-prompt and
  hook time").

### Esc no meio de uma ferramenta, e o `/compact` (s1)

```
616.360 s1 UserPromptSubmit   p17                        ← Esc com o Bash rodando
                                                          (nada mais deste turno)
744.908 s1 PreCompact         p18                        ← /compact (prompt_id novo)
                                                          (a compactação falhou com o haiku:
                                                           nenhum PostCompact)
811.098 s1 Notification       p18  nt=idle_prompt         ← 66 s depois
```

- Com o Esc no meio de um Bash em primeiro plano, **nada** chegou: nem
  `PostToolUseFailure` com `is_interrupt`, nem Stop, nem `idle_prompt` nos
  mais de 100 s seguintes. Pelo código, o hook de falha recebe um sinal de
  cancelamento; é provável que seja o mesmo que o Esc dispara (não
  conferido). Num caso, o próprio harness bloqueou a ferramenta antes de
  rodar, e também nada chegou.
- O `/compact` chega como `PreCompact` com um `prompt_id` novo (o do comando),
  sem `UserPromptSubmit` (comandos não passam pelo hook de prompt). A
  compactação do haiku falhou ("resposta vazia"), e o `PostCompact` nunca
  veio: o estado "compactando" precisa de prazo.
- O `idle_prompt` chegou 60–66 s depois do último evento, com o `prompt_id`
  do último turno.

## O que o binário diz

- **`source` compilado fora.** O hook de prompt monta
  `{...comuns, hook_event_name:"UserPromptSubmit", prompt:…, ...!1,
  session_title:…}`: o espalhamento que poria o `source` virou `...!1`. O
  `source` é calculado (`system` para notificações, `loop_wakeup` e
  `schedule_wakeup` para tiques), mas não sai.
- **Campos comuns:** `session_id`, `transcript_path`, `cwd`,
  `scratchpad_dir`, `prompt_id`, `permission_mode`, `agent_id`, `agent_type`,
  `effort`. O `prompt_id` é o do "diário" da sessão e é **trocado a cada
  entrada que não é resultado de ferramenta**: cada notificação e cada tique
  ganham um id novo.
- **Notificações passam pelo hook de prompt** com a origem `system` (a fila
  do REPL marca como `system` toda entrada `isMeta` ou que não veio de
  pessoa). Os crons entram na fila com `isMeta`, `wakeupSource` e o texto
  agendado, sem embrulho.
- **Formato da notificação:** `<task-notification>`, `<task-id>` (o id da
  tarefa: o `agent_id` do agente, o id do shell), `<status>`, `<summary>` e,
  às vezes, `<result>` e `<usage>`. O texto do resultado é conteúdo: nunca
  pode sair do hook.
- **Stop:** `stop_hook_active`, `last_assistant_message`,
  `background_tasks` (id, tipo, status, descrição, comando… — "running/
  pending + backgrounded"; vazio sem nada) e `session_crons` (id, expressão,
  `recurring` e o texto do prompt agendado; "vazio sem nada agendado").
- **StopFailure.error:** `authentication_failed`, `oauth_org_not_allowed`,
  `account_on_hold`, `verification_required`, `billing_error`, `rate_limit`,
  `overloaded`, `invalid_request`, `model_not_found`, `server_error`,
  `unknown`, `max_output_tokens`, `cloud_credential_error`.
- **PostToolUse.duration_ms** exclui o tempo do pedido de permissão e o dos
  hooks.

## Consequências para o M5

1. **Sem `src`, um turno de máquina parece um prompt do Renan.** As regras do
   PLANO (`src=system` como continuação, o teto para
   `loop_wakeup`/`schedule_wakeup`) não acham nada no 2.1.288. As saídas
   (decisão 0072):
   - a **notificação** tem forma própria: o texto começa com
     `<task-notification>`. O hook olha só esse começo e manda um enum (`orig`:
     `notificacao` ou `comum`), sem o texto sair do processo dele;
   - o **tique** não tem forma própria. O Stop diz se a sessão tem
     agendamentos (`session_crons`): o hook manda só a contagem (`crn`). Com
     agendamento pendente, um prompt sem marca que chega com o Renan longe do
     teclado, ou com outra janela certa em foco, é um tique;
   - com o hook antigo (sem `orig` nem `crn`), o pet usa o que tem: um prompt
     enquanto há agente em voo é tratado como notificação.
2. **A corrente** abre num Stop com agente em voo (`bgt` de agente), soma os
   turnos e as ferramentas do agente e fecha no primeiro Stop sem agente em
   voo, com uma festa só. O shell que o agente deixou rodando não segura a
   corrente; o agente que acorda (o mesmo `aid`) é trabalho em segundo
   plano, não continuação da thread principal (decisão 0073).
3. **O tempo do Agent** em primeiro plano seria a vida inteira do subagente,
   com o pensar dele: fica fora do `min_ativos` (as ferramentas do subagente
   já contam pelo `aid`).
4. **O Esc não manda nada.** O turno fica aberto; quem o fecha é o próximo
   prompt (sem festa, 0,8 s depois), o `idle_prompt` (se vier), o
   `SessionEnd`, e o estado volta a parado pelo prazo de 5 min.
5. **Um diálogo dispara até três gatilhos em 6 s.** A dedupe vale enquanto
   nada andou na sessão, não só 5 s; o tipo sobe (pergunta, plano) e nunca
   desce para permissão.
6. **O "compactando" precisa de prazo:** o `PostCompact` pode não vir.
7. **Bug do M4 achado:** a janela de cada sessão é casada em todo
   `UserPromptSubmit` sem `src` (decisão 0060), então uma notificação ou um
   tique casa a sessão com a janela que estiver em foco (o navegador, por
   exemplo). Com o `orig` e a origem do turno, só o prompt digitado casa.

## Não exercitado ao vivo (coberto por cenários sintéticos)

Workflow (pede a palavra-chave do Renan e muitos tokens), teammate, sessão na
nuvem, Monitor, `StopFailure` (não dá para provocar um limite de uso),
Stop hook de outro plugin (`stop_hook_active`), `PostCompact` de verdade,
`ScheduleWakeup`, `poll_event`, `elicitation_dialog` e duas sessões
terminando juntas.

## Os cenários que saíram daqui (T5.9)

As duas rodadas do `/v1/debug/eventos` do daemon de rascunho passaram pelo
`pet_core::cenario::de_eventos` (os pseudônimos da decisão 0078; o mesmo que
o `bin/pet eventos --salvar` usa) e viraram cenários em `cenarios/`, com o
tempo recomeçando em cada trecho. A primeira rodada foi com o hook instalado,
sem `orig`: o `orig` dos prompts entrou como o hook do M5 o calcula (o
transcript diz quais eram notificação). A segunda já veio com o hook do M5.
As linhas do desktop (o Renan no terminal do Claude, ou longe) são as de um
dia comum, acrescentadas à mão.

| Cenário | Trecho |
|---|---|
| `real-agente-em-segundo-plano` | o agente em segundo plano em três turnos (51–146 s) |
| `real-servidor-e-shell-curto` | o servidor que fica e o shell curto que acaba (163–247 s) |
| `real-pergunta-e-plano` | a pergunta (545–576 s) e o plano (865–889 s) |
| `real-esc-e-compact` | o Esc, o `/compact` sem o `PostCompact` e o `idle_prompt` (616–811 s) |
| `real-agente-dentro-da-acomodacao` | o agente que o modelo pôs em segundo plano e a notificação na acomodação (901–931 s) |
| `real-laco` | a segunda rodada: o shell e a notificação, o `/loop` com o `crn`, o `idle_prompt` e um tique com o Renan longe |
| `pergunta`, `plano-lido-no-terminal` | a pergunta e o plano da primeira rodada, o plano com a leitura esticada |
