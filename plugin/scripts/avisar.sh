#!/bin/sh
# avisar.sh — hook do plugin bichinho: conta ao claude-pet o que o Claude
# Code acabou de fazer (decisões 0009 e 0019).
#
#   sh avisar.sh <Evento>        # o JSON do hook chega pela entrada padrão
#
# Regras de ouro (CLAUDE.md):
# - só METADADOS saem daqui, montados por uma lista branca do jq: nome do
#   evento, ids opacos, nome da ferramenta, enums, contagens, durações, o
#   hash do caminho editado e o nome da pasta do projeto. Prompt, código,
#   resposta, texto de erro, títulos e caminhos nunca saem do host;
# - não imprime nada, nem erro;
# - SEMPRE sai 0: um Stop hook que saísse com 2 seguraria o Claude.
#
# Sem jq, ou com uma entrada que não é JSON, manda só {"v":1,"e":"<Evento>"}.
# Com o pet desligado o curl desiste sozinho: conexão recusada na hora, ou
# 2 s no pior caso.
#
# Ambiente:
#   PET_PORTA               porta do pet no loopback (padrão 27380)
#   PET_TESTE=1             evento sintético do `bin/pet testar`: o pet o
#                           isola das sessões reais e o esquece em 60 s
#   CLAUDE_CODE_ENTRYPOINT  posto pelo Claude Code (cli, sdk-cli, …)
#   XDG_STATE_HOME          onde o Omarchy guarda o "não perturbe"

exec >/dev/null 2>&1

evento=${1-}
case $evento in
'' | *[!A-Za-z]*) exit 0 ;;
esac
[ "${#evento}" -le 40 ] || exit 0

# Hora do evento em ms, antes de tudo: os hooks async podem chegar ao pet
# fora de ordem, e é por ela que o pet ordena.
ts=$(date +%s%3N 2>/dev/null)
case $ts in
'' | *[!0-9]*) ts=$(date +%s 2>/dev/null)000 ;;
esac
case $ts in
'' | *[!0-9]* | 000) ts= ;;
esac

porta=${PET_PORTA:-27380}
case $porta in
'' | *[!0-9]*) porta=27380 ;;
esac
[ "${#porta}" -le 5 ] || porta=27380
url="http://127.0.0.1:$porta/v1/evento"

teste=false
if [ "${PET_TESTE-}" = 1 ]; then
  teste=true
fi

enviar() {
  curl -sS -m 2 -o /dev/null -H "Content-Type: application/json" -H "X-Pet: 1" \
    --data-binary @- "$url" >/dev/null 2>&1
}

# O mínimo, quando não dá para montar o resto ($evento já é só letras).
minimo() {
  if [ "$teste" = true ]; then
    printf '{"v":1,"e":"%s","teste":true}' "$evento"
  else
    printf '{"v":1,"e":"%s"}' "$evento"
  fi
}

if ! command -v jq >/dev/null 2>&1; then
  minimo | enviar
  exit 0
fi

# O JSON do hook fica só na memória deste processo.
entrada=$(cat)

# "Não perturbe" do Omarchy: só o booleano sai do arquivo.
dnd=false
estado_omarchy="${XDG_STATE_HOME:-${HOME-}/.local/state}/omarchy/notifications.json"
if [ -r "$estado_omarchy" ] && jq -e '.dnd == true' "$estado_omarchy" >/dev/null 2>&1; then
  dnd=true
fi

resumo() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256
  else
    openssl dgst -sha256 -r
  fi
}

# Arquivo editado: só os 12 primeiros hexadecimais do sha256 do caminho, para
# o pet contar arquivos diferentes sem saber quais são.
arq=
if [ "$evento" = PostToolUse ]; then
  caminho=$(printf '%s' "$entrada" | jq -j '
    select(.tool_name == "Edit" or .tool_name == "Write"
      or .tool_name == "MultiEdit" or .tool_name == "NotebookEdit")
    | .tool_input | (.file_path // .notebook_path) | strings' 2>/dev/null)
  if [ -n "$caminho" ]; then
    arq=$(printf '%s' "$caminho" | resumo 2>/dev/null | cut -c1-12)
  fi
  caminho=
fi

# A lista branca: cada campo lido aqui passa por um validador (os mesmos de
# pet_core::evento) e só vale no evento que o tem. Nenhum outro campo do
# hook é lido.
corpo=$(printf '%s' "$entrada" | jq -c \
  --arg e "$evento" --arg ts "$ts" --arg ent "${CLAUDE_CODE_ENTRYPOINT-}" \
  --arg arq "$arq" --argjson dnd "$dnd" --argjson teste "$teste" '
  def tok($n): if type == "string" and length >= 1 and length <= $n
    and test("^[A-Za-z0-9_.:-]+$") then . else null end;
  def enum: if type == "string" and test("^[a-z_]{1,40}$") then . else null end;
  def bool: if type == "boolean" then . else null end;
  def natural($max): if type == "number" and . >= 0 and . <= $max
    then floor else null end;
  # "MCP task" -> "mcp_task", "auto-mode scan" -> "auto_mode_scan"
  def tipo: if type == "string" then ascii_downcase | gsub("[^a-z_]+"; "_")
    | .[0:40] | if length > 0 then . else null end else null end;
  # Só o nome da última pasta do cwd, nunca o caminho.
  def pasta: if type == "string"
    then (split("/") | map(select(length > 0)) | last)
      | if type == "string" and test("^[\\p{L}\\p{M}\\p{N} _.-]{1,64}$")
          and (test("^[.]+$") | not) then . else null end
    else null end;
  def de_ferramenta: $e == "PreToolUse" or $e == "PostToolUse"
    or $e == "PostToolUseFailure" or $e == "PermissionRequest";
  (if (.background_tasks | type) == "array" then .background_tasks else [] end) as $bg
  | ([$bg[] | select(type == "object") | {t: (.type | tipo), i: (.id | tok(64))}
      | select(.t != null and .i != null)] | .[0:16]) as $tarefas
  | {
      v: 1,
      e: $e,
      ts: (if $ts == "" then null else ($ts | tonumber) end),
      sid: (.session_id | tok(64)),
      turno: (.prompt_id | tok(64)),
      agente: (if (.agent_id | type) == "string" then true else null end),
      aid: (.agent_id | tok(64)),
      tool: (if de_ferramenta then (.tool_name | tok(128)) else null end),
      nt: (if $e == "Notification" then (.notification_type | enum) else null end),
      err: (if $e == "StopFailure" then (.error | enum) else null end),
      src: (if $e == "UserPromptSubmit" or $e == "SessionStart"
        then (.source | enum) else null end),
      reason: (if $e == "SessionEnd" then (.reason | enum) else null end),
      intr: (if $e == "PostToolUseFailure" then (.is_interrupt | bool) else null end),
      sha: (if $e == "Stop" then (.stop_hook_active | bool) else null end),
      bg: (if $e == "Stop" and (.background_tasks | type) == "array"
        then ($bg | length) else null end),
      bgt: (if $e == "Stop" and ($tarefas | length) > 0 then [$tarefas[].t] else null end),
      bgi: (if $e == "Stop" and ($tarefas | length) > 0 then [$tarefas[].i] else null end),
      dur: (if $e == "PostToolUse" or $e == "PostToolUseFailure"
        then (.duration_ms | natural(86400000)) else null end),
      arq: (if $arq | test("^[0-9a-f]{12}$") then $arq else null end),
      proj: (.cwd | pasta),
      ent: (if $ent | test("^[a-z0-9_-]{1,40}$") then $ent else null end),
      dnd: $dnd,
      teste: (if $teste then true else null end)
    }
  | with_entries(select(.value != null))' 2>/dev/null)
entrada=

if [ -z "$corpo" ]; then
  corpo=$(minimo)
fi
printf '%s' "$corpo" | enviar
exit 0
