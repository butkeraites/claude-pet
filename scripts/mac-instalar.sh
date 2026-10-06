#!/usr/bin/env bash
# scripts/mac-instalar.sh — instala o bichinho no macOS (M8, T9.3, decisão 0104).
#
# Monta o Bichinho.app (scripts/mac-empacotar.sh), põe o hook no PATH e, com
# os pedidos, instala o LaunchAgent e o plugin. As ações fora do repositório
# (PATH, LaunchAgent, plugin global) são avisadas; o plugin, a mais sensível
# (vale para todas as sessões do Claude), pede confirmação.
#
# Uso:
#   scripts/mac-instalar.sh [--launchagent] [--plugin] [--app <caminho.app>]
set -euo pipefail

RAIZ="$(cd "$(dirname "$0")/.." && pwd)"
cd "$RAIZ"

APP="$HOME/Applications/Bichinho.app"
COM_LAUNCHAGENT=0
COM_PLUGIN=0
while [ "$#" -gt 0 ]; do
  case "$1" in
    --launchagent) COM_LAUNCHAGENT=1 ;;
    --plugin) COM_PLUGIN=1 ;;
    --app) shift; APP="$1" ;;
    *) echo "opção desconhecida: $1" >&2; exit 2 ;;
  esac
  shift
done

APP_SUPPORT="$HOME/Library/Application Support/bichinho"
BIN_PATH="$HOME/.local/bin/bichinho"
AGENTE="$HOME/Library/LaunchAgents/dev.bichinho.pet.plist"
BIN_APP="$APP/Contents/MacOS/bichinho"

# 1. monta o .app
mkdir -p "$(dirname "$APP")"
scripts/mac-empacotar.sh "$APP"

# 2. o hook no PATH: um link para o binário do .app (a mesma assinatura, então
#    o TCC vale para os dois). O Claude Code acha «bichinho» pelo PATH.
echo "▸ pondo o hook no PATH: $BIN_PATH -> $BIN_APP"
mkdir -p "$(dirname "$BIN_PATH")"
ln -sf "$BIN_APP" "$BIN_PATH"
case ":$PATH:" in
  *":$(dirname "$BIN_PATH"):"*) : ;;
  *) echo "  atenção: $(dirname "$BIN_PATH") não está no PATH; acrescente ao seu shell" >&2 ;;
esac

# 3. pasta de dados e config (o daemon lê o bichinho.toml de lá).
mkdir -p "$APP_SUPPORT"
if [ ! -f "$APP_SUPPORT/bichinho.toml" ]; then
  echo "▸ criando um config padrão em $APP_SUPPORT/bichinho.toml"
  cat > "$APP_SUPPORT/bichinho.toml" <<'TOML'
# Config do bichinho (macOS). Troque a skin e o tamanho aqui.
[aparencia]
skin = "zeca-livre-escuro"
tamanho = "pequeno"
TOML
fi

# 4. LaunchAgent (sobe no login), se pedido.
if [ "$COM_LAUNCHAGENT" -eq 1 ]; then
  echo "▸ instalando o LaunchAgent em $AGENTE"
  mkdir -p "$(dirname "$AGENTE")" "$HOME/Library/Logs"
  cat > "$AGENTE" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>dev.bichinho.pet</string>
  <key>ProgramArguments</key>
  <array>
    <string>${BIN_APP}</string>
    <string>rodar</string>
  </array>
  <key>EnvironmentVariables</key>
  <dict>
    <key>PET_CONFIG</key><string>${APP_SUPPORT}</string>
  </dict>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><dict><key>SuccessfulExit</key><false/></dict>
  <key>ProcessType</key><string>Interactive</string>
  <key>StandardErrorPath</key><string>${HOME}/Library/Logs/bichinho.log</string>
  <key>StandardOutPath</key><string>${HOME}/Library/Logs/bichinho.log</string>
</dict>
</plist>
PLIST
  launchctl unload "$AGENTE" 2>/dev/null || true
  launchctl load "$AGENTE"
  echo "  carregado (sobe no login; logs em ~/Library/Logs/bichinho.log)"
else
  echo "▸ LaunchAgent: pulado (passe --launchagent para subir no login)"
fi

# 5. plugin (afeta TODAS as sessões do Claude): a partir de uma worktree
#    estável da main, nunca da branch. Pede confirmação.
if [ "$COM_PLUGIN" -eq 1 ]; then
  if ! command -v claude >/dev/null 2>&1; then
    echo "  atenção: «claude» não está no PATH; pulei o plugin" >&2
  else
    ESTAVEL="$APP_SUPPORT/estavel"
    echo "▸ plugin: vai instalar o plugin «bichinho» (hook «bichinho avisar») de uma"
    echo "  worktree estável da main em $ESTAVEL — vale para TODAS as sessões do Claude."
    printf "  continuar? [s/N] "
    read -r resp
    if [ "$resp" = "s" ] || [ "$resp" = "S" ]; then
      git worktree add --force "$ESTAVEL" main
      claude plugin marketplace add "$ESTAVEL"
      claude plugin install bichinho
      echo "  plugin instalado da worktree estável"
    else
      echo "  plugin: pulado"
    fi
  fi
else
  echo "▸ plugin: pulado (passe --plugin para instalar o hook nas sessões do Claude)"
fi

echo
echo "✓ instalado. Diagnóstico:"
"$BIN_APP" diagnostico || true
