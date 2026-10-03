# claude-pet

O **Zeca** é um papagaio de pixel art que mora por cima de tudo na sua tela
e reage ao [Claude Code](https://claude.com/claude-code) rodando no
terminal:

- **terminou** → aceno, pulinho, voo curto com confete ou, se o trabalho
  foi grande, um voo atravessando a tela com chuva de confete;
- **precisa de você** (pergunta, plano para aprovar, permissão) → pia com
  um "!" e um balão com o nome do projeto; se você não estiver olhando, a
  chamada cresce, sempre com teto;
- **trabalhando** → quase parado, bicando sementes de vez em quando;
- **ninguém mexendo** → boceja e dorme.

Pode ser arrastado com o mouse para qualquer lugar e sempre aparece no
monitor que está em foco. Sem som.

> **Estado:** em construção. O M1 (overlay nítido na tela, seguindo o
> orçamento de custo no Hyprland) está pronto na branch `m1-overlay`; o M2
> (o Zeca, com aprovação do personagem) está na branch `m2-zeca`; o M3
> (hooks → reação) está na branch `m3-hooks`, em cima da `m2-zeca`. Veja
> `PLANO.md` para os marcos e `PROGRESS.md` para o andamento.

## Requisitos

- Linux com **Hyprland** (testado no Omarchy, Hyprland 0.56) — a camada
  usa `wlr-layer-shell` e os eventos do Hyprland para seguir o monitor.
- **Docker** com Compose, e o serviço `docker` habilitado no boot
  (`sudo systemctl enable docker.service`).
- `curl` e `jq` no host (os hooks do Claude Code usam).

## Subir

```sh
git clone git@github.com:butkeraites/claude-pet.git ~/Documents/claude-pet
cd ~/Documents/claude-pet
cp .env.example .env            # opcional: porta, uid/gid
bin/pet subir                   # docker compose up -d --build
bin/pet estado
```

O container sobe no boot, espera o Hyprland e se reconecta sozinho depois
de logout, suspensão ou troca de monitor.

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
`aparencia.skin = "zeca-contorno"` em `config/claude-pet.toml` e aprove
`zeca-contorno`. O chapéu, a gravata e o encaixe são arte deste repositório
(`arte/zeca/`); detalhes em `docs/SKINS.md`.

### Hooks do Claude Code

Os hooks vêm no plugin `bichinho`, que mora neste repositório (`plugin/`,
com o marketplace local `bichinho-local` em `.claude-plugin/`): 13 hooks
async que chamam `plugin/scripts/avisar.sh`. Ele manda **só metadados**
para `127.0.0.1:27380` — nome do evento, ids opacos, nome da ferramenta,
contagens e durações, um hash do caminho do arquivo editado e o nome da
pasta do projeto —, nunca o texto dos prompts, código, respostas ou
caminhos. Não imprime nada, sempre sai 0 e não atrasa o Claude: com o pet
desligado, desiste na hora. Precisa de `jq` e `curl` no host.

**Instalação, depois do merge na `main`.** O marketplace aponta para uma
worktree estável, destacada na `main`, para uma branch em andamento nunca
chegar às sessões de outros projetos:

```sh
git -C ~/Documents/claude-pet worktree add --detach ~/.local/share/claude-pet/estavel main
claude plugin validate ~/.local/share/claude-pet/estavel --strict
claude plugin marketplace add ~/.local/share/claude-pet/estavel
claude plugin install bichinho@bichinho-local
```

Depois de cada merge, atualize a worktree
(`git -C ~/.local/share/claude-pet/estavel checkout --detach main`) e rode
`/reload-plugins` nas sessões abertas.

**Até o merge**, ou para testar uma mudança, carregue o plugin só numa
sessão:

```sh
claude --plugin-dir ~/Documents/claude-pet/plugin
```

Para conferir sem o Claude: `bin/pet testar rapido` (aceno) e
`bin/pet testar pequeno` (pulinho) mandam eventos sintéticos pelo mesmo
`avisar.sh`. Só sessões de terminal contam (`sessoes.origens = ["cli"]` em
`config/exemplo.toml`): `claude -p`, SDK e IDE ficam de fora.

Com o Zeca aprovado, uma resposta sem trabalho (sem editar arquivo, rodar
comando nem chamar subagente) ganha o aceno, ele levantando e sentando; uma
resposta com trabalho ganha o pulinho, um pio; e fechar o Claude, um pio de
tchau. Sem personagem aprovado, as reações ficam só em `bin/pet estado`
(`ultima_reacao`).

## Desenvolvimento

```sh
bin/pet verificar               # fmt, clippy, testes, compose, plugin
docker compose -f docker-compose.yml -f docker-compose.dev.yml up --build
```

Documentação para quem mexe no código: `CLAUDE.md`, `DECISIONS.md` e
`docs/`.

## Licença

Código sob MIT (`LICENSE`). Arte de terceiros e créditos em `NOTICE.md`.
