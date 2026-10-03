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

> **Estado:** em construção. O M1 (overlay) está na branch `m1-overlay`,
> com o portão aberto: falta medir nitidez e custo com a tela acesa. Veja
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
`bin/pet skin-instalar <arquivo.zip>` (a partir do M2).

### Hooks do Claude Code

Os hooks vêm no plugin `bichinho`, que mora neste repositório (a partir do
M3). Eles mandam **só metadados** (nome do evento, ferramenta, nome da pasta
do projeto) para `127.0.0.1:27380` — nunca o texto dos prompts, código ou
respostas — e nunca atrasam o Claude.

## Desenvolvimento

```sh
bin/pet verificar               # fmt, clippy, testes, compose, plugin
docker compose -f docker-compose.yml -f docker-compose.dev.yml up --build
```

Documentação para quem mexe no código: `CLAUDE.md`, `DECISIONS.md` e
`docs/`.

## Licença

Código sob MIT (`LICENSE`). Arte de terceiros e créditos em `NOTICE.md`.
