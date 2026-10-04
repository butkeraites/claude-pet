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

> **Estado:** em construção. Na `main`: o overlay nítido no Hyprland (M1),
> o Zeca com aprovação do personagem (M2), hooks → reação (M3) e a costura
> para Windows e macOS com o hook nativo (parte do M8). Na branch
> `m4-arrastar-seguir`: arrastar, seguir o monitor ativo e o clique que leva
> ao terminal (M4), com a conferência na tela ainda pendente. O destino é um
> lançamento open source para Linux, macOS e Windows. O repositório de
> desenvolvimento ainda se chama `claude-pet`. Veja `PLANO.md` para os
> marcos e `PROGRESS.md` para o andamento.

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

O Zeca é feito a partir do pack *Cute Parrots!* da
[exclusiveOlive](https://exclusiveolive.itch.io/cute-parrots-pixel-art-asset-pack).
A licença do pack não permite redistribuir os arquivos, então **eles não
estão neste repositório**: compre/baixe o pack e rode

```sh
bin/pet skin-instalar <arquivo.zip>   # gera o Zeca e as prévias em tmp/previa-zeca-m2/
bin/pet subir                         # a skin entra na imagem local
bin/pet skin-aprovar zeca             # depois de ver a folha de contato
```

Sem aprovação o Zeca fica escondido, e a aprovação só vale para a skin da
folha de contato que você viu. Para o Zeca com contorno creme, ponha
`aparencia.skin = "zeca-contorno"` em `config/bichinho.toml` e aprove
`zeca-contorno`. O chapéu, a gravata e o encaixe são arte deste repositório
(`arte/zeca/`); detalhes em `docs/SKINS.md`.

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
`bin/pet instalar-host` bastam. O binário novo do hook passa a mandar os ids
de terminal (`term`, decisão 0054); o antigo continua funcionando com o pet
novo, sem eles.

**Para testar uma mudança**, carregue o plugin e o binário da branch só
numa sessão (o `~/.local/bin` continua com o da worktree estável):

```sh
cd ~/Documents/claude-pet
~/.cargo/bin/cargo build -p bichinho            # target/debug/bichinho, desta branch
PATH="$PWD/target/debug:$PATH" claude --plugin-dir ~/Documents/claude-pet/plugin
```

Para conferir sem o Claude: `bin/pet testar rapido` (aceno) e
`bin/pet testar pequeno` (pulinho) mandam eventos sintéticos pelo mesmo hook
(o `bichinho avisar` do PATH, ou o de `PET_BICHINHO`, por exemplo
`PET_BICHINHO="$PWD/target/debug/bichinho"`; sem nenhum, o `avisar.sh`, com
aviso), dizem de que commit é o binário, e
`bin/pet tocar nod` toca uma reação e diz se ela apareceu na tela. Só
sessões de terminal contam (`sessoes.origens = ["cli"]` em
`config/exemplo.toml`): `claude -p`, SDK e IDE ficam de fora.

Com o Zeca aprovado, uma resposta sem trabalho (sem editar arquivo, rodar
comando nem chamar subagente) ganha o aceno, ele levantando e sentando; uma
resposta com trabalho ganha o pulinho, um pio; e fechar o Claude, um pio de
tchau. Sem personagem aprovado, as reações ficam só em `bin/pet estado`
(`ultima_reacao` e `turnos`).

### Arrastar, seguir e clicar

- **Arrastar:** segure o Zeca e leve-o para onde quiser (4 pixels ou um
  quarto de segundo segurando já é arraste). Solto noutro monitor, ele fica
  lá. A posição fica guardada por monitor (pela descrição dele, que não muda
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
  terminal dela em foco.
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
posição depois de reiniciar, a proteção de tela e o clique com o mouse).
`scripts/e2e-monitor.sh --autorizo` cria um monitor de mentira para conferir
a troca de monitor: mexe no Hyprland, então só com o seu consentimento.

## Desenvolvimento

```sh
bin/pet verificar               # fmt, clippy, testes, compose, plugin
docker compose -f docker-compose.yml -f docker-compose.dev.yml up --build
```

Documentação para quem mexe no código: `CLAUDE.md`, `DECISIONS.md` e
`docs/`.

## Licença

Código sob MIT (`LICENSE`). Arte de terceiros e créditos em `NOTICE.md`.
