# A tela do M5: o compartilhamento no Hyprland 0.56.2 e o que as skins têm para a base

Pesquisa da segunda metade do M5 (2026-10-05), a que desenha as intenções
do cérebro (decisão 0077) e liga a plataforma. Duas perguntas:

1. o que o `screencast` do socket2 diz de verdade no Hyprland 0.56.2, para a
   discrição do compartilhamento de tela (decisão 0076);
2. o que cada skin tem para os estados que a base segura (decisão 0076), para
   o desenho caber no orçamento de commits sem arte nova.

## Como foi feito

- O código-fonte do Hyprland na tag `v0.56.2` (commit `efb50993…`), lido
  pela API do GitHub (só leitura): `src/managers/screenshare/`
  (`ScreenshareSession.cpp`, `ScreenshareFrame.cpp`, `ScreenshareManager.cpp`
  e `.hpp`) e a página do IPC da wiki (`hyprland-wiki`,
  `content/ipc/_index.md`). O binário do host (`/usr/bin/Hyprland`, pacote
  `0.56.2-2`) tem as duas strings, `screencast` e `screencastv2`.
- As folhas das skins (`skins/zeca-livre-escuro/sheet.json` e o Zeca do pack
  em `skins-locais/zeca/`), com os quadros de cada estado recortados numa
  pasta de rascunho fora do repositório (o pack não pode ir para o git).
- Nada no Hyprland do Renan: nenhum `hyprctl`, nenhuma captura de tela, e a
  sessão ficou bloqueada o tempo todo.

## O `screencast` no 0.56.2

**O formato** (wiki do IPC):

| Evento | Dados | O que diz |
|---|---|---|
| `screencast` | `ESTADO,TIPO` | "emitted when a screencopy state of a client changes. Keep in mind there might be multiple separate clients. State is 0/1, owner is monitor/window/region" |
| `screencastv2` | `ESTADO,TIPO,NOME` | o mesmo, com "name is the identifier of the shared target (monitor name or **window title**)" |

O `TIPO` é texto: `monitor`, `window` ou `region` (o formatter do
`eScreenshareType`, `ScreenshareManager.hpp` L258–268; `ERR NONE` num caso
que não devia acontecer). A pesquisa de origem (`03-hyprland.md`) deixou isso
sem conferir; agora está. **O `screencastv2` carrega o título da janela
compartilhada:** o pet nunca o lê (é conteúdo, regra de ouro do CLAUDE.md).

**Quem manda** (`ScreenshareSession.cpp` L137–153): cada sessão de
compartilhamento posta `screencast>>1,TIPO` (e o v2) quando passa a
compartilhar e `screencast>>0,TIPO` quando para, guardada por `m_sharing`:
numa sessão, os dois sempre alternam. Pode haver várias sessões ao mesmo
tempo (um OBS gravando o monitor e uma chamada mostrando uma janela), e o
evento não diz de qual sessão é.

**O sinal segue os quadros, não a sessão.** É o que muda o desenho da
discrição:

- o `1` sai quando um quadro é copiado com sucesso (`ScreenshareFrame.cpp`
  L145–174, `screenshareEvents(true)`; só passa se `m_sharing` era falso), e
  o quadro só é copiado quando o monitor desenha
  (`ScreenshareManager::onOutputCommit`, L14–41);
- o `0` sai por um temporizador de 500 ms, rearmado a cada quadro copiado
  (`ScreenshareSession.cpp` L68–81: "if this fires, then it's been half a
  second since the last frame, so we aren't sharing"), e no `stop()` da
  sessão (L55–61, também no destrutor).

Numa chamada com a tela parada (slides, um terminal quieto), o Hyprland manda
`0` meio segundo depois do último quadro e `1` de novo no próximo desenho. O
próximo desenho pode ser o do próprio pet: a camada dele cobre o monitor e
cada commit repinta o monitor (decisão 0005). Se a discrição desligasse no
`0`, como a decisão 0076 escreveu, o primeiro quadro de um balão com o nome
do projeto (a festa do Stop, logo depois de o spinner do terminal parar) seria
justamente o quadro que volta a ser compartilhado.

**Capturas de tela:** o `grim` (o do Omarchy e o do `bin/pet foto`) também
abre uma sessão gerenciada (`getManagedSession`, `ScreenshareManager.cpp`
L111–156; a revisão de produto já tinha achado pelo `Screencopy.cpp`), copia
um quadro (`1`) e sai (`0` no `stop` da sessão ou pelo temporizador): um
piscar de no máximo meio segundo.

**Ligar no meio:** o pet que liga (ou religa) o socket2 no meio de um
compartilhamento não sabe das sessões de antes; o `1` delas já passou e só
volta depois de um `0` e de um quadro novo.

### O que o pet faz com isso (T5.12)

- Lê só o `screencast>>ESTADO,TIPO`: o primeiro campo, `0` ou `1`; o `TIPO`
  nem é guardado. O `screencastv2` cai no "evento que o pet não usa", sem
  ser interpretado, como o `windowtitlev2`.
- Conta as sessões na ligação (um `1` soma, um `0` tira; uma ligação nova
  começa do zero) e só manda `Compartilhando(true)` na primeira e
  `Compartilhando(false)` quando não sobra nenhuma (um `0` sem `1` visto, de
  uma sessão de antes da ligação, também vale como fim). Depois de uma perda
  na caixa, manda o estado de agora de novo.
- No Motor, a discrição conta o tempo de sinal somado num episódio (os
  piscares de meio segundo de uma tela parada somam; uma captura não chega
  aos 2 s) e desliga só depois de um tempo sem sinal nenhum, que cobre as
  pausas de uma tela parada com o pet desenhando (os ritmos da base nunca
  passam de 30 s sem um quadro, fora o sono profundo). A fonte que cai conta
  como sinal desligado ali, e a discrição segura o mesmo tempo (decisão 0080:
  na dúvida, discreto, mas nunca para sempre).

## As skins para a base

O que cada estado da base (`Prioridade::estado_da_skin`) toca nas duas skins
de verdade; a pose é o primeiro quadro da primeira tag do estado.

| Estado | Zeca original (`zeca-livre-escuro`) | Zeca do pack (`zeca`) |
|---|---|---|
| `idle` | `respira`, `blink`, `ginga` (4, 5 e 7 quadros) | `sit_idle` ×3 e `stand_look_sit` |
| `working` | `work_session`: 18 quadros, 2,32 s (a pose é de pé; as rajadas digitam numa tecla) | `eating`: 10 × 100 ms |
| `thinking` | `peck`: 12 quadros, 1,73 s | `sit_idle` |
| `waiting` | `attention_call`: 23 quadros, 2,64 s, **com o «!» desenhado na arte**, à direita da cabeça, em todos os quadros | `chirp`: 4 × 100 ms, sem «!» |
| `ready` | `chirp`: 8 quadros, 1,34 s, com as notas (a pose é a mesma do repouso) | `stand`: 3 × 100 ms |
| `error` | `scared`: 12 quadros, 1,76 s (a pose é a do repouso) | `hurt`: 8 quadros, 840 ms |
| `sleep` | `sleep`: 5 × 520 ms, o laço do sono com os «z» na arte (decisão 0068) | `sleep`: 4 × 100 ms |
| `dangle` (o voo) | `fly`: 4 quadros, 340 ms | `fly`: 4 × 100 ms |
| `land` | `landing`: 5 quadros, 710 ms | `landing`: 4 × 100 ms |
| `done_medium` | `short_flight`: 20 quadros, 2,01 s, dentro da célula | `short_flight`: 16 × 100 ms |
| `done_big` | `big_flight`: 40 quadros, 4,37 s, dentro da célula | `big_flight`: 37 quadros, 3,83 s |

Consequências para o desenho:

- **O «!» do aviso não pode ser o da arte.** O Zeca original já tem um «!»
  na chamada e na espera; o do pack, não. O selo do aviso (o que pulsa na L4)
  é um desenho do pet, fora da cabeça (na fileira dos selos), para não ficar
  em cima do «!» da arte nem faltar no pack.
- **O laço do sono já está no ritmo do dormindo** no Zeca original (520 ms
  por quadro, ~1,9 troca/s); no pack, os quadros de 100 ms precisam do piso
  de 500 ms do ritmo.
- **Trabalhando e pensando** têm tags longas (18 e 12 quadros): no ritmo
  quieto (quadros de pelo menos 250 ms, uma rajada a cada 10–30 s) a rajada
  dura ~4,5 s e ~3 s, poucas trocas por minuto.
- **A espera** no Zeca original tem 23 quadros por rajada: no repouso de
  sempre (pausa até caber em 2 commits/s), uma rajada a cada ~12 s.
- **Os voos da festa** (`done_medium`, `done_big`) acontecem dentro da célula:
  o voo atravessando a tela é do M6, e no M5 o T3 é o voo grande da skin com
  a chuva de confete.
- Todas as tags têm quadros de pelo menos 34 ms depois do piso do animador
  (`DURACAO_MIN_MS`): nenhuma passa de 30 quadros por segundo.
