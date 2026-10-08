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
  MINGW* | MSYS* | CYGWIN* | Windows_NT)
    echo "No Windows, instale pelo PowerShell:" >&2
    echo "  irm https://raw.githubusercontent.com/${REPO}/main/scripts/get.ps1 | iex" >&2
    exit 1 ;;
  *) echo "O Zeca tem instalador por download para macOS e Windows (veio $OS). No Linux, rode do código por ora (veja o README)." >&2; exit 1 ;;
esac
case "$ARCH" in
  arm64 | aarch64) arch=arm64 ;;
  x86_64) arch=x64 ;;
  *) echo "arquitetura não suportada: $ARCH" >&2; exit 1 ;;
esac

# Um pacote universal no macOS (binário arm64 + Intel via lipo): não depende do
# arch nem do runner Intel escasso do CI (decisão 0114).
asset="zeca-macos.tar.gz"
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
