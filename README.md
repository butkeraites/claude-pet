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
> o Zeca com aprovação do personagem (M2) e hooks → reação (M3). Em
> andamento: a costura para Windows e macOS e o hook nativo (M8, antes do
> M4); o destino é um lançamento open source para Linux, macOS e Windows.
> O repositório de desenvolvimento ainda se chama `claude-pet`. Veja
> `PLANO.md` para os marcos e `PROGRESS.md` para o andamento.

## Requisitos

- Linux com **Hyprland** (testado no Omarchy, Hyprland 0.56) — a camada
  usa `wlr-layer-shell` e os eventos do Hyprland para seguir o monitor.
- **Docker** com Compose, e o serviço `docker` habilitado no boot
  (`sudo systemctl enable docker.service`).
- `~/.local/bin` no PATH que o Claude Code vê (o hook é o binário
  `bichinho`, copiado da imagem por `bin/pet instalar-host`).
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
e não atrasa o Claude: com o pet desligado, desiste na hora.

**O binário no PATH.** O exec form acha o `bichinho` pelo PATH do Claude
Code. `bin/pet instalar-host` copia o binário estático (musl) da imagem para
`~/.local/bin/bichinho`, que precisa estar nesse PATH (no Omarchy, está).
Confira com `command -v bichinho`. Sem o binário, os hooks não chegam ao pet
(o `claude -p` continua calado); o `plugin/scripts/avisar.sh` (sh + jq +
curl) continua no plugin como reserva até a troca.

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
bin/pet subir                         # a imagem nova (e o nome bichinho no compose)
bin/pet instalar-host                 # o binário do hook, ANTES de atualizar o plugin
claude plugin marketplace update bichinho-local
claude plugin update bichinho@bichinho-local
```

e `/reload-plugins` nas sessões abertas (ou abra outra). A ordem importa: o
plugin 0.2.0 chama o `bichinho` do PATH. Até atualizar, o plugin instalado
continua no `avisar.sh` da cópia dele, que fala com o mesmo pet: nada fica
surdo no meio da troca.

**Para testar uma mudança**, carregue o plugin só numa sessão:

```sh
claude --plugin-dir ~/Documents/claude-pet/plugin
```

Para conferir sem o Claude: `bin/pet testar rapido` (aceno) e
`bin/pet testar pequeno` (pulinho) mandam eventos sintéticos pelo mesmo hook
(o `bichinho avisar` do PATH; sem ele, o `avisar.sh`, com aviso), e
`bin/pet tocar nod` toca uma reação e diz se ela apareceu na tela. Só
sessões de terminal contam (`sessoes.origens = ["cli"]` em
`config/exemplo.toml`): `claude -p`, SDK e IDE ficam de fora.

Com o Zeca aprovado, uma resposta sem trabalho (sem editar arquivo, rodar
comando nem chamar subagente) ganha o aceno, ele levantando e sentando; uma
resposta com trabalho ganha o pulinho, um pio; e fechar o Claude, um pio de
tchau. Sem personagem aprovado, as reações ficam só em `bin/pet estado`
(`ultima_reacao` e `turnos`).

## Desenvolvimento

```sh
bin/pet verificar               # fmt, clippy, testes, compose, plugin
docker compose -f docker-compose.yml -f docker-compose.dev.yml up --build
```

Documentação para quem mexe no código: `CLAUDE.md`, `DECISIONS.md` e
`docs/`.

## Licença

Código sob MIT (`LICENSE`). Arte de terceiros e créditos em `NOTICE.md`.
