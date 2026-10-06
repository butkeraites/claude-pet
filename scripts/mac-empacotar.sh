#!/usr/bin/env bash
# scripts/mac-empacotar.sh — monta o Bichinho.app (M8, T8.5/T9.3, decisão 0104).
#
# Compila o binário de release e monta um .app mínimo: LSUIElement (sem ícone
# no Dock), bundle id neutro (dev.bichinho.pet) e assinatura ad-hoc
# (codesign -s -), para as permissões (Acessibilidade, Gravação de Tela)
# ficarem presas ao app e sobreviverem às atualizações.
#
# Uso: scripts/mac-empacotar.sh [saida.app]   (padrão: tmp/Bichinho.app)
set -euo pipefail

RAIZ="$(cd "$(dirname "$0")/.." && pwd)"
cd "$RAIZ"

SAIDA="${1:-$RAIZ/tmp/Bichinho.app}"
BUNDLE_ID="dev.bichinho.pet"
CARGO="${CARGO:-$HOME/.cargo/bin/cargo}"
VERSAO="$(sed -n 's/^version = "\([^"]*\)".*/\1/p' Cargo.toml | head -n1)"
[ -n "$VERSAO" ] || VERSAO="0.1.0"

fonte="$(git rev-parse --short HEAD 2>/dev/null || echo desconhecida)"
if ! git diff --quiet 2>/dev/null; then
  fonte="${fonte}-sujo"
fi

echo "▸ compilando o binário de release (fonte $fonte)"
BICHINHO_FONTE="$fonte" "$CARGO" build --release -p bichinho

echo "▸ montando $SAIDA"
rm -rf "$SAIDA"
mkdir -p "$SAIDA/Contents/MacOS"
cp target/release/bichinho "$SAIDA/Contents/MacOS/bichinho"

cat > "$SAIDA/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key><string>${BUNDLE_ID}</string>
  <key>CFBundleName</key><string>Bichinho</string>
  <key>CFBundleDisplayName</key><string>Bichinho</string>
  <key>CFBundleExecutable</key><string>bichinho</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>${VERSAO}</string>
  <key>CFBundleVersion</key><string>${VERSAO}</string>
  <key>LSUIElement</key><true/>
  <key>LSMinimumSystemVersion</key><string>13.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

echo "▸ assinando ad-hoc (codesign -s -, id $BUNDLE_ID)"
codesign --force --sign - --identifier "$BUNDLE_ID" "$SAIDA"
codesign --verify --verbose=2 "$SAIDA" 2>&1 | sed 's/^/  /'

echo "✓ pronto: $SAIDA"
echo "  o daemon é «$SAIDA/Contents/MacOS/bichinho rodar» (o LaunchAgent o sobe)"
