#!/usr/bin/env bash
# scripts/linux-empacotar.sh <staging> — monta o pacote do Zeca para Linux (M9):
# o binário musl **estático** (como a imagem Docker), a skin CC0 padrão JÁ
# APROVADA (snapshot + aprovacao.json), o plugin e o instalar-local.sh. Usado
# pelo release e pelo teste do instalador.
#
# Alvo: Hyprland (Wayland com layer-shell) por ora — a descoberta é do Hyprland
# (T8.3, "qualquer Wayland", é futuro). Sem assinatura.
#
#   scripts/linux-empacotar.sh staging
set -euo pipefail

STAGING="${1:?uso: linux-empacotar.sh <staging>}"
RAIZ="$(cd "$(dirname "$0")/.." && pwd)"
cd "$RAIZ"
CARGO="${CARGO:-cargo}"
ALVO=x86_64-unknown-linux-musl

echo "- alvo musl ($ALVO)"
rustup target add "$ALVO"

echo "- compilando o daemon (release, musl estático)"
"$CARGO" build --release --target "$ALVO" -p bichinho

rm -rf "$STAGING"
mkdir -p "$STAGING"
cp "target/$ALVO/release/bichinho" "$STAGING/bichinho"
chmod +x "$STAGING/bichinho"

# A skin padrão (CC0), JA APROVADA. A impressão é o sha256 do texto
# "sha256(arquivo)  nome\n" de skin.json, sheet.json, sheet.png — exatamente o
# que o daemon calcula (pet_core::aprovacao::impressao); no Linux, sha256sum.
SKIN_SRC="$RAIZ/skins/zeca-livre-escuro"
SKIN_DST="$STAGING/skin/zeca-livre-escuro"
mkdir -p "$SKIN_DST"
cp -R "$SKIN_SRC/." "$SKIN_DST/"
fp=$( cd "$SKIN_DST" && sha256sum skin.json sheet.json sheet.png | sha256sum | cut -d' ' -f1 )
printf '{"id":"zeca-livre-escuro","sha256":"%s","aprovada_em_ms":%s000}\n' \
  "$fp" "$(date +%s)" > "$SKIN_DST/aprovacao.json"
echo "  impressão da skin: $fp"

cp -R "$RAIZ/plugin" "$STAGING/plugin"
cp -R "$RAIZ/.claude-plugin" "$STAGING/.claude-plugin"
# O instalador do Linux é um script próprio (o do macOS fica intacto); vai como
# `instalar-local.sh` no pacote, que é o que o get.sh chama.
cp "$RAIZ/scripts/instalar-local-linux.sh" "$STAGING/instalar-local.sh"

echo "✓ pacote em $STAGING"
