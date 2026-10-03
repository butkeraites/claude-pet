#!/usr/bin/env bash
# scripts/verificar-ao-vivo.sh — verificação do M1 na tela de verdade
# (PLANO.md, "M1 — Overlay nítido e barato").
#
# Sobe a pilha de desenvolvimento (PET_DEBUG=1, skin xadrez _teste) e confere:
#   1. saúde: /saude 200, tela ativa, pet visível;
#   2. camada: `hyprctl -j layers` mostra claude-pet no nível 3 do monitor
#      focado, com o retângulo lógico do monitor;
#   3. nitidez: `grim -o` do monitor inteiro + /v1/debug/quadro +
#      `cargo xtask nitidez` (cores ±2 nos pixels opacos, blocos D×D uniformes);
#   4. orçamentos: imagem < 40 MB, RSS < 64 MiB, CPU parado < 1%, commits/min;
#   5. clique: região de input só no corpo (o clique de verdade é manual);
#   6. `docker compose restart pet` volta em até ~3 s, no mesmo lugar;
#   7. `kill -9` pelo host faz o RestartCount subir (na pilha de produção, que
#      é a que tem restart: unless-stopped; `docker kill` cancelaria a política);
#   8. sem compositor, o log diz "aguardando compositor".
#
# No fim, mesmo se algo falhar, deixa a pilha de PRODUÇÃO de pé: sem
# personagem aprovado, o pet fica escondido (tela sem_personagem).
#
# Do Hyprland só usa consultas de leitura (`hyprctl -j monitors|layers`).
# A captura do monitor inteiro é apagada depois do uso; a foto que fica (para
# o PR) mostra só os pixels opacos do pet sobre um fundo neutro.
set -uo pipefail

RAIZ="$(cd "$(dirname "$(readlink -f "${BASH_SOURCE[0]}")")/.." && pwd)"
cd "$RAIZ"
export PATH="$HOME/.cargo/bin:$PATH"
PORTA="${PET_PORTA:-27380}"
URL="http://127.0.0.1:${PORTA}"
DEV=(docker compose -f docker-compose.yml -f docker-compose.dev.yml)
PROD=(docker compose)
SAIDA="$RAIZ/tmp/ao-vivo/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$SAIDA"

declare -a LINHAS=()
FALHAS=0
passou() { LINHAS+=("✓ $1"); printf '✓ %s\n' "$1"; }
falhou() { LINHAS+=("✗ $1"); printf '✗ %s\n' "$1" >&2; FALHAS=$((FALHAS + 1)); }
manual() { LINHAS+=("• $1"); printf '• %s\n' "$1"; }

api() { curl -fsS -m 3 -H 'X-Pet: 1' "${URL}$1"; }
post() {
  curl -fsS -m 3 -X POST -H 'X-Pet: 1' -H 'Content-Type: application/json' \
    --data "${2:-}" "${URL}$1" >/dev/null
}
campo() { api /v1/estado 2>/dev/null | jq -r "$1" 2>/dev/null; }
campo_json() { api /v1/estado 2>/dev/null | jq -c "$1" 2>/dev/null; }
agora_ms() { echo $(($(date +%s%N) / 1000000)); }

# esperar_campo <segundos> <filtro jq> <valor>
esperar_campo() {
  local limite=$(($(agora_ms) + $1 * 1000))
  while [ "$(agora_ms)" -lt "$limite" ]; do
    [ "$(campo "$2")" = "$3" ] && return 0
    sleep 0.1
  done
  return 1
}

RESTAURADO=0
restaurar() {
  [ "$RESTAURADO" = 1 ] && return 0
  RESTAURADO=1
  echo "▸ deixando a pilha de produção de pé (sem personagem → pet escondido)"
  docker rm -f claude-pet-sem-compositor >/dev/null 2>&1 || true
  "${PROD[@]}" up -d >/dev/null 2>&1 || echo "não consegui subir a produção" >&2
  esperar_campo 20 .tela sem_personagem || true
  printf '  produção: tela=%s visivel=%s\n' "$(campo .tela)" "$(campo .visivel)"
}
trap restaurar EXIT

# --- 1. saúde -------------------------------------------------------------
echo "▸ subindo a pilha de desenvolvimento (PET_DEBUG=1, skin _teste)"
if ! "${DEV[@]}" up -d --build >"$SAIDA/build.log" 2>&1; then
  falhou "subida da pilha de desenvolvimento (veja $SAIDA/build.log)"
  exit 1
fi
if esperar_campo 20 .tela ativa && esperar_campo 10 .visivel true &&
  curl -fsS -m 3 "${URL}/saude" >/dev/null; then
  passou "saúde: /saude 200, tela ativa, pet visível"
else
  falhou "saúde: tela=$(campo .tela) visivel=$(campo .visivel)"
fi

# --- 2. camada ------------------------------------------------------------
MONITORES="$(hyprctl -j monitors)"
FOCADO="$(jq -c '.[] | select(.focused)' <<<"$MONITORES" | head -n1)"
NOME="$(jq -r .name <<<"$FOCADO")"
read -r MX MY MW MH < <(jq -r '
  (if (.transform % 2) == 1 then [.height, .width] else [.width, .height] end) as $m
  | "\(.x) \(.y) \(($m[0] / .scale) | round) \(($m[1] / .scale) | round)"' <<<"$FOCADO")
CAMADAS="$(hyprctl -j layers | jq -c --arg m "$NOME" \
  '[.[$m].levels["3"][]? | select(.namespace == "claude-pet" and .pid > 0)]')"
QUANTAS="$(jq length <<<"$CAMADAS")"
GEOM="$(jq -r '.[0] | "\(.x) \(.y) \(.w) \(.h)"' <<<"$CAMADAS")"
if [ "$QUANTAS" = 1 ] && [ "$GEOM" = "$MX $MY $MW $MH" ] && [ "$(campo .monitor)" = "$NOME" ]; then
  passou "camada: claude-pet no nível 3 de $NOME, ${MW}x${MH} lógicos em ($MX,$MY)"
else
  falhou "camada: $QUANTAS camada(s) viva(s) em $NOME, geometria «$GEOM» (esperado «$MX $MY $MW $MH»), estado.monitor=$(campo .monitor)"
fi

# --- 3. nitidez -----------------------------------------------------------
if [ "$(jq -r .dpmsStatus <<<"$FOCADO")" != true ]; then
  falhou "nitidez: NÃO VERIFICADA — a tela de $NOME está apagada (DPMS) e o grim só captura com ela acesa"
else
  CAPTURA="$SAIDA/monitor.png"
  Q1=""
  for _ in $(seq 1 10); do
    Q1="$(api /v1/debug/quadro)" || { sleep 0.3; continue; }
    # O commit precisa ter ido para a tela antes da captura.
    if [ "$(jq -r .idade_ms <<<"$Q1")" -lt 300 ]; then
      sleep 0.3
      continue
    fi
    timeout 10 grim -o "$NOME" "$CAPTURA" || { Q1=""; break; }
    Q2="$(api /v1/debug/quadro)" || continue
    [ "$(jq -r .seq <<<"$Q1")" = "$(jq -r .seq <<<"$Q2")" ] && break
    Q1=""
  done
  if [ -z "$Q1" ] || [ ! -s "$CAPTURA" ]; then
    falhou "nitidez: não consegui um quadro estável e uma captura"
  else
    jq -r .png_base64 <<<"$Q1" | base64 -d >"$SAIDA/esperado.png"
    read -r QX QY QW QH QD GX GY < <(jq -r '"\(.x) \(.y) \(.w) \(.h) \(.d) \(.grade.x) \(.grade.y)"' <<<"$Q1")
    if cargo xtask nitidez --captura "$CAPTURA" --esperado "$SAIDA/esperado.png" \
      --x "$QX" --y "$QY" --d "$QD" --grade "$GX,$GY" >"$SAIDA/nitidez.txt" 2>&1; then
      passou "nitidez: $(head -n2 "$SAIDA/nitidez.txt" | tr '\n' ' ')"
    else
      falhou "nitidez: $(tr '\n' ' ' <"$SAIDA/nitidez.txt")"
    fi
    # Foto para o PR: só os pixels do pet (máscara = alfa do esperado).
    magick "$CAPTURA" -crop "${QW}x${QH}+${QX}+${QY}" +repage \
      \( "$SAIDA/esperado.png" -alpha extract \) -compose CopyOpacity -composite \
      -background '#202028' -compose Over -flatten "$SAIDA/pet.png"
    magick "$SAIDA/pet.png" -filter point -resize 300% "$SAIDA/pet-x3.png"
  fi
  rm -f "$CAPTURA"
fi

# --- 4. orçamentos do container -------------------------------------------
CONTAINER="$("${DEV[@]}" ps -q pet)"
TAMANHO="$(docker image inspect claude-pet:local --format '{{.Size}}')"
if [ "$TAMANHO" -lt 40000000 ]; then
  passou "imagem: $((TAMANHO / 1000)) kB (< 40 MB)"
else
  falhou "imagem: $((TAMANHO / 1000)) kB (orçamento 40 MB)"
fi
mib() { # "12.3MiB / 128MiB" → MiB do primeiro número
  awk '{v=$1; u=v; gsub(/[0-9.]/,"",u); gsub(/[^0-9.]/,"",v);
        f=(u=="KiB"?1/1024:(u=="GiB"?1024:(u=="B"?1/1048576:1))); printf "%.1f", v*f}'
}
RSS="$(docker stats --no-stream --format '{{.MemUsage}}' "$CONTAINER" | mib)"
if awk -v r="$RSS" 'BEGIN{exit !(r < 64)}'; then
  passou "RSS do container: ${RSS} MiB (< 64 MiB)"
else
  falhou "RSS do container: ${RSS} MiB (orçamento 64 MiB)"
fi
CPU_SOMA=0
for _ in 1 2 3 4 5; do
  C="$(docker stats --no-stream --format '{{.CPUPerc}}' "$CONTAINER" | tr -d '%')"
  CPU_SOMA="$(awk -v a="$CPU_SOMA" -v b="$C" 'BEGIN{print a+b}')"
done
CPU="$(awk -v s="$CPU_SOMA" 'BEGIN{printf "%.2f", s/5}')"
if awk -v c="$CPU" 'BEGIN{exit !(c < 1)}'; then
  passou "CPU do container parado: ${CPU}% (< 1%, média de 5 amostras)"
else
  falhou "CPU do container parado: ${CPU}% (orçamento 1%)"
fi
CPM="$(campo .commits_por_min)"
if [ "$CPM" -le 120 ]; then
  passou "commits no último minuto: $CPM (orçamento parado: 120)"
else
  falhou "commits no último minuto: $CPM (orçamento parado: 120)"
fi

# --- 5. clique --------------------------------------------------------------
REGIAO="$(campo_json .regiao_entrada)"
if [ "$REGIAO" != null ] && [ -n "$REGIAO" ] &&
  [ "$(jq -r --argjson w "$MW" --argjson h "$MH" '(.w * .h) < ($w * $h / 50)' <<<"$REGIAO")" = true ]; then
  passou "região de input só no corpo: $REGIAO (lógico)"
else
  falhou "região de input: $REGIAO"
fi
manual "clique: clicar AO LADO do pet tem que chegar na janela de baixo (conferir à mão)"

# --- 6. reinício ------------------------------------------------------------
ANTES="$(campo_json .sprite_disp)"
T0="$(agora_ms)"
"${DEV[@]}" restart pet >/dev/null 2>&1
if esperar_campo 15 .visivel true; then
  DT=$(($(agora_ms) - T0))
  DEPOIS="$(campo_json .sprite_disp)"
  if [ "$DT" -le 3500 ] && [ "$DEPOIS" = "$ANTES" ]; then
    passou "restart: de volta em ${DT} ms, no mesmo lugar"
  else
    falhou "restart: ${DT} ms, sprite_disp antes $ANTES depois $DEPOIS"
  fi
else
  falhou "restart: o pet não voltou em 15 s (tela=$(campo .tela))"
fi

# --- 7. crash (produção: restart unless-stopped) ----------------------------
echo "▸ trocando para a pilha de produção para o teste de crash"
"${PROD[@]}" up -d >/dev/null 2>&1
if ! esperar_campo 20 .tela sem_personagem; then
  falhou "produção não ficou sem_personagem (tela=$(campo .tela))"
else
  ID="$("${PROD[@]}" ps -q pet)"
  ANTES="$(docker inspect -f '{{.RestartCount}}' "$ID")"
  PID="$(pgrep -u "$(id -u)" -f '^/usr/local/bin/claude-pet rodar$' | head -n1)"
  if [ -z "$PID" ]; then
    falhou "crash: não achei o processo do daemon pelo host"
  else
    T0="$(agora_ms)"
    kill -9 "$PID"
    VOLTOU=0
    for _ in $(seq 1 150); do
      DEPOIS="$(docker inspect -f '{{.RestartCount}}' "$ID" 2>/dev/null || echo "$ANTES")"
      if [ "$DEPOIS" -gt "$ANTES" ] && [ "$(campo .tela)" = sem_personagem ]; then
        VOLTOU=1
        break
      fi
      sleep 0.1
    done
    if [ "$VOLTOU" = 1 ]; then
      passou "crash: kill -9 no PID $PID; RestartCount $ANTES → $DEPOIS, de volta em $(($(agora_ms) - T0)) ms"
    else
      falhou "crash: RestartCount $ANTES → $DEPOIS, tela=$(campo .tela)"
    fi
  fi
fi

# --- 8. sem compositor ------------------------------------------------------
docker rm -f claude-pet-sem-compositor >/dev/null 2>&1 || true
"${PROD[@]}" run -d --no-deps --name claude-pet-sem-compositor \
  -e PET_HOST_RUNTIME=/tmp/nada pet >/dev/null 2>&1
sleep 4
LOG="$(docker logs claude-pet-sem-compositor 2>&1)"
docker rm -f claude-pet-sem-compositor >/dev/null 2>&1 || true
if grep -q 'aguardando compositor' <<<"$LOG"; then
  passou "sem compositor: $(grep -m1 'aguardando compositor' <<<"$LOG")"
else
  falhou "sem compositor: log sem «aguardando compositor»: $LOG"
fi

restaurar
echo
echo "Resumo (artefatos em ${SAIDA#"$RAIZ"/}):"
printf '  %s\n' "${LINHAS[@]}"
if [ "$FALHAS" -eq 0 ]; then
  echo "✓ verificação ao vivo do M1 passou"
else
  echo "✗ $FALHAS item(ns) falharam" >&2
  exit 1
fi
