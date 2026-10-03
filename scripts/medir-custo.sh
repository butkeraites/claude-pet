#!/usr/bin/env bash
# scripts/medir-custo.sh — custo do pet no compositor (decisão 0005).
#
# No Hyprland 0.56.2 cada commit de uma camada repinta o monitor inteiro, e
# uma camada do tamanho do monitor sempre mapeada entra em toda repintura que
# os OUTROS causam (vídeo, rolagem). Nada disso aparece no `docker stats`:
# aparece no Hyprland e na GPU. Com a pilha de desenvolvimento (PET_DEBUG=1):
#
#   A. escondido × parado, intercalados (PET_RODADAS vezes, PET_FASE_S cada):
#      o custo do pet sozinho num desktop parado;
#   B. carga+escondido × carga+parado, intercalados: o mesmo, com uma repintura
#      de tela cheia no ritmo do monitor (`cargo xtask carga`: camada OVERLAY
#      transparente, invisível) — o custo estrutural da camada sempre mapeada;
#   C. estresse (PET_FASE_ESTRESSE_S): 40 confetes a 30 fps.
#
# Por fase: CPU do Hyprland = Δ(utime+stime) de /proc/<pid>/stat ÷ CLK_TCK ÷ Δt;
# GPU ocupada = 1 − Δrc6_residency_ms ÷ Δt (PET_RC6 muda o arquivo);
# commits/s do pet = Δcommits_total ÷ Δt; RSS do Hyprland no fim.
#
# Cada transição é conferida (estado do pet, commits/s dentro do esperado,
# carga rodando, tela acesa); se algo não bate, o script para e diz o quê,
# em vez de imprimir um ✓ sem valor. Não mexa no mouse durante a medição.
#
# Orçamento (decisão 0005): parado ≤ escondido + 1 ponto de CPU do Hyprland e
# ≤ 2 commits/s. Custo estrutural (B, critério da decisão 0018): até +1 ponto
# de CPU do Hyprland e +5 pontos de GPU. Estourou → plano B (superfície
# pequena de repouso + palco temporário). Do Hyprland só usa consultas de
# leitura (`hyprctl -j monitors|layers`). No fim deixa a produção de pé.
set -uo pipefail

RAIZ="$(cd "$(dirname "$(readlink -f "${BASH_SOURCE[0]}")")/.." && pwd)"
cd "$RAIZ" || exit 1
export PATH="$HOME/.cargo/bin:$PATH"
URL="http://127.0.0.1:${PET_PORTA:-27380}"
DEV=(docker compose -f docker-compose.yml -f docker-compose.dev.yml)
PROD=(docker compose)
RC6="${PET_RC6:-/sys/class/drm/card1/gt/gt0/rc6_residency_ms}"
FASE="${PET_FASE_S:-20}"
FASE_ESTRESSE="${PET_FASE_ESTRESSE_S:-15}"
RODADAS="${PET_RODADAS:-3}"
XTASK="$RAIZ/target/debug/xtask"

api() { curl -fsS -m 3 -H 'X-Pet: 1' "${URL}$1"; }
post() {
  curl -fsS -m 3 -X POST -H 'X-Pet: 1' -H 'Content-Type: application/json' \
    --data "${2:-}" "${URL}$1" >/dev/null
}
campo() { api /v1/estado 2>/dev/null | jq -r "$1" 2>/dev/null; }
agora_ms() { echo $(($(date +%s%N) / 1000000)); }
esperar_campo() { # <segundos> <filtro jq> <valor>
  local limite=$(($(agora_ms) + $1 * 1000))
  while [ "$(agora_ms)" -lt "$limite" ]; do
    [ "$(campo "$2")" = "$3" ] && return 0
    sleep 0.1
  done
  return 1
}
parar() {
  echo "✗ $*" >&2
  exit 1
}

CARGA_PID=""
restaurar() {
  [ -n "$CARGA_PID" ] && kill "$CARGA_PID" 2>/dev/null
  echo "▸ deixando a pilha de produção de pé (sem personagem → pet escondido)"
  "${PROD[@]}" up -d >/dev/null 2>&1 || echo "não consegui subir a produção" >&2
  esperar_campo 20 .tela sem_personagem || true
}
trap restaurar EXIT

PID_H="$(pgrep -x Hyprland | head -n1)"
CLK="$(getconf CLK_TCK)"
[ -n "$PID_H" ] && [ -r "$RC6" ] || parar "preciso do Hyprland rodando e de $RC6 legível"
FOCADO="$(hyprctl -j monitors | jq -c '.[] | select(.focused)' | head -n1)"
MONITOR="$(jq -r .name <<<"$FOCADO")"
tela_acesa() {
  [ "$(hyprctl -j monitors | jq -r --arg m "$MONITOR" '.[] | select(.name == $m) | .dpmsStatus')" = true ]
}
tela_acesa || parar "a tela de $MONITOR está apagada (DPMS): o Hyprland não desenha e a medida não vale"

echo "▸ compilando o xtask (carga) e subindo a pilha de desenvolvimento"
cargo build -q -p xtask || parar "falha ao compilar o xtask"
"${DEV[@]}" up -d --build >/dev/null 2>&1 || parar "falha ao subir a pilha dev"
esperar_campo 20 .visivel true || parar "o pet não apareceu"
echo "  (não mexa no mouse nem no teclado durante a medição: ~$(((RODADAS * 4 * (FASE + 4) + FASE_ESTRESSE + 30) / 60)) min)"

cpu_h() { awk '{print $14 + $15}' "/proc/$PID_H/stat"; }

# medir <nome> <segundos> <commits/s mínimo> <commits/s máximo>
declare -A CPU GPU CPS RSS
medir() {
  local nome=$1 dur=$2 min=$3 max=$4 t0 t1 c0 c1 g0 g1 k0 k1 dt
  t0="$(agora_ms)"; c0="$(cpu_h)"; g0="$(cat "$RC6")"; k0="$(campo .commits_total)"
  sleep "$dur"
  t1="$(agora_ms)"; c1="$(cpu_h)"; g1="$(cat "$RC6")"; k1="$(campo .commits_total)"
  dt=$((t1 - t0))
  CPU[$nome]="$(awk -v a="$c0" -v b="$c1" -v k="$CLK" -v t="$dt" 'BEGIN{printf "%.2f", (b-a)/k/(t/1000)*100}')"
  GPU[$nome]="$(awk -v a="$g0" -v b="$g1" -v t="$dt" 'BEGIN{v=(1-(b-a)/t)*100; if (v<0) v=0; printf "%.1f", v}')"
  CPS[$nome]="$(awk -v a="$k0" -v b="$k1" -v t="$dt" 'BEGIN{printf "%.2f", (b-a)/(t/1000)}')"
  RSS[$nome]="$(ps -o rss= -p "$PID_H" | awk '{printf "%.1f", $1/1024}')"
  printf '  %-18s %3s s   CPU Hyprland %6s%%   GPU %5s%%   commits/s %6s   RSS Hyprland %s MiB\n' \
    "$nome" "$((dt / 1000))" "${CPU[$nome]}" "${GPU[$nome]}" "${CPS[$nome]}" "${RSS[$nome]}"
  tela_acesa || parar "a tela apagou durante «$nome»: medida inválida"
  awk -v c="${CPS[$nome]}" -v a="$min" -v b="$max" 'BEGIN{exit !(c >= a && c <= b)}' ||
    parar "«$nome»: ${CPS[$nome]} commits/s fora do esperado ($min..$max)"
}

esconder() {
  post /v1/debug/esconder "" || parar "POST esconder falhou"
  esperar_campo 5 .visivel false || parar "o pet não escondeu"
  sleep 2 # fade de saída do Hyprland
}
mostrar() {
  post /v1/debug/mostrar "" || parar "POST mostrar falhou"
  esperar_campo 10 .visivel true || parar "o pet não voltou"
  sleep 3 # fade de entrada e primeira rajada fora da janela
}

carga_iniciar() { # <segundos>
  CARGA_LOG="$(mktemp)"
  "$XTASK" carga --segundos "$1" >"$CARGA_LOG" 2>&1 &
  CARGA_PID=$!
  sleep 2
  hyprctl -j layers | jq -e --arg m "$MONITOR" \
    '[.[$m].levels[]?[] | select(.namespace == "claude-pet-carga" and .pid > 0)] | length == 1' \
    >/dev/null || parar "a camada de carga não apareceu em $MONITOR: $(cat "$CARGA_LOG")"
}
carga_fim() {
  wait "$CARGA_PID" || parar "a carga falhou: $(cat "$CARGA_LOG")"
  CARGA_PID=""
  local por_s
  por_s="$(grep -oE '\(([0-9.]+)/s\)' "$CARGA_LOG" | tr -dc '0-9.')"
  echo "    $(cat "$CARGA_LOG")"
  awk -v c="${por_s:-0}" 'BEGIN{exit !(c >= 20)}' ||
    parar "a carga repintou só ${por_s:-0}/s (esperado o ritmo do monitor)"
  rm -f "$CARGA_LOG"
}

echo "▸ A. escondido × parado, ${RODADAS} rodadas de ${FASE} s"
for r in $(seq 1 "$RODADAS"); do
  esconder
  medir "escondido-$r" "$FASE" 0 0.05
  mostrar
  medir "parado-$r" "$FASE" 0.1 2.0
done

echo "▸ B. com repintura de tela cheia (carga invisível), ${RODADAS} rodadas de ${FASE} s"
for r in $(seq 1 "$RODADAS"); do
  esconder
  carga_iniciar $((FASE + 4))
  medir "carga-escondido-$r" "$FASE" 0 0.05
  carga_fim
  mostrar
  carga_iniciar $((FASE + 4))
  medir "carga-parado-$r" "$FASE" 0.1 2.0
  carga_fim
done

echo "▸ C. estresse (${FASE_ESTRESSE} s, 40 confetes a 30 fps)"
post /v1/debug/estresse "{\"fps\":30,\"segundos\":$((FASE_ESTRESSE + 3))}" || parar "POST estresse falhou"
esperar_campo 3 .estresse true || parar "o estresse não começou"
sleep 1
medir estresse "$FASE_ESTRESSE" 20 31
esperar_campo 10 .estresse false || parar "o estresse não acabou"

# media <prefixo> <tabela>: média e faixa das rodadas
resumo() {
  local prefixo=$1 tabela=$2 valores=()
  for r in $(seq 1 "$RODADAS"); do
    case "$tabela" in
      CPU) valores+=("${CPU[$prefixo-$r]}") ;;
      GPU) valores+=("${GPU[$prefixo-$r]}") ;;
      CPS) valores+=("${CPS[$prefixo-$r]}") ;;
    esac
  done
  printf '%s\n' "${valores[@]}" | awk '{s+=$1; if (NR==1||$1<m) m=$1; if (NR==1||$1>M) M=$1}
    END{printf "%.2f (%.2f–%.2f)", s/NR, m, M}'
}
media() { resumo "$1" "$2" | cut -d' ' -f1; }

echo
echo "| fase | CPU do Hyprland % | GPU ocupada % | commits/s do pet |"
echo "|---|---|---|---|"
for f in escondido parado carga-escondido carga-parado; do
  printf '| %s | %s | %s | %s |\n' "$f" "$(resumo "$f" CPU)" "$(resumo "$f" GPU)" "$(resumo "$f" CPS)"
done
printf '| estresse | %s | %s | %s |\n' "${CPU[estresse]}" "${GPU[estresse]}" "${CPS[estresse]}"
echo "(média das ${RODADAS} rodadas, entre parênteses a menor e a maior; RSS do Hyprland: ${RSS[escondido-1]} → ${RSS[parado-1]} MiB)"

D_PARADO="$(awk -v p="$(media parado CPU)" -v e="$(media escondido CPU)" 'BEGIN{printf "%.2f", p-e}')"
CPS_PARADO="$(media parado CPS)"
D_CARGA_CPU="$(awk -v p="$(media carga-parado CPU)" -v e="$(media carga-escondido CPU)" 'BEGIN{printf "%.2f", p-e}')"
D_CARGA_GPU="$(awk -v p="$(media carga-parado GPU)" -v e="$(media carga-escondido GPU)" 'BEGIN{printf "%.2f", p-e}')"
echo
FALHOU=0
if awk -v d="$D_PARADO" -v c="$CPS_PARADO" 'BEGIN{exit !(d <= 1.0 && c <= 2.0)}'; then
  echo "✓ parado: +${D_PARADO} ponto(s) de CPU do Hyprland (≤ 1) e ${CPS_PARADO} commits/s (≤ 2)"
else
  echo "✗ parado ESTOUROU: +${D_PARADO} ponto(s) de CPU do Hyprland (≤ 1) e ${CPS_PARADO} commits/s (≤ 2)" >&2
  FALHOU=1
fi
if awk -v c="$D_CARGA_CPU" -v g="$D_CARGA_GPU" 'BEGIN{exit !(c <= 1.0 && g <= 5.0)}'; then
  echo "✓ estrutural (repintura de tela cheia): +${D_CARGA_CPU} ponto(s) de CPU do Hyprland (≤ 1) e +${D_CARGA_GPU} de GPU (≤ 5)"
else
  echo "✗ estrutural ESTOUROU: +${D_CARGA_CPU} ponto(s) de CPU do Hyprland (≤ 1) e +${D_CARGA_GPU} de GPU (≤ 5)" >&2
  FALHOU=1
fi
if [ "$FALHOU" = 1 ]; then
  echo "  → plano B da decisão 0005: superfície pequena de repouso + palco de tela cheia só para arraste, voo e confete" >&2
  exit 1
fi
