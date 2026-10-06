#!/usr/bin/env bash
# scripts/mac-desinstalar.sh — tira o bichinho do macOS (M8, T9.3, decisão 0104).
#
# Descarrega e remove o LaunchAgent, o link do PATH e o Bichinho.app. Deixa os
# dados (~/Library/Application Support/bichinho, com a aprovação do Zeca) e o
# plugin, que se tira com «claude plugin uninstall bichinho». Com --tudo,
# remove também os dados.
set -euo pipefail

APP="$HOME/Applications/Bichinho.app"
BIN_PATH="$HOME/.local/bin/bichinho"
AGENTE="$HOME/Library/LaunchAgents/dev.bichinho.pet.plist"
APP_SUPPORT="$HOME/Library/Application Support/bichinho"

COM_DADOS=0
[ "${1:-}" = "--tudo" ] && COM_DADOS=1

if [ -f "$AGENTE" ]; then
  echo "▸ descarregando e removendo o LaunchAgent"
  launchctl unload "$AGENTE" 2>/dev/null || true
  rm -f "$AGENTE"
fi

# Para um daemon avulso que tenha sobrado (sem o LaunchAgent).
pkill -f "Bichinho.app/Contents/MacOS/bichinho rodar" 2>/dev/null || true

if [ -L "$BIN_PATH" ] || [ -f "$BIN_PATH" ]; then
  echo "▸ removendo o hook do PATH ($BIN_PATH)"
  rm -f "$BIN_PATH"
fi

if [ -d "$APP" ]; then
  echo "▸ removendo $APP"
  rm -rf "$APP"
fi

if [ "$COM_DADOS" -eq 1 ]; then
  echo "▸ removendo os dados ($APP_SUPPORT) — a aprovação do Zeca vai junto"
  rm -rf "$APP_SUPPORT"
else
  echo "▸ dados mantidos em $APP_SUPPORT (use --tudo para remover)"
fi

echo "▸ plugin: remova com «claude plugin uninstall bichinho» se quiser"
echo "✓ desinstalado"
