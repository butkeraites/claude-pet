# bichinho

O **bichinho** põe o **Zeca**, um papagaio de pixel art, por cima de tudo na
sua tela, reagindo ao [Claude Code](https://claude.com/claude-code) rodando
no terminal:

- **terminou** → aceno, pulinho, voo curto com confete ou, se o trabalho
  foi grande, um voo atravessando a tela com chuva de confete;
- **precisa de você** (pergunta, plano para aprovar, permissão) → pia com
  um "!" e um balão com o nome do projeto; se você não estiver olhando, a
  chamada cresce, sempre com teto;
- **trabalhando** → quase parado, bicando sementes de vez em quando;
- **ninguém mexendo** → boceja e dorme.

Pode ser arrastado com o mouse para qualquer lugar e sempre aparece no
monitor que está em foco. Sem som.

> **Estado:** em construção. Na `main` (`v0.4.1`): o overlay nítido no
> Hyprland (M1), o Zeca com aprovação do personagem (M2), hooks → reação
> (M3), a costura para Windows e macOS com o hook nativo (parte do M8),
> arrastar, seguir o monitor ativo e o clique que leva ao terminal (M4) e o
> Zeca original, arte livre em CC0. Na branch `m5-cerebro`: o M5 inteiro,
> o cérebro completo (as festas pelo trabalho, as correntes de agentes, os
> avisos com a escalada, a tela com selos, sono e discrição), decidido num
> registro de intenções com cenários dourados, e a tela que desenha isso (o
> Zeca em cada estado, os selos das outras sessões, o voo da escalada, o
> confete das festas grandes, a discrição ao compartilhar a tela). O destino
> é um lançamento open source para Linux, macOS e Windows. O repositório de
> desenvolvimento ainda se chama `claude-pet`. Veja `PLANO.md` para os marcos
> e `PROGRESS.md` para o andamento.

## Requisitos

- Linux com **Hyprland** (testado no Omarchy, Hyprland 0.56) — a camada
  usa `wlr-layer-shell`, os eventos do Hyprland para seguir o monitor e o
  `zwlr_foreign_toplevel_manager_v1` com o
  `hyprland_toplevel_mapping_manager_v1` para o clique levar ao terminal
  (`cargo xtask globais` confere se o compositor oferece os dois).
- **Docker** com Compose, e o serviço `docker` habilitado no boot
  (`sudo systemctl enable docker.service`).
- `~/.local/bin` no PATH que o Claude Code vê (o hook é o binário
  `bichinho`, copiado da imagem por `bin/pet instalar-host`).
- Claude Code com hooks em exec form (`command` + `args`; testado no
  2.1.288). Um Claude Code que ignorasse o `args` chamaria só `bichinho`, que
  sem subcomando não faz nada: o pet fica surdo, mas nada mais roda.
- `curl` e `jq` no host para o `bin/pet` (e para o `avisar.sh`, o hook de
  reserva até a troca).
- Para refazer as skins no host (`bin/pet skin-livre`, `bin/pet
  skin-instalar`) e para o `bin/pet verificar`: o Rust do `rustup` (o
  `~/.cargo/bin`, que o `bin/pet` acrescenta ao PATH) e, para o Zeca original,
  `python3` (só a biblioteca padrão). Para só usar as skins que já estão no
  repositório, nenhum dos dois: o `bin/pet subir` compila o pet dentro do
  Docker e leva as skins de `skins/` para a imagem.

## Subir

```sh
git clone git@github.com:butkeraites/claude-pet.git ~/Documents/claude-pet
cd ~/Documents/claude-pet
cp .env.example .env            # opcional: porta, uid/gid
bin/pet subir                   # docker compose up -d --build
bin/pet instalar-host           # o binário do hook em ~/.local/bin/bichinho
bin/pet estado
```

O container sobe no boot, espera o Hyprland e se reconecta sozinho depois
de logout, suspensão ou troca de monitor.

Para mudar alguma coisa, copie `config/exemplo.toml` para
`config/bichinho.toml` (fora do git). Por exemplo, o Zeca menor:

```toml
[aparencia]
tamanho = "pequeno"   # pequeno (~10% da altura do monitor), normal (~12%) ou grande (~16%)
```

O tamanho muda só a escala do desenho, nunca a skin: não pede outra
aprovação. Vale quando o pet reinicia (`bin/pet parar && bin/pet subir`; um
`bin/pet subir` sozinho não recria o container quando só o config muda) ou na
próxima aprovação, que relê o config.

### Arte

O Zeca tem duas artes, e as duas podem ficar instaladas:

- **O Zeca original:** arte original feita com o Claude para o projeto
  bichinho, em domínio público (**CC0 1.0**, `arte/zeca-livre/LICENSE`).
  Mora no repositório (o gerador em `arte/zeca-livre/`, as skins em
  `skins/`), em duas skins: `zeca-livre-escuro`, com um anel de 1 px por
  fora, para tema escuro (sem ele o contorno some no fundo escuro), e
  `zeca-livre`, para tema claro. Não precisa de pack nenhum.
- **O Zeca do pack:** feito a partir do pack *Cute Parrots!* da
  [exclusiveOlive](https://exclusiveolive.itch.io/cute-parrots-pixel-art-asset-pack),
  com chapéu, gravata e encaixe deste repositório (`arte/zeca/`). A licença
  do pack não permite redistribuir os arquivos, então **eles não estão neste
  repositório**: compre/baixe o pack e rode `bin/pet skin-instalar
  <arquivo.zip>` (gera `zeca` e `zeca-contorno`, com contorno creme, e as
  prévias em `tmp/previa-zeca-m2/`).

Quem aparece é o `aparencia.skin` do `config/bichinho.toml` (sem a chave, o
`zeca`), e cada skin só aparece depois de você aprovar a folha de contato
dela: a aprovação vale para a skin exata que você viu, e sem ela o Zeca fica
escondido. Para o Zeca original:

```sh
bin/pet skin-livre                      # refaz as duas skins e põe as prévias em tmp/previa-zeca-livre/
bin/pet subir                           # as skins entram na imagem local
# olhe tmp/previa-zeca-livre/contato-zeca-livre-escuro.png e os GIFs, e então,
# em config/bichinho.toml, na seção [aparencia]: skin = "zeca-livre-escuro"
bin/pet skin-aprovar zeca-livre-escuro  # relê o config e troca na hora
```

**Trocar de Zeca** (as aprovações ficam guardadas por skin; voltar a uma já
aprovada não pede aprovação nova):

| Para | Em `config/bichinho.toml` | Depois |
|---|---|---|
| o original, tema escuro | `skin = "zeca-livre-escuro"` | `bin/pet skin-aprovar zeca-livre-escuro` na primeira vez; já aprovada, `bin/pet parar && bin/pet subir` |
| o original, tema claro | `skin = "zeca-livre"` | `bin/pet skin-aprovar zeca-livre` na primeira vez; já aprovada, `bin/pet parar && bin/pet subir` |
| o do pack | `skin = "zeca"` (ou tire a linha) | `bin/pet skin-aprovar zeca` na primeira vez; já aprovado, `bin/pet parar && bin/pet subir` |

O `bin/pet subir` sozinho não recria o container quando só o config muda: por
isso o `parar` antes. Com o Zeca original, o `pequeno` e o `normal` dão o
mesmo tamanho no eDP-1 (o desenho tem mais pixels; o D é sempre inteiro).
Detalhes, estados e o gerador em `docs/SKINS.md`.

### Hooks do Claude Code

Os hooks vêm no plugin `bichinho`, que mora neste repositório (`plugin/`,
com o marketplace local `bichinho-local` em `.claude-plugin/`): 13 hooks
async em exec form que chamam o próprio binário, `bichinho avisar <Evento>`.
Ele lê o JSON do hook e manda **só metadados** para `127.0.0.1:27380` —
nome do evento, ids opacos, nome da ferramenta, contagens e durações, um
hash do caminho do arquivo editado e o nome da pasta do projeto —, nunca o
texto dos prompts, código, respostas ou caminhos. E só para lá: fala TCP
direto com o 127.0.0.1, sem proxy nem curlrc. Não imprime nada, sempre sai 0
e não atrasa o Claude: com o pet desligado, desiste na hora. Lê o JSON em
fluxo e guarda só os campos da lista branca: o resto (prompt, resposta,
saída das ferramentas) só passa, sem ser interpretado nem ficar na memória.

**O binário no PATH.** O exec form acha o `bichinho` pelo PATH do Claude
Code. `bin/pet instalar-host` copia o binário estático (musl) da imagem para
`~/.local/bin/bichinho`, que precisa estar nesse PATH (no Omarchy, está).
Confira com `command -v bichinho` e `bichinho versao`, que diz o commit de
onde o binário saiu. Sem o binário, os hooks não chegam ao pet (o `claude -p`
continua calado); o `plugin/scripts/avisar.sh` (sh + jq + curl) continua no
plugin como reserva até a troca.

Esse binário é a lista branca de **todas** as sessões do Claude Code da
máquina. Por isso o `bin/pet instalar-host` só aceita a imagem do commit da
worktree estável (numa máquina sem ela, o da `main`) e de árvore limpa: o
`bin/pet subir` grava na imagem o commit de onde ela saiu. Uma branch nunca
vai para o `~/.local/bin`; para testar uma, veja abaixo.

**Instalação**, a partir da `main`. O marketplace aponta para uma worktree
estável, destacada na `main`, para uma branch em andamento nunca chegar às
sessões de outros projetos:

```sh
git -C ~/Documents/claude-pet worktree add --detach ~/.local/share/claude-pet/estavel main
claude plugin validate ~/.local/share/claude-pet/estavel --strict
claude plugin marketplace add ~/.local/share/claude-pet/estavel
claude plugin install bichinho@bichinho-local
claude plugin list                    # bichinho@bichinho-local habilitado
```

**Depois de cada merge** (e, na primeira vez, a troca do `avisar.sh` pelo
hook nativo, plugin 0.2.0), com o clone na `main`:

```sh
git -C ~/Documents/claude-pet switch main && git -C ~/Documents/claude-pet pull
git -C ~/.local/share/claude-pet/estavel checkout --detach main
bin/pet subir                         # a imagem nova, com o commit da main (e o nome bichinho no compose)
bin/pet instalar-host                 # o binário do hook, ANTES de atualizar o plugin
bichinho versao                       # o commit tem de ser o da worktree estável
claude plugin marketplace update bichinho-local
claude plugin update bichinho@bichinho-local
```

e `/reload-plugins` nas sessões abertas (ou abra outra). A ordem importa: o
plugin 0.2.0 chama o `bichinho` do PATH, e o `instalar-host` recusa uma
imagem que não seja do commit da worktree estável. Até atualizar, o plugin
instalado continua no `avisar.sh` da cópia dele, que fala com o mesmo pet:
nada fica surdo no meio da troca.

No M4 o plugin não muda (continua 0.2.0): o `bin/pet subir` e o
`bin/pet instalar-host` bastam, rodados no clone (nunca na worktree
estável, que não tem o seu `config/bichinho.toml`: o Zeca voltaria ao
tamanho normal; decisão 0064). O binário novo do hook passa a mandar os ids
de terminal (`term`, decisão 0054); o antigo continua funcionando com o pet
novo, sem eles.

No M5 também não muda o plugin, e a troca é a mesma (`bin/pet subir` e
`bin/pet instalar-host`, no clone). O binário novo do hook passa a mandar a
forma do prompt (`orig`: `notificacao` quando o Claude Code acorda a sessão
com o aviso de uma tarefa em segundo plano, `comum` para o resto; o texto
nunca sai do hook) e quantos agendamentos o Stop lista (`crn`; decisão
0072). Com o antigo, o pet novo funciona sem eles: a notificação de uma
tarefa vira continuação quando há uma corrente de agentes aberta (e um
prompt que você digita com ela aberta também, sem festa própria), a
notificação de um shell em segundo plano (sem corrente) conta como prompt
digitado (a festa normal no lugar do pulinho discreto, e a janela da sessão
casada com a que estiver em foco), e o tique de um `/loop` não é visto
(festeja como um prompt digitado). A tela do M5 (a base, os selos, o voo, o
confete, o compartilhamento de tela pelo socket2) só muda o pet: o
`bin/pet subir` basta para ela.

**Para testar uma mudança**, carregue o plugin e o binário da branch só
numa sessão (o `~/.local/bin` continua com o da worktree estável):

```sh
cd ~/Documents/claude-pet
~/.cargo/bin/cargo build -p bichinho            # target/debug/bichinho, desta branch
PATH="$PWD/target/debug:$PATH" claude --plugin-dir ~/Documents/claude-pet/plugin
```

Para conferir sem o Claude: `bin/pet testar rapido` (aceno),
`bin/pet testar pequeno` (pulinho), `medio`, `grande`, `pergunta` e
`dois-prontos` (a tela do M5, logo abaixo) mandam eventos sintéticos pelo mesmo hook
(o `bichinho avisar` do PATH, ou o de `PET_BICHINHO`, por exemplo
`PET_BICHINHO="$PWD/target/debug/bichinho"`; sem nenhum, o `avisar.sh`, com
aviso), dizem de que commit é o binário, e
`bin/pet tocar nod` toca uma reação e diz se ela apareceu na tela. Só
sessões de terminal contam (`sessoes.origens = ["cli"]` em
`config/exemplo.toml`): `claude -p`, SDK e IDE ficam de fora.

Com o Zeca aprovado, cada resposta ganha a festa do tamanho do trabalho
(veja "O que o Zeca mostra", abaixo) e fechar o Claude, o tchau. No Zeca
original o aceno é a tirada de chapéu, o pulinho é o pulo comemorando e o
tchau é com a asa; no do pack, levantar e sentar, um pio e um pio de tchau.
Sem personagem aprovado, as reações ficam só em `bin/pet estado`
(`ultima_reacao` e `turnos`).

### Arrastar, seguir e clicar

- **Arrastar:** segure o Zeca e leve-o para onde quiser (4 pixels ou um
  quarto de segundo segurando já é arraste). Solto noutro monitor, ele fica
  lá (numa área de trabalho vazia o Hyprland pode largar o arraste na borda;
  aí ele pousa ali e segue o foco para o outro monitor). A posição fica guardada por monitor (pela descrição dele, que não muda
  quando o dock troca o nome da porta) e volta depois de reiniciar.
- **Seguir o monitor ativo:** quando o foco muda de monitor, ele some com
  um "poof" e reaparece no outro, na posição guardada daquele monitor.
- **Clique esquerdo:** com algo pendente — o Claude esperando você (uma
  permissão, uma pergunta, um plano), um erro da API ou uma resposta pronta
  —, ele dá uma risadinha com um coração e leva você à janela do terminal
  daquela sessão, do mais urgente para o menos urgente; cada clique passa
  para o próximo. Sem nada pendente, um balão com as sessões abertas (a
  pasta do projeto, o estado e há quanto tempo). Quando não dá para levar
  (a sessão começou antes de o pet subir, a janela fechou), o balão diz por
  quê. Uma resposta pronta também sai sozinha depois de uns 10 s com o
  terminal dela em foco e você mexendo no teclado ou no mouse (longe, com a
  sessão bloqueada ou a tela apagada, ela fica até o clique ou o próximo
  prompt). Cliques seguidos passam de um aviso ao próximo; um clique mais
  de 15 s depois do anterior volta ao mais urgente.
- **Clique direito:** soneca de 30 minutos, com um "zZ" e só reações
  pequenas; outro clique direito acorda.
- **Proteção de tela** do Omarchy: o Zeca se esconde e volta quando ela
  fecha.

`bin/pet clique` (ou `bin/pet clique direito`) clica no pet como o mouse e
diz o que ele fez.

**Como ele acha o terminal**, sem ler títulos de janela: o pet guarda as
últimas trocas de janela ativa que o Hyprland conta (só o endereço da janela
e a hora) e casa cada prompt com a janela que estava ativa quando ele saiu;
o foco vai pelo protocolo Wayland de gerenciar janelas, na mesma conexão da
camada do pet. O pet nunca abre o socket de comandos do Hyprland nem roda
`hyprctl`, e nenhum título de janela vai para o log, o `/v1/estado` ou o
disco. Duas sessões no mesmo terminal (painéis do tmux, abas do kitty ou do
WezTerm) caem na mesma janela; o hook manda os ids de terminal que vê no
ambiente para separá-las no futuro.

**Regra opcional do Hyprland** (proposta, não aplicada: só pela skill
`omarchy` e com o seu consentimento; detalhes em
`docs/pesquisa/03-hyprland.md`):

```lua
hl.layer_rule({ name = "bichinho", match = { namespace = "^bichinho$" }, order = 1, no_anim = true })
```

`order = 1` deixa o Zeca abaixo dos popups do Omarchy (polkit, menus,
toasts), e `no_anim` tira o fade de ~180 ms em cada troca de monitor. Sem
ela, tudo funciona.

**Conferir na tela:** `scripts/verificar-m4.sh` (com a sessão desbloqueada:
abre dois `foot`, casa uma sessão de teste com cada um e confere que o
clique leva a cada um) e `scripts/verificar-m4.sh --manual` (arrastar, a
posição depois de reiniciar, a proteção de tela, o clique com o mouse,
também levando a um `foot` que você mandou para outra área de trabalho, e o
checklist do HDMI, da tampa fechada e da suspensão).
`scripts/e2e-monitor.sh --autorizo` cria um monitor de mentira para conferir
a troca de monitor: mexe no Hyprland, então só com o seu consentimento (ele
pede que você digite «sim» no terminal).

### O que o Zeca mostra (M5)

- **O estado de agora**, da sessão que mais precisa de você (na ordem:
  esperando você, erro, cansado, pronto, trabalhando, compactando, pensando,
  parado): trabalhando e pensando ele fica quase parado (uma micro-ação a
  cada 10–30 s, sorteada); parado, respira e pisca; sem nada por uns
  minutos, boceja e dorme, e no sono profundo não desenha nada.
- **A festa** é do tamanho do trabalho: a resposta sem trabalho ganha o
  aceno; trabalho pequeno, o pulinho e o balão "Prontinho! ‹projeto›";
  médio, o voo curto com 12 confetes; grande, o voo grande com uma chuva de
  confete pela tela (no máximo um a cada 10 min; um que não apareceu, com a
  proteção de tela ou na soneca, não conta). Duas sessões terminando juntas
  viram uma festa só ("2 prontos: api, web"). Um pedido com agentes em
  segundo plano (um workflow inclusive) festeja uma vez só, quando o último
  agente volta, pela soma; um pedido que você digita enquanto eles trabalham
  festeja sozinho, pelo trabalho dele.
- **Quando o Claude precisa de você**, ele chama na hora (o pio, um «!»
  amarelo ao lado e o balão do tipo: pergunta, plano, permissão). Se você
  não está olhando o terminal daquela sessão (ou está sem mexer há 1 min), a
  chamada cresce: rajadas aos 30 s, um voo até o alto do monitor com "!!" aos
  90 s (até 3 vezes, e de volta para o lugar de sempre), e aos 5 min o «!»
  troca de cor uma vez por segundo, com uma rajada por minuto, até parar de
  vez aos 35 min: aí a pose de espera sai, o «!» fica parado e ele pode
  dormir. Quando você volta depois de 1 min longe, um voo mostra o aviso
  (assim que ele aparece na tela: com a proteção de tela ou a sessão
  bloqueada, depois que ela some). Com 5 s no terminal da sessão, ele dá o
  diálogo por visto e não escala mais (um Esc numa pergunta ou um plano
  recusado não mandam evento nenhum, e a pose de espera sai 2 min depois);
  o clique que leva ao terminal também a tira. Com o "não perturbe" do
  Omarchy ou na soneca, só a primeira chamada. O "não perturbe" vem com os
  eventos do Claude: ligado no meio de uma espera, vale no próximo evento de
  qualquer sessão; para calar na hora, o clique direito (a soneca).
- **As outras sessões** viram selos ao lado dele: "+N" (as ocupadas), uma
  bandeirinha na cor do projeto para cada resposta pronta e "…" quando um
  agente trabalha em segundo plano.
- **Compartilhando a tela**, os balões perdem o nome do projeto (depois de 2
  s de compartilhamento, e até 5 min depois que ele acaba: no Hyprland 0.56 o
  sinal pisca quando a tela fica parada), e ele fica discreto como no "não
  perturbe": só a primeira chamada e nenhum voo.

`bin/pet testar medio`, `grande`, `pergunta` e `dois-prontos` mostram cada
uma dessas coisas com sessões de teste (que somem em 60 s), e o
`bin/pet estado` diz o que a tela está desenhando (`desenho`), mesmo com a
sessão bloqueada.

## Desenvolvimento

```sh
bin/pet verificar               # fmt, clippy, testes, compose, plugin
docker compose -f docker-compose.yml -f docker-compose.dev.yml up --build
bin/pet simular pergunta        # um cenário no cérebro, num relógio falso: as intenções
```

O cérebro decide num registro de intenções, e os cenários de `cenarios/`
(com o dourado de cada um) são o teste e a documentação do que ele faz: o
formato, as intenções e como gravar um cenário de verdade com pseudônimos
(`bin/pet eventos --salvar`, só de um pet de debug) estão em
`docs/CENARIOS.md`.

Documentação para quem mexe no código: `CLAUDE.md`, `DECISIONS.md` e
`docs/`.

## Licença

Código sob MIT (`LICENSE`), exceto a arte do Zeca original: `arte/zeca-livre/`
(inclusive o gerador `zeca.py`, que é a fonte da arte) e as skins
`skins/zeca-livre/` e `skins/zeca-livre-escuro/` são dedicadas ao domínio
público pela **CC0 1.0** (`arte/zeca-livre/LICENSE`; crédito de cortesia, não
obrigatório: "arte original feita com o Claude para o projeto bichinho"). Arte
de terceiros e créditos em `NOTICE.md`; os arquivos do pack *Cute Parrots!*
nunca estão no repositório.
