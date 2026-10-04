#!/usr/bin/env bash
# scripts/verificar-ao-vivo.sh — verificação do M1 na tela de verdade
# (PLANO.md, "M1 — Overlay nítido e barato").
#
# Sobe a pilha de desenvolvimento (PET_DEBUG=1, skin xadrez _teste) e confere:
#   1. saúde: /saude 200, tela ativa, pet visível;
#   2. camada: `hyprctl -j layers` mostra claude-pet no nível 3 do monitor
#      focado, com o retângulo lógico do monitor;
#   3. esconder e mostrar seguidos: o pet volta (a corrida da revisão);
#   4. nitidez: `grim -o` do monitor inteiro + /v1/debug/quadro +
#      `cargo xtask nitidez` (cores ±2 nos pixels opacos, blocos D×D uniformes);
#   5. sem pixel velho: com o pet na tela, onde o quadro é transparente aparece
#      o fundo; logo depois de esconder, a área volta a ser o fundo (sem
#      fantasma no fade do Hyprland) — `cargo xtask fantasma`;
#   6. orçamentos: imagem < 40 MB, RSS < 64 MiB, CPU parado < 1%, commits:
#      com a tela acesa ≤ 2/s numa janela de 20 s; com ela apagada, 0;
#   7. clique: região de input só no corpo (o clique de verdade é manual);
#   8. `docker compose restart pet` volta em até ~3 s, no mesmo lugar;
#   9. troca para a produção (SIGTERM no daemon de dev) sem fantasma, e
#      `kill -9` pelo host faz o RestartCount subir (a produção é a que tem
#      restart: unless-stopped; `docker kill` cancelaria a política);
#  10. sem compositor, o log diz "aguardando compositor".
#
# Com a tela apagada (DPMS) o Hyprland não desenha: nitidez, pixel velho,
# fantasma e o ritmo parado saem como NÃO VERIFICADOS (contam como falha).
#
# No fim, mesmo se algo falhar, deixa a pilha de PRODUÇÃO de pé: sem
# personagem aprovado, o pet fica escondido (tela sem_personagem).
#
# Do Hyprland só usa consultas de leitura (`hyprctl -j monitors|layers`).
# Capturas do monitor e recortes com o fundo são apagados no fim; o que fica
# (para o PR) mostra só os pixels opacos do pet sobre um fundo neutro.
set -uo pipefail

RAIZ="$(cd "$(dirname "$(readlink -f "${BASH_SOURCE[0]}")")/.." && pwd)"
cd "$RAIZ" || exit 1
export PATH="$HOME/.cargo/bin:$PATH"
URL="http://127.0.0.1:${PET_PORTA:-27380}"
DEV=(docker compose -f docker-compose.yml -f docker-compose.dev.yml)
PROD=(docker compose)
SAIDA="$RAIZ/tmp/ao-vivo/$(date +%Y%m%d-%H%M%S)"
PRIVADO="$SAIDA/.privado" # capturas com o fundo da tela: apagadas no fim
mkdir -p "$PRIVADO"

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
  rm -rf "$PRIVADO"
  echo "▸ deixando a pilha de produção de pé (sem personagem → pet escondido)"
  docker rm -f claude-pet-sem-compositor >/dev/null 2>&1 || true
  "${PROD[@]}" up -d >/dev/null 2>&1 || echo "não consegui subir a produção" >&2
  esperar_campo 20 .tela sem_personagem || true
  printf '  produção: tela=%s visivel=%s\n' "$(campo .tela)" "$(campo .visivel)"
}
trap restaurar EXIT

echo "▸ compilando o xtask (nitidez, fantasma)"
cargo build -q -p xtask || { falhou "o xtask não compila"; exit 1; }
XTASK="$RAIZ/target/debug/xtask"

# --- 1. saúde ---------------------------------------------------------------
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

# --- 2. camada --------------------------------------------------------------
FOCADO="$(hyprctl -j monitors | jq -c '.[] | select(.focused)' | head -n1)"
NOME="$(jq -r .name <<<"$FOCADO")"
ACESA="$(jq -r .dpmsStatus <<<"$FOCADO")"
read -r MX MY MW MH < <(jq -r '
  (if (.transform % 2) == 1 then [.height, .width] else [.width, .height] end) as $m
  | "\(.x) \(.y) \(($m[0] / .scale) | round) \(($m[1] / .scale) | round)"' <<<"$FOCADO")
camadas_vivas() {
  hyprctl -j layers | jq -c --arg m "$NOME" \
    '[.[$m].levels["3"][]? | select(.namespace == "claude-pet" and .pid > 0)]'
}
CAMADAS="$(camadas_vivas)"
QUANTAS="$(jq length <<<"$CAMADAS")"
GEOM="$(jq -r '.[0] | "\(.x) \(.y) \(.w) \(.h)"' <<<"$CAMADAS")"
if [ "$QUANTAS" = 1 ] && [ "$GEOM" = "$MX $MY $MW $MH" ] && [ "$(campo .monitor)" = "$NOME" ]; then
  passou "camada: claude-pet no nível 3 de $NOME, ${MW}x${MH} lógicos em ($MX,$MY)"
else
  falhou "camada: $QUANTAS camada(s) viva(s) em $NOME, geometria «$GEOM» (esperado «$MX $MY $MW $MH»), estado.monitor=$(campo .monitor)"
fi

# --- 3. esconder e mostrar seguidos -------------------------------------------
post /v1/debug/esconder ""
post /v1/debug/mostrar ""
if esperar_campo 3 .visivel true && sleep 0.5 && [ "$(camadas_vivas | jq length)" = 1 ]; then
  passou "esconder e mostrar seguidos: o pet volta, com uma camada só"
else
  falhou "esconder e mostrar seguidos: visivel=$(campo .visivel), $(camadas_vivas | jq length) camada(s) viva(s)"
fi

# Recorta a área do pet (pixels do monitor) de uma captura nova do monitor.
# recortar <saída.png> <x> <y> <w> <h>
recortar() {
  local completa
  completa="$PRIVADO/monitor-$(agora_ms).png"
  timeout 10 grim -o "$NOME" "$completa" || return 1
  magick "$completa" -crop "${4}x${5}+${2}+${3}" +repage "$1"
  local ok=$?
  rm -f "$completa"
  return $ok
}

# --- 4. nitidez ---------------------------------------------------------------
if [ "$ACESA" != true ]; then
  falhou "nitidez: NÃO VERIFICADA — a tela de $NOME está apagada (DPMS) e o grim só captura com ela acesa"
elif ! "$RAIZ/scripts/capturar-pet.sh" "$PRIVADO/nitidez"; then
  falhou "nitidez: não consegui um quadro estável e uma captura"
else
  Q="$PRIVADO/nitidez"
  cp "$Q/esperado.png" "$SAIDA/esperado.png"
  read -r QX QY QW QH QD GX GY < <(jq -r '"\(.x) \(.y) \(.w) \(.h) \(.d) \(.grade.x) \(.grade.y)"' "$Q/quadro.json")
  if "$XTASK" nitidez --captura "$Q/monitor.png" --esperado "$Q/esperado.png" \
    --x "$QX" --y "$QY" --d "$QD" --grade "$GX,$GY" >"$SAIDA/nitidez.txt" 2>&1; then
    passou "nitidez: $(head -n2 "$SAIDA/nitidez.txt" | tr '\n' ' ')"
  else
    falhou "nitidez: $(tr '\n' ' ' <"$SAIDA/nitidez.txt")"
  fi
  # Foto para o PR: só os pixels opacos do pet (máscara = alfa 255 do esperado).
  magick "$Q/monitor.png" -crop "${QW}x${QH}+${QX}+${QY}" +repage \
    \( "$Q/esperado.png" -alpha extract -threshold 99.9% \) -compose CopyOpacity -composite \
    -background '#202028' -compose Over -flatten "$SAIDA/pet.png" &&
    magick "$SAIDA/pet.png" -filter point -resize 300% "$SAIDA/pet-x3.png"
  rm -f "$Q/monitor.png"
fi

# --- 5. sem pixel velho e sem fantasma ------------------------------------------
if [ "$ACESA" != true ]; then
  falhou "pixel velho e fantasma: NÃO VERIFICADOS — tela apagada (DPMS)"
else
  read -r SX SY SW SH < <(campo '.sprite_disp | "\(.x) \(.y) \(.w) \(.h)"')
  F="$PRIVADO/fantasma"
  mkdir -p "$F"
  post /v1/debug/esconder ""
  esperar_campo 5 .visivel false && sleep 1.5 # o fade de saída termina
  BASES=()
  for n in 1 2 3; do
    recortar "$F/base-$n.png" "$SX" "$SY" "$SW" "$SH" && BASES+=(--base "$F/base-$n.png")
    sleep 0.3
  done
  post /v1/debug/mostrar ""
  esperar_campo 10 .visivel true && sleep 1
  if [ "${#BASES[@]}" -lt 6 ] || ! "$RAIZ/scripts/capturar-pet.sh" "$F/com-pet"; then
    falhou "pixel velho e fantasma: capturas falharam"
  else
    magick "$F/com-pet/monitor.png" -crop "${SW}x${SH}+${SX}+${SY}" +repage "$F/com-pet.png"
    rm -f "$F/com-pet/monitor.png"
    if "$XTASK" fantasma "${BASES[@]}" --depois "$F/com-pet.png" \
      --esperado "$F/com-pet/esperado.png" >"$SAIDA/pixel-velho.txt" 2>&1; then
      passou "sem pixel velho: $(head -n1 "$SAIDA/pixel-velho.txt")"
    else
      falhou "pixel velho: $(tr '\n' ' ' <"$SAIDA/pixel-velho.txt")"
    fi
    # Esconde e captura durante o fade de saída e depois dele.
    post /v1/debug/esconder ""
    sleep 0.1
    ERROS=""
    for n in 1 2 3; do
      recortar "$F/depois-$n.png" "$SX" "$SY" "$SW" "$SH" || { ERROS+=" captura $n falhou;"; continue; }
      "$XTASK" fantasma "${BASES[@]}" --depois "$F/depois-$n.png" >"$SAIDA/fantasma-$n.txt" 2>&1 ||
        ERROS+=" $(tr '\n' ' ' <"$SAIDA/fantasma-$n.txt");"
      sleep 0.25
    done
    if [ -z "$ERROS" ]; then
      passou "esconder sem fantasma: $(head -n1 "$SAIDA/fantasma-1.txt") (3 capturas em ~1 s)"
    else
      falhou "fantasma ao esconder:$ERROS"
    fi
    post /v1/debug/mostrar ""
    esperar_campo 10 .visivel true
  fi
fi

# --- 6. orçamentos do container ---------------------------------------------------
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
NOTA=""
[ "$ACESA" != true ] && NOTA=", tela apagada: sem animação"
if awk -v c="$CPU" 'BEGIN{exit !(c < 1)}'; then
  passou "CPU do container parado: ${CPU}% (< 1%, média de 5 amostras${NOTA})"
else
  falhou "CPU do container parado: ${CPU}% (orçamento 1%)"
fi
# Commits numa janela fixa, contada a partir de agora (o commits_por_min é
# uma janela móvel que ainda inclui a subida e os testes acima).
JANELA=20
[ "$ACESA" != true ] && JANELA=10
K0="$(campo .commits_total)"
sleep "$JANELA"
K1="$(campo .commits_total)"
DK=$((K1 - K0))
if [ "$ACESA" = true ]; then
  if awk -v k="$DK" -v t="$JANELA" 'BEGIN{exit !(k / t <= 2.0 && k > 0)}'; then
    passou "commits parado: $DK em ${JANELA} s ($(awk -v k="$DK" -v t="$JANELA" 'BEGIN{printf "%.2f", k/t}')/s, orçamento 2/s)"
  else
    falhou "commits parado: $DK em ${JANELA} s (orçamento 2/s, e o repouso tem de animar)"
  fi
else
  if [ "$DK" = 0 ]; then
    passou "tela apagada: 0 commits em ${JANELA} s (o pet espera o frame callback)"
  else
    falhou "tela apagada: $DK commits em ${JANELA} s (esperado 0)"
  fi
  falhou "commits parado com a tela acesa: NÃO VERIFICADO — tela apagada (DPMS)"
fi

# --- 7. clique ---------------------------------------------------------------
REGIAO="$(campo_json .regiao_entrada)"
if [ "$REGIAO" != null ] && [ -n "$REGIAO" ] &&
  [ "$(jq -r --argjson w "$MW" --argjson h "$MH" '(.w * .h) < ($w * $h / 50)' <<<"$REGIAO")" = true ]; then
  passou "região de input só no corpo: $REGIAO (lógico)"
else
  falhou "região de input: $REGIAO"
fi
manual "clique: clicar AO LADO do pet tem que chegar na janela de baixo; em cima dele o cursor vira «pegar» (conferir à mão)"

# --- 8. reinício -------------------------------------------------------------
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
if docker logs "$CONTAINER" 2>&1 | grep -q 'o compositor processou o quadro transparente'; then
  passou "saída confirmada: o compositor processou o quadro transparente antes do SIGTERM terminar"
else
  falhou "saída sem confirmação do compositor no log do restart"
fi

# --- 9. troca para a produção sem fantasma, e crash ----------------------------
echo "▸ trocando para a pilha de produção (SIGTERM no daemon de dev)"
if [ "$ACESA" = true ] && read -r SX SY SW SH < <(campo '.sprite_disp | "\(.x) \(.y) \(.w) \(.h)"') &&
  [ -n "$SX" ] && [ "$SX" != null ]; then
  T="$PRIVADO/troca"
  mkdir -p "$T"
  post /v1/debug/esconder ""
  esperar_campo 5 .visivel false && sleep 1.5
  BASES=()
  for n in 1 2; do
    recortar "$T/base-$n.png" "$SX" "$SY" "$SW" "$SH" && BASES+=(--base "$T/base-$n.png")
    sleep 0.3
  done
  post /v1/debug/mostrar ""
  esperar_campo 10 .visivel true && sleep 1
  "${PROD[@]}" up -d >/dev/null 2>&1 &
  TROCA=$!
  ERROS=""
  for n in 1 2 3 4; do
    sleep 0.3
    recortar "$T/depois-$n.png" "$SX" "$SY" "$SW" "$SH" || continue
    "$XTASK" fantasma "${BASES[@]}" --depois "$T/depois-$n.png" >"$SAIDA/troca-$n.txt" 2>&1 ||
      ERROS+=" captura $n: $(head -n1 "$SAIDA/troca-$n.txt");"
  done
  wait "$TROCA"
  if [ -z "$ERROS" ]; then
    passou "SIGTERM sem fantasma: a área do pet volta a ser o fundo durante a troca"
  else
    falhou "fantasma no SIGTERM:$ERROS"
  fi
else
  "${PROD[@]}" up -d >/dev/null 2>&1
  [ "$ACESA" != true ] && falhou "SIGTERM sem fantasma: NÃO VERIFICADO — tela apagada (DPMS)"
fi
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

# --- 10. sem compositor ----------------------------------------------------------
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
