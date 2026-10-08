#!/bin/sh
# scripts/get.sh — instale o Zeca com UM comando, sem o repo e sem cargo:
#
#   curl -fsSL https://raw.githubusercontent.com/butkeraites/claude-pet/main/scripts/get.sh | sh
#
# Detecta o sistema, baixa o pacote de release do GitHub (binário já
# compilado e assinado) e roda o setup local. Variáveis opcionais:
#   ZECA_VERSION=vX.Y.Z  fixa a versão (padrão: a última release)
#   ZECA_REPO=dono/repo  outro repositório
set -eu

REPO="${ZECA_REPO:-butkeraites/claude-pet}"

OS=$(uname -s)
ARCH=$(uname -m)
case "$OS" in
  Darwin) plat=macos ;;
  Linux) plat=linux ;;
  MINGW* | MSYS* | CYGWIN* | Windows_NT)
    echo "No Windows, instale pelo PowerShell:" >&2
    echo "  irm https://raw.githubusercontent.com/${REPO}/main/scripts/get.ps1 | iex" >&2
    exit 1 ;;
  *) echo "O Zeca tem instalador por download para macOS, Linux e Windows (veio $OS)." >&2; exit 1 ;;
esac
case "$ARCH" in
  arm64 | aarch64) arch=arm64 ;;
  x86_64) arch=x64 ;;
  *) echo "arquitetura não suportada: $ARCH" >&2; exit 1 ;;
esac

case "$plat" in
  # macOS: um pacote universal (binário arm64 + Intel via lipo; decisão 0114).
  macos) asset="zeca-macos.tar.gz" ;;
  # Linux: musl estático x86_64, por ora (precisa de Hyprland; decisão 0115).
  linux)
    [ "$arch" = x64 ] || { echo "no Linux há pacote só para x86_64 por ora (veio $ARCH); rode do código (veja o README)." >&2; exit 1; }
    asset="zeca-linux-x64.tar.gz" ;;
esac
if [ -n "${ZECA_VERSION:-}" ]; then
  url="https://github.com/${REPO}/releases/download/${ZECA_VERSION}/${asset}"
else
  url="https://github.com/${REPO}/releases/latest/download/${asset}"
fi

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT INT TERM

printf '🦜 Baixando o Zeca (%s/%s)…\n' "$plat" "$arch"
if ! curl -fSL --proto '=https' --tlsv1.2 "$url" -o "$tmp/zeca.tar.gz"; then
  echo "falhou baixar $url" >&2
  echo "  (há um release com esse pacote? veja https://github.com/${REPO}/releases)" >&2
  exit 1
fi
tar -xzf "$tmp/zeca.tar.gz" -C "$tmp"
sh "$tmp/instalar-local.sh"
