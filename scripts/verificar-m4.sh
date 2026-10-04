#!/usr/bin/env bash
# scripts/verificar-m4.sh — verificação do M4 na tela de verdade (PLANO.md,
# "M4 — Arrastar, seguir o monitor ativo e levar ao terminal").
#
# Roda contra o pet que está de pé (bin/pet subir) e confere:
#   1. o desktop no /v1/estado: o socket de eventos do Hyprland ligado, os
#      dois protocolos do clique (`zwlr_foreign_toplevel_manager_v1` e
#      `hyprland_toplevel_mapping_manager_v1`, decisão 0056) ligados, janelas
#      com endereço, e o anel de ativações e as janelas das sessões só com
#      endereços (hexadecimal, nada que pareça título);
#   2. a janela ativa e o monitor em foco que o pet sabe são os do Hyprland
#      (`hyprctl -j activewindow` e `hyprctl -j monitors`, só leitura);
#   3. o clique leva ao terminal (decisão 0057), com a tela acesa e
#      desbloqueada: abre dois `foot` (A e B) com um título-canário
#      (`SEGREDO-M4-…`), e com cada um em foco manda o começo e o prompt de
#      uma sessão de TESTE (`teste: true`, como o `bin/pet testar`; o pet as
#      esquece em 60 s). A pede permissão (esperando você) e B termina
#      (pronto). O primeiro `bin/pet clique` tem de levar ao foot de A (o
#      `hyprctl -j activewindow`), o segundo ao de B, e o terceiro mostra a
#      lista das sessões. Depois, o canário: o título nunca aparece no log do
#      pet, no /v1/estado nem no /v1/debug/eventos. Os dois `foot` fecham no
#      fim, até numa falha;
#   4. `--manual` (com o Renan, que mexe no mouse): arrastar numa área de
#      trabalho cheia e numa vazia, a posição depois de um restart, a
#      proteção de tela (`omarchy-launch-screensaver force`) escondendo o pet
#      e ele voltando, e o clique de verdade (esquerdo: o balão; direito: a
#      soneca e o selo zZ).
#
# Com a sessão bloqueada ou a tela apagada, o 3 e o 4 saem NÃO VERIFICADOS
# (contam como falha) e nenhuma janela é aberta. Com avisos de sessões reais
# pendentes o 3 também: o clique iria a eles primeiro (as reais vêm antes
# das de teste).
#
# Do Hyprland, só consultas de leitura (`hyprctl -j`): nunca `dispatch` nem
# `keyword`. O `foot` abre na área de trabalho de agora. Nada fica em disco
# além do resumo em tmp/m4-ao-vivo/ (sem título nenhum).
set -uo pipefail

RAIZ="$(cd "$(dirname "$(readlink -f "${BASH_SOURCE[0]}")")/.." && pwd)"
cd "$RAIZ" || exit 1

MANUAL=0
case "${1:-}" in
  --manual) MANUAL=1 ;;
  "") ;;
  *)
    echo "uso: scripts/verificar-m4.sh [--manual]" >&2
    exit 2
    ;;
esac

URL="http://127.0.0.1:${PET_PORTA:-27380}"
SAIDA="$RAIZ/tmp/m4-ao-vivo/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$SAIDA"

declare -a LINHAS=()
FALHAS=0
passou() { LINHAS+=("✓ $1"); printf '✓ %s\n' "$1"; }
falhou() { LINHAS+=("✗ $1"); printf '✗ %s\n' "$1" >&2; FALHAS=$((FALHAS + 1)); }
nota() { LINHAS+=("• $1"); printf '• %s\n' "$1"; }

api() { curl -q --noproxy '*' -fsS -m 3 -H 'X-Pet: 1' "${URL}$1"; }
estado() { api /v1/estado 2>/dev/null; }
campo() { estado | jq -r "$1" 2>/dev/null; }
campo_c() { estado | jq -c "$1" 2>/dev/null; }
# campo_e <filtro> <valor>: lido de novo a cada chamada (para o `esperar`).
campo_e() { [ "$(campo "$1")" = "$2" ]; }
agora_ms() { echo $(($(date +%s%N) / 1000000)); }

# esperar <segundos> <comando…>: até o comando dar certo.
esperar() {
  local limite=$(($(agora_ms) + $1 * 1000))
  shift
  while [ "$(agora_ms)" -lt "$limite" ]; do
    "$@" && return 0
    sleep 0.1
  done
  return 1
}

# A janela ativa no Hyprland (só leitura): o endereço sem 0x e o pid.
ativa_endereco() { hyprctl -j activewindow 2>/dev/null | jq -r '.address // empty' | sed 's/^0x//'; }
ativa_pid() { hyprctl -j activewindow 2>/dev/null | jq -r '.pid // 0'; }

resumo() {
  printf '%s\n' "${LINHAS[@]}" >"$SAIDA/resumo.txt"
  echo
  if [ "$FALHAS" -eq 0 ]; then
    echo "✓ M4 ao vivo: tudo verde (resumo em $SAIDA/resumo.txt)"
  else
    echo "✗ M4 ao vivo: $FALHAS falha(s) ou NÃO VERIFICADO(S) (resumo em $SAIDA/resumo.txt)" >&2
  fi
}

# --- 0. o pet e a tela -----------------------------------------------------------
if ! estado >/dev/null; then
  falhou "o pet não respondeu em ${URL} — está de pé? (bin/pet subir)"
  resumo
  exit 1
fi
INICIO="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
FOCADO="$(hyprctl -j monitors | jq -c '.[] | select(.focused)' | head -n1)"
BLOQUEADA="$(hyprctl -j monitors | jq -r 'any(.[]; (.solitaryBlockedBy // []) | index("LOCK") != null)')"
LIVRE=true
if [ "$BLOQUEADA" = true ]; then
  LIVRE=false
  MOTIVO="sessão bloqueada (o Hyprland recusa o foco e a tela de senha cobre o pet)"
elif [ "$(jq -r .dpmsStatus <<<"$FOCADO")" != true ]; then
  LIVRE=false
  MOTIVO="tela apagada (DPMS)"
fi
printf '▸ pet %s (fonte %s), tela %s, skin %s, D=%s\n' "$(campo .versao)" \
  "$(campo '.fonte[0:12]')" "$(campo .tela)" "$(campo '.skin.id')" "$(campo .d)"

# --- 1. o desktop no /v1/estado ------------------------------------------------
E="$(estado)"
if [ "$(jq -r .desktop.eventos <<<"$E")" = ligado ]; then
  passou "eventos do Hyprland: o socket2 ligado"
else
  falhou "eventos do Hyprland: desktop.eventos=$(jq -r .desktop.eventos <<<"$E") (esperado ligado)"
fi
PROTOCOLOS="$(jq -r '.desktop.protocolos | join(", ")' <<<"$E")"
if jq -e '(.desktop.protocolos | map(split(" ")[0])) as $p
    | ($p | index("zwlr_foreign_toplevel_manager_v1")) != null
      and ($p | index("hyprland_toplevel_mapping_manager_v1")) != null
      and .desktop.foca_janelas == true' <<<"$E" >/dev/null; then
  passou "focar janelas: $PROTOCOLOS; $(jq -r .desktop.janelas <<<"$E") janela(s) com endereço"
else
  falhou "focar janelas: protocolos «$PROTOCOLOS», foca_janelas=$(jq -r .desktop.foca_janelas <<<"$E")"
fi
# Só endereços: hexadecimal nas janelas do anel, na ativa e nas das sessões,
# e o anel só com as chaves de sempre.
if jq -e '
    def endereco: . == null or (type == "string" and test("^[0-9a-f]{1,16}$"));
    (.desktop.anel | all(.janela | endereco))
    and (.desktop.anel | all(keys - ["em_ms", "janela", "tipo"] | length == 0))
    and (.desktop.janela_ativa | endereco)
    and (.sessoes | all(.janela.endereco | endereco))' <<<"$E" >/dev/null; then
  passou "anel de ativações e janelas das sessões só com endereços ($(jq '.desktop.anel | length' <<<"$E") troca(s) no anel)"
else
  falhou "o /v1/estado tem janela que não é endereço: $(jq -c '{anel: .desktop.anel, ativa: .desktop.janela_ativa}' <<<"$E")"
fi

# --- 2. a janela ativa e o monitor em foco são os do Hyprland -------------------
HA="$(ativa_endereco)"
PA="$(jq -r '.desktop.janela_ativa // empty' <<<"$E")"
if [ "$HA" = "$PA" ]; then
  passou "janela ativa: a do Hyprland (${PA:-nenhuma})"
else
  falhou "janela ativa: o pet diz «${PA:-nenhuma}», o Hyprland «${HA:-nenhuma}»"
fi
HM="$(jq -r .name <<<"$FOCADO")"
PM="$(jq -r '.desktop.monitor_em_foco // empty' <<<"$E")"
if [ -z "$PM" ]; then
  nota "monitor em foco: o socket2 ainda não contou troca nenhuma desde que o pet subiu (o pet está em $(jq -r .monitor <<<"$E"), o Hyprland focado é $HM)"
elif [ "$PM" = "$HM" ]; then
  passou "monitor em foco: $PM, o do Hyprland"
else
  falhou "monitor em foco: o pet diz «$PM», o Hyprland «$HM»"
fi

# --- 3. o clique leva ao terminal ------------------------------------------------
REAIS="$(jq '[.sessoes[] | select(.teste == false and .aviso != null)] | length' <<<"$E")"
declare -a FOOTS=()
SID_A=""
SID_B=""
fechar() {
  for pid in "${FOOTS[@]}"; do
    kill "$pid" 2>/dev/null || true
  done
  FOOTS=()
  # As sessões de teste saem caladas (o motivo `clear` não dá tchau).
  for sid in "$SID_A" "$SID_B"; do
    [ -n "$sid" ] && hook "$sid" m4 SessionEnd '{"reason":"clear"}' 2>/dev/null
  done
  SID_A=""
  SID_B=""
}
trap fechar EXIT

# hook <sid> <proj> <evento> [json a mais]: um evento de TESTE no fio v1, com
# o `ts` de agora (o que casa a janela ativa).
hook() {
  local extra="${4:-}"
  [ -z "$extra" ] && extra='{}'
  jq -nc --arg e "$3" --arg sid "$1" --arg proj "$2" --argjson ts "$(agora_ms)" \
    --argjson extra "$extra" \
    '{v: 1, e: $e, ts: $ts, sid: $sid, turno: ($sid + "-p"), ent: "cli", proj: $proj,
      teste: true} + $extra' |
    curl -q --noproxy '*' -fsS -m 3 -X POST -H 'X-Pet: 1' -H 'Content-Type: application/json' \
      --data-binary @- "${URL}/v1/evento" >/dev/null
}

sessao() { estado | jq -c --arg s "${1:0:8}" '.sessoes[] | select(.sid8 == $s)'; }
aviso_de() { sessao "$1" | jq -r '.aviso.tipo // empty'; }
sem_aviso() { [ -z "$(aviso_de "$1")" ]; }
ativa_e() { [ "$(ativa_endereco)" = "$1" ]; }
ativa_e_o_pid() { [ "$(ativa_pid)" = "$1" ]; }

# abrir_foot <título>: o foot com o título, até ele ficar em foco.
abrir_foot() {
  foot -T "$1" sleep 300 >/dev/null 2>&1 &
  local pid=$!
  FOOTS+=("$pid")
  esperar 5 ativa_e_o_pid "$pid"
}

clique_leva_ao_terminal() {
  local segredo ea eb r
  segredo="SEGREDO-M4-$(od -An -N4 -tx1 /dev/urandom | tr -d ' \n')"
  SID_A="$(od -An -N8 -tx1 /dev/urandom | tr -d ' \n')-m4-a"
  SID_B="$(od -An -N8 -tx1 /dev/urandom | tr -d ' \n')-m4-b"
  echo "▸ abrindo dois foot (o título é um canário) e duas sessões de teste"
  if ! abrir_foot "$segredo-A"; then
    falhou "clique: o foot A não ficou em foco em 5 s"
    return
  fi
  ea="$(ativa_endereco)"
  # O prompt sai 1,5 s depois da troca (a menos de 1 s há dúvida).
  sleep 1.5
  hook "$SID_A" m4-a SessionStart '{"src":"startup"}'
  hook "$SID_A" m4-a UserPromptSubmit
  if ! abrir_foot "$segredo-B"; then
    falhou "clique: o foot B não ficou em foco em 5 s"
    return
  fi
  eb="$(ativa_endereco)"
  sleep 1.5
  hook "$SID_B" m4-b SessionStart '{"src":"startup"}'
  hook "$SID_B" m4-b UserPromptSubmit
  sleep 0.3
  local ja jb
  ja="$(sessao "$SID_A" | jq -r '.janela.endereco // empty')"
  jb="$(sessao "$SID_B" | jq -r '.janela.endereco // empty')"
  if [ "$ja" = "$ea" ] && [ "$jb" = "$eb" ] && [ -n "$ea" ] && [ "$ea" != "$eb" ]; then
    passou "identidade: cada sessão casou com o próprio foot ($ea e $eb) pelo ts do prompt"
  else
    falhou "identidade: A=${ja:-?} (esperado $ea), B=${jb:-?} (esperado $eb)"
  fi
  hook "$SID_A" m4-a PermissionRequest '{"tool":"Bash"}'
  hook "$SID_B" m4-b Stop
  sleep 1.2 # a acomodação do Stop (0,8 s)
  if [ "$(aviso_de "$SID_A")" = esperando ] && [ "$(aviso_de "$SID_B")" = pronto ]; then
    passou "avisos: A esperando você, B pronto"
  else
    falhou "avisos: A=$(aviso_de "$SID_A"), B=$(aviso_de "$SID_B") (esperado esperando e pronto)"
  fi
  # Com B em foco, o primeiro clique vai a A (a mais urgente).
  r="$("$RAIZ/bin/pet" clique)"
  if [ "$(jq -r .acao <<<"$r")" = focou ] && [ "$(jq -r .janela <<<"$r")" = "$ea" ] &&
    esperar 2 ativa_e "$ea" && esperar 2 sem_aviso "$SID_A"; then
    passou "1º clique: foco no foot de A (esperando você), e o aviso de A saiu"
  else
    falhou "1º clique: $(jq -c . <<<"$r"); ativa=$(ativa_endereco), aviso de A=$(aviso_de "$SID_A")"
  fi
  r="$("$RAIZ/bin/pet" clique)"
  if [ "$(jq -r .acao <<<"$r")" = focou ] && [ "$(jq -r .janela <<<"$r")" = "$eb" ] &&
    esperar 2 ativa_e "$eb" && esperar 2 sem_aviso "$SID_B"; then
    passou "2º clique: foco no foot de B (pronto), e o aviso de B saiu"
  else
    falhou "2º clique: $(jq -c . <<<"$r"); ativa=$(ativa_endereco), aviso de B=$(aviso_de "$SID_B")"
  fi
  r="$("$RAIZ/bin/pet" clique)"
  if [ "$(jq -r .acao <<<"$r")" = lista ] && [ "$(campo '.balao | length')" -ge 2 ]; then
    passou "3º clique, sem aviso: o balão com as $(jq -r .sessoes <<<"$r") sessões"
  else
    falhou "3º clique: $(jq -c . <<<"$r"); balão $(campo_c .balao)"
  fi
  # O canário do título: nem no log, nem no /v1/estado, nem no debug.
  local onde=""
  docker compose logs --no-log-prefix --since "$INICIO" bichinho 2>&1 | grep -qi segredo && onde="$onde log"
  estado | grep -qi segredo && onde="$onde /v1/estado"
  if [ "$(campo .debug)" = true ]; then
    api /v1/debug/eventos 2>/dev/null | grep -qi segredo && onde="$onde /v1/debug/eventos"
  fi
  if [ -z "$onde" ]; then
    passou "canário do título: os títulos dos dois foot não apareceram no log nem no /v1/estado"
  else
    falhou "canário do título: o título de uma janela vazou em:$onde"
  fi
  fechar
  if esperar 5 sem_foots; then
    passou "os dois foot fecharam"
  else
    falhou "um foot do teste ficou aberto"
  fi
}

# Nenhuma janela do teste aberta (o título só é lido aqui, no host).
sem_foots() {
  [ "$(hyprctl -j clients | jq '[.[] | select(.title | startswith("SEGREDO-M4-"))] | length')" = 0 ]
}

if [ "$LIVRE" != true ]; then
  falhou "clique leva ao terminal: NÃO VERIFICADO — $MOTIVO; nenhuma janela foi aberta"
elif [ "$REAIS" -gt 0 ]; then
  falhou "clique leva ao terminal: NÃO VERIFICADO — $REAIS aviso(s) de sessões reais pendente(s): o clique iria a eles primeiro"
elif ! command -v foot >/dev/null; then
  falhou "clique leva ao terminal: NÃO VERIFICADO — sem o foot"
else
  clique_leva_ao_terminal
fi

# --- 4. com o Renan (--manual) -----------------------------------------------
pausa() {
  printf '\n  %s\n  [Enter quando terminar] ' "$1"
  read -r _
}
celula() { campo '.sprite_disp | "\(.x),\(.y)"'; }
manual() {
  local antes depois
  antes="$(celula)"
  pausa "Numa área de trabalho com janelas, arraste o Zeca para outro lugar e solte."
  depois="$(celula)"
  if [ "$depois" != "$antes" ] && [ "$(campo .arrastando)" = false ]; then
    passou "arrastar numa área cheia: o pet foi de $antes para $depois"
  else
    falhou "arrastar numa área cheia: célula $antes → $depois, arrastando=$(campo .arrastando)"
  fi
  antes="$depois"
  pausa "Vá a uma área de trabalho vazia (Super+número), arraste o Zeca de novo e solte."
  depois="$(celula)"
  if [ "$depois" != "$antes" ]; then
    passou "arrastar numa área vazia: o pet foi de $antes para $depois"
  else
    falhou "arrastar numa área vazia: o pet não mudou de lugar ($depois)"
  fi
  echo "▸ reiniciando o pet (docker compose restart bichinho)"
  docker compose restart bichinho >/dev/null 2>&1
  esperar 20 campo_e .tela ativa
  sleep 1
  if [ "$(celula)" = "$depois" ]; then
    passou "a posição sobreviveu ao restart ($depois)"
  else
    falhou "a posição depois do restart: $(celula) (esperado $depois)"
  fi
  echo "▸ omarchy-launch-screensaver force"
  omarchy-launch-screensaver force >/dev/null 2>&1 &
  if esperar 10 campo_e .desktop.protetor_de_tela true &&
    esperar 5 campo_e .visivel false; then
    passou "a proteção de tela esconde o pet"
  else
    falhou "a proteção de tela: protetor=$(campo .desktop.protetor_de_tela), visivel=$(campo .visivel)"
  fi
  pausa "Mexa o mouse para fechar a proteção de tela."
  if esperar 10 campo_e .visivel true; then
    passou "o pet voltou quando a proteção de tela fechou"
  else
    falhou "o pet não voltou depois da proteção de tela"
  fi
  pausa "Clique com o botão ESQUERDO no Zeca (sem arrastar)."
  if [ "$(campo '.balao | length')" -ge 1 ]; then
    passou "clique esquerdo de verdade: o balão ($(campo_c .balao))"
  else
    falhou "clique esquerdo de verdade: nenhum balão"
  fi
  pausa "Clique com o botão DIREITO no Zeca."
  if [ "$(campo .soneca_restante_s)" != null ]; then
    passou "clique direito: a soneca ($(campo .soneca_restante_s) s) com o selo zZ"
  else
    falhou "clique direito: sem soneca"
  fi
  pausa "Clique com o botão DIREITO de novo (acorda)."
  if [ "$(campo .soneca_restante_s)" = null ]; then
    passou "clique direito de novo: acordou"
  else
    falhou "clique direito de novo: ainda cochilando"
  fi
}

if [ "$MANUAL" = 1 ]; then
  if [ "$LIVRE" = true ]; then
    manual
  else
    falhou "manual: NÃO VERIFICADO — $MOTIVO"
  fi
else
  nota "arrastar, a posição depois do restart, a proteção de tela e o clique com o mouse: scripts/verificar-m4.sh --manual, com o Renan"
fi

resumo
[ "$FALHAS" -eq 0 ]
