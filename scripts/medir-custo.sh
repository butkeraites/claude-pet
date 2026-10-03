#!/usr/bin/env bash
# scripts/medir-custo.sh — custo do pet no compositor (decisão 0005).
#
# No Hyprland 0.56.2 cada commit de uma camada repinta o monitor inteiro;
# o custo não aparece no `docker stats`, aparece no Hyprland e na GPU.
# Com a pilha de desenvolvimento (PET_DEBUG=1) mede três fases:
#   escondido  30 s   pet sem camada (a linha de base)
#   parado     30 s   pose fixa com rajadas
#   estresse   15 s   40 confetes a 30 fps pela tela inteira
# e imprime uma tabela com:
#   CPU do Hyprland = Δ(utime+stime) de /proc/<pid>/stat ÷ CLK_TCK ÷ Δt
#   GPU ocupada     = 1 − Δrc6_residency_ms ÷ Δt   (PET_RC6 muda o arquivo)
#   commits/s       = ΔCommits_total do /v1/estado ÷ Δt
#   RSS do Hyprland no fim da fase (a textura da camada mora fora do container)
# Orçamento (decisão 0005): parado ≤ escondido + 1 ponto de CPU do Hyprland
# e ≤ 2 commits/s. Exige a tela acesa (com DPMS desligado o Hyprland não
# desenha e a medida não diz nada). No fim deixa a produção de pé.
set -uo pipefail

RAIZ="$(cd "$(dirname "$(readlink -f "${BASH_SOURCE[0]}")")/.." && pwd)"
cd "$RAIZ"
PORTA="${PET_PORTA:-27380}"
URL="http://127.0.0.1:${PORTA}"
DEV=(docker compose -f docker-compose.yml -f docker-compose.dev.yml)
PROD=(docker compose)
RC6="${PET_RC6:-/sys/class/drm/card1/gt/gt0/rc6_residency_ms}"
FASE_PARADO="${PET_FASE_S:-30}"
FASE_ESTRESSE="${PET_FASE_ESTRESSE_S:-15}"

api() { curl -fsS -m 3 -H 'X-Pet: 1' "${URL}$1"; }
post() {
  curl -fsS -m 3 -X POST -H 'X-Pet: 1' -H 'Content-Type: application/json' \
    --data "${2:-}" "${URL}$1" >/dev/null
}
campo() { api /v1/estado 2>/dev/null | jq -r "$1" 2>/dev/null; }
agora_ms() { echo $(($(date +%s%N) / 1000000)); }
esperar_campo() {
  local limite=$(($(agora_ms) + $1 * 1000))
  while [ "$(agora_ms)" -lt "$limite" ]; do
    [ "$(campo "$2")" = "$3" ] && return 0
    sleep 0.1
  done
  return 1
}

restaurar() {
  echo "▸ deixando a pilha de produção de pé (sem personagem → pet escondido)"
  "${PROD[@]}" up -d >/dev/null 2>&1 || echo "não consegui subir a produção" >&2
  esperar_campo 20 .tela sem_personagem || true
}
trap restaurar EXIT

PID_H="$(pgrep -x Hyprland | head -n1)"
CLK="$(getconf CLK_TCK)"
if [ -z "$PID_H" ] || [ ! -r "$RC6" ]; then
  echo "preciso do Hyprland rodando e de $RC6 legível" >&2
  exit 1
fi
FOCADO="$(hyprctl -j monitors | jq -c '.[] | select(.focused)' | head -n1)"
if [ "$(jq -r .dpmsStatus <<<"$FOCADO")" != true ]; then
  echo "a tela de $(jq -r .name <<<"$FOCADO") está apagada (DPMS): o Hyprland não desenha e a medida não vale" >&2
  exit 1
fi

echo "▸ subindo a pilha de desenvolvimento (PET_DEBUG=1, skin _teste)"
"${DEV[@]}" up -d --build >/dev/null 2>&1 || { echo "falha ao subir a pilha dev" >&2; exit 1; }
esperar_campo 20 .visivel true || { echo "o pet não apareceu" >&2; exit 1; }

cpu_h() { awk '{print $14 + $15}' "/proc/$PID_H/stat"; }

declare -A CPU GPU CPS RSS
medir() { # medir <nome> <segundos>
  local nome=$1 dur=$2 t0 t1 c0 c1 g0 g1 k0 k1 dt
  t0="$(agora_ms)"; c0="$(cpu_h)"; g0="$(cat "$RC6")"; k0="$(campo .commits_total)"
  sleep "$dur"
  t1="$(agora_ms)"; c1="$(cpu_h)"; g1="$(cat "$RC6")"; k1="$(campo .commits_total)"
  dt=$((t1 - t0))
  CPU[$nome]="$(awk -v a="$c0" -v b="$c1" -v k="$CLK" -v t="$dt" 'BEGIN{printf "%.2f", (b-a)/k/(t/1000)*100}')"
  GPU[$nome]="$(awk -v a="$g0" -v b="$g1" -v t="$dt" 'BEGIN{v=(1-(b-a)/t)*100; if (v<0) v=0; printf "%.1f", v}')"
  CPS[$nome]="$(awk -v a="$k0" -v b="$k1" -v t="$dt" 'BEGIN{printf "%.2f", (b-a)/(t/1000)}')"
  RSS[$nome]="$(ps -o rss= -p "$PID_H" | awk '{printf "%.1f", $1/1024}')"
  printf '  %-10s %6s s   CPU Hyprland %6s%%   GPU %5s%%   commits/s %6s   RSS Hyprland %s MiB\n' \
    "$nome" "$((dt / 1000))" "${CPU[$nome]}" "${GPU[$nome]}" "${CPS[$nome]}" "${RSS[$nome]}"
}

echo "▸ escondido (${FASE_PARADO} s)"
post /v1/debug/esconder ""
esperar_campo 5 .visivel false
sleep 2
medir escondido "$FASE_PARADO"

echo "▸ parado (${FASE_PARADO} s)"
post /v1/debug/mostrar ""
esperar_campo 10 .visivel true
sleep 5
medir parado "$FASE_PARADO"

echo "▸ estresse (${FASE_ESTRESSE} s, 40 confetes a 30 fps)"
post /v1/debug/estresse "{\"fps\":30,\"segundos\":$((FASE_ESTRESSE + 3))}"
sleep 1
medir estresse "$FASE_ESTRESSE"
esperar_campo 10 .estresse false

echo
echo "| fase | CPU do Hyprland | GPU ocupada | commits/s | RSS do Hyprland |"
echo "|---|---|---|---|---|"
for f in escondido parado estresse; do
  printf '| %s | %s%% | %s%% | %s | %s MiB |\n' "$f" "${CPU[$f]}" "${GPU[$f]}" "${CPS[$f]}" "${RSS[$f]}"
done
DELTA="$(awk -v p="${CPU[parado]}" -v e="${CPU[escondido]}" 'BEGIN{printf "%.2f", p-e}')"
echo
if awk -v d="$DELTA" -v c="${CPS[parado]}" 'BEGIN{exit !(d <= 1.0 && c <= 2.0)}'; then
  echo "✓ orçamento parado: +${DELTA} ponto(s) de CPU do Hyprland (≤ 1) e ${CPS[parado]} commits/s (≤ 2)"
else
  echo "✗ orçamento parado ESTOURADO: +${DELTA} ponto(s) de CPU do Hyprland (≤ 1) e ${CPS[parado]} commits/s (≤ 2); plano B: superfície pequena de repouso + palco temporário (decisão 0005)" >&2
  exit 1
fi
