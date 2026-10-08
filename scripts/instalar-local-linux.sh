#!/bin/sh
# scripts/instalar-local-linux.sh — instala o Zeca no Linux a partir de
# artefatos já prontos (sem cargo, sem o repo). Miolo do instalador por download
# (M9, Linux); o get.sh baixa o pacote e chama este, que acha os artefatos ao
# lado dele:
#
#   <dir>/bichinho                          o daemon (musl estático)
#   <dir>/skin/zeca-livre-escuro/           a skin padrão CC0 já aprovada
#   <dir>/.claude-plugin/ , <dir>/plugin/   o plugin do Claude Code
#
# Precisa de Hyprland (Wayland com layer-shell). Não-interativo.
set -eu

OS=$(uname -s)
[ "$OS" = Linux ] || { echo "instalar-local-linux.sh: só Linux (veio $OS)"; exit 1; }

DIR=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)

SUP="$HOME/.local/state/bichinho"
APP_DIR="$HOME/.local/share/bichinho/bin"
APP_BIN="$APP_DIR/bichinho"
BIN_PATH="$HOME/.local/bin/bichinho"
SERVICE="$HOME/.config/systemd/user/bichinho.service"
PORTA=27380
URL="127.0.0.1:${PORTA}"

passo() { printf '▸ %s\n' "$1"; }
info()  { printf '  %s\n' "$1"; }

esperar_pet() {
  i=0
  while [ "$i" -lt 30 ]; do
    curl -s --noproxy '*' -H 'X-Pet: 1' "$URL/v1/estado" >/dev/null 2>&1 && return 0
    i=$((i + 1)); sleep 0.3
  done
  return 1
}

printf '\n🦜 Instalando o Zeca…\n\n'

# 1. o binário ------------------------------------------------------------
passo "instalando o bichinho"
mkdir -p "$APP_DIR"
cp "$DIR/bichinho" "$APP_BIN"
chmod +x "$APP_BIN"
info "$("$APP_BIN" versao 2>&1 | head -1)"

# 2. o hook no PATH -------------------------------------------------------
passo "pondo o hook no PATH"
mkdir -p "$(dirname "$BIN_PATH")"
ln -sf "$APP_BIN" "$BIN_PATH"
case ":$PATH:" in
  *":$(dirname "$BIN_PATH"):"*) : ;;
  *) info "atenção: $(dirname "$BIN_PATH") não está no seu PATH — acrescente ao shell" ;;
esac

# 3. config + a skin padrão JÁ APROVADA (snapshot em <estado>/skins/<id>) --
passo "config e a skin padrão (Zeca, CC0)"
mkdir -p "$SUP/skins"
[ -f "$SUP/bichinho.toml" ] || printf '[aparencia]\nskin = "zeca-livre-escuro"\ntamanho = "pequeno"\n' > "$SUP/bichinho.toml"
rm -rf "$SUP/skins/zeca-livre-escuro"
cp -R "$DIR/skin/zeca-livre-escuro" "$SUP/skins/zeca-livre-escuro"

# 4. autostart no login (systemd --user) e sobe agora ---------------------
passo "subindo o Zeca no login (systemd --user)"
mkdir -p "$(dirname "$SERVICE")"
cat > "$SERVICE" <<UNIT
[Unit]
Description=Zeca (bichinho) — pet de mesa que reage ao Claude Code
PartOf=graphical-session.target
After=graphical-session.target

[Service]
ExecStart=${APP_BIN} rodar
Environment=PET_CONFIG=${SUP}
Restart=on-failure
RestartSec=2

[Install]
WantedBy=graphical-session.target
UNIT

systemctl --user daemon-reload 2>/dev/null || true
systemctl --user enable bichinho.service >/dev/null 2>&1 || true
systemctl --user start bichinho.service >/dev/null 2>&1 || true
if ! esperar_pet; then
  # sem systemd --user acessível (ou sessão gráfica) — sobe em segundo plano;
  # o autostart no próximo login fica pelo serviço gravado.
  info "subindo em segundo plano (o autostart no login fica pelo serviço)"
  PET_CONFIG="$SUP" "$APP_BIN" rodar >/dev/null 2>&1 &
  esperar_pet || true
fi
if esperar_pet; then info "o Zeca está no ar em $URL"; else info "o Zeca ainda não respondeu (sobe no próximo login)"; fi

# 5. pluga no Claude Code (fonte estável, não o tmp do download) ----------
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
printf '  Precisa de Hyprland (Wayland com layer-shell).\n'
printf '  Tirar: systemctl --user disable --now bichinho; apague %s, %s e %s.\n\n' \
  "$SERVICE" "$SUP" "$HOME/.local/share/bichinho"
