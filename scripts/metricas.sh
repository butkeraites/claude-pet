#!/usr/bin/env bash
# scripts/metricas.sh — métricas públicas do repo no GitHub, sem telemetria.
#
# O Zeca não telefona para casa (privacidade por design), então "quantas
# pessoas instalaram" se estima pelos sinais do GitHub. O melhor proxy de
# instalação é o **download dos assets de release** (é o que o get.sh baixa);
# clones e views ajudam a ler o interesse, mas vêm inflados por CI e automação.
#
# Precisa do gh autenticado (`gh auth login`). Clones e views (tráfego) pedem
# acesso de escrita ao repo; sem ele, saem como indisponíveis.
#
# Uso: scripts/metricas.sh [dono/repo]   (padrão: o repo do diretório atual)
set -euo pipefail

REPO="${1:-}"
if [ -z "$REPO" ]; then
  REPO="$(gh repo view --json nameWithOwner -q .nameWithOwner 2>/dev/null || true)"
fi
if [ -z "$REPO" ]; then
  echo "não descobri o repo; passe dono/repo (ex.: butkeraites/claude-pet)" >&2
  exit 2
fi

if ! command -v gh >/dev/null 2>&1; then
  echo "o gh não está instalado; veja https://cli.github.com" >&2
  exit 2
fi

echo "== métricas de $REPO =="
echo "   ($(date '+%Y-%m-%d %H:%M'))"
echo

echo "— repositório"
gh api "repos/$REPO" \
  --jq '"   estrelas: \(.stargazers_count)   forks: \(.forks_count)   watchers: \(.subscribers_count)   issues abertas: \(.open_issues_count)"' \
  || echo "   (não consegui ler o repo)"
echo

echo "— downloads de release (o proxy de instalação; o get.sh baixa esses)"
if ! gh api "repos/$REPO/releases?per_page=100" \
  --jq '.[] | "   \(.tag_name) (\(.published_at[0:10])):", (.assets[]? | "      \(.name): \(.download_count)"), (if (.assets|length)==0 then "      (sem assets)" else empty end)'
then
  echo "   (não consegui ler as releases)"
fi
total="$(gh api "repos/$REPO/releases?per_page=100" --jq '[.[].assets[]?.download_count] | add // 0' 2>/dev/null || echo "?")"
echo "   TOTAL de downloads de assets: $total"
echo

echo "— tráfego dos últimos 14 dias (pede acesso de escrita; inflado por CI)"
clones="$(gh api "repos/$REPO/traffic/clones" --jq '"\(.count) total / \(.uniques) únicos"' 2>/dev/null || true)"
views="$(gh api "repos/$REPO/traffic/views" --jq '"\(.count) total / \(.uniques) únicos"' 2>/dev/null || true)"
echo "   clones: ${clones:-indisponível (sem acesso de escrita?)}"
echo "   views:  ${views:-indisponível (sem acesso de escrita?)}"
echo
echo "nota: clones/views incluem CI e automação — não são instalações de gente."
