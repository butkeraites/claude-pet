# Cenários e intenções do cérebro

O M5 decide **o que** o Zeca faz e **quando** num registro de intenções do
Motor (decisão 0077): cada decisão vira uma linha normalizada, com a hora no
relógio do laço. Os cenários (`cenarios/*.jsonl`) são sequências de eventos
do Claude Code e do desktop; o executor roda o Motor com eles num relógio
falso, e a linha do tempo das intenções tem de ser a do dourado
(`cenarios/*.esperado.jsonl`). É o contrato com quem desenha (a segunda
metade do M5 e o M6) e o que os testes comparam.

## Comandos

```sh
bin/pet simular pergunta                  # cenarios/pergunta.jsonl no relógio falso: as intenções, uma por linha
bin/pet simular ~/algum/cenario.jsonl     # ou um arquivo
PET_ATUALIZAR_OURO=1 ~/.cargo/bin/cargo test -p pet-core cenario   # regera os dourados (leia o diff!)
PET_PORTA=27391 bin/pet eventos           # o /v1/debug/eventos de um pet de debug
PET_PORTA=27391 bin/pet eventos --salvar tmp/meu-dia.jsonl          # o cenário com pseudônimos
```

- `bin/pet simular` usa o `bichinho simular` da branch (o `PET_BICHINHO`, ou
  o `cargo run`): o do PATH é o da worktree estável. Roda sem personagem: as
  intenções são as mesmas com a skin (um teste confere em todos os cenários),
  então a saída é o `.esperado.jsonl`.
- `bin/pet eventos --salvar` só fala com um pet de debug (`PET_DEBUG=1`: a
  pilha de dev, ou um daemon de rascunho noutra porta). A produção nunca
  guarda eventos, e o comando recusa antes de pedir. O arquivo sai com os
  pseudônimos e só os campos do fio v1 que o pet lê; mesmo assim, confira
  antes de pôr no git.

## O formato

Uma linha JSON por passo, com `t` em ms desde o começo do cenário:

```json
{"cenario": "pergunta", "descricao": "…", "config": {"celebracao.modo": "discreta"}, "padrao": {"sid": "s1", "ent": "cli", "proj": "api"}}
{"t": 0, "desktop": {"ocioso": false}}
{"t": 0, "evento": {"e": "UserPromptSubmit", "turno": "p1", "orig": "comum"}}
{"t": 2210, "evento": {"e": "PreToolUse", "turno": "p1", "ts": 2205, "tool": "AskUserQuestion"}}
{"t": 30000, "clique": "esquerdo"}
{"t": 60000, "fim": true}
```

- A primeira linha pode ser o cabeçalho: `cenario` (o nome do arquivo),
  `descricao`, `config` (chaves do `bichinho.toml`, pela mesma validação do
  daemon) e `padrao` (campos que todo evento leva se não tiver os seus).
- `evento`: o corpo do fio v1, como o hook manda (`v` opcional; o `ts`, se
  houver, é relativo, como o `t`). Um campo que o pet descartaria é erro.
- `desktop`: uma coisa por linha: `ocioso`, `olhando_claude`, `protetor`,
  `compartilhando`, `ligado` (booleanos), `janela_ativa` (um id ou `null`) e
  `monitor` (o nome).
- `clique`: `esquerdo` ou `direito`. `fim`: o relógio anda até ali.
- Linhas vazias e as que começam por `#` são comentários.

O executor (`pet_core::cenario`) roda o Motor com a janela de mentira pronta
no eDP-1 (1920x1200, escala 1,5), vence os prazos como o laço do daemon e
reprova um prazo que vence e continua armado.

## As intenções

Cada linha tem `t` (ms do relógio do laço) e `i` (o tipo). O
`/v1/estado.intencoes` mostra as 50 mais novas, com `ha_ms` no lugar do `t`.

| `i` | Quando | Campos |
|---|---|---|
| `turno` | um turno fechou | `sid8`, `turno8`, `fim`, `nivel`, `pontuacao`, `teto`, `reacao`, `origem` (de máquina), `corrente`, `teste` |
| `festa` | a festa de um fim | `sid8`, `nivel`, `reacao`, `confete` (12 no T2, 40 no T3), `voo` (`curto`, `atravessar`), `faixa` (T3), `escondida` |
| `festa_mesclada` | um fim entrou na festa de agora (até 3 s) | `sid8`, `sessoes`, `nivel`, e `reacao`, `confete`, `voo`, `faixa` só se o nível subiu |
| `reacao` | o animador toca uma vez | `nome` (`alert`, `error`, `yawn`, `wake`, `bye`, `giggle`, `done_small`, …), `motivo` (`aviso`, `erro`, `cansado`, `sono`, `acordou`, `tchau`, `fim_discreto`, `clique`, `soneca`), `sid8`, `nivel` |
| `balao` | um balão | `linhas`, `motivo` (`festa`, `aviso`, `aviso_refinado`, `erro`, `cansado`, `lista`, `sem_foco`, `pedido`) |
| `base` | a base que o pet segura mudou | `estado` (o da skin: `waiting`, `error`, `sleep`, `ready`, `working`, `thinking`, `idle`), `prioridade`, `sid8`, `profundo` (o sono profundo, sem commit) |
| `selos` | os selos das outras sessões mudaram | `mais` (o "+N"), `bandeiras` (a cor de cada pronto, 0 a 7), `corrente` (o "…") |
| `escalada` | a escalada do aviso de espera mais velho | `sid8`, `nivel` (1 a 4; 0 no fim), `espera` (no começo), `motivo` (`aviso`, `vez`, `tempo`, `voltou`, `andou`, `visto`, `sessao_saiu`, `outro_aviso`) |
| `rajada` | a chamada (`alert`) de novo, na L2 e na L4 | `sid8`, `nivel` |
| `voo` | o voo até o alto-centro do monitor e de volta | `destino` (`alto_centro`), `motivo` (`escalada`, `voltou`), `sid8` |
| `pulso` | o selo do aviso pulsando a 1 Hz liga ou desliga (L4) | `ligado`, `sid8` |
| `discricao` | a tela compartilhada (2 s de sinal somados) liga; 5 min sem sinal desligam (decisão 0081) | `ligada`, `tirou_balao` |
| `clique` | o que um clique fez | `resultado` (`focou`, `nao_focou`, `lista`, `soneca`, `nada`), `sid8` |

As reações (`reacao`, `rajada`, a `reacao` de cada `festa`) vão para o
animador pelo caminho do M3, e os balões pelo balão do M4 (decisão 0079). O
resto é desenhado pela segunda metade do M5 (a seção abaixo); a `faixa`
"PRONTO!" e o `voo` `atravessar` do T3 ficam para o M6.

## A fotografia de agora

O `/v1/estado.fotografia` (o `tela` de lá é o da aprovação; decisão 0080):
`base` (o estado da skin), `prioridade`, `sid8` (a sessão que manda),
`sono` (`acordado`, `bocejou`, `dormindo`, `profundo`), `selos`, `escalada`
(`sid8`, `nivel`, `espera`, `pulso`), `festa` (`nivel`, `sessoes`, `ha_ms`,
nos 3 s dela) e `discricao`.

## A tela: o que cada intenção desenha

O desenho nunca muda as intenções (o teste que roda todos os cenários sem
personagem confere). Com o personagem na tela:

| Intenção | O que a janela mostra |
|---|---|
| `base` | o animador segura o estado da skin no ritmo dele (decisão 0082): `repouso` (parado, pronto, erro, espera até a L3: a pose e rajadas, até 2 commits/s), `quieto` (trabalhando, pensando: até 4 fps, uma micro-ação sorteada a cada 10–30 s), `laco` (dormindo, cansado: até 2 fps) e `parado` (`profundo`, e a espera na L4: só a pose, nenhum commit) |
| `selos` | a fileira ao lado do corpo, na altura da cabeça (decisão 0083): o "+N", o "…" e as bandeirinhas na cor do projeto, em blocos da metade do D, fora da área de toque e dentro do monitor |
| `escalada` | o «!» amarelo do aviso na fileira, da L1 até o aviso sair |
| `pulso` | o «!» trocando de cor (amarelo, vermelho) uma vez por segundo |
| `voo` (`alto_centro`) | o voo da casa ao alto-centro com o "!!" piscando abaixo de 2 Hz e de volta, ~4,2 s em passos de 34 ms, a área de toque junto, sem gravar posição (decisão 0084) |
| `festa` e `festa_mesclada` | a reação do nível (o `done_medium` e o `done_big` são voos dentro da célula), o balão, e o confete: a fonte de 12 no T2, a chuva de 40 pela tela no T3, em passos de 34 ms, até 4,5 s (decisão 0085) |
| `discricao` | os balões sem nome de projeto (o sinal do `screencast` somando 2 s; 5 min depois do último sinal, os nomes voltam; decisão 0081) |

O `/v1/estado.desenho` diz o que a janela está desenhando agora (só
metadados; vazio sem o pet na tela): `base` e `ritmo` (do animador),
`selos` (`aviso`: `normal` ou `aceso`, `pulso`, `mais`, `corrente` e quantas
`bandeiras`), `voo` (`fase`: `subindo`, `pairando`, `descendo`; `motivo`) e
`confete` (quantos pedaços na tela). É o que o Motor manda a janela
desenhar, pelo relógio: com a tela apagada o compositor não pede quadros e o
`commits_total` não anda (decisão 0018), mas o `desenho` sim; com a sessão
bloqueada, é por ele que se confere o desenho sem olhar a tela.

