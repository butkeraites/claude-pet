#!/bin/sh
# scripts/instalar-local.sh — instala o Zeca a partir de artefatos já prontos
# (sem cargo, sem o repo). É o miolo do instalador por download (M9): o
# `scripts/get.sh` baixa o pacote de release e chama este script, que acha os
# artefatos ao lado dele:
#
#   <dir>/Bichinho.app/                     o app assinado (CI)
#   <dir>/skin/zeca-livre-escuro/           a skin padrão CC0 já aprovada
#   <dir>/.claude-plugin/marketplace.json   o marketplace do plugin
#   <dir>/plugin/                           o plugin do Claude Code
#
# Não-interativo. Só macOS por ora (Linux/Windows: próximos).
set -eu

OS=$(uname -s)
[ "$OS" = Darwin ] || { echo "instalar-local.sh: só macOS por enquanto (veio $OS)"; exit 1; }

DIR=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)

APP_DST="$HOME/Applications/Bichinho.app"
APP_BIN="$APP_DST/Contents/MacOS/bichinho"
BIN_PATH="$HOME/.local/bin/bichinho"
AGENTE="$HOME/Library/LaunchAgents/dev.bichinho.pet.plist"
SUP="$HOME/Library/Application Support/bichinho"
PORTA=27380
URL="127.0.0.1:${PORTA}"

passo() { printf '▸ %s\n' "$1"; }
info()  { printf '  %s\n' "$1"; }

printf '\n🦜 Instalando o Zeca…\n\n'

# 1. o app -----------------------------------------------------------------
passo "instalando o Bichinho.app"
mkdir -p "$HOME/Applications"
rm -rf "$APP_DST"
cp -R "$DIR/Bichinho.app" "$APP_DST"
info "$("$APP_BIN" versao 2>&1 | head -1)"

# 2. o hook no PATH --------------------------------------------------------
passo "pondo o hook no PATH"
mkdir -p "$(dirname "$BIN_PATH")"
ln -sf "$APP_BIN" "$BIN_PATH"
case ":$PATH:" in
  *":$(dirname "$BIN_PATH"):"*) : ;;
  *) info "atenção: $(dirname "$BIN_PATH") não está no seu PATH — acrescente ao shell" ;;
esac

# 3. config + a skin padrão JÁ APROVADA (sem precisar aprovar nada) --------
passo "config e a skin padrão (Zeca, CC0)"
mkdir -p "$SUP/skins"
[ -f "$SUP/bichinho.toml" ] || printf '[aparencia]\nskin = "zeca-livre-escuro"\ntamanho = "pequeno"\n' > "$SUP/bichinho.toml"
rm -rf "$SUP/skins/zeca-livre-escuro"
cp -R "$DIR/skin/zeca-livre-escuro" "$SUP/skins/zeca-livre-escuro"

# 4. o daemon sobe no login ------------------------------------------------
passo "subindo o Zeca no login (LaunchAgent)"
mkdir -p "$(dirname "$AGENTE")" "$HOME/Library/Logs"
cat > "$AGENTE" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>dev.bichinho.pet</string>
  <key>ProgramArguments</key>
  <array><string>${APP_BIN}</string><string>rodar</string></array>
  <key>EnvironmentVariables</key>
  <dict><key>PET_CONFIG</key><string>${SUP}</string></dict>
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
i=0
while [ "$i" -lt 20 ]; do
  curl -s --noproxy '*' -H 'X-Pet: 1' "$URL/v1/estado" >/dev/null 2>&1 && break
  i=$((i + 1)); sleep 0.3
done

# 5. pluga no Claude Code (fonte estável, não o tmp do download) -----------
if command -v claude >/dev/null 2>&1; then
  passo "plugando no Claude Code"
  FONTE="$SUP/plugin-fonte"
  rm -rf "$FONTE"; mkdir -p "$FONTE"
  cp -R "$DIR/.claude-plugin" "$FONTE/.claude-plugin"
  cp -R "$DIR/plugin" "$FONTE/plugin"
  claude plugin marketplace add "$FONTE" >/dev/null 2>&1 || true
  claude plugin install bichinho@bichinho-local >/dev/null 2>&1 || true
  PLUGADO=1
else
  info "o «claude» não está no PATH — pulei o plugin (o Zeca aparece, mas não reage às sessões)"
  PLUGADO=0
fi

printf '\n✓ Zeca instalado e no ar.\n'
if [ "${PLUGADO:-0}" = 1 ]; then
  printf '  Abra um terminal novo e rode «claude» — ele já reage sozinho.\n'
  printf '  (Nas abas do Claude já abertas, um /reload-plugins liga o Zeca nelas.)\n'
fi
printf '  Tirar: scripts/mac-desinstalar.sh --tudo (e claude plugin uninstall bichinho)\n\n'
