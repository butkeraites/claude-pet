#!/usr/bin/env bash
# scripts/e2e-monitor.sh --autorizo — o pet segue o monitor ativo, num
# monitor de mentira (PLANO.md, "M4", verificação): cria um output headless,
# foca nele e confere o Zeca lá; volta ao monitor de antes e confere de novo;
# remove o output e confere que o pet ficou bem.
#
# MEXE NO HYPRLAND DO RENAN: cria e remove um monitor e troca o monitor em
# foco (`hyprctl output create|remove` e `hyprctl dispatch focusmonitor`).
# Por isso só roda com `--autorizo`, e só com o consentimento dele A CADA
# VEZ. Um `trap` sempre restaura: remove o output criado e devolve o foco ao
# monitor de antes, até numa falha ou num Ctrl+C.
#
# O daemon continua sem o socket de comandos (decisão 0006): quem fala com
# ele aqui é este script do host, com o consentimento do Renan.
#
# Pegadinha: `hyprctl output create headless` não aceita nome; o nome novo sai
# do diff de `hyprctl -j monitors` antes e depois.
set -uo pipefail

RAIZ="$(cd "$(dirname "$(readlink -f "${BASH_SOURCE[0]}")")/.." && pwd)"
cd "$RAIZ" || exit 1

if [ "${1:-}" != --autorizo ]; then
  cat >&2 <<'TEXTO'
uso: scripts/e2e-monitor.sh --autorizo

Cria e remove um monitor headless no Hyprland e troca o foco entre monitores.
Só com o consentimento do Renan, a cada vez.
TEXTO
  exit 2
fi

URL="http://127.0.0.1:${PET_PORTA:-27380}"
# Debounce de 300 ms, 1,5 s entre viagens e os dois poofs: 2,5 s de folga.
PRAZO_S=4

declare -a LINHAS=()
FALHAS=0
passou() { LINHAS+=("✓ $1"); printf '✓ %s\n' "$1"; }
falhou() { LINHAS+=("✗ $1"); printf '✗ %s\n' "$1" >&2; FALHAS=$((FALHAS + 1)); }

api() { curl -q --noproxy '*' -fsS -m 3 -H 'X-Pet: 1' "${URL}$1"; }
campo() { api /v1/estado 2>/dev/null | jq -r "$1" 2>/dev/null; }
campo_e() { [ "$(campo "$1")" = "$2" ]; }
agora_ms() { echo $(($(date +%s%N) / 1000000)); }
esperar() {
  local limite=$(($(agora_ms) + $1 * 1000))
  shift
  while [ "$(agora_ms)" -lt "$limite" ]; do
    "$@" && return 0
    sleep 0.1
  done
  return 1
}

nomes() { hyprctl -j monitors all | jq -r '.[].name' | sort; }
focado() { hyprctl -j monitors | jq -r '.[] | select(.focused) | .name' | head -n1; }
# A camada do pet viva (pid > 0) no monitor.
camada_em() {
  [ "$(hyprctl -j layers | jq --arg m "$1" \
    '[.[$m].levels["3"][]? | select(.namespace == "bichinho" and .pid > 0)] | length')" = 1 ]
}
pet_em() { campo_e .monitor "$1" && campo_e .visivel true && camada_em "$1"; }

if ! api /v1/estado >/dev/null; then
  echo "o pet não respondeu em ${URL} — está de pé? (bin/pet subir)" >&2
  exit 1
fi
if hyprctl -j monitors | jq -e 'any(.[]; (.solitaryBlockedBy // []) | index("LOCK") != null)' >/dev/null; then
  echo "sessão bloqueada: o Hyprland não desenha o pet; desbloqueie e rode de novo" >&2
  exit 1
fi
if [ "$(campo .tela)" != ativa ]; then
  echo "o pet não está na tela (tela=$(campo .tela)); aprove o personagem ou use a pilha de dev" >&2
  exit 1
fi

ANTES_FOCO="$(focado)"
ANTES="$(nomes)"
NOVO=""
restaurar() {
  if [ -n "$NOVO" ]; then
    hyprctl output remove "$NOVO" >/dev/null 2>&1 || echo "✗ NÃO consegui remover o output $NOVO: hyprctl output remove $NOVO" >&2
    NOVO=""
  fi
  [ -n "$ANTES_FOCO" ] && hyprctl dispatch focusmonitor "$ANTES_FOCO" >/dev/null 2>&1
}
trap restaurar EXIT INT TERM

echo "▸ criando um output headless"
hyprctl output create headless >/dev/null
for _ in $(seq 1 30); do
  NOVO="$(comm -13 <(printf '%s\n' "$ANTES") <(nomes) | head -n1)"
  [ -n "$NOVO" ] && break
  sleep 0.1
done
if [ -z "$NOVO" ]; then
  falhou "o output headless não apareceu em hyprctl -j monitors"
  exit 1
fi
echo "  output novo: $NOVO"

hyprctl dispatch focusmonitor "$NOVO" >/dev/null
if esperar "$PRAZO_S" pet_em "$NOVO"; then
  passou "focar $NOVO: o pet foi para lá (camada viva, /v1/estado.monitor=$NOVO)"
else
  falhou "focar $NOVO: o pet ficou em $(campo .monitor) (visivel=$(campo .visivel))"
fi

hyprctl dispatch focusmonitor "$ANTES_FOCO" >/dev/null
if esperar "$PRAZO_S" pet_em "$ANTES_FOCO"; then
  passou "voltar a $ANTES_FOCO: o pet voltou"
else
  falhou "voltar a $ANTES_FOCO: o pet ficou em $(campo .monitor)"
fi

hyprctl output remove "$NOVO" >/dev/null
NOVO=""
sleep 1
if esperar "$PRAZO_S" pet_em "$ANTES_FOCO" && [ "$(campo .viagem)" = null ]; then
  passou "remover o output: o pet seguiu em $ANTES_FOCO, sem viagem pendurada"
else
  falhou "remover o output: monitor=$(campo .monitor), viagem=$(campo .viagem)"
fi

echo
printf '%s\n' "${LINHAS[@]}"
[ "$FALHAS" -eq 0 ]
