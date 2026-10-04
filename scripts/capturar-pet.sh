#!/usr/bin/env bash
# scripts/capturar-pet.sh <pasta> — captura o monitor onde o pet está.
#
# Lê o /v1/estado (monitor e sprite_disp) e captura o monitor INTEIRO com
# `grim -o` (transformação identidade: pixel do monitor = pixel da imagem;
# nunca `grim -g`, que passa por filtro bilinear). Com PET_DEBUG=1 pega também
# o quadro esperado do /v1/debug/quadro e garante que captura e quadro são do
# mesmo commit: o quadro está há pelo menos 300 ms na tela e o `seq` é o mesmo
# antes e depois do grim.
#
# Escreve em <pasta>:
#   monitor.png    o monitor inteiro. Contém o que estiver na tela (janelas,
#                  texto): QUEM CHAMA APAGA depois do uso, e nunca vai para o git;
#   estado.json    o /v1/estado no momento da captura;
#   quadro.json    (só em debug) monitor, x, y, w, h, d, grade, seq;
#   esperado.png   (só em debug) o RGBA exato do sprite em pixels do monitor.
#
# Saída: 0 ok; 1 erro; 3 tela apagada (com DPMS o grim esperaria para sempre)
# ou sessão bloqueada (a captura mostraria só a tela de senha).
# Do Hyprland só usa a consulta de leitura `hyprctl -j monitors`.
set -uo pipefail

PASTA="${1:?uso: scripts/capturar-pet.sh <pasta>}"
URL="http://127.0.0.1:${PET_PORTA:-27380}"
api() { curl -q --noproxy '*' -fsS -m 3 -H 'X-Pet: 1' "${URL}$1"; }

mkdir -p "$PASTA"
if ! ESTADO="$(api /v1/estado)"; then
  echo "o pet não respondeu em ${URL} — está de pé? (bin/pet subir)" >&2
  exit 1
fi
MONITOR="$(jq -r '.monitor // empty' <<<"$ESTADO")"
if [ -z "$MONITOR" ] || [ "$(jq -r '.sprite_disp == null' <<<"$ESTADO")" = true ]; then
  echo "o pet não está na tela (tela: $(jq -r .tela <<<"$ESTADO"))" >&2
  exit 1
fi
MONITORES="$(hyprctl -j monitors)"
DPMS="$(jq -r --arg m "$MONITOR" '.[] | select(.name == $m) | .dpmsStatus' <<<"$MONITORES")"
if [ "$DPMS" != true ]; then
  echo "a tela de ${MONITOR} está apagada (DPMS): o grim só captura com ela acesa" >&2
  exit 3
fi
# Sessão bloqueada (lock do Omarchy): o Hyprland desenha só a tela de senha.
if [ "$(jq -r 'any(.[]; (.solitaryBlockedBy // []) | index("LOCK") != null)' <<<"$MONITORES")" = true ]; then
  echo "a sessão está bloqueada: a tela de senha cobre o pet (desbloqueie e tente de novo)" >&2
  exit 3
fi

capturar() {
  if ! timeout 10 grim -o "$MONITOR" "$PASTA/monitor.png"; then
    rm -f "$PASTA/monitor.png"
    echo "o grim falhou ao capturar ${MONITOR}" >&2
    exit 1
  fi
}

if [ "$(jq -r .debug <<<"$ESTADO")" != true ]; then
  capturar
  api /v1/estado >"$PASTA/estado.json" 2>/dev/null || printf '%s\n' "$ESTADO" >"$PASTA/estado.json"
  exit 0
fi

for _ in $(seq 1 20); do
  Q1="$(api /v1/debug/quadro)" || { sleep 0.3; continue; }
  # O commit precisa ter ido para a tela antes da captura.
  if [ "$(jq -r .idade_ms <<<"$Q1")" -lt 300 ]; then
    sleep 0.3
    continue
  fi
  capturar
  Q2="$(api /v1/debug/quadro)" || continue
  if [ "$(jq -r .seq <<<"$Q1")" = "$(jq -r .seq <<<"$Q2")" ]; then
    jq 'del(.png_base64)' <<<"$Q1" >"$PASTA/quadro.json"
    jq -r .png_base64 <<<"$Q1" | base64 -d >"$PASTA/esperado.png"
    api /v1/estado >"$PASTA/estado.json" 2>/dev/null || printf '%s\n' "$ESTADO" >"$PASTA/estado.json"
    exit 0
  fi
done
rm -f "$PASTA/monitor.png"
echo "não consegui um quadro estável junto com a captura" >&2
exit 1
