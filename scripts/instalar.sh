#!/bin/sh
# scripts/instalar.sh — instala o Zeca com UM comando (protótipo do M9).
#
#   Hoje (dev, do repo):   sh scripts/instalar.sh
#   Meta (M9):             curl -fsSL get.zeca.dev | sh
#
# Princípio (decisão do Renan, 2026-10-06): instalar tem que ser tão fácil
# quanto um `npx` — um comando, zero pergunta, o Zeca aparece e funciona. Este
# script é não-interativo: monta o binário, sobe o daemon no login, pluga no
# Claude Code, aprova a skin padrão (CC0) e deixa o Zeca na tela.
#
# TODO (M9, pra virar `curl | sh` de verdade):
#   - baixar o binário assinado do GitHub Releases por SO/arch, em vez de
#     compilar (hoje compila, porque roda do repo);
#   - Linux (daemon nativo ou container + systemd user) e Windows;
#   - a skin padrão já vir pré-aprovada no pacote (sem o passo da folha de
#     contato), e o hook/plugin num instalador que não precisa do repo.
set -eu

PORTA=27380
SKIN=zeca-livre-escuro
URL="127.0.0.1:${PORTA}"

passo() { printf '▸ %s\n' "$1"; }
info()  { printf '  %s\n' "$1"; }
erro()  { printf 'erro: %s\n' "$1" >&2; exit 1; }

# --- 1. plataforma -----------------------------------------------------------
OS=$(uname -s)
case "$OS" in
  Darwin) ;;
  *) erro "o protótipo do instalador só faz macOS por enquanto (veio $OS); Linux e Windows são os próximos" ;;
esac

# --- 2. raiz do projeto (dev: roda do repo) ----------------------------------
RAIZ=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
cd "$RAIZ"
command -v cargo >/dev/null 2>&1 || command -v "$HOME/.cargo/bin/cargo" >/dev/null 2>&1 \
  || erro "preciso do cargo pra compilar (no release isto vira download do binário)"
CARGO=$(command -v cargo 2>/dev/null || printf '%s' "$HOME/.cargo/bin/cargo")

APP="$HOME/Applications/Bichinho.app"
APP_BIN="$APP/Contents/MacOS/bichinho"
BIN_PATH="$HOME/.local/bin/bichinho"
AGENTE="$HOME/Library/LaunchAgents/dev.bichinho.pet.plist"
APP_SUPPORT="$HOME/Library/Application Support/bichinho"
SKINS_FONTE="$APP_SUPPORT/skins-fonte"

printf '\n🦜 Instalando o Zeca…\n\n'

# --- 3. binário + .app -------------------------------------------------------
passo "montando o Bichinho.app (compila o binário de release e assina ad-hoc)"
PATH="$(dirname "$CARGO"):$PATH" scripts/mac-empacotar.sh "$APP" >/dev/null 2>&1 \
  || erro "falhou montar o .app"
info "$("$APP_BIN" versao 2>&1 | head -1)"

# --- 4. hook no PATH (o Claude Code acha «bichinho avisar») -------------------
passo "pondo o hook no PATH"
mkdir -p "$(dirname "$BIN_PATH")"
ln -sf "$APP_BIN" "$BIN_PATH"
case ":$PATH:" in
  *":$(dirname "$BIN_PATH"):"*) : ;;
  *) info "atenção: $(dirname "$BIN_PATH") não está no seu PATH — acrescente ao shell" ;;
esac

# --- 5. config + skin-fonte estável ------------------------------------------
mkdir -p "$APP_SUPPORT" "$SKINS_FONTE"
if [ ! -f "$APP_SUPPORT/bichinho.toml" ]; then
  passo "config padrão (skin $SKIN, tamanho pequeno)"
  printf '[aparencia]\nskin = "%s"\ntamanho = "pequeno"\n' "$SKIN" > "$APP_SUPPORT/bichinho.toml"
fi
# A skin-fonte fica num lugar estável (não depende do repo), pro daemon achar
# o conteúdo na hora de aprovar.
cp -R "skins/$SKIN" "$SKINS_FONTE/"

# --- 6. sobe o daemon no login (LaunchAgent) ---------------------------------
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
  <dict>
    <key>PET_CONFIG</key><string>${APP_SUPPORT}</string>
    <key>PET_SKINS</key><string>${SKINS_FONTE}</string>
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
launchctl load "$AGENTE" || erro "não carreguei o LaunchAgent"

# espera o daemon responder
i=0
while [ "$i" -lt 20 ]; do
  curl -s --noproxy '*' -H 'X-Pet: 1' "$URL/v1/estado" >/dev/null 2>&1 && break
  i=$((i + 1)); sleep 0.3
done
curl -s --noproxy '*' -H 'X-Pet: 1' "$URL/v1/estado" >/dev/null 2>&1 \
  || erro "o daemon não respondeu em $URL (veja ~/Library/Logs/bichinho.log)"

# --- 7. aprova a skin padrão (CC0, nossa) ------------------------------------
# TODO (release): a skin padrão vem pré-aprovada no pacote; some o passo da
# folha de contato. Aqui (dev) geramos a folha e aprovamos pelo fluxo normal.
passo "liberando o Zeca (skin $SKIN)"
PATH="$(dirname "$CARGO"):$PATH" "$CARGO" run --quiet -p xtask -- \
  contato "skins/$SKIN" --copia tmp/previa-zeca-livre >/dev/null 2>&1 \
  || erro "falhou gerar a folha de contato da skin"
PET_SKINS="$SKINS_FONTE" PET_PORTA="$PORTA" bin/pet skin-aprovar "$SKIN" >/dev/null 2>&1 \
  || erro "falhou aprovar a skin $SKIN"

# --- 8. pluga no Claude Code (hook em todas as sessões) ----------------------
if command -v claude >/dev/null 2>&1; then
  passo "plugando no Claude Code"
  ESTAVEL="$APP_SUPPORT/estavel"
  git worktree prune 2>/dev/null || true
  if ! git -C "$ESTAVEL" rev-parse --git-dir >/dev/null 2>&1; then
    rm -rf "$ESTAVEL"
    git worktree add --detach --force "$ESTAVEL" HEAD >/dev/null 2>&1 \
      || erro "não criei a worktree estável do plugin"
  fi
  claude plugin marketplace add "$ESTAVEL" >/dev/null 2>&1 || true
  claude plugin install bichinho@bichinho-local >/dev/null 2>&1 || true
  PLUGADO=1
else
  info "o «claude» não está no PATH — pulei o plugin (o Zeca aparece, mas não reage às sessões)"
  PLUGADO=0
fi

# --- 9. uma linha e pronto ---------------------------------------------------
printf '\n✓ Zeca instalado e no ar.\n'
if [ "$PLUGADO" = 1 ]; then
  printf '  Abra um terminal novo e rode «claude» — ele já reage sozinho.\n'
  printf '  (Nas abas do Claude já abertas, um /reload-plugins liga o Zeca nelas.)\n'
fi
printf '  Tirar tudo: scripts/mac-desinstalar.sh --tudo (e claude plugin uninstall bichinho)\n\n'
