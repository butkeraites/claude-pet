# Multiplataforma: Linux, macOS e Windows

> Pesquisa só de leitura de 2026-10-03, feita por quatro pesquisas
> (janelas por sistema; Linux além do Hyprland; Claude Code e terminais;
> distribuição e licenças) e um desenho que junta tudo. **O que vale é o
> `PLANO.md` (M8, M9 e a nova ordem) e o `DECISIONS.md` (0035 em diante)**;
> isto é a evidência, com os links conferidos naquela data. A parte do
> Linux amplo está em inglês, como veio. Trechos que citavam um personagem
> de terceiros foram reescritos de forma neutra (decisão 0035). Pode
> envelhecer: confira antes de confiar num detalhe.

## O que o Renan decidiu (2026-10-03)

- Lançamento **open source** para Linux, macOS e Windows; macOS e Windows
  como apps nativos, Docker só no Linux.
- O produto e o binário se chamam **bichinho** (o repositório público será
  novo e limpo); o personagem continua **Zeca** em público, nunca associado
  a personagem de terceiros (decisões 0035 e 0036).
- Antes do M4 entram só a costura de plataforma (T8.0) e o hook nativo
  (T8.1), mais o tamanho do Zeca como opção de config (TP.2); o resto do M8
  e o M9 ficam depois do M7 (decisão 0038).
- Clicar no Zeca leva ao terminal da sessão do Claude, pelo
  foreign-toplevel do Wayland, sem o socket de comandos do Hyprland
  (decisão 0039; é escopo do M4).
- As outras perguntas abaixo (arte do personagem público, assinatura,
  canais, GNOME e X11 na v1.0, CI) continuam abertas.

## Desenho

### Resposta curta

**Sim: dá para Linux, macOS e Windows.** Todo o núcleo (`pet-core`: cérebro, animação, skin, raster) e o backend Wayland atual são reaproveitados. Por sistema muda só a janela do bicho e a ligação com o desktop.

**Por sistema**
- **Linux:** além do Hyprland, o backend atual (layer-shell) serve com pouco trabalho para KDE Plasma 6, Sway, niri, COSMIC, labwc e Wayfire. A mudança principal é parar de depender do Hyprland para achar a tela.
  - O **GNOME** (Ubuntu, Fedora) é o ponto fraco. Ele não tem layer-shell e, desde o GNOME 50, nem sessão X11. Precisa de uma extensão do GNOME Shell: funciona, mas é uma peça a mais, passa por revisão e pede atualização a cada 6 meses.
  - Mint, XFCE e outros desktops X11 pedem um backend X11 próprio (esforço médio).
- **macOS: dá**, com um painel AppKit nativo que aparece em todos os Spaces e por cima de apps em tela cheia.
  - O ponto incerto é o clique atravessar a área transparente em volta do bicho. Há plano B, sem pedir permissão.
  - Dá para distribuir de graça pelo Homebrew. A conta da Apple (US$ 99/ano) só fica necessária para um .dmg notarizado ou para focar a aba exata do terminal sem pedir permissão de novo a cada atualização.
- **Windows: dá, sim.** Para a janela, é até o mais simples: transparência e clique por pixel são recursos documentados. As ressalvas são duas:
  - Aparecer em todas as áreas de trabalho virtuais depende de um comportamento não documentado. Há reserva por API pública.
  - Binário sem assinatura assusta com o SmartScreen. Dá para assinar de graça (SignPath, com condições) ou a partir de €69 (Certum).

**Hooks:** o `avisar.sh` (sh + jq + curl) vira um subcomando do próprio binário, igual nos três sistemas. No Windows não precisa de Git Bash nem de jq.

**Docker:** no Mac e no Windows não serve para mostrar o pet, porque lá o Docker é uma VM Linux sem acesso à tela. Fica só como opção no Linux e como laboratório de testes.

**Arte e nome (o maior bloqueio)**
- O Zeca de hoje não pode ir no download: a licença do pack proíbe redistribuir, mesmo editado.
- Dá para manter o "traga seu pack": cada pessoa compra o pack por US$ 0,50 e o app monta o Zeca na máquina dela. Mas o app público precisa de um personagem padrão original.
- Papagaio verde com chapéu-palheta e gravata pode lembrar personagens de terceiros: o risco está na combinação (não é parecer jurídico). O Renan decidiu manter o nome Zeca e nunca associá-lo a personagem de terceiros (decisão 0035).

**Sem Mac nem Windows:** os runners de macOS e Windows do GitHub Actions (grátis em repositório público) compilam, testam e tiram prints da tela. A conferência visual fica para um Mac alugado por dia (~€2,64) e uma VM do Windows de avaliação (grátis por 90 dias).

**Custo e prazo**
- **Dinheiro:** pode começar em zero. O realista no primeiro ano fica entre algumas centenas e ~US$ 1.500, quase tudo arte.
- **Tempo** (estimativa conservadora): uma beta pública nos três sistemas em ~25–40 dias de trabalho, e a v1.0 completa em ~40–65 dias. O gargalo é testar em máquina de verdade e esperar a arte, não o código.

**Ordem:** os marcos M8 (multiplataforma) e M9 (publicação) ficam depois do M7, como você pediu. Recomendo só antecipar duas peças pequenas, a costura de plataforma e o hook nativo, para antes do M4. Assim arraste, balões e voos já nascem portáveis.

### Arquitetura

#### 1. Visão geral

```
Claude Code (Linux | macOS | Windows | WSL)
  hook em exec form: claude-pet avisar <Evento>   (lista branca em Rust; sai 0; não imprime)
        │ POST 127.0.0.1:<porta>/v1/evento · X-Pet: 1 · X-Pet-Token (porta e token no arquivo do usuário)
        ▼
┌──────────────────── daemon (um processo por usuário) ────────────────────┐
│ ingress HTTP (std::net, feito à mão) ──► Caixa = mpsc + despertador do SO │
│ vigia (aborta se o laço parar) ─► supervisor / systemd / launchd reinicia │
│ laço do SO: Linux = calloop · macOS = NSApplication.run ·                 │
│             Windows = MsgWaitForMultipleObjectsEx                         │
│   └─ Motor (puro, pet-core): cérebro · animador · cena · palco ·          │
│      arraste · seguir monitor · balões · aprovação                        │
│        pede  → mostrar/esconder · desenhar(cena, dano) · região de toque  │
│        recebe ← ponteiro · monitores · monitor/janela ativos · discrição  │
│   ├─ Overlay (a janela):                                                  │
│   │    wayland (layer-shell) · x11 · gnome · windows · macos              │
│   └─ Desktop (a ligação com o ambiente):                                  │
│        hyprland · sway/i3 · niri · kwin · wlr-foreign-toplevel ·          │
│        cosmic · ewmh · gnome · windows · macos                            │
└───────────────────────────────────────────────────────────────────────────┘
```

O núcleo continua um só. Por SO mudam só dois contratos pequenos:
- **Overlay:** a janela do bicho.
- **Desktop:** monitor ativo, janela ativa, focar janela e "não perturbe".

O laço de eventos passa a ser o do próprio SO, por dois motivos:
- o AppKit exige a thread principal;
- o README do calloop só cita Linux, FreeBSD e macOS, sem o Windows.

#### 2. Crates

| crate | conteúdo | SO | `unsafe` |
|---|---|---|---|
| `pet-core` | o de hoje, mais `motor` (extraído de `laco.rs` e `wl/mod.rs`, com relógio injetado) e `plataforma` (traits e tipos) | todos | forbid |
| `claude-pet` | daemon (`rodar`), hook (`avisar`), CLI (`estado`, `tocar`, `doutor`, `skin`, `autostart`, `configurar`), ingress, `Caixa`, vigia, supervisor, caminhos por SO e escolha do backend | todos | forbid |
| `pet-wayland` | o `wl/` e o `descoberta.rs` atuais, mais os adaptadores Wayland | Linux/BSD | forbid (o SCTK é seguro) |
| `pet-x11` | x11rb 0.14 com `RustConnection` (sem libxcb) | Linux/BSD | forbid |
| `pet-gnome` | cliente zbus 5.19, mais `extensions/gnome-shell/` (GJS) | Linux | forbid |
| `pet-windows` | windows-sys 0.61 | Windows | `deny` no crate, `allow` local com `// SAFETY:` e `clippy::undocumented_unsafe_blocks` |
| `pet-macos` | objc2 0.6; objc2-app-kit, objc2-quartz-core e objc2-core-graphics 0.3.2; dispatch2 0.3 | macOS | idem |
| `pet-arte` | o montador e o importador que hoje estão no `xtask` (zip, asefile), mais a skin padrão embutida | todos | forbid |

Versões conferidas no crates.io em 2026-10-03.

- **Linux:** um binário musl estático com os três backends, todos em Rust puro. O backend é escolhido em tempo de execução, nesta ordem:
  1. Wayland com `zwlr_layer_shell_v1`;
  2. a extensão do GNOME;
  3. X11;
  4. se nada servir, espera.

  A opção `tela.backend = auto|wayland|x11|gnome` força a escolha.
- **Windows:** dois executáveis saem do mesmo crate:
  - `claude-pet.exe`, de console, para a CLI e o hook;
  - o daemon, com `windows_subsystem = "windows"`, sem console.

```rust
// pet_core::plataforma (ilustrativo)
pub trait Overlay {
    fn capacidades(&self) -> CapOverlay;          // JANELA_PEQUENA, ALFA_DECIDE_CLIQUE…
    fn mostrar(&mut self, monitor: Option<&str>); // None: o que o SO/compositor achar ativo
    fn esconder(&mut self);                       // sem quadro fantasma
    fn desenhar(&mut self, cena: &[Elemento], skin: &Skin, toque: Option<Ret>) -> Desenho;
}
pub trait Desktop {
    fn capacidades(&self) -> CapDesktop;          // SEGUE_FOCO, FOCA_JANELA, PID, JANELA_ATIVA, DND
    fn focar(&mut self, alvo: &Alca) -> Result<(), Motivo>;
}
// Eventos (ponteiro, monitores, monitor/janela ativa com hora, discrição)
// entram no Motor pelo laço do SO.
```

#### 3. O que fica igual em todo SO

- **`pet-core` inteiro:** config, evento (fio v1), cérebro, animador, estados, cena, confete, geometria, skin e aprovação.
- **O raster.** Ele já escreve BGRA pré-multiplicado, que é exatamente o formato de quatro alvos, sem nenhuma conversão:
  - `wl_shm` ARGB8888;
  - o DIB do `UpdateLayeredWindow`;
  - o `CGImage` com `PremultipliedFirst | ByteOrder32Little`;
  - o visual ARGB do X11.
- **O Motor**, extraído e puro:
  - recebe comandos, ponteiro, monitores e foco;
  - devolve pedidos e o próximo prazo;
  - é testado com relógio falso, sem tela.
- **O ingress HTTP** com as checagens de hoje (`Host`, `X-Pet`, `Content-Type`, 8 KiB), mais a aprovação e o vigia.
- **O hook e a CLI**, no mesmo binário.
- **A nitidez por construção:** D inteiro por monitor, em pixels do dispositivo.

#### 4. O que muda por SO: a janela

| | Wayland com layer-shell | X11 | GNOME (extensão) | Windows | macOS |
|---|---|---|---|---|---|
| Superfície | camada OVERLAY do tamanho do monitor (decisão 0005, medida) | janela ARGB pequena que anda | ator do Shell (`addTopChrome`) | janela layered pequena que anda | NSPanel pequeno que anda |
| Só o corpo recebe clique | região de input | SHAPE de entrada | `affectsInputRegion` | alfa por pixel (alfa 1/255 na área de toque) | alfa (a validar) ou alternar `ignoresMouseEvents` |
| Por cima de tudo | OVERLAY | `_NET_WM_STATE_ABOVE` ou DOCK | top chrome | `WS_EX_TOPMOST`, reafirmado | nível flutuante + `FullScreenAuxiliary` |
| Todas as áreas / Spaces | a camada é do monitor | `_NET_WM_DESKTOP=0xFFFFFFFF` + STICKY | sim | `WS_EX_TOOLWINDOW` (não documentado) + `MoveWindowToDesktop` | `CanJoinAllSpaces` |
| Não rouba o foco | teclado NONE | `WM_HINTS input=False` | — | `WS_EX_NOACTIVATE` + `MA_NOACTIVATE` | `NonactivatingPanel` + app `Accessory` |
| Nitidez | blocos D×D em px do monitor, com escala fracionária | px do dispositivo | NEAREST a partir da arte | PMv2, D por monitor (125/150/175% → 5/6/7) | escala 1 ou 2; os modos escalados reamostram |
| Laço / como acordar | calloop / ping | calloop / ping | calloop + zbus | `MsgWaitForMultipleObjectsEx` + timer de alta resolução / `PostMessageW` | `NSApplication.run` + `CFRunLoopTimer` / fila principal |

Nos backends de janela pequena, o voo T3 e o confete usam um palco transitório: uma janela do tamanho do monitor, toda atravessável pelo clique, aberta só durante o efeito. É o plano B que a decisão 0005 já previa.

#### 5. Como os hooks chegam ao pet

| Onde roda o Claude | Hook | Caminho | Identidade para focar (só em SessionStart e UserPromptSubmit) |
|---|---|---|---|
| Linux nativo | `claude-pet avisar <Evento>` | 127.0.0.1, com porta e token lidos de `$XDG_RUNTIME_DIR/claude-pet/` (0600) | `CLAUDE_PID`, tty, `TERM_PROGRAM`, kitty, WezTerm, tmux. No Hyprland, `hyprctl -j clients` só leitura, se precisar |
| Linux, daemon no Docker | o mesmo, rodando no host | 127.0.0.1:27380 publicado pelo compose; token num arquivo do host montado somente leitura | idem |
| macOS | o mesmo; o binário precisa estar no PATH que o Claude vê | arquivo em `~/Library/Application Support/claude-pet/` | `CLAUDE_PID`, tty, `ITERM_SESSION_ID`, `TERM_PROGRAM`, `WARP_FOCUS_URL` |
| Windows | `claude-pet.exe avisar` (o exec form exige um `.exe` real; não precisa de Git Bash nem de jq) | arquivo em `%LOCALAPPDATA%\claude-pet\` | `CLAUDE_PID`, `WT_SESSION` e a janela do WT via `AttachConsole(CLAUDE_PID)` |
| WSL | `claude-pet avisar` versão Linux, dentro do WSL | com a rede em modo espelhado, 127.0.0.1 chega ao Windows; em NAT, chamar o `claude-pet.exe` por interop (v1.1) | — |

- **O que está conferido:**
  - o exec form, na doc oficial dos hooks: `command` é resolvido no PATH e os `args` passam sem shell;
  - o `CLAUDE_PID`, observado nesta máquina no Claude Code 2.1.288;
  - o SessionEnd async, que chegou no gate do M3 no Linux. Falta conferir nos outros SOs.
- **O que está fora:** o hook `type: http`, porque manda o JSON inteiro, com prompt e código.
- **O fio v1 continua.** Os campos novos de identidade são opcionais e validados, o que pede uma decisão nova.

#### 6. Seguir o monitor e focar o terminal

| Ambiente | Monitor ativo | Janela ativa no UserPromptSubmit | Focar no clique | Confiança |
|---|---|---|---|---|
| Hyprland | `focusedmonv2` (M4) | `activewindowv2` (endereço) | foreign-toplevel `activate` + `hyprland_toplevel_mapping_manager_v1`, sem `.socket.sock` | alta: os dois protocolos estão registrados no `ProtocolManager.cpp` do Hyprland main; conferir na 0.56.2 |
| KDE Plasma 6 | D-Bus `activeOutputName` ou script | `windowActivated` | script do KWin | alta/média |
| Sway, i3 | IPC | IPC | `[con_id=N] focus` | alta |
| niri | IPC `EventStream` | `WindowFocusChanged` | `FocusWindow{id}` | alta |
| COSMIC, labwc, Wayfire, river | convocar (output NULL) | foreign-toplevel `activated` | foreign-toplevel `activate` (sem PID) | média |
| GNOME (extensão) | monitor sob o ponteiro | `focus-window` | `Main.activateWindow` | média |
| X11 (EWMH) | `_NET_ACTIVE_WINDOW` + RandR | `_NET_ACTIVE_WINDOW` | `_NET_ACTIVE_WINDOW` com source=2 + `_NET_WM_PID` | alta |
| Windows | `EVENT_SYSTEM_FOREGROUND` + `MonitorFromWindow` | `GetForegroundWindow`, mais o dono da pseudo-janela do ConPTY | `SetForegroundWindow` dentro do clique; a aba do WT por UI Automation é opcional | alta (janela) / média (aba) |
| macOS | `NSScreen.main` + `NSWorkspace` | o app em primeiro plano | ativar o app do terminal (sem permissão); a aba exata por AppleScript pede permissão de Automação | média |

- **Qual janela era da sessão:** o daemon guarda um anel de ativações com hora e, pelo `ts` do hook, descobre qual janela estava ativa.
- **Cadeia de PIDs:** entra como reforço. tmux, zellij e terminais de processo único quebram essa cadeia, e aí vale o retrato da janela ativa.
- **Quando não há como focar:** o bicho responde com o balão "não consigo focar aqui".
- **Hyprland:** o foco por foreign-toplevel preserva a decisão 0006. O daemon nunca abre o `.socket.sock` nem chama `hyprctl dispatch`.

#### 7. Ciclo de vida por SO

| | Linux | macOS | Windows |
|---|---|---|---|
| Config | `$XDG_CONFIG_HOME/claude-pet/` | `~/Library/Application Support/claude-pet/` | `%APPDATA%\claude-pet\` |
| Estado, aprovações e skins do usuário | `$XDG_STATE_HOME` e `$XDG_DATA_HOME` | idem | `%LOCALAPPDATA%\claude-pet\` |
| Iniciar com o sistema | autostart XDG (padrão) ou unit `systemd --user`, mais o trecho de config para Hyprland, Sway, labwc e niri | LaunchAgent (`RunAtLoad`, `KeepAlive` com SuccessfulExit=false) | chave `Run` do HKCU |
| Reinício quando o vigia aborta | systemd `Restart=on-failure` ou o supervisor | launchd | supervisor no próprio binário |
| Log | stderr (vai para o journal) | `~/Library/Logs/claude-pet.log` | arquivo com rotação |
| Instância única | trava de arquivo por usuário (`File::try_lock`) | idem | idem |
| Sair e esconder | CLI, mais menu no botão direito | idem (sem ícone no Dock) | idem (sem ícone na barra de tarefas) |

No modo Docker, as variáveis `PET_*` e os caminhos de hoje continuam valendo.

#### 8. Papel do Docker

**Linux**
- O pet em contêiner continua possível como modo avançado, mas só com o Docker Engine nativo. O Docker Desktop para Linux é uma VM e não enxerga o socket Wayland do host.
- Para o público, o padrão passa a ser o binário nativo.
- O Docker ganha outro papel, o de laboratório: compositores aninhados (Sway, labwc, niri, KWin) e X11 (Xephyr ou Xvfb com i3, Openbox, xfwm4) rodando em contêineres, sem instalar nada no host. Continua também o build musl reprodutível do Dockerfile atual.

**macOS e Windows: não serve para o pet.**
- O Docker Desktop, o OrbStack e o Colima rodam uma VM Linux, e o contêiner não tem acesso ao WindowServer nem ao DWM.
- Pelo XQuartz ou pelo WSLg, o Zeca viraria uma janela comum. O WSLg é Weston, sem layer-shell, então o backend nem conecta.
- Contêiner Windows não tem área de trabalho interativa.
- Também não dá para compilar o macOS num contêiner Linux: o SDK da Apple só pode ser usado em hardware Apple. O build do macOS acontece no runner `macos-15`.

#### 9. Limites conhecidos

**Windows: áreas de trabalho virtuais**
- A API pública não fixa uma janela em todas as áreas.
- A janela do tipo tool aparece em todas por comportamento não documentado. Há relatos de campo a favor e um relato contrário.
- Reserva por API pública: quando o foco muda, `MoveWindowToDesktop` leva o bicho para a área da janela em foco.
- Jogos em tela cheia exclusiva ficam por cima do pet.

**macOS: Spaces e tela cheia**
- `CanJoinAllSpaces` põe o pet em todos os Spaces.
- Apps em tela cheia são Spaces à parte. Só um NSPanel não ativador com `FullScreenAuxiliary` entra neles; isso vem de relatos de 2026 e do BongoCat, e pode mudar a cada versão do macOS.
- O Stage Manager não foi testado.
- Nos modos de tela escalados, o WindowServer reamostra a imagem: nitidez perfeita só nos modos padrão.
- O App Nap atrasa os timers se o app não segurar uma atividade.
- Focar a aba exata pede permissão de Automação. Com assinatura ad-hoc, essa permissão se perde a cada atualização.

**GNOME**
- O Mutter não tem layer-shell até a versão 51, e o GNOME 50 é só Wayland.
- Sem a extensão, sobra o modo X11 experimental pelo XWayland:
  - não segue o foco;
  - não foca terminais Wayland;
  - pode borrar com escala fracionária.
- Com a extensão, a experiência fica completa, mas:
  - a instalação é pelo extensions.gnome.org (EGO);
  - pede logout e login na primeira vez;
  - precisa ser atualizada a cada versão do GNOME.

**Fora de escopo**
- Flatpak e Snap: Hyprland, Sway, niri e COSMIC escondem o layer-shell de clientes em sandbox.
- Weston e WSLg.

#### 10. Arte e licença no desenho

- O binário de release só embute skin com `redistribuivel: true`. O campo já existe no `skin.json`, e o build falha se encontrar outra coisa.
- A skin padrão original vem embutida e já aprovada pelo build (a impressão digital é conhecida).
- "Traga seu pack": `claude-pet skin instalar <zip>` monta o personagem na pasta de dados do usuário.
- Nada do pack vai para o git, para o CI, para imagens publicadas, prints ou GIFs públicos. Artefatos de CI num repositório público ficam públicos.
- Se a receita do Zeca (chapéu-palheta e gravata) vai no binário público é decisão sua.

#### 11. Segurança e privacidade

- Rodando nativo, o daemon tem os mesmos poderes do usuário; a decisão 0014 ("o contêiner é empacotamento") vira história.
- Qualquer usuário local alcança 127.0.0.1. Por isso a porta passa a exigir um token por usuário, guardado num arquivo 0600 ou no perfil do Windows. O `X-Pet` continua barrando requisições do navegador.
- O daemon só aceita ações de alto nível, como "focar a janela da sessão X". Nunca repassa comandos arbitrários.
- A decisão 0006 fica mantida: no Hyprland, o foco vai pelo foreign-toplevel.
- A lista branca do hook continua, com os testes canário rodando nos três SOs.
- Nenhuma telemetria.

#### 12. Testar sem Mac nem Windows

| O quê | Notebook Linux | CI (GitHub Actions) | Máquina real ou VM |
|---|---|---|---|
| Compilar e passar o lint de Windows e macOS | `cargo check` e `clippy` com `--target …` (não linka) | build completo | — |
| Core, Motor e canários do hook | sim | sim, nos três SOs | — |
| Nitidez e clique do overlay | Hyprland de verdade; Sway, labwc e niri aninhados; Xephyr | Sway e labwc headless com `grim`; Xvfb; prints do macOS (`screencapture`) e do Windows (`BitBlt`) como artefato | — |
| Spaces, tela cheia, áreas virtuais, monitores com DPI diferente, sensação de uso | — | — | Mac alugado (~€2,64 por dia), VM de avaliação do Windows, voluntários |
| Focar o terminal | Hyprland, Sway e niri aninhados, Xephyr | — | Terminal.app, iTerm2 e Ghostty; WT e VS Code |
| Build de release do macOS | impossível | runner macOS | — |

Sobre os runners:
- **Disponíveis:**
  - Linux: `ubuntu-24.04`;
  - macOS: `macos-14`, `macos-15` e `macos-26` (M1, 3 vCPU, 7 GB) e `macos-15-intel`;
  - Windows: `windows-2025`, `windows-2022` e `windows-11-arm`.
- **Custo:** grátis e sem limite em repositório público.
- **macOS:** a imagem faz login gráfico automático e dá ao `/bin/bash` permissão de gravação de tela e de Acessibilidade (conferido no repositório actions/runner-images).
- **Windows:** se o runner tem sessão interativa, só o primeiro job vai confirmar. Se a captura vier preta, o resultado conta como NÃO VERIFICADO.

#### 13. Ordem recomendada

- **Antes do M4:** T8.0 (costura) e T8.1 (hook nativo). Assim o M4–M6 (arraste, monitor, balões, voos) nasce no Motor, e não dentro do `wl/`.
- **Já, em paralelo e sem código:** T9.0 (nome, arte e pedido à exclusiveOlive).
- **Depois do M7:**
  1. T8.2 (spikes);
  2. T8.3;
  3. T8.4 e T8.5;
  4. T8.6 e T8.7;
  5. T9.1 a T9.6.
- **GNOME:** a T8.8 fica para a v1.1, a menos que você decida antes.
- **Beta pública mínima:** T8.0 a T8.5, mais T9.0 a T9.4.

##### Critical Files for Implementation

- `~/Documents/claude-pet/crates/claude-pet/src/laco.rs`: vira o Motor, mais a ligação com o calloop.
- `~/Documents/claude-pet/crates/claude-pet/src/wl/mod.rs`: separar o que é pet e cena do que é superfície Wayland.
- `~/Documents/claude-pet/crates/claude-pet/src/ingress.rs` e `~/Documents/claude-pet/crates/claude-pet/src/comando.rs`: a `Caixa` no lugar do canal do calloop, mais o token.
- `~/Documents/claude-pet/crates/claude-pet/src/descoberta.rs`: a descoberta Wayland genérica; o Hyprland vira um adaptador.
- `~/Documents/claude-pet/plugin/scripts/avisar.sh` e `~/Documents/claude-pet/plugin/hooks/hooks.json`: viram `claude-pet avisar` em exec form.
- `~/Documents/claude-pet/xtask/src/zeca/mod.rs`: vira o `pet-arte` ("traga seu pack" no binário).
- `~/Documents/claude-pet/crates/pet-core/src/raster.rs`: o BGRA pré-multiplicado já serve aos três SOs.

### Marcos propostos

#### T8.0 — M8 · Costura de plataforma no Linux, sem mudar o comportamento (recomendado antes do M4)

**Entregas:**

- `pet_core::motor`, extraído de laco.rs e wl/mod.rs:
  - leva o cérebro, Pet e palco, a máquina de mostrar e esconder (passo_de_visibilidade já é pura), o estresse e o Painel;
  - mede o tempo só como prazos em ms, sem tipos do calloop.
- `pet_core::plataforma`: traits Overlay e Desktop com Capacidades; tipos Monitor, EventoPonteiro e Alca.
- Caixa (std mpsc::SyncSender mais um despertador do SO) no lugar do SyncSender do calloop em ingress.rs e comando.rs.
- O wl/ e o descoberta.rs vão para crates/pet-wayland. O que é só do Hyprland (hyprland.lock, FALLBACK, socket2) vira o adaptador hyprland.
- pet-macos e pet-windows vazios, mas compilando.
- Decisão nova no DECISIONS, e a mudança de ordem se antecipar.

**Verificação:**

- Notebook Linux:
  - `bin/pet verificar` verde, com testes novos do Motor em relógio falso, sem tela;
  - com a tela acesa, `scripts/verificar-ao-vivo.sh --personagem` passa (nitidez ±2, sem fantasma, restart e kill -9);
  - `scripts/medir-custo.sh` dá parado ≤ 2 commits/s e até +1 ponto de CPU do Hyprland, iguais ao M1 dentro do ruído;
  - `cargo check --workspace` e `cargo clippy` verdes para x86_64-pc-windows-msvc e aarch64-apple-darwin. O check não linka, então não precisa de SDK, só de `rustup target add`, que muda o sistema e pede seu consentimento.

**Risco:**

Baixo a médio: regredir o ritmo de quadros ou o esconder sem fantasma (o frame callback continua no backend Wayland).
- Mitigação: os scripts ao vivo do M1 são o portão.
- Feito antes do M4, evita reescrever arraste, balões e voos.
- Feito depois do M7, custa mais ou menos o dobro.

#### T8.1 — M8 · Hook nativo (claude-pet avisar) e CLI no próprio binário

**Entregas:**

- Subcomando `avisar <Evento>`:
  - lê o JSON do hook na entrada padrão;
  - aplica a lista branca em Rust, com os validadores de pet_core::evento (hoje duplicados no jq);
  - faz o POST com conexão em até 300 ms e total em até 2 s;
  - nunca imprime e sempre sai 0;
  - mantém o PET_TESTE.
- hooks.json em exec form, igual nos três SOs: type command, command claude-pet, args [avisar, Stop], async true. Sem sh, jq, curl nem Git Bash; o avisar.sh sai.
- Porta e token por usuário:
  - o daemon tenta a 27380 e, se estiver ocupada, uma porta livre;
  - grava porta e token num arquivo só do usuário;
  - o hook manda X-Pet-Token, e o /v1/* exige o token;
  - no modo Docker, porta fixa e token num arquivo do host montado somente leitura.
- CLI de usuário no binário: estado, tocar, esconder, mostrar, testar, doutor e versao. O bin/pet fica como atalho de desenvolvimento.

**Verificação:**

- CI nos runners ubuntu-24.04, macos-15 e windows-2025: os canários de tests/avisar.rs, portados para o binário, rodam sem tela (só std e um TcpListener local) e conferem que:
  - SEGREDO-n em qualquer campo de conteúdo nunca sai;
  - o hook sai 0 e não imprime nada;
  - termina em até 2,1 s com o pet desligado.
- Notebook Linux: o gate ao vivo do M3 de novo, com `claude --plugin-dir` (aceno, pulinho e tchau no /exit), mais `claude plugin validate --strict`.
- Windows real ou VM, no spike T8.2: o hook não pisca janela de console com o Claude no Windows Terminal, no VS Code e no app Desktop.

**Risco:**

Médio, por três motivos:
- o claude-pet fora do PATH quando o Claude é aberto pelo app Desktop ou pelo VS Code (macOS);
- o console piscando no Windows;
- um hook async perdido no fim da sessão.

Mitigação:
- o doutor mostra se os eventos chegam;
- a fonte command do marketplace gera caminhos absolutos;
- um executável auxiliar sem console (windows_subsystem), se o spike mostrar o pisca;
- reconciliação com `claude agents --json`.

#### T8.2 — M8 · Spikes de risco no Windows e no macOS, e CI nos três SOs

**Entregas:**

- Código descartável que abre o bicho com a skin _teste e mede o que é incerto. Os resultados viram decisões no DECISIONS.
- Windows:
  - janela WS_EX_LAYERED, TOOLWINDOW, TOPMOST e NOACTIVATE com o clique atravessando pelo alfa;
  - aparece em todas as áreas de trabalho virtuais (GetWindowDesktopId) no Windows 11 24H2, 25H2 e 26H2, e no Windows 10 22H2;
  - fica por cima da barra de tarefas e de vídeo em tela cheia sem borda;
  - nítida a 125, 150 e 175%, com dois monitores de DPI diferente;
  - sem aviso do firewall ao escutar em 127.0.0.1.
- macOS:
  - NSPanel não ativador com CanJoinAllSpaces, FullScreenAuxiliary, Stationary e IgnoresCycle;
  - clique decidido pelo alfa com conteúdo em CALayer; se não der, o plano B alterna ignoresMouseEvents pela posição do mouse, sem pedir permissão;
  - por cima de app em tela cheia, em todos os Spaces e no Stage Manager;
  - arrastar sem ativar o app;
  - NSScreen.main acompanhando o foco;
  - escutar no loopback não dispara o pedido de Rede Local nem o firewall;
  - App Nap.
- Workflow de CI com matriz ubuntu-24.04, macos-15 e windows-2025: fmt, clippy -D warnings, testes, build e prints como artefato.

**Verificação:**

- macOS:
  - um dia de Mac alugado (Scaleway Mac mini M1, €0,11/h, mínimo de 24 h), por acesso remoto, seguindo uma checklist;
  - antes e depois, prints do runner macos-15. A imagem oficial faz login gráfico automático e dá ao /bin/bash permissão de gravação de tela (conferido em actions/runner-images).
- Windows:
  - VM com a ISO de avaliação do Windows 11 (90 dias, grátis) ou um PC emprestado;
  - prints do runner windows-2025. Se a sessão é interativa, o primeiro job vai dizer; captura preta conta como NÃO VERIFICADO.
- Resultado: uma tabela com «funciona / plano B / não dá» por item, registrada como decisão antes de codar os backends.

**Risco:**

É o passo que decide o resto.
- Se o clique pelo alfa falhar no macOS, o plano B custa cerca de 1 dia.
- Se aparecer em todas as áreas falhar no Windows, a reserva por API pública (MoveWindowToDesktop quando o foco muda) faz o bicho pular para a área nova.

#### T8.3 — M8 · Linux: qualquer Wayland com layer-shell (KDE Plasma 6, Sway, niri, COSMIC, labwc, Wayfire, river)

**Entregas:**

- Descoberta pelo WAYLAND_DISPLAY (ou varredura do XDG_RUNTIME_DIR), exigindo só zwlr_layer_shell_v1. A varredura do hyprland.lock fica só no modo Docker.
- «Convocar» como base universal: recriar a camada com output NULL nos eventos de atenção. O compositor põe a camada no monitor ativo, e o superficie.rs já espera o enter e o preferred_scale.
- Adaptadores:
  - hyprland (socket2, o plano do M4);
  - sway e i3 (IPC);
  - niri (IPC com tipos serde próprios e tolerantes);
  - kwin (D-Bus e script);
  - wlr-foreign-toplevel genérico.
- Reservas quando faltar fractional-scale (escala inteira) ou cursor-shape.
- doutor mostra backend, adaptador e capacidades.
- Autostart XDG por padrão, --systemd opcional, e o trecho de config para Hyprland, Sway, labwc e niri.

**Verificação:**

- Notebook Linux, sem hardware extra, num laboratório em Docker:
  - Sway, labwc e niri aninhados como janelas dentro do Hyprland, com o pet nativo do contêiner conectado neles;
  - KWin aninhado, ainda a validar;
  - conferir no /v1/estado o monitor, a escala, o D e a região, e a troca entre saídas virtuais.
- CI ubuntu-24.04: Sway e labwc headless (WLR_BACKENDS=headless, renderer pixman) a 1.0 e 1.5, com grim. A checagem de nitidez do M1 (±2, blocos D×D uniformes) roda automática.
- KDE e COSMIC de verdade, numa VM ou com voluntários, seguindo uma checklist:
  - aparece no monitor certo;
  - fica por cima de tela cheia;
  - o clique atravessa.

**Risco:**

Médio.
- Os IPCs mudam: o niri não segue semver e o river mudou de arquitetura.
- «Monitor focado» significa coisas diferentes em cada desktop.

Mitigação: parsers tolerantes, smoke por compositor no CI e a opção seguir = foco, ponteiro ou fixo.

#### T8.4 — M8 · Windows (Win32 nativo)

**Entregas:**

- Crate pet-windows com windows-sys 0.61. O unsafe fica só ali, com SAFETY e clippy::undocumented_unsafe_blocks.
- Janela e desenho:
  - janela pequena que anda, com sprite e balão;
  - DIB de 32 bpp pré-multiplicado com UpdateLayeredWindowIndirect e prcDirty;
  - alfa 1/255 invisível na área de toque, para ela receber clique.
- Entrada:
  - WM_MOUSEACTIVATE responde MA_NOACTIVATE;
  - arraste manual;
  - HWND_TOPMOST reafirmado a cada EVENT_SYSTEM_FOREGROUND.
- Monitores:
  - monitor ativo por SetWinEventHook e MonitorFromWindow, com rcWork para não cobrir a barra;
  - PMv2 por SetProcessDpiAwarenessContext e WM_DPICHANGED.
- Reserva de área virtual com IsWindowOnCurrentVirtualDesktop e MoveWindowToDesktop.
- Palco transitório, todo atravessável, só durante voo e confete.
- Laço MsgWaitForMultipleObjectsEx com timer de alta resolução por objeto, nunca timeBeginPeriod. O ingress acorda o laço com PostMessageW.
- Dois executáveis do mesmo crate: claude-pet.exe (console, para CLI e hook) e o daemon com windows_subsystem windows.
- Arquivos e início:
  - config em %APPDATA%;
  - estado, skins e log com rotação em %LOCALAPPDATA%;
  - autostart na chave Run do HKCU;
  - supervisor que reinicia o daemon.
- Discrição por SHQueryUserNotificationState.

**Verificação:**

- CI windows-2025 a cada PR: build, testes e canários.
- Smoke gráfico no CI, manual ou noturno:
  - o daemon sobe em debug com _teste e recebe um evento;
  - uma rota de debug copia a tela com BitBlt(SRCCOPY|CAPTUREBLT) e compara com /v1/debug/quadro (±2);
  - um SendInput ao lado do bicho chega numa janela de teste embaixo;
  - os prints viram artefato, que você abre no Linux.
- VM ou PC Windows, com checklist manual:
  - áreas virtuais;
  - vídeo em tela cheia;
  - dois monitores com DPI diferente;
  - Windows Terminal e VS Code;
  - custo parado: CPU do dwm até +1 ponto e 0 quadros dormindo.

**Risco:**

- Baixo para a janela, porque a API é documentada.
- Médio para as áreas virtuais, que dependem de comportamento não documentado.
- Médio para SmartScreen e Defender com um binário novo sem assinatura (ver T9.5).
- Jogos em tela cheia exclusiva cobrem o pet.

#### T8.5 — M8 · macOS (AppKit nativo)

**Entregas:**

- Crate pet-macos com objc2 0.6; objc2-app-kit, objc2-quartz-core e objc2-core-graphics 0.3.2; dispatch2.
- App e painel:
  - app Accessory;
  - NSPanel Borderless e NonactivatingPanel, flutuante, becomesKeyOnlyIfNeeded, hidesOnDeactivate=false, transparente e sem sombra;
  - collectionBehavior definido antes do primeiro orderFrontRegardless.
- Desenho: CALayer com CGImage BGRA pré-multiplicado, contentsScale = backingScaleFactor e filtro nearest. Janela pequena que anda, mais o palco transitório.
- Clique: o que o spike decidir.
- Monitor ativo:
  - NSScreen.main reavaliado em NSWorkspaceDidActivateApplicationNotification, mais um timer lento;
  - posição padrão no visibleFrame, que evita o Dock e o entalhe.
- Laço:
  - NSApplication.run;
  - prazos do Motor num CFRunLoopTimer;
  - o ingress acorda o laço pela fila principal;
  - beginActivity enquanto houver sessão ativa, por causa do App Nap.
- Arquivos e início:
  - dados em ~/Library/Application Support;
  - log em ~/Library/Logs;
  - LaunchAgent com KeepAlive.

**Verificação:**

- CI macos-15 (M1) a cada PR: build arm64 e x86_64 (cruzado), testes e canários.
- Smoke gráfico no CI: daemon com _teste, um evento e screencapture -x, comparado com /v1/debug/quadro (±2). Os prints viram artefato.
- Mac real, alugado ou emprestado, com checklist:
  - Spaces, tela cheia e Stage Manager;
  - arrastar e clique atravessando;
  - monitor externo 1x junto com o Retina 2x;
  - macOS 14, 15 e 26 (o CI tem macos-14, macos-15 e macos-26 para o build);
  - custo parado: CPU do WindowServer e Impacto de Energia.

**Risco:**

Médio.
- O clique pelo alfa e o comportamento sobre tela cheia vêm de relatos de campo e podem mudar a cada versão do macOS.
- Nos modos de tela escalados, o WindowServer reamostra a imagem, e a nitidez deixa de ser perfeita para qualquer app.

#### T8.6 — M8 · Linux X11 (Mint Cinnamon, XFCE, MATE, i3, KDE X11) e GNOME experimental via XWayland

**Entregas:**

- x11rb 0.14 com RustConnection, sem libxcb: o binário musl estático continua.
- Janela pequena:
  - visual ARGB quando há compositor;
  - máscara SHAPE bounding sem compositor (pixel art tem alfa binário);
  - SHAPE de entrada na área de toque.
- EWMH:
  - DOCK ou UTILITY;
  - ABOVE, STICKY, SKIP_TASKBAR e SKIP_PAGER;
  - _NET_WM_DESKTOP=0xFFFFFFFF;
  - WM_HINTS input=False;
  - opção override-redirect.
- Monitor ativo por _NET_ACTIVE_WINDOW e RandR.
- O D vem da RandR, do Xft.dpi ou do config.
- No GNOME Wayland, um modo experimental que não segue o foco.

**Verificação:**

- Notebook Linux: Xephyr ou Xvfb em Docker com i3, Openbox e xfwm4, com compositor ligado e desligado:
  - captura com xwd, em pixels do dispositivo, sem o filtro do grim -g;
  - checagem de nitidez;
  - um xdotool click ao lado do bicho chega num xterm embaixo.
- CI ubuntu-24.04 com Xvfb: os mesmos testes, automáticos.
- Mint, XFCE e GNOME 49/50: VM (QEMU, com seu consentimento) ou voluntários.

**Risco:**

Médio.
- Cada gerenciador de janelas trata DOCK e ABOVE de um jeito.
- Sem compositor não há alfa.
- Pelo XWayland não há cursor global nem o foco das janelas Wayland, então nada pode depender de polling do cursor.

#### T8.7 — M8 · Clicar no bicho foca o terminal da sessão (todos os SOs) e a bolha de sessões

**Entregas:**

- Identidade opaca por sessão:
  - no SessionStart e no UserPromptSubmit, o hook manda CLAUDE_PID e as dicas do terminal (TERM_PROGRAM, WT_SESSION, ITERM_SESSION_ID, KITTY_WINDOW_ID, WEZTERM_PANE, TMUX_PANE), nunca títulos;
  - o daemon guarda um anel de ativações com hora e usa o ts do hook para achar a janela que estava ativa.
- Desktop::focar por ambiente:
  - Hyprland: zwlr_foreign_toplevel_handle_v1.activate com hyprland_toplevel_mapping_manager_v1, pelo endereço do activewindowv2. Mantém a decisão 0006: o daemon nunca abre o .socket.sock. Se a cadeia de PIDs for necessária, só o hook roda hyprctl -j clients, em modo leitura.
  - Sway: [con_id] focus.
  - niri: FocusWindow.
  - KWin: script.
  - Outros wlroots: foreign-toplevel.
  - X11: _NET_ACTIVE_WINDOW com source=2.
  - Windows: SetForegroundWindow dentro do clique. A janela do WT vem do dono da pseudo-janela, achado com AttachConsole(CLAUDE_PID) no hook. A aba, por UI Automation, é opcional.
  - macOS: ativa o app do terminal, sem permissão. Opcional: a aba exata por AppleScript (Terminal, iTerm2, Ghostty), que pede permissão de Automação.
- Quando não há como focar, o bicho mostra o balão «não consigo focar aqui».
- Bolha de sessões: os hooks ao vivo, reconciliados com claude agents --json.

**Verificação:**

- Notebook Linux:
  - duas sessões em dois foot, em áreas de trabalho diferentes: o clique foca a certa no Hyprland e, no laboratório, no Sway e no niri;
  - X11 no Xephyr, com dois xterm.
- VM Windows: duas janelas do WT e o VS Code.
- Mac alugado: Terminal.app, iTerm2 e Ghostty.
- Canário: um título de janela nunca aparece no log, no /v1/estado nem no /v1/debug/eventos.

**Risco:**

Médio.
- tmux, zellij e terminais de processo único quebram a cadeia de PIDs; o retrato da janela ativa cobre esses casos.
- O macOS 14+ pode negar a ativação.
- A Microsoft não dá suporte à troca de aba do WT.

#### T8.8 — M8 · GNOME (Ubuntu, Fedora): extensão «overlay remoto» — v1.1, a menos que você decida antes

**Entregas:**

- Extensão GJS mínima:
  - addTopChrome com affectsInputRegion;
  - trackFullscreen opcional;
  - filtro NEAREST;
  - sinais de clique e arraste;
  - métodos para focar janela, ler a janela ativa e o monitor.
- Fala D-Bus com o daemon (zbus 5.19, Rust puro). Toda a lógica fica no Rust.
- Publicada no extensions.gnome.org, separada do binário, porque o EGO não aceita binários.

**Verificação:**

- VM com GNOME 49/50 (Fedora ou Ubuntu), ou GNOME aninhado em modo devkit (a validar).
- Revisão aprovada no EGO.
- Checklist:
  - fica por cima de tudo;
  - o clique fora do bicho atravessa;
  - foca a janela;
  - o logout/login da primeira instalação está documentado.
- Nitidez em escala fracionária: a conferir.

**Risco:**

Médio a alto, de manutenção: o shell-version muda a cada 6 meses, a API muda e a revisão demora. Mesmo assim, é o único jeito decente no GNOME Wayland.

#### T9.0 — M9 · Trilha paralela: personagem, nome e licença (começar já, não depende de código)

**Entregas:**

- Nome público do personagem e do produto, com busca no INPI e no USPTO.
- Comentário na página do pack pedindo à exclusiveOlive permissão por escrito. O modelo da mensagem já está pronto na pesquisa.
- Encomenda de um personagem original, com três orçamentos:
  - cerca de 55 a 60 quadros de 48×48 no vocabulário do pet;
  - entrega do .aseprite e do json-array;
  - CC BY 4.0 ou CC0 escrito na fatura;
  - obra original, sem IA;
  - briefing sem citar personagem ou estúdio de terceiros e sem mandar o pack.

**Verificação:**

- Permissão ou licença por escrito arquivada.
- Decisão registrada no DECISIONS.
- A folha de contato do personagem novo passa pelo mesmo portão do M2: você aprova com skin-aprovar, e cobertura --nativos mvp não aponta faltas.

**Risco:**

- A arte pode atrasar ou sair cara: 3 a 8 semanas de calendário.
- A artista pode não responder, e silêncio não é permissão.

Mitigação: lançar a beta com «traga seu pack» e uma skin provisória livre.

#### T9.1 — M9 · Repositório público limpo

**Entregas:**

- Repositório público:
  - de preferência, um repositório novo, exportado sem o histórico (o privado vira arquivo);
  - ou o atual, com o histórico reescrito.
- DECISIONS e PLANO sem referência a personagem de terceiros (já feito no TP.1, decisão 0035); NOTICE e CREDITS em ordem.
- Guarda no CI que reprova qualquer skin rastreada com redistribuivel: false e qualquer arquivo em skins-locais/.
- Dockerfile publicável, sem COPY skins-locais/.
- SECURITY.md, página de privacidade (só metadados), CONTRIBUTING e modelos de issue com a saída do doutor.

**Verificação:**

- Script no CI: git log --all --name-only sem nenhum arquivo derivado do pack, comparando com as impressões geradas localmente.
- A guarda de nomes do `bin/pet verificar` (decisão 0035) continua verde.
- Uma skin plantada com redistribuivel:false faz o CI falhar (teste da própria guarda).

**Risco:**

Vazar arte por print, GIF ou artefato de CI: num repositório público, os artefatos podem ser baixados por qualquer usuário logado. Mitigação: o CI só usa a _teste e a skin livre.

#### T9.2 — M9 · Skin padrão livre embutida e «traga seu pack» no binário

**Entregas:**

- Skin original embutida (include_bytes!), aprovada pelo build: origem embutida e impressão conhecida.
- O montador do xtask (zip em memória, asefile, receita) vira crates/pet-arte.
- claude-pet skin instalar <zip>:
  - grava na pasta de dados do usuário;
  - nunca baixa nada do itch.io;
  - valida layout e hashes;
  - abre as prévias no visualizador do SO e pede aprovação.
- O build de release falha se tentar embutir uma skin com redistribuivel:false.

**Verificação:**

- CI nos três SOs: skin instalar contra um pack sintético gerado no teste dá a mesma impressão sha256 nos três (determinismo).
- O pack de verdade só roda no seu notebook, nunca no CI.

**Risco:**

O pack pode mudar de formato ou sair de venda. Mitigação: validação com mensagem clara, e a skin livre continua sendo o padrão.

#### T9.3 — M9 · Instalar, iniciar com o sistema, desinstalar e doutor, por SO

**Entregas:**

- claude-pet configurar: autostart, mais claude plugin marketplace add <dono>/<repo> e claude plugin install, sempre com confirmação.
- autostart ativar e desativar: XDG ou systemd, LaunchAgent, chave Run do HKCU.
- Supervisor.
- Instância única por usuário (File::try_lock), que também detecta o modo Docker.
- Menu no botão direito com esconder, soneca e sair: no macOS e no Windows o app não aparece no Dock nem na barra de tarefas.
- desinstalar.
- doutor confere:
  - se o binário está no PATH que o Claude vê;
  - porta e token;
  - backend e capacidades;
  - permissões no macOS.

**Verificação:**

- Contêineres Linux limpos (Arch, Ubuntu, Fedora) para instalar e desinstalar, sem tela.
- Runners macOS e Windows: instalar dos artefatos, doutor verde e desinstalar sem sobras (diff de pastas e do registro).
- Mac alugado e VM Windows: depois de reiniciar a máquina, o pet sobe sozinho.

**Risco:**

No autostart do Sway, ou do Hyprland sem uwsm, pode faltar o WAYLAND_DISPLAY. Mitigação: a varredura do XDG_RUNTIME_DIR que já existe, mais os trechos de config.

#### T9.4 — M9 · Pipeline de release e canais

**Entregas:**

- dist 0.33, com dist = true no crate (que tem publish = false):
  - alvos: Linux musl x86_64 e aarch64, macOS aarch64 e x86_64, Windows x86_64;
  - instaladores: shell, PowerShell, Homebrew (tap próprio) e MSI;
  - atestados do GitHub e SHA256.
- Jobs extras: cargo-deb, cargo-generate-rpm, AUR -bin, PR no winget e, opcional, Scoop.
- Rust fixo no CI (1.98.1), com --locked e --remap-path-prefix.
- Marketplace do plugin no próprio repositório.
- Imagem no GHCR opcional, sem arte de pack.

**Verificação:**

- Tag de ensaio num repositório de teste: todos os jobs verdes e gh attestation verify passando.
- Instalação por canal nos runners:
  - brew install <dono>/tap/<fórmula> no macOS;
  - MSI e PowerShell no Windows;
  - curl | sh e .deb no Ubuntu.
- Dois builds Linux dão o mesmo SHA256.

**Risco:**

O dist não gera .deb, .rpm nem .dmg e não notariza. Mitigação: jobs extras, ou o cargo-packager se o macOS pedir .app ou .dmg.

#### T9.5 — M9 · Assinatura

**Entregas:**

- Windows, uma de duas rotas:
  - SignPath Foundation: grátis, mas exige todos os componentes com licença OSI e nenhuma arte proprietária no binário. Combina com a skin original.
  - Certum Open Source: a partir de €69, com cartão físico.
- macOS:
  - começa sem notarização, porque a fórmula do Homebrew não recebe quarentena;
  - Developer ID e notarização (US$ 99/ano) entram quando houver .dmg ou .pkg pelo navegador, ou o foco na aba exata do terminal.

**Verificação:**

- Windows:
  - signtool verify /pa no runner;
  - baixado pelo navegador numa VM limpa, mostra o nome do editor.
- macOS, depois: spctl -a -vv e codesign -dv no runner.

**Risco:**

O Smart App Control bloqueia binário sem assinatura, sem exceção, e a reputação no SmartScreen leva semanas. Mitigação: assinar antes da divulgação ampla.

#### T9.6 — M9 · Beta pública e v1.0.0

**Entregas:**

- README com:
  - tabela de suporte por SO e desktop, com tiers e recursos;
  - GIF só da skin livre;
  - instalação por canal;
  - plugin;
  - privacidade.
- Checklist manual por SO a cada release.
- Beta de 2 a 4 semanas com modelo de issue.
- Tag v1.0.0.

**Verificação:**

- Instalação nova e completa em cada SO:
  - Linux: Hyprland no notebook, o laboratório Docker e uma VM KDE;
  - macOS: Mac alugado;
  - Windows: VM.
- O fluxo testado em cada um: instalar, configurar, abrir sessão do Claude, ver aceno, pulinho e chamada, clicar e focar o terminal, desinstalar sem sobras.
- Matriz da beta preenchida por voluntários.

**Risco:**

Carga de suporte e regressões a cada atualização de SO ou compositor. Mitigação: tiers claros, doutor e smoke no CI.

### Perguntas para o Renan (como a pesquisa as fez)

- Personagem público (arte), uma de três: (a) encomendar um papagaio original em CC BY 4.0 ou CC0, que é o recomendado; (b) pedir à exclusiveOlive, por escrito, licença para embutir o Zeca nos binários oficiais; (c) lançar só com «traga seu pack» e uma skin provisória livre. A (b) pode correr em paralelo: silêncio não é permissão, e com arte proprietária embutida o projeto não passa na SignPath. Decida também quanto topa gastar: a estimativa, de baixa confiança, é de US$ 165 a 1.400 para uns 55 a 60 quadros.
- Nome do personagem no público: manter «Zeca» ou escolher outro. Decidido em 2026-10-03: Zeca (decisão 0035).
- Nome do produto e do binário: «claude-pet» colide com o xtrimsystems/claude-pet (MIT, desde fev/2026) e usa a marca Claude. Sugestão: um nome próprio (ex.: «bichinho»), com «para o Claude Code» só na descrição. Decida antes da T8.1, porque o nome do binário entra no hooks.json e no PATH de todo mundo. Faça a busca no INPI e no USPTO.
- Receita do Zeca no app público: o montador «traga seu pack», com chapéu-palheta e gravata, vai no binário público como skin opcional, ou fica só no seu fork pessoal (o que reduz o risco de associação com personagens de terceiros).
- Como abrir o código: repositório público novo e limpo, sem o histórico, ou o atual com o histórico reescrito. Recomendo o novo, porque o histórico cita um personagem de terceiros (os textos atuais já foram reescritos, decisão 0035). Decida também a licença da arte original (CC BY 4.0 ou CC0); o código segue MIT.
- Escopo Linux da v1.0: o Tier 1 sugerido é Hyprland, KDE Plasma 6, Sway e niri. O GNOME (Ubuntu e Fedora, via extensão) entra na v1.0 ou na v1.1? E o X11 (Mint, XFCE, MATE, i3), na v1.0 ou na v1.1?
- Windows na v1.0 (dá) e a rota de assinatura: SignPath Foundation (grátis, com condições), Certum Open Source (€69+), Microsoft Store (MSIX, grátis) ou beta sem assinatura com instruções. Mínimo suportado: Windows 11, com o Windows 10 (fora de suporte desde out/2025) como melhor esforço.
- macOS: começar sem pagar (Homebrew, sem notarização, macOS 14+) e entrar no Apple Developer Program (US$ 99/ano) só quando chegar o .dmg pelo navegador ou o foco na aba exata do terminal; ou pagar já.
- Testes e CI: ligar o GitHub Actions (o PLANO deixou opcional; é grátis em repo público e cobrado por minuto em privado); alugar um Mac por dia (Scaleway, ~€2,64 por rodada) ou pedir um emprestado; para o Windows, VM de avaliação no notebook ou na nuvem. A VM no notebook exige instalar QEMU/virt-manager e os targets do rustup, o que muda o sistema, e 7,4 GiB de RAM fica apertado. E aceitar ou não voluntários na beta.
- Ordem: antecipar a costura de plataforma (T8.0) e o hook nativo (T8.1) para antes do M4, ou deixar tudo para depois do M7, como você pediu («no final de tudo»). Recomendo antecipar: evita reescrever arraste, balões e voos.
- Docker: manter o modo Docker como opção oficial no Linux (só com Docker Engine, e com mais trabalho de suporte) ou oferecer ao público só a instalação nativa, com o Docker como ferramenta de desenvolvimento e laboratório de compositores.
- Ação sua quando decidir: postar o comentário na página do pack pedindo permissão à exclusiveOlive (o modelo da mensagem está pronto na pesquisa) e pedir os orçamentos de arte.
- Suporte: quanto tempo quer dedicar a issues, e se aceita contribuições de terceiros, inclusive skins, sempre com licença declarada.

### Custos e prazos

#### Dinheiro

| Item | Quando | Custo | Fonte | Confiança |
|---|---|---|---|---|
| GitHub Actions (runners Linux, macOS e Windows) | M8 e M9 | US$ 0 em repositório público ("free and unlimited") | https://docs.github.com/en/actions/reference/runners/github-hosted-runners | alta |
| GitHub Actions, se o repositório continuar privado | até abrir | 2.000 min/mês grátis; depois US$ 0,006/min no Linux, 0,010 no Windows e 0,062 no macOS | https://docs.github.com/en/billing/concepts/product-billing/github-actions | alta |
| Mac de verdade (Scaleway Mac mini M1) | spike T8.2 e cada release | €0,11/h, mínimo de 24 h: ≈ €2,64 por rodada (ou €75/mês) | https://www.scaleway.com/en/pricing/apple-silicon/ | alta |
| Windows para conferir | spike e releases | €0 com o Windows 11 Enterprise de avaliação (90 dias, ISO x64 e Arm64) numa VM; VM na nuvem por hora (preço não verificado) | https://www.microsoft.com/en-us/evalcenter/evaluate-windows-11-enterprise | alta |
| Pack Cute Parrots! (só para quem quiser o Zeca) | cada usuário | US$ 0,50+ | https://exclusiveolive.itch.io/cute-parrots-pixel-art-asset-pack | alta |
| Personagem original encomendado (~55–60 quadros de 48×48) | antes da v1.0 | ~US$ 165–1.400; para ter paridade com o pack (~92 quadros), até ~US$ 2.300 | PixelJoint 2026 (https://pixeljoint.com/forum/forum_topics.asp?FID=20), extrapolado | baixa |
| Apple Developer Program | quando houver .dmg pelo navegador ou foco exato no macOS | US$ 99/ano; pessoa física não tem isenção | https://developer.apple.com/help/account/membership/fee-waivers/ | alta |
| Assinatura no Windows | antes da divulgação ampla | ver lista abaixo | https://signpath.org/terms · https://shop.certum.eu/open-source-code-signing.html · https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options · https://learn.microsoft.com/en-us/azure/artifact-signing/quickstart | alta |
| Homebrew tap, AUR, winget, Scoop, GitHub Releases/Pages, GHCR público | sempre | US$ 0 | — | alta |
| Revisão jurídica de nome e visual | opcional; recomendada se ficar Zeca + chapéu + gravata | não estimado | — | — |

Opções de assinatura no Windows:
- **SignPath Foundation:** grátis. Exige todos os componentes com licença OSI e nenhuma arte proprietária no binário.
- **Certum Open Source:** a partir de €69, com cartão e leitora; o envio para o Brasil não foi conferido.
- **Certificado OV:** US$ 150–300/ano.
- **Microsoft Store:** grátis.
- **Azure Artifact Signing:** não serve, porque para pessoa física só atende EUA e Canadá.

**Cenários de gasto**
- **Mínimo, US$ 0:** beta sem assinatura, "traga seu pack" com uma skin provisória livre, prints do CI e voluntários para a conferência visual.
- **Recomendado no 1º ano, ~US$ 300–1.500:**
  - a maior parte é a arte encomendada;
  - ~€10–15 de Mac alugado (4 a 5 rodadas);
  - €0–69 de assinatura no Windows;
  - US$ 99/ano da Apple quando entrar o foco exato ou o .dmg.

#### Tempo

Estimativa de baixa confiança, em dias de trabalho focado, com o Claude implementando e você revisando.

| Passo | Esforço | Calendário / espera |
|---|---|---|
| T8.0 costura | 1–2 d | — |
| T8.1 hook nativo + CLI | 1–2 d | — |
| T8.2 spikes + CI | 2–3 d | 1 dia de Mac alugado; montar a VM do Windows |
| T8.3 Wayland genérico | 3–5 d | — |
| T8.4 Windows | 5–8 d | idas e voltas pelo CI e pela VM |
| T8.5 macOS | 5–8 d | 1–2 dias de Mac alugado |
| T8.6 X11 | 3–4 d | — |
| T8.7 focar o terminal (todos) | 4–7 d | Mac e VM |
| T8.8 GNOME | 4–6 d | revisão do EGO: dias a semanas (não verificado) |
| T9.0 arte, nome e permissão | 1–2 d seus | 3–8 semanas (encomenda) |
| T9.1 repositório limpo | 1–2 d | — |
| T9.2 skin embutida + traga seu pack | 2–3 d | depende da arte |
| T9.3 instalar, autostart, doutor | 2–4 d | — |
| T9.4 release e canais | 3–5 d | PR no winget: dias |
| T9.5 assinatura | 1–2 d | SignPath com aprovação manual (semanas, não verificado); inscrição na Apple |
| T9.6 beta → v1.0 | 2–3 d | 2–4 semanas de beta |

**Totais**
- M8: ≈ 28–45 dias.
- M9: ≈ 12–21 dias.
- Beta pública mínima nos três SOs (T8.0–T8.5 + T9.1–T9.4, sem X11, sem GNOME e sem foco exato): ≈ 25–40 dias.
- v1.0 completa: ≈ 40–65 dias de trabalho, ou 2 a 3 meses em tempo integral; em meio período, mais. A arte, a assinatura e a beta correm em paralelo e puxam o calendário.

O código anda rápido: o M0–M3 saiu em uns 2 dias. O que não acelera é conferir em Mac e Windows de verdade e esperar arte, revisões e beta.

## Achado 1 — Janelas por sistema (macOS, Windows, X11)

*Zeca em macOS, Windows e Linux X11: a janela do pet em Rust*

*Pesquisa só de leitura, feita em 2026-10-03. Li o código-fonte pelo `gh api`: winit v0.30.13 e v0.31.0-beta.3, softbuffer (master), SDL3, BongoCat, eSheep e Shimeji-Desktop. Também consultei a documentação oficial (Microsoft Learn, Apple Developer, EWMH da freedesktop, wayland.app) e o crates.io. A cota de WebSearch da sessão estava esgotada, então usei WebFetch e a API do GitHub. O que vem de issues ou PRs de terceiros está marcado como relato de campo.*

### Resposta curta
- **Dá para ter Windows, e para esta janela ele é a plataforma mais fácil.** Uma janela *layered* atualizada com `UpdateLayeredWindow` resolve duas coisas de uma vez, e isso é documentado:
  - alfa por pixel;
  - o clique atravessa os pixels com alfa 0, sem polling.

  Os estilos `WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST` tiram a janela da barra de tarefas, impedem que ela roube o foco e a deixam por cima de tudo.
- **macOS também dá**, com três peças:
  - um `NSPanel` não ativador;
  - o app como `Accessory`;
  - `collectionBehavior = CanJoinAllSpaces | FullScreenAuxiliary | Stationary | IgnoresCycle`.

  O ponto incerto é deixar só o sprite clicável. Um terceiro mediu que o WindowServer decide o clique pelo alfa de cada pixel numa janela AppKit não opaca. Isso falhou com GTK4, então depende do jeito de desenhar. O plano B robusto é ligar e desligar `ignoresMouseEvents` conforme a posição do cursor, que é o que o SDL3 faz.
- **No Linux:**
  - O backend atual (SCTK + layer-shell) já atende Hyprland e serve também para KWin 6, Sway, niri, COSMIC, Wayfire e labwc.
  - X11 se resolve com `x11rb`, que é Rust puro e mantém o binário musl estático.
  - **GNOME Wayland é o ponto fraco.** O Mutter não implementa layer-shell. Pelo XWayland, o app não vê a posição do cursor nem o foco das janelas Wayland nativas.
- **winit + softbuffer não serve como camada única**:
  - softbuffer 0.4.8 não tem alfa, e mesmo no master Win32 e X11 continuam sem transparência.
  - winit só torna a janela inteira atravessável ao clique.
  - No Windows, winit reescreve `GWL_EXSTYLE` sozinho.
  - No Linux, winit carrega libX11, libxkbcommon e libwayland por dlopen e não tem layer-shell.
- **Recomendação:**
  - Manter `pet-core` puro.
  - Escrever três backends nativos e finos, ao lado do `wl/` atual: `win/` (windows-sys), `mac/` (objc2-app-kit) e `x11/` (x11rb).
  - Em macOS, Windows e X11, usar **uma janela pequena que se move** (sprite + balão) e um palco grande só durante voo e confete. É assim que fazem o eSheep, o Shimeji e o BongoCat (23,7 mil estrelas), que em 2026 trocou o Tauri por overlays nativos em Rust.

### 1. winit e softbuffer, conferidos no código
Versões atuais:
- winit **0.30.13**, estável, de 2026-03-02;
- winit 0.31.0-beta.3, de 2026-09-04;
- softbuffer **0.4.8**, de 2025-12-13.

| Necessidade | Como o winit 0.30.13 faz | Problema |
|---|---|---|
| Transparência | macOS: `setOpaque(false)` + `clearColor`; Windows: `DwmEnableBlurBehindWindow`; X11: visual TrueColor de 32 bits | No Windows, esse caminho não decide o clique pelo alfa |
| Sempre por cima | `WindowLevel::AlwaysOnTop`: `kCGFloatingWindowLevel` (macOS), `WS_EX_TOPMOST` (Windows), `_NET_WM_STATE_ABOVE` (X11) | Não funciona no Wayland |
| Clique atravessa | `set_cursor_hittest`: Windows liga `WS_EX_TRANSPARENT\|WS_EX_LAYERED`; macOS chama `setIgnoresMouseEvents`; X11 usa SHAPE de entrada vazio ou cheio | Só a janela inteira; não há API de região |
| Sem foco e fora da barra | Windows: só `with_skip_taskbar`, via `ITaskbarList::DeleteTab`. macOS 0.30: `WinitWindow` responde `canBecomeKeyWindow = true`, então rouba o foco | 0.31-beta: `with_panel(true)` (NSPanel não ativador) e `with_fullscreen_auxiliary`; continua sem `CanJoinAllSpaces` |
| Estilos próprios no Windows | `apply_diff` recalcula as flags e chama `SetWindowLongW(GWL_EXSTYLE, …)` | Um `WS_EX_TOOLWINDOW` ou `WS_EX_NOACTIVATE` posto à mão some no próximo `set_cursor_hittest` ou `set_window_level`; continua assim no 0.31-beta.3 |
| Linux | `x11-dl`, x11rb com `dl-libxcb`, `xkbcommon-dl`, SCTK 0.19 | dlopen não combina com o binário musl estático (decisão 0004); não há layer-shell |

**softbuffer:**
- Na 0.4.8, "os 8 bits mais altos devem ser 0", ou seja, não há alfa.
- O master tem `AlphaMode`, ainda não lançado, mas `supports_alpha_mode` aceita só `Opaque | Ignored` em Win32 ("TODO: Support transparency") e em X11.
- No master, macOS aceita só `Postmultiplied` e Wayland, `Premultiplied`.
- Conclusão: para pixel art transparente, seria preciso escrever o apresentador de cada plataforma de qualquer jeito.

### 2. Receita por plataforma

#### Windows (`windows-sys` 0.61 ou `windows` 0.62)

**Criar a janela e desenhar**
- Criar com `CreateWindowExW(WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE, …, WS_POPUP, …)`.
- Desenhar num DIB de 32 bpp (`CreateDIBSection`, BGRA **pré-multiplicado**).
- Enviar com `UpdateLayeredWindow` ou `UpdateLayeredWindowIndirect`:
  - `BLENDFUNCTION{AC_SRC_OVER, 0, 255, AC_SRC_ALPHA}` e `ULW_ALPHA`;
  - `prcDirty`, para atualizar só o retângulo sujo;
  - posição e conteúdo vão na mesma chamada (`pptDst`).

**Só o sprite clicável**
- Vem de graça. A documentação diz que nas áreas "whose alpha value is zero" o mouse passa.
- **Não** pôr `WS_EX_TRANSPARENT`: com ele, a forma é ignorada.
- Truque: pixels com alfa 1/255 em volta do bicho aumentam a área clicável sem aparecer.
- Mecanismos a evitar:
  - `WM_NCHITTEST → HTTRANSPARENT`: só repassa o clique para janelas da mesma thread.
  - `SetWindowRgn`: recorta também o desenho. É o que SDL3 e Godot usam.

**Foco e arraste**
- `WS_EX_NOACTIVATE`: a janela "does not become the foreground window when the user clicks it" e não aparece na barra.
- Responder `WM_MOUSEACTIVATE → MA_NOACTIVATE`, por garantia.
- Arraste manual: `SetCapture`, `GetCursorPos` e `UpdateLayeredWindow` com `pptDst`.

**Ficar por cima**
- Reafirmar `SetWindowPos(HWND_TOPMOST, …, SWP_NOMOVE|SWP_NOSIZE|SWP_NOACTIVATE)` sempre que mudar a janela em primeiro plano. O eSheep reafirma a cada animação.
- Não aparece sobre jogos em tela cheia exclusiva.

**Áreas de trabalho virtuais**
- A API pública `IVirtualDesktopManager` só tem `GetWindowDesktopId`, `IsWindowOnCurrentVirtualDesktop` e `MoveWindowToDesktop`. Não há como fixar uma janela em todas as áreas.
- Relatos de campo: janelas `WS_EX_TOOLWINDOW`, ou tiradas da barra por `DeleteTab`, aparecem em **todas** as áreas. O sistema de áreas virtuais deixa de rastreá-las, e `GetWindowDesktopId` devolve 0x8002802B.
  - É exatamente o que o pet quer, mas não é documentado.
  - Há um relato contrário, com uma janela Tk.
  - Testar no Win10 e no Win11 24H2/25H2.
- Plano B com API pública: quando a janela em primeiro plano mudar e `IsWindowOnCurrentVirtualDesktop` for falso, chamar `MoveWindowToDesktop(nossa, GetWindowDesktopId(janela_em_foco))`.
- Fixar de verdade só pelas interfaces internas (VirtualDesktopAccessor / `winvd`), que exigem um build específico do Win11.

**Monitor ativo**
- `SetWinEventHook(EVENT_SYSTEM_FOREGROUND, …, WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS)`: não precisa de DLL e chega na thread que tem o laço de mensagens.
- Depois, `MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST)` e `GetMonitorInfoW` (`rcWork`).
- `EVENT_SYSTEM_MOVESIZEEND` cobre a janela arrastada para outro monitor.

**DPI**
- Per-Monitor V2, por manifesto ou `SetProcessDpiAwarenessContext`. A documentação garante "raw pixels of each display" e nunca esticar o bitmap.
- Tratar `WM_DPICHANGED` usando o retângulo sugerido.
- Com base 4, as escalas 125/150/175% viram D = 5/6/7: blocos inteiros e nítidos.

**Clicar para focar o terminal**
- `SetForegroundWindow` é permitido quando "the calling process received the last input event". O clique foi no Zeca, então foi nosso.

#### macOS (objc2 0.6 + objc2-app-kit 0.3.2 + objc2-quartz-core/core-graphics)

**App e painel**
- `NSApplication.setActivationPolicy(.Accessory)`: sem ícone no Dock. Mas a Apple avisa que um app Accessory "may be activated … by clicking on one of its windows", por isso o painel precisa ser não ativador.
- Criar com `NSPanel::initWithContentRect_styleMask_backing_defer(…, Borderless | NonactivatingPanel, Buffered, false)`.
- Chamar `setFloatingPanel(true)` e `setBecomesKeyOnlyIfNeeded(true)`.
- Chamar **`setHidesOnDeactivate(false)`**: no NSPanel o padrão é `true`.
- Chamar `setOpaque(false)`, `setBackgroundColor(clearColor)` e `setHasShadow(false)`. Esses métodos do objc2-app-kit 0.3 são `safe`.

**Spaces e tela cheia**
- `setCollectionBehavior(CanJoinAllSpaces | FullScreenAuxiliary | Stationary | IgnoresCycle)` **antes** do primeiro `orderFrontRegardless()`.
- Relatos de 2026 (typelite, claude-buddy, voicebox) e a prática do BongoCat:
  - uma `NSWindow` comum não entra no Space de tela cheia de outro app;
  - um `NSPanel` não ativador com `FullScreenAuxiliary` entra.
- Nível da janela:
  - `NSFloatingWindowLevel` (3) fica abaixo do Dock (20) e da barra de menus (24);
  - para ficar acima deles, usar `NSStatusWindowLevel` (25); o BongoCat usa `NSMainMenuWindowLevel`.

**Desenho**
- `CALayer.contents` recebe um `CGImage` BGRA pré-multiplicado (`PremultipliedFirst | ByteOrder32Little`).
- `contentsScale = backingScaleFactor` e `magnificationFilter = nearest`.

**Só o sprite clicável (spike curto antes de codar)**
- (a) Clique decidido pelo alfa no WindowServer:
  - medido por terceiros numa `NSWindow` AppKit pura, com `opaque=false` e fundo `clearColor`: os pontos transparentes caíram na janela de baixo;
  - o LICEcap usa isso desde 2011;
  - com GTK4 não funcionou, então pode falhar com conteúdo de CALayer.
- (b) Plano B: `setIgnoresMouseEvents(true)` e alternar pela posição do cursor:
  - `NSEvent::addGlobalMonitorForEventsMatchingMask_handler(MouseMoved|LeftMouseDragged)`: eventos de mouse não pedem Acessibilidade, só os de teclado pedem;
  - ou `NSEvent::mouseLocation()` a 20–30 Hz, só perto do bicho;
  - é o que o SDL3 faz (`updateIgnoreMouseState`);
  - quando a janela estiver clicável, usar `NSTrackingArea` com `activeAlways` (o app nunca fica ativo) e `acceptsFirstMouse`.

**Monitor ativo**
- `NSScreen::mainScreen`, que a Apple descreve como "the screen containing the window that is currently receiving keyboard events". O Hammerspoon o usa como "tela da janela em foco".
- Reavaliar em `NSWorkspaceDidActivateApplicationNotification`, mais um timer lento.

**Escala**
- `backingScaleFactor` vale 2.0 em modos HiDPI e 1.0 nos demais.
- Nos modos escalados, o WindowServer reamostra o quadro inteiro: nitidez perfeita só existe na resolução nativa, e isso vale para qualquer app.

#### Linux X11 e XWayland (x11rb 0.14, features `shape` e `randr`, `shm` opcional)

**Janela e desenho**
- Visual TrueColor de 32 bits + colormap.
- Desenhar com `put_image` (ZPixmap, BGRA pré-multiplicado) ou com MIT-SHM.

**Só o sprite clicável**
- `shape_rectangles(SO::SET, SK::INPUT, ClipOrdering::UNSORTED, win, 0, 0, &faixas)`: o mesmo mecanismo que o winit usa, sem polling.
- Sem compositor (ninguém é dono de `_NET_WM_CM_S0`), ARGB não fica transparente. Nesse caso, usar `SK::BOUNDING` com a máscara do sprite. Pixel art de alfa binário recorta perfeito, como o xeyes.

**Dicas EWMH**
- Tipo de janela: `_NET_WM_WINDOW_TYPE_DOCK` (o Shimeji usa) ou `_NOTIFICATION` / `_UTILITY`.
- `_NET_WM_STATE_ABOVE | STICKY | SKIP_TASKBAR | SKIP_PAGER`.
- `_NET_WM_DESKTOP = 0xFFFFFFFF`: "SHOULD appear on all desktops".
- `WM_HINTS.input = False`.
- Override-redirect é a alternativa sem WM; aí o próprio pet se reergue.

**Monitor ativo**
- `PropertyNotify` de `_NET_ACTIVE_WINDOW` na raiz, geometria da janela e `randr::get_monitors`.

**XWayland (GNOME Wayland)**
- `_NET_ACTIVE_WINDOW` e a posição do cursor só refletem janelas X11. Há um relato no KDE 6.6 de cursor desatualizado sobre janelas Wayland.
- Por isso o clique só no sprite tem de vir da forma (SHAPE), nunca de polling.
- Com escala fracionária, o compositor pode borrar janelas X11 se escalar o XWayland.

**Binário estático**
- x11rb sem `allow-unsafe-code` é Rust puro, sem libxcb, então o binário musl estático continua possível.

#### Wayland fora do Hyprland
- Layer-shell existe no KWin 6.7, Sway, niri, COSMIC, Wayfire e labwc. **Mutter (GNOME) e Weston não têm.**
- O backend SCTK atual serve nesses compositores. Muda só a fonte do "monitor em foco": IPC do Sway ou do niri, script do KWin, ou `wlr-foreign-toplevel`.
- No GNOME Wayland, as saídas são:
  - cair no backend X11/XWayland: fica por cima e com clique por SHAPE, mas não segue o monitor;
  - mais tarde, uma extensão do GNOME Shell, como o Shijima-Qt fez.

### 3. Janela do tamanho do monitor ou janela pequena
- **No Hyprland, a camada do tamanho do monitor fica.** Lá ela foi medida e cabe no orçamento (decisão 0005).
- **Em macOS, Windows e X11, prefira janela pequena** do tamanho da união sprite + balão, com margem. Ela anda com `SetWindowPos`/`UpdateLayeredWindow`, `setFrameOrigin` ou `ConfigureWindow`.
- O custo por quadro escala com a área atualizada:
  - sprite 48×48 com D=6 dá 288×288 px, cerca de 324 KiB por quadro;
  - um monitor 4K tem cerca de 33 MB por quadro, e um 5K, cerca de 59 MB.
- No macOS, softbuffer, e CALayer de modo geral, copia o buffer inteiro a cada apresentação. Não há atualização parcial no AppKit.
- Uma janela pequena também limita o estrago se o clique pelo alfa falhar, e simplifica a disputa de "sempre por cima".
- Voo pela tela e confete:
  - aumentar a janela temporariamente, ou abrir uma segunda janela de efeitos, toda atravessável ao clique, só durante o efeito;
  - é o "plano B" que a própria decisão 0005 já descrevia.

### 4. Custo parado e bateria
- Parado deve dar 0 quadros: laço guiado por eventos, sem vsync contínuo.
- Uma janela layered ou um CALayer estático não custa CPU; o compositor guarda a imagem.
- Animação: o custo é quadros por segundo × área suja. Usar `prcDirty` (Windows), regiões sujas no `put_image` (X11) e janela pequena (macOS).
- Timers de 16 ms ou mais. Nunca usar `timeBeginPeriod(1)`: a Microsoft avisa que "high resolutions can also prevent the CPU power management system from entering power-saving modes".

### 5. Alternativas
| Opção (versão) | Clique só no sprite | Sem foco / todos os Spaces | Overlay no Wayland | Veredito |
|---|---|---|---|---|
| winit 0.30.13 + softbuffer 0.4.8 | não (janela inteira) e sem alfa | parcial; Windows reescreve estilos; macOS rouba foco | não (dlopen, sem layer-shell) | descartar |
| winit 0.31-beta + apresentadores próprios | à mão, por plataforma | macOS ok com `with_panel`; falta `CanJoinAllSpaces`; Windows ainda reescreve | não | só no macOS, se o objc2 direto pesar |
| tao 0.37 | não (janela inteira) | `visible_on_all_workspaces` não funciona no Windows | GTK3, sem "por cima" no Wayland | descartar |
| Tauri 2.12 | não nativo (polling) | sim com `macos-private-api` + tauri-nspanel | fraco (WebKitGTK) | descartar: refaria o render e pesa |
| egui/eframe 0.36 | `with_mouse_passthrough` só da janela inteira | parcial | não | descartar (GPU) |
| SDL3 (crate `sdl3` 0.20) | `SDL_SetWindowShape`: SetWindowRgn (Win), XShape de entrada (X11), alterna `ignoresMouseEvents` (macOS); sem Wayland | `UTILITY`, `NOT_FOCUSABLE`; Spaces exigem objc2 | não | plano B para Win/mac/X11 |
| Godot 4 | `window_set_mouse_passthrough` com polígono (Win/mac/X11; no Win recorta o desenho) | `NO_FOCUS` | não | descartar (runtime grande) |
| **Nativo fino** (windows-sys / objc2-app-kit / x11rb + SCTK atual) | sim, sem polling em Win/X11/Wayland; macOS a validar | sim | sim (já existe) | **recomendado** |

### 6. Como os pets existentes fazem
| Projeto | Stack e plataformas | Técnica |
|---|---|---|
| eSheep (C# WinForms) | Windows | Janela pequena por ovelha que se move; color key `TransparencyKey` magenta; `WS_EX_TOOLWINDOW\|TOPMOST\|LAYERED\|NOACTIVATE`; reafirma TopMost a cada animação |
| Shimeji-Desktop (Java 25, port do Shimeji-ee) | Windows, macOS, Linux | `JWindow` pequena por mascote; alfa por pixel; `contains()` pelo alfa; no X11, `_NET_WM_WINDOW_TYPE_DOCK` |
| Shijima-Qt (C++/Qt6, GPL-3.0) | Windows, macOS, KDE/GNOME | Arquivado em 2026 ("Qt was not the right framework"); rastreia janelas por plugin de shell; no macOS pede Acessibilidade |
| Desktop Goose | Windows, macOS | Código fechado; sem Linux |
| VPet (WPF) | Windows | Código Apache-2.0; arte com copyright separado |
| BongoCat (Rust, Apache-2.0, 23,7 mil estrelas) | Windows, macOS; Linux adiado (ADR-0006) | Win32 + D3D11 + DirectComposition (`WS_POPUP`, `WS_EX_TOOLWINDOW\|NOACTIVATE\|NOREDIRECTIONBITMAP`); NSPanel não ativador + Metal com `CanJoinAllSpaces\|FullScreenAuxiliary` |
| Pets de Claude Code (Clyde, CoPet, sidecrab, tokibean, agent-pet) | Tauri 2 | `macos-private-api`; alguns usam tauri-nspanel, objc2 ou o crate windows |
| dsh-plugin-pet-rs | winit 0.30 + softbuffer 0.4 + tiny-skia | Janela pequena de 280×340 pt |
| gnat | SCTK 0.21 | Hyprland/Wayland, a mesma base do Zeca |

### 7. Stack recomendada e confiança
- `pet-core` puro, mais um contrato de plataforma no `claude-pet`: janela, apresentar com dano, posição, região de entrada, monitores, foco, ponteiro, acordar. Confiança **alta**.
- Windows com windows-sys e janela layered + `UpdateLayeredWindowIndirect`. Confiança **alta**.
- macOS com objc2-app-kit: NSPanel não ativador, app Accessory, `collectionBehavior` e CALayer. Confiança **média-alta**; o clique pelo alfa é **média**, a validar.
- X11 com x11rb (ARGB + SHAPE + EWMH). APIs: confiança **alta**. Comportamento entre WMs e XWayland: **média**.
- Wayland com o SCTK atual. wlroots e KWin: confiança **alta**. GNOME só como fallback: **baixa**.
- Janela pequena + palco transitório. Confiança **média-alta**.
- Não usar winit + softbuffer. Confiança **alta**.
- winit 0.31 só como alternativa no macOS, quando sair a versão estável.

### Fatos conferidos

- A versão estável atual do winit é a 0.30.13 (2026-03-02); a mais nova é a 0.31.0-beta.3 (2026-09-04). A do softbuffer é a 0.4.8 (2025-12-13). *Evidência:* https://crates.io/api/v1/crates/winit ; https://crates.io/api/v1/crates/softbuffer. *Confiança:* alta.
- softbuffer 0.4.8 não tem alfa: em cada u32, os 8 bits mais altos devem ser 0. A atualização parcial (present_with_damage) só existe em Wayland, X (com XShm), Win32 e Web; no AppKit, cada apresentação faz uma cópia bloqueante do buffer inteiro. *Evidência:* https://docs.rs/softbuffer/latest/softbuffer/struct.Buffer.html. *Confiança:* alta.
- O master do softbuffer (não lançado) tem AlphaMode, mas Win32 e X11 só aceitam Opaque/Ignored (o win32 diz “TODO: Support transparency”). macOS aceita Postmultiplied (Premultiplied “doesn't seem to work”) e Wayland, Premultiplied. *Evidência:* github.com/rust-windowing/softbuffer master: src/backends/win32.rs, x11.rs, cg.rs (supports_alpha_mode), wayland/mod.rs; src/lib.rs (enum AlphaMode). *Confiança:* alta.
- winit 0.30.13 no Windows: set_cursor_hittest(false) liga WS_EX_TRANSPARENT|WS_EX_LAYERED (janela inteira); AlwaysOnTop vira WS_EX_TOPMOST; a transparência vem de DwmEnableBlurBehindWindow. *Evidência:* winit v0.30.13 src/platform_impl/windows/window_state.rs (to_window_styles) e window.rs (DwmEnableBlurBehindWindow, ~L1239). *Confiança:* alta.
- winit no Windows recalcula e grava GWL_EXSTYLE inteiro a cada mudança de flag (apply_diff → SetWindowLongW). Estilos WS_EX_* postos à mão somem; continua assim no 0.31.0-beta.3. *Evidência:* winit v0.30.13 window_state.rs apply_diff; v0.31.0-beta.3 winit-win32/src/window_state.rs L463. *Confiança:* alta.
- winit no Windows não tem opção de WS_EX_TOOLWINDOW nem de WS_EX_NOACTIVATE para janelas comuns: só with_skip_taskbar, via ITaskbarList::DeleteTab. No 0.31-beta, WS_EX_NOACTIVATE só entra em WindowType::Popup. *Evidência:* winit v0.30.13 src/platform/windows.rs (WindowAttributesExtWindows); window.rs set_skip_taskbar; v0.31.0-beta.3 winit-win32 window_state.rs L307-313. *Confiança:* alta.
- winit 0.30.13 no macOS: AlwaysOnTop = kCGFloatingWindowLevel; set_cursor_hittest = setIgnoresMouseEvents; WinitWindow (subclasse de NSWindow) responde canBecomeKeyWindow = true; ActivationPolicy::Accessory existe. *Evidência:* winit v0.30.13 src/platform_impl/macos/window_delegate.rs (set_window_level, set_cursor_hittest); macos/window.rs (declare_class WinitWindow); src/platform/macos.rs. *Confiança:* alta.
- winit 0.31.0-beta.1 adicionou WindowAttributesMacOS::with_panel (NSPanel + NonactivatingPanel) e a beta.3 adicionou with_fullscreen_auxiliary. Não há API para CanJoinAllSpaces; a issue #4670 pede mais opções de NSPanel. *Evidência:* winit master winit/src/changelog/v0.31.md; winit-appkit/src/lib.rs e window_delegate.rs @v0.31.0-beta.3; https://github.com/rust-windowing/winit/issues/4670. *Confiança:* alta.
- winit no X11: set_cursor_hittest usa a forma de entrada do SHAPE (shape_rectangles SO::SET, SK::INPUT) com retângulo vazio ou cheio. Tem with_override_redirect e with_x11_window_type, mas não tem skip-taskbar nem sticky. *Evidência:* winit v0.30.13 src/platform_impl/linux/x11/window.rs L1623-1656; src/platform/x11.rs. *Confiança:* alta.
- winit 0.30.13 no Linux depende de x11-dl, x11rb com dl-libxcb, xkbcommon-dl e SCTK 0.19 (wayland-backend client_system): carrega bibliotecas do sistema e não tem layer-shell. *Evidência:* winit v0.30.13 Cargo.toml L254-288. *Confiança:* alta.
- Windows: o clique numa janela layered segue a forma e o alfa; áreas de alfa zero deixam o mouse passar, a menos que a janela tenha WS_EX_TRANSPARENT. A doc também recomenda UpdateLayeredWindow para animar posição e tamanho. *Evidência:* https://learn.microsoft.com/en-us/windows/win32/winmsg/window-features (seção Layered Windows). *Confiança:* alta.
- HTTRANSPARENT no WM_NCHITTEST só repassa o clique para janelas da mesma thread, então não serve para atravessar até outro processo. *Evidência:* https://learn.microsoft.com/en-us/windows/win32/inputdev/wm-nchittest. *Confiança:* alta.
- Com UPDATELAYEREDWINDOWINFO.prcDirty, UpdateLayeredWindowIndirect atualiza só o retângulo indicado. *Evidência:* https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-updatelayeredwindowinfo. *Confiança:* alta.
- WS_EX_NOACTIVATE: a janela não vira primeiro plano quando clicada e não aparece na barra de tarefas por padrão. WS_EX_TOOLWINDOW: fica fora da barra e do Alt+Tab. *Evidência:* https://learn.microsoft.com/en-us/windows/win32/winmsg/extended-window-styles. *Confiança:* alta.
- A API pública de áreas de trabalho virtuais (IVirtualDesktopManager) só tem GetWindowDesktopId, IsWindowOnCurrentVirtualDesktop e MoveWindowToDesktop; não há como fixar em todas. Fixar só pelas interfaces internas (VirtualDesktopAccessor/winvd, que exige Win11 24H2 26100.2605 ou mais novo). *Evidência:* https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nn-shobjidl_core-ivirtualdesktopmanager ; https://github.com/Ciantic/VirtualDesktopAccessor. *Confiança:* alta.
- Relatos de campo: janelas WS_EX_TOOLWINDOW, ou tiradas da barra por DeleteTab, aparecem em todas as áreas virtuais porque o sistema deixa de rastreá-las (GetWindowDesktopId devolve 0x8002802B). Popups layered/tool sem dono também aparecem em todas. Há um relato contrário com uma janela Tk. *Evidência:* https://github.com/logecolib/sharesticky/issues/22 ; https://github.com/luke-you/tacky-borders/pull/58 ; contrário: https://github.com/MattVAllen/ditinha/issues/22. *Confiança:* media.
- SetForegroundWindow é permitido quando o processo que chama recebeu o último evento de entrada; isso cobre focar o terminal depois de um clique no Zeca. *Evidência:* https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setforegroundwindow. *Confiança:* alta.
- SetWinEventHook com WINEVENT_OUTOFCONTEXT dispensa DLL e exige laço de mensagens na thread. EVENT_SYSTEM_FOREGROUND avisa toda troca da janela em primeiro plano. *Evidência:* https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwineventhook ; https://learn.microsoft.com/en-us/windows/win32/winauto/event-constants. *Confiança:* alta.
- Com Per-Monitor V2, o app vê os pixels crus de cada monitor e nunca é esticado como bitmap; a mudança de DPI chega por WM_DPICHANGED, com um retângulo sugerido. *Evidência:* https://learn.microsoft.com/en-us/windows/win32/hidpi/high-dpi-desktop-application-development-on-windows. *Confiança:* alta.
- timeBeginPeriod com resolução alta pode impedir o modo de economia da CPU. No Win11, processos com janelas ocluídas não têm resolução maior garantida. *Evidência:* https://learn.microsoft.com/en-us/windows/win32/api/timeapi/nf-timeapi-timebeginperiod. *Confiança:* alta.
- macOS: um app Accessory “may be activated … by clicking on one of its windows”. O estilo nonactivatingPanel não ativa o app dono. hidesOnDeactivate vale false por padrão no NSWindow e true no NSPanel. *Evidência:* developer.apple.com/documentation/appkit: nsapplication/activationpolicy-swift.enum/accessory ; nswindow/stylemask-swift.struct/nonactivatingpanel ; nswindow/hidesondeactivate. *Confiança:* alta.
- macOS: monitores globais de eventos (addGlobalMonitorForEvents) só pedem Acessibilidade para eventos de teclado, e não recebem eventos enviados ao próprio app. NSEvent.mouseLocation devolve a posição atual em coordenadas de tela. No objc2-app-kit, addGlobalMonitorForEventsMatchingMask_handler e mouseLocation são safe. *Evidência:* developer.apple.com/documentation/appkit/nsevent/addglobalmonitorforevents(matching:handler:) ; https://docs.rs/objc2-app-kit/latest/objc2_app_kit/struct.NSEvent.html. *Confiança:* alta.
- NSScreen.main é a tela da janela que está recebendo teclado. O Hammerspoon usa [NSScreen mainScreen] como “a tela da janela em foco”, ou seja, com semântica global. *Evidência:* developer.apple.com/documentation/appkit/nsscreen/main ; github.com/Hammerspoon/hammerspoon extensions/screen/libscreen.m L931-945. *Confiança:* media.
- NSWindow.backingScaleFactor vale 2.0 em modos HiDPI e 1.0 nos demais, e “does not represent anything concrete, such as pixel density”. Em modos escalados há reamostragem. *Evidência:* developer.apple.com/documentation/appkit/nswindow/backingscalefactor. *Confiança:* alta.
- Para aparecer sobre o Space de tela cheia de outro app, uma NSWindow comum não basta. Funciona um NSPanel não ativador com FullScreenAuxiliary (e CanJoinAllSpaces), com nível acima do Dock e da barra de menus se for preciso. O BongoCat usa NSPanel Borderless|NonactivatingPanel, NSMainMenuWindowLevel, CanJoinAllSpaces|FullScreenAuxiliary e Accessory. *Evidência:* https://github.com/sennett-lau/typelite/pull/15 ; https://github.com/aviaddantz/claude-buddy/pull/3 ; https://github.com/jamiepine/voicebox/pull/1157 ; github.com/ayangweb/BongoCat crates/bongocat-overlay/src/macos/renderer.rs L102-124, macos/geometry.rs, macos/session.rs L74. *Confiança:* media.
- macOS decide o clique pelo alfa de cada pixel numa NSWindow AppKit pura não opaca com fundo clearColor (medido com windowNumberAtPoint; o LICEcap usa isso desde 2011). Com GTK4 no macOS não funciona, então depende do jeito de desenhar. *Evidência:* https://github.com/viniciusdc/glimpse/issues/1. *Confiança:* media.
- SDL3 SDL_SetWindowShape: no Windows usa SetWindowRgn (recorta o desenho); no X11, XShape com ShapeInput; no macOS, alterna ignoresMouseEvents nos eventos mouseMoved. Não há implementação para Wayland. *Evidência:* github.com/libsdl-org/SDL src/video/windows/SDL_windowsshape.c L118; src/video/x11/SDL_x11shape.c L84,102; src/video/cocoa/SDL_cocoawindow.m updateIgnoreMouseState; https://wiki.libsdl.org/SDL3/SDL_SetWindowShape. *Confiança:* alta.
- Godot 4: window_set_mouse_passthrough (polígono) existe em X11, macOS e Windows; no Windows, a parte fora da região não é desenhada. WINDOW_FLAG_MOUSE_PASSTHROUGH e NO_FOCUS não existem no Wayland. *Evidência:* github.com/godotengine/godot doc/classes/DisplayServer.xml. *Confiança:* alta.
- tao: set_ignore_cursor_events vale para a janela inteira; visible_on_all_workspaces não funciona no Windows; always_on_top não funciona no Wayland; no Linux depende de GTK3. egui ViewportBuilder::with_mouse_passthrough também vale só para a janela inteira. *Evidência:* https://docs.rs/tao/latest/tao/window/struct.Window.html ; https://docs.rs/egui/latest/egui/viewport/struct.ViewportBuilder.html. *Confiança:* alta.
- wlr-layer-shell: Mutter (GNOME) e Weston não implementam; KWin 6.7, Hyprland, Sway, niri, COSMIC, Wayfire e labwc implementam. *Evidência:* https://wayland.app/protocols/wlr-layer-shell-unstable-v1. *Confiança:* alta.
- Pelo XWayland, a posição global do cursor vista por apps X11 fica desatualizada quando o ponteiro está sobre janelas Wayland (KDE Plasma 6.6.6). No Wayland nativo não existe posição global do cursor, então ligar/desligar o clique por polling é impossível. *Evidência:* https://github.com/OpenWhispr/openwhispr/issues/1507 ; https://github.com/electron/electron/issues/54635. *Confiança:* media.
- BongoCat (Rust, Apache-2.0, cerca de 23,7 mil estrelas) usa overlays nativos: Win32 + D3D11 + DirectComposition (WS_POPUP, WS_EX_TOOLWINDOW|NOACTIVATE|NOREDIRECTIONBITMAP; para atravessar o clique, TRANSPARENT|LAYERED) e NSPanel + Metal. O Linux foi adiado para depois do lançamento (ADR-0006). *Evidência:* github.com/ayangweb/BongoCat docs/adr/0003-native-overlay-renderers.md, 0006-linux-post-launch-strategy.md, crates/bongocat-overlay/src/windows/window.rs. *Confiança:* alta.
- eSheep usa uma janela pequena por ovelha, que se move: WinForms com TransparencyKey magenta e ExStyle WS_EX_TOOLWINDOW|WS_EX_TOPMOST|WS_EX_LAYERED|WS_EX_NOACTIVATE, reafirmando TopMost a cada nova animação. *Evidência:* github.com/Adrianotiger/desktopPet src/dotNet/FormPet.cs L169-183, L444; FormPet.Designer.cs L71-84. *Confiança:* alta.
- Shimeji-Desktop (port do Shimeji-ee para Java 25) usa uma JWindow por mascote, com alfa por pixel e contains() pelo alfa; no X11 marca a janela como _NET_WM_WINDOW_TYPE_DOCK. Shijima-Qt (Qt6) foi arquivado: o autor diz que o Qt foi a escolha errada; no macOS ele pede Acessibilidade e no KDE/GNOME usa plugins de shell. *Evidência:* github.com/DalekCraft2/Shimeji-Desktop .../platform/win/WindowsTranslucentWindow.java, x11/X11TranslucentWindow.java ; github.com/pixelomer/Shijima-Qt README. *Confiança:* alta.
- Os pets que reagem ao Claude Code (Clyde, CoPet, sidecrab, tokibean, agent-pet) usam Tauri 2 com macos-private-api, alguns com tauri-nspanel, objc2 ou o crate windows. Desktop Goose é fechado (Windows/macOS, sem Linux); VPet é WPF, só Windows, com o copyright da arte separado do código. *Evidência:* src-tauri/Cargo.toml de QingJ01/Clyde, ChanceYu/CoPet, zvoque/sidecrab, ZGhey/tokibean, xiangking/agent-pet ; https://samperson.itch.io/desktop-goose ; github.com/LorisYounger/VPet README. *Confiança:* alta.
- x11rb sem a feature allow-unsafe-code usa uma conexão em Rust puro, sem libxcb; as extensões (shape, shm, randr) ficam atrás de features. *Evidência:* github.com/psychon/x11rb README. *Confiança:* alta.
- No projeto, o workspace proíbe unsafe (unsafe_code = forbid). Pela decisão 0005, cada commit da camada repinta o monitor no Hyprland, e o plano B era uma superfície pequena com palco grande só para voo e confete. *Evidência:* ~/Documents/claude-pet-m2/Cargo.toml ; ~/Documents/claude-pet-m2/DECISIONS.md (decisões 0004, 0005, 0016). *Confiança:* alta.

### Recomendações

1. Não adotar winit + softbuffer como camada única. O softbuffer não tem alfa em Win32/X11, nem na versão lançada nem no master; o winit no Windows reescreve GWL_EXSTYLE, no macOS 0.30 rouba o foco e no Linux carrega bibliotecas por dlopen, sem layer-shell.
2. Criar no claude-pet um contrato mínimo de plataforma: criar overlay, apresentar BGRA pré-multiplicado com retângulos sujos, mover/redimensionar, definir região de entrada (retângulos derivados do alfa do sprite), listar monitores, avisar o monitor ativo, eventos de ponteiro e acordar o laço. Implementar win/, mac/ e x11/ ao lado do wl/ atual; pet-core continua sem nenhuma dependência de plataforma.
3. Windows (windows-sys 0.61): CreateWindowExW com WS_EX_LAYERED|WS_EX_TOOLWINDOW|WS_EX_TOPMOST|WS_EX_NOACTIVATE e WS_POPUP; DIB de 32 bpp pré-multiplicado + UpdateLayeredWindowIndirect (ULW_ALPHA, AC_SRC_ALPHA, prcDirty) dá o clique por pixel de graça. Completar com WM_MOUSEACTIVATE→MA_NOACTIVATE, arraste manual (SetCapture/GetCursorPos), SetWindowPos(HWND_TOPMOST, SWP_NOACTIVATE) reafirmado no EVENT_SYSTEM_FOREGROUND, SetWinEventHook + MonitorFromWindow para o monitor ativo, Per-Monitor V2 + WM_DPICHANGED, e SetForegroundWindow no clique para focar o terminal.
4. macOS (objc2-app-kit 0.3.2): ActivationPolicy Accessory; NSPanel Borderless|NonactivatingPanel com setFloatingPanel(true), setBecomesKeyOnlyIfNeeded(true) e setHidesOnDeactivate(false); collectionBehavior CanJoinAllSpaces|FullScreenAuxiliary|Stationary|IgnoresCycle definido antes do primeiro orderFrontRegardless; nível NSFloatingWindowLevel, ou NSStatusWindowLevel para ficar acima do Dock; opaco false, fundo clearColor, sem sombra; CALayer.contents com CGImage pré-multiplicado, contentsScale = backingScaleFactor e filtro nearest.
5. macOS, clique só no sprite: fazer um spike antes. Se o WindowServer decidir o clique pelo alfa com o CALayer, ótimo; senão, deixar ignoresMouseEvents=true e alterná-lo pela posição do cursor (addGlobalMonitorForEventsMatchingMask_handler com MouseMoved|LeftMouseDragged, ou mouseLocation a 20–30 Hz só perto do bicho), com NSTrackingArea activeAlways e acceptsFirstMouse quando a janela estiver clicável.
6. X11/XWayland (x11rb 0.14, features shape e randr): visual ARGB de 32 bits; put_image pré-multiplicado; entrada só no sprite com shape_rectangles(SO::SET, SK::INPUT, …); sem compositor, usar SK::BOUNDING com a máscara binária. Dicas: _NET_WM_WINDOW_TYPE_DOCK ou NOTIFICATION, _NET_WM_STATE_ABOVE|STICKY|SKIP_TASKBAR|SKIP_PAGER, _NET_WM_DESKTOP=0xFFFFFFFF, WM_HINTS input=false. Monitor ativo por _NET_ACTIVE_WINDOW + randr::get_monitors. Nunca depender de polling do cursor (falha no XWayland).
7. Wayland: manter o backend SCTK/layer-shell e abstrair a fonte do monitor em foco por compositor (eventos do Hyprland já existem; IPC do Sway/niri, script do KWin, wlr-foreign-toplevel). GNOME Wayland: documentar como suporte reduzido (fallback X11/XWayland que não segue o monitor) e só depois pensar numa extensão do GNOME Shell.
8. Em macOS, Windows e X11, usar uma janela pequena (união sprite + balão + margem) que se move; para voo e confete, aumentá-la temporariamente ou abrir uma janela de efeitos toda atravessável ao clique durante o efeito. No Hyprland a camada do tamanho do monitor fica, como a decisão 0005 mediu.
9. Nitidez: continuar com D inteiro por monitor em pixels de dispositivo (Windows PMv2: 125/150/175% viram 5/6/7 com base 4; macOS: D em pixels de backing com escala 1 ou 2; X11: pixels físicos). Aceitar a reamostragem do macOS em modos escalados e, para XWayland com escala fracionária, documentar a opção de escala nativa de cada compositor.
10. Custo: 0 quadros parado, timers de 16 ms ou mais, nada de timeBeginPeriod(1), só retângulos sujos (prcDirty, put_image parcial) e janela pequena no macOS (onde cada apresentação copia o buffer inteiro).
11. Isolar o unsafe: deixar pet-core e wl/ como estão (forbid) e pôr Windows e macOS em crates próprios com unsafe permitido e comentários SAFETY. Preferir os métodos safe do objc2-app-kit e a conexão Rust pura do x11rb.
12. Antes de codar, rodar spikes curtos em hardware real: macOS (clique pelo alfa com CALayer, tela cheia de outro app, todos os Spaces, arrastar sem ativar, NSScreen.main seguindo o foco); Windows (todas as áreas virtuais no Win10 e no Win11 24H2/25H2, conferindo GetWindowDesktopId; dois monitores com DPI diferente; foco do terminal); X11 (GNOME Xorg, GNOME Wayland via XWayland, KDE X11, i3/XFCE sem compositor; DOCK vs override-redirect).
13. Plano B se o código nativo pesar: SDL3 (crate sdl3 0.20) cobre clique por forma em Windows, X11 e macOS, mas não no Wayland nem nos Spaces. Evitar Tauri, egui e Godot para o Zeca.
14. Fora do tema da janela, mas bloqueia o download por qualquer pessoa: trocar o plugin/scripts/avisar.sh (sh + jq + curl) por um subcomando do próprio binário, porque jq não vem no Windows; e definir uma skin padrão original para a versão pública, já que o pack pago não pode ser redistribuído e o visual lembra um personagem de terceiros.

### Riscos

- **Risco:** Windows: aparecer em todas as áreas de trabalho virtuais depende de comportamento não documentado (tool window ou DeleteTab deixam a janela fora do rastreio), que pode mudar entre builds; existe até um relato contrário. **Mitigação:** Conferir na inicialização e a cada troca de foco com GetWindowDesktopId e IsWindowOnCurrentVirtualDesktop; se a janela estiver presa a uma área, usar MoveWindowToDesktop(nossa, GetWindowDesktopId(janela_em_foco)), que é API pública. Testar no Win10 e no Win11 24H2/25H2. Não usar as interfaces internas de fixar.
- **Risco:** macOS: o WindowServer pode não decidir o clique pelo alfa quando o conteúdo vem de um CALayer (falhou com GTK4); aí a área transparente da janela engoliria cliques. **Mitigação:** Janela pequena limita o estrago. Plano B já desenhado: ignoresMouseEvents alternado por monitor global de mouse (sem permissão) ou por mouseLocation perto do bicho. Validar em spike antes da implementação.
- **Risco:** macOS: ficar sobre apps em tela cheia e em todos os Spaces depende de NSPanel não ativador + FullScreenAuxiliary, comportamento que vem de relatos de campo e pode mudar no macOS 26/27. **Mitigação:** Seguir exatamente a receita usada pelo BongoCat e por outros apps em 2026; definir collectionBehavior antes de mostrar; testar a cada versão do macOS; aceitar degradação (o Zeca some só no Space de tela cheia).
- **Risco:** GNOME Wayland não tem layer-shell; pelo XWayland não há cursor global nem foco das janelas Wayland nativas, e a escala fracionária pode borrar. **Mitigação:** Documentar como suporte reduzido: fallback X11 com _NET_WM_STATE_ABOVE + SHAPE de entrada, posição fixa ou arrastada pelo usuário. Extensão opcional do GNOME Shell só se houver demanda. Testar no GNOME 49/50.
- **Risco:** Três backends novos sem máquinas macOS/Windows próprias: regressões silenciosas de comportamento (foco, nível, DPI). **Mitigação:** CI com build e testes do núcleo em runners macOS e Windows; testes de unidade para a geometria e a região de entrada; checklist manual por release em hardware real (ou com voluntários); manter o contrato de plataforma pequeno.
- **Risco:** O workspace proíbe unsafe, mas as APIs Win32 (e parte do CoreGraphics) exigem unsafe. **Mitigação:** Crates de plataforma separados com unsafe permitido e localizado, comentários SAFETY e deny(unsafe_op_in_unsafe_fn); pet-core e x11/ continuam safe (x11rb com conexão Rust pura).
- **Risco:** Windows: disputa por "sempre por cima" com outras janelas topmost (a barra de tarefas, outros overlays); não aparece sobre jogos em tela cheia exclusiva. **Mitigação:** Reafirmar HWND_TOPMOST com SWP_NOACTIVATE a cada EVENT_SYSTEM_FOREGROUND, como o eSheep faz a cada animação; aceitar a tela cheia exclusiva como limitação documentada.
- **Risco:** Custo de bateria em macOS e Windows se o design de tela inteira do Hyprland for copiado: copiar quadros de 33–59 MB por apresentação (4K/5K). **Mitigação:** Janela pequena que se move; palco grande só durante voo e confete; prcDirty e put_image parcial; 0 quadros parado; timers de 16 ms ou mais.
- **Risco:** Focar a janela do terminal no clique: no macOS, ativar outro app é cooperativo desde o 14 (não garantido) e escolher a janela exige Acessibilidade; no Windows, o Windows Terminal tem várias abas por janela. **Mitigação:** Tratar como esforço máximo: ativar o app pelo PID; levantar a janela só se o usuário der a permissão de Acessibilidade; no Windows, SetForegroundWindow logo no clique. Validar em spike.
- **Risco:** Se o caminho winit for escolhido mesmo assim: a 0.31 ainda é beta (API mudando) e o Windows continua reescrevendo os estilos estendidos. **Mitigação:** Usar objc2 direto no macOS; se usar winit, fixar a versão exata e só no macOS, reaplicando estilos e collectionBehavior após cada setter.
- **Risco:** Fora do tema da janela: o pack de arte pago não pode ser redistribuído, o visual lembra um personagem de terceiros, e binários sem assinatura sofrem com Gatekeeper/SmartScreen. **Mitigação:** Skin padrão original para a versão pública, com o pack como opção BYO (já é o fluxo skin-instalar). Revisar nome e visual. Planejar notarização no macOS e assinatura no Windows em pesquisa própria.

## Achado 2 — Linux além do Hyprland (em inglês, como veio)

*Zeca on Linux beyond Hyprland: compositors, focus, packaging (research, 2026-10-03)*

Read-only research. Nothing on the host was modified. The WebSearch budget for this session was already used up, so every claim below comes from fetching known primary URLs directly: wayland.app compatibility tables (wayland-explorer snapshot: Hyprland 0.52.1, KWin 6.7, Sway 1.11, niri 26.04, COSMIC 1.0 beta, Mutter 51), compositor sources on their main branches (sway, niri, cosmic-comp, labwc, KWin, Hyprland, Mutter, gnome-shell, river), official docs (sway-ipc(7), sway(5), niri-ipc 26.4.0, KWin scripting API, EWMH, systemd.special(7), GNOME release notes and Mutter NEWS, EGO review guidelines, Docker docs, Flatpak NEWS, cargo-dist), plus a read-only look at the repo. Anything I could not check against a source is marked as not verified.

---
### 0. TL;DR
1. **Outside GNOME, layer-shell is the de-facto standard.** zwlr_layer_shell_v1 is implemented by:
   * Hyprland v5, KWin/Plasma 6 v5, COSMIC v5, niri v5;
   * sway v4, labwc v4, Wayfire v4, river 0.3 v4;
   * Mir, Treeland, Muffin (Cinnamon's Wayland session) and gamescope at v4, phoc at v3;
   * Jay, Louvre and Cage.
   **It is missing on Mutter/GNOME (still none in GNOME 51) and on Weston, and therefore on WSLg.**
2. **The current `wl/` code is already almost generic.** It uses output NULL, maps a 1x1 transparent buffer until `wl_surface.enter` + `preferred_scale`, and uses fractional-scale + viewporter. The real Hyprland coupling is **discovery**: `descoberta.rs` only connects to Wayland after it finds a live `hypr/<HIS>/hyprland.lock` + `.socket2.sock`. On any other desktop the daemon waits forever in "sem compositor".
3. **Following the focused monitor and click-to-focus need a different backend for each desktop.** There is no cross-desktop standard:
   * IPC sockets: Hyprland, sway/i3, niri, Wayfire.
   * D-Bus + KWin scripting: KDE.
   * Private Wayland protocols: COSMIC (`cosmic-toplevel-*`) and KDE (`org_kde_plasma_window_management`).
   * wlr-foreign-toplevel: the long tail of wlroots compositors.
   * A Shell extension: GNOME.
   * EWMH: X11.
4. **Universal fallback with no IPC ("summon"):** on each attention event, re-create the overlay with output NULL. The spec says the compositor picks the output that, "generally", the user "most recently interacted with", and `wl_surface.enter` then says which one it picked.
5. **Ship a native binary, not Docker or Flatpak.**
   * Hyprland, sway, niri and COSMIC all hide layer-shell from security-context clients, which is what Flatpak ≥ 1.15.6 creates.
   * Docker Desktop for Linux runs a VM, so containers cannot reach the host's Wayland socket.
   * A static musl binary stays possible for every backend: wayland-client, x11rb and zbus are all pure Rust.
6. **Recommended v1.0 for Linux: every desktop that has layer-shell.**
   * Tier 1: Hyprland, KDE Plasma 6, sway, niri, COSMIC.
   * Tier 2: the generic wlroots family.
   * **GNOME needs an extension and comes in v1.1.** It is the biggest desktop, and GNOME 50 is Wayland-only.
   * **The X11 backend comes in v1.1/v1.2.** It serves Mint Cinnamon, XFCE, MATE and i3; Plasma's X11 session ends in early 2027.

---
### 1. Compatibility matrix
| Desktop | layer-shell | OVERLAY vs fullscreen | output NULL goes to | Focused monitor from | Focusing a window | Window PID? | Layer-shell for a Flatpak client? |
|---|---|---|---|---|---|---|---|
| Hyprland | v5 | above (03-hyprland.md) | focused monitor | socket2 `focusedmonv2` | `.socket.sock` `dispatch focuswindow` | yes (`clients`) | no (privileged) |
| KDE Plasma 6 (KWin) | v5 | above: `OverlayLayer` is KWin's top layer, above `ActiveLayer` (fullscreen) | `workspace()->activeOutput()` | D-Bus `/KWin org.kde.KWin.activeOutputName`, or a KWin script (`windowActivated`, `cursorPosChanged`, `activeScreen`) that calls back with `callDBus` | script `workspace.activeWindow = w`, or `org_kde_plasma_window.set_state(active)` | yes (`Window.pid`; `pid_changed` v8+ in the Plasma protocol) | yes (not on KWin's restricted list) |
| sway | v4 | above (scene tree: `shell_overlay` comes after `fullscreen`) | output of the focused workspace | IPC `$SWAYSOCK`: `GET_WORKSPACES` + subscribe to `workspace` (change `focus`; the workspace carries `output`) | `RUN_COMMAND` `[pid=N] focus` / `[con_id=N] focus` | yes (`GET_TREE`) | no |
| niri | v5 | above (docs: a focused fullscreen window covers floating windows and the top layer, not overlay) | `layout.active_output()` | IPC `$NIRI_SOCKET` (JSON lines): `EventStream` (`WorkspacesChanged` with `output`/`is_focused`; `WorkspaceActivated{id,focused}`), `FocusedOutput` | `Action::FocusWindow{id}` | yes (`Window.pid`) | no |
| COSMIC | v5 | not verified | `seat.active_output()` | no IPC: `zcosmic_toplevel_info_v1` (`activated` + `output_enter`) or summon | `zcosmic_toplevel_manager_v1.activate` | no | no (only the CosmicPanel engine) |
| labwc | v4 | not verified | output nearest the cursor | no IPC: wlr-foreign-toplevel or summon | wlr-foreign-toplevel `activate` | no | not verified |
| Wayfire | v4 | not verified | not verified | IPC (needs the `ipc` + `ipc-rules` plugins): `window-rules/get-focused-output`, event `output-gain-focus`; otherwise wlr-foreign-toplevel | `window-rules/focus-view` | yes (`list-views`) | n/a (no security-context) |
| river | v4 (0.3.x) | not verified | not verified | 0.3: wlr-foreign-toplevel. On main, policy moved to a separate WM (`river-window-management-v1`) and `river-status` is gone | wlr-foreign-toplevel (0.3) | no | not verified |
| Mir, Treeland, Jay, Louvre, phoc, Cage | v3–v5 | — | — | generic | wlr-foreign-toplevel | no | — |
| Muffin (Cinnamon Wayland), gamescope | v4 | — | — | summon only | — (no wlr-foreign-toplevel) | — | — |
| GNOME (Mutter ≤ 51) | **none** | — | — | only through an extension (`global.display.get_current_monitor()` = monitor under the pointer) | extension `Main.activateWindow()` or the third-party 'Window Calls' extension | through the extension (`get_pid`) | — |
| Weston / WSLg | none | — | — | — | — | — | — |
| X11 (EWMH WM) | n/a | — | — | `_NET_ACTIVE_WINDOW` (PropertyNotify on root) + RandR | `_NET_ACTIVE_WINDOW` ClientMessage, source=2 | `_NET_WM_PID` | — |

**Supporting protocols (wayland.app):**
* **wlr-foreign-toplevel-management** gives `activated`, `output_enter` and `activate`, **without PID**.
  * Supported: Hyprland, sway, niri, labwc, Wayfire, river 0.3, Mir, phoc, Treeland, Jay, Louvre, Cage.
  * Not supported: KWin, COSMIC, Mutter, Muffin, gamescope, Weston.
* **ext-foreign-toplevel-list** only exposes title, app_id and identifier (no state, output or PID), so it is not enough on its own.
* **ext-workspace-v1** has active workspaces per output but no notion of focus, so it is useless here.
* **There is no standard 'ext' protocol for activating toplevels.**
* **fractional-scale-v1** is supported everywhere except Muffin, Cage, gamescope and Weston, so keep the integer-scale fallback.
* **cursor-shape-v1** is missing on Wayfire and Mir, so a cursor fallback is needed.

---
### 2. Following the focused monitor
* **Event sources** exist on Hyprland, sway, niri, Wayfire (with the plugins) and KDE (D-Bus/script); see the table.
* **What "focused monitor" means differs:**
  * Hyprland follows the mouse (with its defaults).
  * sway follows the focused workspace.
  * GNOME's `get_current_monitor` is the monitor under the pointer.
  * On KWin it depends on the "active screen follows mouse" setting (not verified).
* **Fallback A, "summon" (works on every layer-shell compositor).** On Stop, permission or question events, destroy and re-create the layer surface with output NULL, then read `wl_surface.enter`.
  * Verified NULL-output policies: Hyprland → focused monitor; sway → output of the focused workspace; niri → active output; KWin → activeOutput; COSMIC → seat.active_output(); labwc → output nearest the cursor.
  * `superficie.rs` already maps with a 1x1 transparent buffer and waits for `enter` + `preferred_scale`, so this costs little.
* **Fallback B, continuous.** Follow the output of the toplevel that is `activated`: wlr-foreign-toplevel on wlroots compositors, cosmic-toplevel-info on COSMIC. It misses focus moving to an empty workspace.
* **Not usable:**
  * ext-workspace and ext-foreign-toplevel-list (no focus information).
  * Pointer heuristics: a click-through overlay never receives `wl_pointer.enter` outside the sprite.

---
### 3. Clicking Zeca focuses the session's terminal
**Capture the window when the prompt is submitted (`UserPromptSubmit`; works everywhere).** At that moment the active toplevel is almost always the terminal of that session. Each backend can take a snapshot:

| Backend | Where the active window comes from |
|---|---|
| Hyprland | `activewindowv2` |
| sway | focused container |
| niri | `WindowFocusChanged` |
| KWin | `windowActivated` |
| wlroots / COSMIC | `activated` state |
| X11 | `_NET_ACTIVE_WINDOW` |
| GNOME | `focus-window` |

* Store an opaque handle per `session_id`, with no title, which respects the "metadata only" rule.
* It covers desktops without PIDs (COSMIC, labwc, river, Mir) and the cases where the PID chain breaks.

**Walk the PID chain in the daemon, not in the hook.** Natively, the daemon can read `/proc/<pid>/stat`. The hook then only needs to send:
* Claude's PID (`$PPID`);
* hints from the environment: `TMUX_PANE`, `KITTY_WINDOW_ID`, `WEZTERM_PANE`, `TERM_PROGRAM`.

**Where the PID chain fails** (general knowledge about terminals, not verified in this session):
* Multiplexers (tmux, zellij, screen): the chain ends at the multiplexer's server, not at the terminal.
* Terminals that run one process for many windows: gnome-terminal-server, Ghostty, WezTerm, kitty `--single-instance`, foot in server mode.
* Terminals inside an IDE.
* SSH sessions and containers, which use a different PID namespace.

**Focus stealing.** Commands executed by the compositor itself bypass focus-stealing prevention: sway, niri and Hyprland commands, and KWin scripts. On X11, EWMH defines source 2 as a pager, i.e. a direct user action.

---
### 4. GNOME
* **No layer-shell through Mutter 51.** The upstream issue (mutter#973) never landed.
* **GNOME is Wayland-only from 50.** GNOME 49 "disabled by default" the X11 session, and Mutter 50.alpha says "Drop the X11 backend".
* **Plasma 6.8 will also be Wayland-only**, with X11 supported into early 2027.

**Options, in order of preference:**
1. **Thin Shell extension acting as a "remote overlay" (recommended).**
   * **Placement:** `Main.layoutManager.addTopChrome(actor, {affectsInputRegion: true, trackFullscreen: false})` puts the actor "above all windows, including popups". Input is captured only over the actor; clicks elsewhere go through.
   * **Crispness:** `set_content_scaling_filters(NEAREST, NEAREST)` keeps the pixels sharp if the daemon sends frames at **art resolution**.
   * **D-Bus API:** SetFrame/Move/Hide, FocusPid/FocusWindowId, ActiveWindow, CurrentMonitor, plus Clicked and Dragged signals.
   * **Focusing:** `Main.activateWindow(win)` switches workspace and closes the overview; PIDs come from `Meta.Window.get_pid()`.
   * **Daemon side:** the Rust daemon keeps the brain, the animator and the HTTP ingress, and talks to the extension through zbus.
   * **Costs:**
     * EGO does not accept binaries, so the daemon ships separately.
     * `shell-version` must be updated every 6 months.
     * Shijima-Qt reports that GNOME needed a logout/login on first install.
2. **XWayland fallback ("experimental", degraded).**
   * An override-redirect ARGB window with an XShape input region. Mutter puts override-redirect windows in `META_LAYER_OVERRIDE_REDIRECT`.
   * It cannot follow focus: XWayland only sees the pointer over X11 surfaces.
   * It cannot focus Wayland-native terminals.
   * It is blurry at fractional scale unless Xwayland native scaling is on.
   * Click-through and drawing over fullscreen are not verified and need testing.
3. **xdg_toplevel: not viable.** A plain window cannot keep itself on top and cannot choose its position.
4. **Focus only:** the 'Window Calls' extension (D-Bus `List` with pid, `Activate`).

**Prior art:** Shijima-Qt only supported KDE Plasma 6 and GNOME 46 (Wayland/X11). It did so by auto-installing a KWin script or a GNOME extension, and the project is now archived.

---
### 5. X11 sessions
**Still in use on:** Mint Cinnamon (X11 by default; this is medium confidence), XFCE, MATE, i3/bspwm/Openbox, and KDE X11 until early 2027.

**Design** (x11rb 0.14 with the pure-Rust `RustConnection`; features `shape`, `xfixes`, `randr`):
* **Window type**, two choices:
  * A managed window: `_NET_WM_WINDOW_TYPE_DOCK` or `UTILITY`, states `_NET_WM_STATE_ABOVE`, `STICKY`, `SKIP_TASKBAR`, `SKIP_PAGER`, no decorations, and `WM_HINTS input=False`.
  * An override-redirect window that raises itself again when covered.
* **Transparency:** use an ARGB visual only if a compositing manager owns `_NET_WM_CM_S<n>`. Otherwise use an XShape bounding mask; pixel art has binary alpha, so this works.
* **Input shape:** the sprite.
* **Moving between monitors** is free, since the root is one coordinate space.
* **No per-monitor scale on X11:** derive D from RandR or the config.
* **Focusing:** `_NET_ACTIVE_WINDOW` with source 2 and a timestamp; map PID to window through `_NET_CLIENT_LIST` + `_NET_WM_PID`.
* **Focused monitor:** the geometry of the `_NET_ACTIVE_WINDOW`, or the pointer, mapped to a RandR monitor.
* **i3:** it uses the sway IPC protocol (`I3SOCK`), so the sway client can be reused.

---
### 6. Packaging
**Distributions to produce:**
* **Main artifact:** static musl binaries (x86_64 and aarch64).
  * They stay possible because wayland-client, wayland-protocols-wlr/-plasma 0.3.12, cosmic-protocols 0.2.0, x11rb and zbus 5.19 are all pure Rust.
  * Package with them: the `.desktop` for autostart, the systemd user unit, LICENSE/NOTICE, and an **original** default skin.
* **cargo-dist 0.33.0** (2026-09-11, active):
  * Produces tarballs + a `curl | sh` installer + Homebrew (which also works on Linux), plus PowerShell/MSI/npm for the other OSes.
  * It does not produce deb/rpm/AUR (the book lists them as "requested").
  * For those, add cargo-deb + cargo-generate-rpm (or nFPM), an AUR `claude-pet-bin` + `claude-pet`, and a Nix flake.

**Avoid:**
* **Flatpak:**
  * It runs apps with a Wayland security context. Hyprland (`isGlobalPrivileged`), sway (`is_privileged`), niri (`client_is_unrestricted`) and COSMIC (`client_not_sandboxed`) hide layer-shell and foreign-toplevel from such clients; only KWin does not.
  * It would also need holes for the IPC sockets, for D-Bus to KWin and for the autostart portal.
* **Snap:** not researched; similar confinement is likely.
* **AppImage:** brings nothing to a static binary and adds FUSE friction.
* **Docker for end users:** Docker Desktop for Linux is a VM, and many mounts would be needed. Keep Docker only for development.

**The Claude Code plugin:**
* It comes from the plugin marketplace, separate from the OS packages.
* `avisar.sh` depends on jq (and degrades without it), curl, GNU `date +%s%3N`, `sha256sum`/`shasum`, and Omarchy's "do not disturb" file.
* A `claude-pet hook <Evento>` subcommand in the same binary would remove those dependencies, and would also serve macOS and Windows.

---
### 7. Autostart
* **Recommended default: XDG autostart.** Write `~/.config/autostart/claude-pet.desktop`.
  * GNOME, KDE, COSMIC (cosmic-session reads the folders or defers to systemd), XFCE, LXQt, MATE and Cinnamon run it themselves.
  * `systemd-xdg-autostart-generator` runs it on sessions that start `xdg-desktop-autostart.target`: niri.service `Wants=` it, and uwsm binds it.
* **Alternative: systemd user unit** with `PartOf=`, `After=` and `WantedBy=graphical-session.target`, plus `Restart=on-failure`.
  * Works on GNOME, KDE, COSMIC, niri-session and uwsm.
  * Does **not** start on plain sway (the default config has no systemd integration), on Hyprland without uwsm, or on labwc ("does not activate the target itself").
* **Snippets to print for the other cases:**

| Desktop | Where to add the start line |
|---|---|
| Hyprland | `exec-once`, or the Lua equivalent in 0.56 |
| sway | `exec claude-pet` |
| labwc | `~/.config/labwc/autostart` |
| Wayfire | `[autostart]` |
| river | init script |
| niri | `spawn-at-startup` |

* **Pick one default.** Binding the ingress already makes a second instance exit, but systemd plus XDG autostart together would race.
* **Environment:** keep scanning `$XDG_RUNTIME_DIR` as a fallback for when `WAYLAND_DISPLAY` was not imported.

---
### 8. Recommended code structure (names are illustrative)
```
crates/pet-core/        pure, unchanged; produces frames at ART resolution + position + damage in art px
crates/claude-pet/src/
  session.rs            detection: HYPRLAND_INSTANCE_SIGNATURE, NIRI_SOCKET, SWAYSOCK/I3SOCK, WAYFIRE_SOCKET,
                        XDG_CURRENT_DESKTOP (KDE/COSMIC/GNOME), WAYLAND_DISPLAY, DISPLAY; XDG_RUNTIME_DIR scan
                        (keep hyprland.lock for Docker)
  overlay/              trait OverlayBackend: outputs(), show(frame, pos), hide(), rehome(None = NULL)
    wayland/            today's wl/ (layer-shell); the FALLBACK name check moves into the Hyprland adapter
    x11/                x11rb
    gnome/              zbus client of the extension (remote overlay)
  desktop/              trait DesktopAdapter + Capabilities {FOLLOWS_FOCUS, FOCUS_WINDOW, PID, ACTIVE_WINDOW}
    hyprland.rs sway.rs(+i3) niri.rs kwin.rs wayfire.rs cosmic.rs wlr_toplevel.rs ewmh.rs gnome.rs none.rs
extensions/gnome-shell/ GJS (ESM), released on EGO separately
```
* **Upscaling** to device pixels (D×D blocks) becomes a backend step. On GNOME the compositor does it with NEAREST, and the same applies to macOS and Windows.
* **Adapter events** flow into the calloop channel that already exists.
* **Degradation:** when FOCUS_WINDOW is missing, the click answers with a reaction and a "não consigo focar aqui" bubble.
* **`claude-pet doctor`** prints the detected backend and its capabilities.
* **Security:**
  * Natively, the daemon gains control over the compositor (sway/niri IPC, KWin scripts, Hyprland dispatch).
  * Close the ingress before adding that: a Unix socket in `$XDG_RUNTIME_DIR` or a token, and an allowlist of actions. Never forward arbitrary commands.

---
### 9. v1.0 recommendation (Linux)
* **Tier 1 (tested on every release, full features):** Hyprland, KDE Plasma 6 Wayland, sway, niri, COSMIC.
* **Tier 2 (should work, tested by the community):** labwc, Wayfire, river, Mir/Miriway, Treeland, Cinnamon-Wayland. These use generic layer-shell + summon + wlr-foreign-toplevel where it exists.
* **Outside v1.0:**
  * GNOME: detect it and explain, with an optional experimental XWayland flag.
  * X11.
  * Weston/WSLg.
* **Order after v1.0:** v1.1 adds the GNOME extension; v1.2 adds the X11 backend, which also enables the GNOME XWayland fallback.
* **If the audience is mostly Ubuntu/Fedora, bring GNOME forward into v1.0.** No market-share data was collected; this is an estimate.
* **Suggested implementation order:** discovery decoupled from Hyprland + summon → sway/i3 → niri → KWin → wlr-foreign-toplevel → COSMIC → Wayfire IPC → GNOME → X11.

---
### Sources
**Protocol support (wayland.app):**
* https://wayland.app/protocols/wlr-layer-shell-unstable-v1
* https://wayland.app/protocols/wlr-foreign-toplevel-management-unstable-v1
* https://wayland.app/protocols/ext-foreign-toplevel-list-v1
* https://wayland.app/protocols/ext-workspace-v1
* https://wayland.app/protocols/security-context-v1
* https://wayland.app/protocols/fractional-scale-v1
* https://wayland.app/protocols/cursor-shape-v1
* https://wayland.app/protocols/cosmic-toplevel-info-unstable-v1
* https://wayland.app/protocols/kde-plasma-window-management

**sway:**
* https://man.archlinux.org/man/sway-ipc.7.en
* https://man.archlinux.org/man/sway.5.en
* https://raw.githubusercontent.com/swaywm/sway/master/sway/server.c
* https://raw.githubusercontent.com/swaywm/sway/master/sway/desktop/layer_shell.c
* https://raw.githubusercontent.com/swaywm/sway/master/sway/tree/root.c
* https://raw.githubusercontent.com/swaywm/sway/master/config.in

**niri:**
* https://github.com/YaLTeR/niri/wiki/IPC
* https://docs.rs/niri-ipc/latest/niri_ipc/ (Event, Window, Workspace, Request, Action)
* https://raw.githubusercontent.com/YaLTeR/niri/main/src/handlers/layer_shell.rs
* https://raw.githubusercontent.com/YaLTeR/niri/main/src/niri.rs
* https://raw.githubusercontent.com/YaLTeR/niri/main/resources/niri.service
* https://niri-wm.github.io/niri/Fullscreen-and-Maximize.html

**COSMIC:**
* https://raw.githubusercontent.com/pop-os/cosmic-comp/master/src/wayland/handlers/layer_shell.rs
* https://raw.githubusercontent.com/pop-os/cosmic-comp/master/src/state.rs
* https://raw.githubusercontent.com/pop-os/cosmic-session/master/src/main.rs

**labwc, Wayfire, river, Hyprland:**
* https://raw.githubusercontent.com/labwc/labwc/master/src/layers.c
* https://raw.githubusercontent.com/labwc/labwc/master/docs/labwc.1.scd
* https://raw.githubusercontent.com/WayfireWM/pywayfire/master/wayfire/ipc.py
* https://codeberg.org/river/river/raw/branch/main/README.md
* https://codeberg.org/river/river/raw/branch/main/build.zig
* https://raw.githubusercontent.com/hyprwm/Hyprland/main/src/managers/ProtocolManager.cpp

**KWin / KDE:**
* https://invent.kde.org/plasma/kwin/-/raw/master/src/layershellv1integration.cpp
* https://invent.kde.org/plasma/kwin/-/raw/master/src/layershellv1window.cpp
* https://invent.kde.org/plasma/kwin/-/raw/master/src/effect/globals.h
* https://invent.kde.org/plasma/kwin/-/raw/master/src/wayland_server.cpp
* https://invent.kde.org/plasma/kwin/-/raw/master/src/org.kde.KWin.xml
* https://invent.kde.org/plasma/kwin/-/raw/master/src/scripting/scripting.cpp
* https://develop.kde.org/docs/plasma/kwin/api/
* https://github.com/jinliu/kdotool
* https://blogs.kde.org/2025/11/26/going-all-in-on-a-wayland-future/

**GNOME:**
* https://gitlab.gnome.org/GNOME/mutter/-/issues/973
* https://gitlab.gnome.org/GNOME/mutter/-/raw/main/NEWS
* https://gitlab.gnome.org/GNOME/mutter/-/raw/main/src/core/window.c
* https://release.gnome.org/49/developers/
* https://gitlab.gnome.org/GNOME/gnome-shell/-/raw/main/js/ui/layout.js
* https://gitlab.gnome.org/GNOME/gnome-shell/-/raw/main/js/ui/main.js
* https://gnome.pages.gitlab.gnome.org/mutter/meta/method.Display.get_current_monitor.html
* https://gnome.pages.gitlab.gnome.org/mutter/clutter/method.Actor.set_content_scaling_filters.html
* https://gjs.guide/extensions/review-guidelines/review-guidelines.html
* https://github.com/ickyicky/window-calls
* https://github.com/pixelomer/Shijima-Qt

**X11 and crates:**
* https://specifications.freedesktop.org/wm/latest/ar01s03.html
* https://specifications.freedesktop.org/wm/latest/ar01s05.html
* https://docs.rs/crate/x11rb/latest/features
* https://docs.rs/zbus/latest/zbus/
* https://crates.io/api/v1/crates/cosmic-protocols
* https://crates.io/api/v1/crates/wayland-protocols-plasma
* https://crates.io/api/v1/crates/wayland-protocols-wlr

**Session startup and packaging:**
* https://man.archlinux.org/man/systemd.special.7.en
* https://man.archlinux.org/man/systemd-xdg-autostart-generator.8.en
* https://raw.githubusercontent.com/Vladimir-csp/uwsm/master/README.md
* https://github.com/axodotdev/cargo-dist/releases
* https://axodotdev.github.io/cargo-dist/book/installers/index.html
* https://raw.githubusercontent.com/flatpak/flatpak/main/NEWS
* https://docs.docker.com/desktop/setup/install/linux/

**Local files:**
* ~/Documents/claude-pet/crates/claude-pet/src/descoberta.rs
* ~/Documents/claude-pet/crates/claude-pet/src/wl/superficie.rs
* ~/Documents/claude-pet/crates/claude-pet/src/wl/saida.rs
* ~/Documents/claude-pet/plugin/scripts/avisar.sh
* ~/Documents/claude-pet-m2/docs/pesquisa/03-hyprland.md

### Fatos conferidos

- zwlr_layer_shell_v1 is implemented by COSMIC (v5), Hyprland (v5), Jay (v5), KWin 6.7 (v5), Louvre (v5), niri (v5), labwc (v4), Mir (v4), Muffin (v4), river 0.3 (v4), sway 1.11 (v4), Treeland (v4), Wayfire (v4), gamescope (v4), phoc (v3) and Cage (limited). Mutter (GNOME) and Weston do not implement it. *Evidência:* https://wayland.app/protocols/wlr-layer-shell-unstable-v1 (compatibility table); Muffin also ships src/wayland/meta-wayland-layer-shell.c (https://github.com/linuxmint/muffin/tree/master/src/wayland). *Confiança:* alta.
- The layer-shell spec lets the client pass output NULL; the compositor then decides, and 'generally this will be the one that the user most recently interacted with'. *Evidência:* https://wayland.app/protocols/wlr-layer-shell-unstable-v1. *Confiança:* alta.
- Output chosen when the client passes NULL. KWin: workspace()->activeOutput(). niri: layout.active_output(). COSMIC: seat.active_output(). sway: output of the seat's focused workspace, else the first output. labwc: output_nearest_to_cursor(). Hyprland: the focused monitor. *Evidência:* invent.kde.org/plasma/kwin/-/raw/master/src/layershellv1integration.cpp; raw.githubusercontent.com/YaLTeR/niri/main/src/handlers/layer_shell.rs; raw.githubusercontent.com/pop-os/cosmic-comp/master/src/wayland/handlers/layer_shell.rs; raw.githubusercontent.com/swaywm/sway/master/sway/desktop/layer_shell.c; raw.githubusercontent.com/labwc/labwc/master/src/layers.c; docs/pesquisa/03-hyprland.md. *Confiança:* alta.
- OVERLAY is drawn above fullscreen windows. KWin maps the overlay layer to OverlayLayer, the last entry of its Layer enum, above ActiveLayer ('active fullscreen'). In sway's scene tree shell_overlay comes after fullscreen and fullscreen_global. niri: a focused fullscreen window covers floating windows and the top layer only. Not verified for COSMIC, labwc or Wayfire. *Evidência:* invent.kde.org/plasma/kwin/-/raw/master/src/layershellv1window.cpp and src/effect/globals.h; raw.githubusercontent.com/swaywm/sway/master/sway/tree/root.c; https://niri-wm.github.io/niri/Fullscreen-and-Maximize.html. *Confiança:* alta.
- Layer-shell is hidden from clients that carry a security context (sandboxed). Hyprland lists PROTO::layerShell, foreignToplevel and foreignToplevelWlr in isGlobalPrivileged. sway includes layer_shell and both foreign-toplevel globals in is_privileged. niri creates WlrLayerShellState with the client_is_unrestricted filter. COSMIC uses client_not_sandboxed, with an exception only for the com.system76.CosmicPanel engine. KWin does not restrict layer-shell: its restricted list has org_kde_plasma_window_management, fake_input, screencast and others. *Evidência:* raw.githubusercontent.com/hyprwm/Hyprland/main/src/managers/ProtocolManager.cpp; raw.githubusercontent.com/swaywm/sway/master/sway/server.c; raw.githubusercontent.com/YaLTeR/niri/main/src/niri.rs; raw.githubusercontent.com/pop-os/cosmic-comp/master/src/state.rs; invent.kde.org/plasma/kwin/-/raw/master/src/wayland_server.cpp. *Confiança:* alta.
- Since 1.15.6 (2023-11), Flatpak creates the app's Wayland socket with the security-context extension, so the compositor can identify sandboxed connections. *Evidência:* https://raw.githubusercontent.com/flatpak/flatpak/main/NEWS. *Confiança:* alta.
- wlr-foreign-toplevel-management exposes output_enter/output_leave, the activated state and activate(seat), but no PID. It is supported by Hyprland, sway, niri, labwc, Wayfire, river 0.3, Mir, phoc, Treeland, Jay, Louvre and Cage. KWin, COSMIC, Mutter, Muffin, gamescope and Weston do not support it. *Evidência:* https://wayland.app/protocols/wlr-foreign-toplevel-management-unstable-v1. *Confiança:* alta.
- ext-foreign-toplevel-list only exposes title, app_id and identifier: no output, no activated state, no PID. KWin, Mutter and Wayfire do not implement it. ext-workspace-v1 (COSMIC, Hyprland, Jay, labwc, niri) cannot tell which output has focus. wayland.app has no standard 'ext' protocol for activating toplevels. *Evidência:* https://wayland.app/protocols/ext-foreign-toplevel-list-v1; https://wayland.app/protocols/ext-workspace-v1; 404 at https://wayland.app/protocols/ext-foreign-toplevel-management-v1. *Confiança:* alta.
- sway IPC. The socket is in $SWAYSOCK, or $I3SOCK for i3 compatibility. Workspace objects carry 'output' and 'focused', and the workspace event with change 'focus' carries the workspace. GET_TREE includes the pid of windows. Criteria accept pid and con_id, so '[pid=N] focus' is a valid command. *Evidência:* https://man.archlinux.org/man/sway-ipc.7.en; https://man.archlinux.org/man/sway.5.en. *Confiança:* alta.
- niri IPC. The socket is $NIRI_SOCKET and requests are single-line JSON. EventStream first sends the full current state. Window has pid, is_focused and workspace_id; Workspace has output, is_active and is_focused. The API offers WorkspaceActivated{id,focused}, Request::FocusedOutput and Action::FocusWindow{id}. The niri-ipc crate (26.4.0) follows niri's version and is not semver-stable; the JSON only gains fields and variants. *Evidência:* https://github.com/YaLTeR/niri/wiki/IPC; https://docs.rs/niri-ipc/latest/niri_ipc/ (Event, Window, Workspace, Request, Action). *Confiança:* alta.
- KWin D-Bus API: org.kde.KWin on /KWin has activeOutputName(), queryWindowInfo and getWindowInfo. /Scripting loadScript plus the script's run() load and run JavaScript with no check on who calls. Scripts reach the outside world through callDBus(). kdotool works exactly this way: it registers a D-Bus connection and the script calls back into it. *Evidência:* invent.kde.org/plasma/kwin/-/raw/master/src/org.kde.KWin.xml; src/scripting/scripting.cpp; https://github.com/jinliu/kdotool (README and src/main.rs). *Confiança:* alta.
- KWin scripting API in Plasma 6. workspace has activeWindow (read-write), a windowActivated signal, activeScreen, cursorPos and cursorPosChanged. Window has pid, output and outputChanged. *Evidência:* https://develop.kde.org/docs/plasma/kwin/api/. *Confiança:* alta.
- org_kde_plasma_window offers pid_changed (since v8), the 'active' state, set_state (which can activate a window) and geometry events. KWin exposes it to non-sandboxed clients. *Evidência:* https://wayland.app/protocols/kde-plasma-window-management; KWin wayland_server.cpp allowInterface(). *Confiança:* alta.
- Wayfire's IPC has window-rules/list-views (with pid), window-rules/focus-view, window-rules/get-focused-output and window-rules/events/watch, with events such as output-gain-focus and view-focused. It requires the 'ipc' and 'ipc-rules' plugins. *Evidência:* https://raw.githubusercontent.com/WayfireWM/pywayfire/master/wayfire/ipc.py. *Confiança:* alta.
- COSMIC has no IPC socket. zcosmic_toplevel_info_v1 (attached to ext_foreign_toplevel_list since v2) gives output_enter/leave and the activated state, without PID. Activation goes through zcosmic_toplevel_manager_v1. The cosmic-protocols crate 0.2.0 is on crates.io. *Evidência:* https://wayland.app/protocols/cosmic-toplevel-info-unstable-v1; https://crates.io/api/v1/crates/cosmic-protocols. *Confiança:* alta.
- On river's main branch, window-management policy lives in a separate window-manager process (river-window-management-v1). The build no longer generates river-status-unstable-v1 but still has wlr-layer-shell. *Evidência:* https://codeberg.org/river/river/raw/branch/main/README.md; https://codeberg.org/river/river/raw/branch/main/build.zig. *Confiança:* media.
- labwc has no IPC (only signals and D-Bus environment updates) and does not activate the systemd session target by itself. *Evidência:* https://raw.githubusercontent.com/labwc/labwc/master/docs/labwc.1.scd. *Confiança:* alta.
- GNOME 49 disabled the X11 session by default. Mutter 50.alpha has the entry 'Drop the X11 backend', so GNOME 50+ is Wayland-only and X11 apps run through Xwayland. Mutter 49 added xdg-toplevel-tag and similar protocols, but not layer-shell. *Evidência:* https://release.gnome.org/49/developers/; https://gitlab.gnome.org/GNOME/mutter/-/raw/main/NEWS. *Confiança:* alta.
- KDE announced that Plasma 6.8 will be Wayland-exclusive, with the X11 session supported into early 2027. *Evidência:* https://blogs.kde.org/2025/11/26/going-all-in-on-a-wayland-future/. *Confiança:* alta.
- GNOME Shell. addTopChrome puts an actor 'above all windows, including popups'. With trackFullscreen it hides when there is a fullscreen window. Chrome changes the input region, so clicks outside the actor still reach the windows. Main.activateWindow(window) switches workspace and leaves the overview. Meta.Display.get_current_monitor() is the monitor under the pointer. Clutter supports the NEAREST scaling filter. *Evidência:* gitlab.gnome.org/GNOME/gnome-shell/-/raw/main/js/ui/layout.js and js/ui/main.js; gnome.pages.gitlab.gnome.org/mutter/meta/method.Display.get_current_monitor.html; .../clutter/method.Actor.set_content_scaling_filters.html. *Confiança:* alta.
- EGO review rules: an extension must not include executable binaries or libraries, must clean up everything in disable(), and its shell-version may only list stable releases plus at most one development release. *Evidência:* https://gjs.guide/extensions/review-guidelines/review-guidelines.html. *Confiança:* alta.
- Mutter puts override-redirect windows in META_LAYER_OVERRIDE_REDIRECT. Whether such a window really gets click-through and stays above fullscreen Wayland windows under XWayland was not tested. *Evidência:* gitlab.gnome.org/GNOME/mutter/-/raw/main/src/core/window.c (meta_window_constructed). *Confiança:* media.
- Shijima-Qt, a desktop-pet app, supported only KDE Plasma 6 and GNOME 46 (Wayland and X11). It auto-installed a shell plugin to read the frontmost window, and on GNOME the first run required logging out and back in. The project is archived. *Evidência:* https://github.com/pixelomer/Shijima-Qt. *Confiança:* alta.
- The third-party 'Window Calls' GNOME extension provides D-Bus methods List (with pid), Details and Activate, among others. *Evidência:* https://github.com/ickyicky/window-calls. *Confiança:* alta.
- EWMH: in a _NET_ACTIVE_WINDOW ClientMessage, data.l[0] is the source indication (1 = application, 2 = pager), followed by a timestamp and the requester's active window. _NET_WM_PID contains the PID of the client. A WM 'typically' keeps DOCK windows above all others. *Evidência:* https://specifications.freedesktop.org/wm/latest/ar01s03.html; https://specifications.freedesktop.org/wm/latest/ar01s05.html. *Confiança:* alta.
- x11rb 0.14.0: the default RustConnection is pure Rust (libxcb only with dl-libxcb), with optional features shape, xfixes, randr, render and composite. zbus 5.19.0 is pure Rust, and its blocking API uses its own block_on. wayland-protocols-wlr and wayland-protocols-plasma are at 0.3.12. A static musl binary therefore stays possible for every Linux backend. *Evidência:* https://docs.rs/crate/x11rb/latest/features; https://docs.rs/zbus/latest/zbus/; crates.io API for wayland-protocols-wlr and wayland-protocols-plasma. *Confiança:* alta.
- fractional-scale-v1 is supported by COSMIC, Hyprland, KWin, labwc, Mir, Mutter, niri, phoc, river, sway, Treeland and Wayfire, but not by Muffin, Cage, gamescope or Weston. cursor-shape-v1 is missing on Wayfire, Mir, Louvre and Weston. *Evidência:* https://wayland.app/protocols/fractional-scale-v1; https://wayland.app/protocols/cursor-shape-v1. *Confiança:* alta.
- systemd: services tied to the graphical session should use PartOf=graphical-session.target, and desktops opt in to xdg-desktop-autostart.target with Wants=. systemd-xdg-autostart-generator turns XDG autostart files into units under that target, honouring OnlyShowIn/NotShowIn, Hidden and TryExec, and skipping files that have X-GNOME-Autostart-Phase. *Evidência:* https://man.archlinux.org/man/systemd.special.7.en; https://man.archlinux.org/man/systemd-xdg-autostart-generator.8.en. *Confiança:* alta.
- Which sessions start XDG autostart and graphical-session.target. niri.service has BindsTo=graphical-session.target and Wants=xdg-desktop-autostart.target. uwsm binds graphical-session-pre.target, graphical-session.target and xdg-desktop-autostart.target. cosmic-session processes ~/.config/autostart (or defers to systemd) and signals graphical-session.target. sway's default config has no systemd integration. *Evidência:* raw.githubusercontent.com/YaLTeR/niri/main/resources/niri.service; raw.githubusercontent.com/Vladimir-csp/uwsm/master/README.md; raw.githubusercontent.com/pop-os/cosmic-session/master/src/main.rs; raw.githubusercontent.com/swaywm/sway/master/config.in. *Confiança:* alta.
- cargo-dist is actively maintained (v0.33.0 released 2026-09-11, with musl and gnu Linux targets). Its installers are shell, powershell, npm, homebrew and msi. deb, rpm, AUR and Flatpak appear only as requested future installers. *Evidência:* https://github.com/axodotdev/cargo-dist/releases; https://axodotdev.github.io/cargo-dist/book/installers/index.html. *Confiança:* media.
- Docker Desktop for Linux runs a virtual machine with its own docker context (desktop-linux), so containers there do not share the host's Wayland socket the way native Docker Engine or Podman do. *Evidência:* https://docs.docker.com/desktop/setup/install/linux/. *Confiança:* alta.
- In the current code, the daemon only connects to Wayland after descoberta::procurar finds base/hypr/<HIS> with a hyprland.lock and both .socket2.sock and wayland-N accept a connection. On a desktop without Hyprland it never connects. The wl/ layer itself is almost generic (NULL output, 1x1 transparent map until enter + preferred_scale, viewporter, fractional-scale, cursor-shape); the main Hyprland-specific detail is the 'FALLBACK' output name. *Evidência:* ~/Documents/claude-pet/crates/claude-pet/src/descoberta.rs (header and procurar); crates/claude-pet/src/wl/superficie.rs (header); crates/claude-pet/src/wl/saida.rs:12. *Confiança:* alta.
- The plugin hook avisar.sh depends on jq (with only a minimal payload without it), curl, GNU date +%s%3N (with a fallback), sha256sum or shasum, and Omarchy's DND state file. jq is not installed by default on many distributions (general knowledge, not verified). *Evidência:* ~/Documents/claude-pet/plugin/scripts/avisar.sh. *Confiança:* media.

### Recomendações

1. Decouple discovery from Hyprland: connect to any Wayland display ($WAYLAND_DISPLAY, falling back to scanning $XDG_RUNTIME_DIR/wayland-*) and require only zwlr_layer_shell_v1. Treat the Hyprland IPC as an optional adapter, and keep the hyprland.lock scan only for the Docker mode.
2. Split the platform layer into two traits with capability flags: OverlayBackend (wayland layer-shell, x11, gnome-remote; later macOS and Windows) and DesktopAdapter (hyprland, sway/i3, niri, kwin, wayfire, cosmic, wlr_toplevel, ewmh, gnome, none). Have pet-core produce frames at art resolution and move the DxD upscaling into each backend.
3. Make 'summon' the universal baseline: on attention events, re-create the layer surface with output NULL and read wl_surface.enter. It needs no IPC and works on Hyprland, sway, niri, KWin, COSMIC and labwc thanks to the NULL-output semantics.
4. Implement adapters in this order: sway/i3 IPC (workspace focus events; '[pid=N] focus'), niri IPC (EventStream; FocusWindow{id}), KWin (zbus: activeOutputName plus a script loaded through /Scripting that calls back with callDBus and sets workspace.activeWindow), generic wlr-foreign-toplevel (labwc, Wayfire, river, Mir, Treeland), COSMIC (cosmic-toplevel-info/management), then Wayfire IPC as an optional extra. Speak the niri JSON with your own serde types rather than pinning the niri-ipc crate.
5. Map each session to its window in two ways. First, take a snapshot of the active window at UserPromptSubmit and store an opaque handle per session_id (no titles); this works on every desktop. Second, where the compositor exposes PIDs (Hyprland, sway, niri, KWin, Wayfire, X11, GNOME extension), have the native daemon walk the PID chain in /proc while the hook only sends $PPID plus terminal hints (TMUX_PANE, KITTY_WINDOW_ID, WEZTERM_PANE, TERM_PROGRAM).
6. Linux v1.0: Tier 1 = Hyprland, KDE Plasma 6 Wayland, sway, niri, COSMIC. Tier 2 = labwc, Wayfire, river, Mir/Miriway, Treeland, Cinnamon-Wayland (best effort). Detect GNOME and X11 and print a clear message pointing to the roadmap.
7. GNOME (v1.1, or v1.0 if the audience is mostly Ubuntu/Fedora): write a thin GJS extension that acts as a remote overlay. It uses addTopChrome with affectsInputRegion, NEAREST scaling and art-resolution frames received over D-Bus, plus FocusPid/FocusWindowId via Main.activateWindow, the active window and the current monitor. Publish it on EGO separately from the binary, target GNOME 47-51, and document the logout/login needed on first install.
8. X11 backend (v1.1/v1.2) on x11rb: either a managed window with DOCK/UTILITY and ABOVE, STICKY and SKIP_* states, or override-redirect. Use an ARGB visual only when _NET_WM_CM_Sn has an owner, otherwise an XShape bounding mask; set an input shape covering the sprite; focus with _NET_ACTIVE_WINDOW source=2 plus _NET_WM_PID; take the monitor from RandR. Reuse the sway IPC client for i3. As a bonus, this enables an 'experimental' XWayland mode on GNOME.
9. Distribute static musl binaries (x86_64 and aarch64) through cargo-dist (tarball, curl|sh installer, Homebrew) and add .deb/.rpm (cargo-deb and cargo-generate-rpm, or nFPM), AUR packages (claude-pet-bin and claude-pet) and a Nix flake. Do not ship Flatpak, Snap or AppImage. Keep Docker only as a development or maintainer mode.
10. Autostart: a 'claude-pet autostart enable' command that writes the XDG autostart .desktop by default. A --systemd option writes a user unit with PartOf=, After= and WantedBy=graphical-session.target plus Restart=on-failure. Print the startup snippet for sway, Hyprland without uwsm, labwc, Wayfire, river and niri without niri-session. Use only one of the two methods so they do not race; single-instance comes from binding the ingress.
11. Replace avisar.sh (jq, curl, GNU date) with a 'claude-pet hook <Evento>' subcommand of the same binary. Keep the same metadata allowlist, always exit 0, never print anything. This removes the dependencies on Linux and also covers macOS and Windows.
12. Add 'claude-pet doctor': it shows the detected session, the overlay backend, the desktop adapter, its capabilities (follows focus, focuses window, PID, active window) and the missing protocols (fractional-scale, cursor-shape).
13. Before giving the daemon compositor-control powers on other desktops (sway/niri IPC, KWin scripts, Hyprland dispatch), harden the ingress. Use a Unix socket in $XDG_RUNTIME_DIR (0700) or a per-user token, keep X-Pet, accept only high-level actions such as 'focus the window of session X', and never forward arbitrary commands.
14. Generalize what is Omarchy-specific behind the adapter. Do-not-disturb: the Inhibited property of org.freedesktop.Notifications, GNOME show-banners, makoctl/swaync/dunst. Session lock. The 'FALLBACK' output name check moves into the Hyprland adapter.
15. Set up CI with headless sessions for smoke tests: sway or labwc with WLR_BACKENDS=headless and grim, kwin_wayland --virtual, and Xvfb with a simple WM. Validate the specific invocations; they were not verified in this research.
16. Publish a support table (tiers and features per desktop) in the README, and add an option to hide Zeca while a fullscreen window is focused on adapters that know about fullscreen (Hyprland 'fullscreen' event, sway IPC, KWin script).

### Riscos

- **Risco:** Flatpak, or any sandbox that uses wp_security_context_v1, loses layer-shell and foreign-toplevel on Hyprland, sway, niri and COSMIC. Users who expect a Flathub release would get a pet that does not appear. **Mitigação:** Do not offer Flatpak. Ship a native static binary plus deb, rpm, AUR, Nix and Homebrew, and explain why in the README.
- **Risco:** GNOME, the largest Linux desktop and Wayland-only from GNOME 50, gets no support in v1.0 because it has no layer-shell. That would disappoint a large share of Ubuntu and Fedora users. **Mitigação:** Detect GNOME and explain clearly. Prioritize the thin extension (v1.1, or v1.0 if the audience calls for it). Offer an 'experimental' XWayland mode as an interim option once the X11 backend exists.
- **Risco:** The GNOME extension needs maintenance. shell-version must be bumped every 6 months, the API changes, EGO reviews take time, binaries cannot be included, and the first install needs a logout/login. **Mitigação:** Keep the extension minimal (overlay, focus, monitor) with all logic in Rust. Version the D-Bus API. Test against nested or headless GNOME Shell in CI (to be validated).
- **Risco:** KWin's loadScript over D-Bus has no access control today. KDE may restrict it in future, which would break the KDE adapter. **Mitigação:** Isolate it in the adapter. Fall back to org_kde_plasma_window_management (pid_changed, set_state) for focusing, and to summon (NULL output) or activeOutputName for the monitor.
- **Risco:** niri-ipc is not semver-stable and follows niri's version; river 0.4 changed its architecture and removed river-status. Adapters can break when the compositor is upgraded. **Mitigação:** Parse the JSON with tolerant serde types (ignore unknown fields). Treat river as Tier 2 with the generic adapter. Run per-compositor smoke tests in CI wherever there is a headless backend.
- **Risco:** Mapping a session to its window through the PID chain fails with tmux, zellij or screen, with terminals that use one process for many windows (gnome-terminal-server, Ghostty, WezTerm, kitty single-instance, foot server), with IDE terminals, and with SSH or containers. On COSMIC, labwc, river and Mir there are no PIDs at all. **Mitigação:** Use the active-window snapshot taken at UserPromptSubmit as the main mechanism and the PID chain as reinforcement. Add optional terminal adapters (tmux, kitty, wezterm). When focusing is impossible, give visible feedback with a reaction or bubble.
- **Risco:** 'Focused monitor' means different things on different desktops (pointer versus keyboard focus versus workspace). The pet may show up on the 'wrong' monitor. **Mitigação:** Offer a config option such as seguir = foco / ponteiro / fixo where both sources exist. Keep summon on attention events. Document how each desktop behaves.
- **Risco:** (Not measured.) A visible full-output OVERLAY surface above fullscreen apps may prevent direct scanout or VRR on some compositors and cost performance in games and video. **Mitigação:** Measure per compositor, as was done on Hyprland. Add a 'hide while fullscreen' option where the adapter knows about fullscreen. Consider a sprite-sized surface mode on compositors where moving margins is cheap.
- **Risco:** Once installed natively, the daemon can control the compositor (sway/niri IPC, KWin scripts, Hyprland dispatch), while localhost TCP is reachable by other local users and processes. **Mitigação:** Use a Unix socket in $XDG_RUNTIME_DIR (0700) or a token, keep the X-Pet header (which blocks browser CSRF), accept only an allowlist of high-level actions, and never relay arbitrary commands.
- **Risco:** Under systemd or XDG autostart the environment may lack WAYLAND_DISPLAY and the compositor variables (sway or Hyprland without uwsm, labwc), so the daemon would start without a display. **Mitigação:** Use XDG autostart by default plus the startup snippets. Keep the XDG_RUNTIME_DIR scan and the retry loop that already exist. Report the cause in 'claude-pet doctor'.
- **Risco:** Less common compositors may lack protocols the current path relies on: fractional-scale (Muffin, Cage, gamescope, Weston) and cursor-shape (Wayfire, Mir). The input region on layer surfaces, implicit grabs while dragging, and COSMIC behaviour are not verified. **Mitigação:** Fall back to integer buffer_scale and to a cursor-less or xcursor pointer. Publish Tier 2 as 'best effort' and keep a community-run test matrix.
- **Risco:** On X11 without a compositing manager there is no real alpha, and XWayland on GNOME is blurry at fractional scale and cannot see Wayland windows. **Mitigação:** Use an XShape bounding mask (binary alpha, which suits pixel art). Mark the GNOME XWayland mode as experimental. The real GNOME solution is the extension.
- **Risco:** (Cross-cutting; outside the Linux scope but it blocks distribution.) Zeca's art is derived from a paid pack that cannot be redistributed, and its look resembles a third-party character. No open-source package (deb, AUR, tarball) can include it. **Mitigação:** Ship an original default skin under a free licence. Keep 'skin-instalar <zip>' for people who bought the pack. Avoid names or trade dress that reference third-party characters, and get a legal review before release.

## Achado 3 — Claude Code e terminais: hooks e foco

*Zeca × Claude Code em Linux, macOS e Windows — hooks cross-platform e foco do terminal*

Pesquisa read-only em 2026-10-03. Fontes primárias: docs oficiais em code.claude.com (hooks, plugins, env-vars, setup, agent-view, mods, marketplace), código-fonte (Windows Terminal, WinUI, iTerm2, Ghostty, Warp, kitty, Hyprland, Konsole), docs da Microsoft e da Apple, e o código de 6 projetos parecidos. Também conferi nesta máquina: Claude Code 2.1.288 e Hyprland 0.56.2.

### TL;DR

- **Windows dá, sim.** Os hooks de plugin rodam no Windows. O que quebra é o `avisar.sh` (POSIX sh + jq + curl). No Windows, o shell form usa **Git Bash** quando ele existe e **PowerShell** quando não existe. Git Bash não traz `jq`, então só sai o evento mínimo. Sem Git Bash, `sh "..." || true` falha e nenhum evento chega.
- **Transporte recomendado:** um único `hooks.json` em **exec form** (`"command": "claude-pet", "args": ["avisar","Stop"], "async": true`). Ele chama um helper nativo que lê o JSON do stdin, aplica a lista branca e faz o POST no loopback. Não depende de shell, quoting nem jq, e é igual nos 3 SOs. A doc oficial diz que, no Windows, o exec form precisa resolver para um `.exe` real. `"command": "node"` é o exemplo oficial, então um nome simples no PATH funciona.
- **Âncora de identidade:** `CLAUDE_PID`, documentado desde a v2.1.214 e exportado para os hooks com o PID do próprio Claude Code. Isso evita andar pelo `$PPID`. No Windows sob Git Bash, `$PPID` é um PID do MSYS e não do Windows.
- **Foco:** o hook só captura uma identidade opaca. O foco acontece **no processo do pet, no clique**:
  - **Windows** só deixa chamar `SetForegroundWindow` a quem "recebeu o último evento de input".
  - **macOS 14+** usa ativação cooperativa.
  - O **TCC** atribui o prompt ao app do pet.
- **Precisão alcançável:**

| Sistema | O que dá para focar |
|---|---|
| macOS | Aba exata no Terminal.app, iTerm2, Ghostty ≥1.3, Warp, kitty, WezTerm e tmux. Só janela ou app no VS Code, Cursor e JetBrains |
| Windows | Janela exata do Windows Terminal (WT) com o truque `AttachConsole` + dono da pseudo-janela. Aba via UI Automation, que funciona na prática mas não é suportado pelo time do WT. O próprio WT 1.26 Preview (02/10/2026) passou a abrir toast via OSC 777 ou BEL, e o clique foca a aba certa |
| Linux | Janela via IPC do compositor (Hyprland, sway, X11, KDE). Aba via IPC do terminal (kitty, WezTerm, tmux, zellij, Konsole, Warp). GNOME Wayland só com extensão |

### 1. Hooks: o que vale em cada SO

#### 1.1 Shell e formas de execução (doc oficial `hooks.md`)

- **Shell form** (sem `args`): "`sh -c` on macOS and Linux, Git Bash on Windows, or PowerShell when Git Bash isn't installed".
  - O campo `shell` aceita `"bash"` ou `"powershell"` e é ignorado quando há `args`.
  - O Git Bash pode carregar o perfil do usuário. Um `echo` no perfil corrompe o JSON de saída (`hooks-guide.md`).
- **Exec form** (com `args`): "Claude Code resolves `command` as an executable on `PATH` and spawns it directly". Não há tokenização. `${CLAUDE_PLUGIN_ROOT}` é substituído em `command` e em `args`.
  - No Windows: "exec form requires `command` to resolve to a real executable such as a `.exe`". Os shims `.cmd`/`.bat` não funcionam.
  - No exec form, plugins também substituem `${user_config.KEY}`, o que serve para passar a porta.
- **Windows:** o Git for Windows é opcional. Sem ele, o Claude usa a ferramenta PowerShell: `pwsh.exe` quando existe, senão `powershell.exe`.
- **`CLAUDE_PLUGIN_ROOT`:**
  - Aponta para `~/.claude/plugins/cache/<marketplace>/<plugin>/<versão>/` e muda a cada versão.
  - No Windows vem com barras normais.
  - Estado persistente fica em `CLAUDE_PLUGIN_DATA` (`~/.claude/plugins/data/<id>/`).
- **`bin/` do plugin:** entra só no PATH da *ferramenta Bash*, não no dos hooks. Além disso, claude.ai e Cowork **recusam** plugins que têm `bin/` no topo.

#### 1.2 Async, timeouts e erros

- `async: true` vale só para `type: "command"`.
  - O `timeout` **não** é imposto em hooks async, então o helper precisa se limitar sozinho.
  - A saída chega no próximo turno.
  - Os avisos de conclusão ficam ocultos sem `--verbose`.
  - Com `-p`, hooks async ainda vivos são mortos no fim da sessão.
- **Timeouts padrão:**
  - 600 s para `command` e `http`.
  - 30 s em `UserPromptSubmit`.
  - `SessionEnd` tem um orçamento **compartilhado de 1,5 s**, e "Timeouts set on plugin-provided hooks don't raise the budget".
  - Recomendação: `SessionEnd` **síncrono** com `"timeout": 1`, e o POST no loopback leva milissegundos.
- **Exit codes:** só o 2 bloqueia, então manter sempre exit 0 (já é assim).
- **Hooks no macOS e no Linux** rodam "in their own session without a controlling terminal". Não têm `/dev/tty`. Para escrever no terminal existe o campo de saída **`terminalSequence`**:
  - Aceita só OSC 0/1/2/9/99/777 e BEL.
  - Só vale em sessão interativa.
  - Funciona também em `Notification` e `StopFailure`.
  - A doc não diz se vale para hooks async. Se for usar, use um hook síncrono.
- **Windows:** a doc não tem nada específico sobre async ou timeout. Na prática (projeto agent-notifications), o hook nasce com um console oculto próprio (`CREATE_NO_WINDOW`) e consegue `AttachConsole` no console do Claude ancestral.

#### 1.3 Variáveis úteis no hook

**Postas pelo Claude Code:**
- `CLAUDE_PID` (v2.1.214+) e `CLAUDE_CODE_SESSION_ID`, que é igual ao `session_id` e muda no `/clear`.
- `CLAUDE_PLUGIN_ROOT`, `CLAUDE_PLUGIN_DATA`, `CLAUDE_PROJECT_DIR` e `CLAUDE_PLUGIN_OPTION_<KEY>`.
- `CLAUDECODE=1`, `CLAUDE_CODE_CHILD_SESSION=1`, `CLAUDE_EFFORT` e `CLAUDE_CODE_MESSAGING_SOCKET`.

**Herdadas do terminal** (o hook herda o ambiente do Claude):
- `TERM_PROGRAM`, `ITERM_SESSION_ID` (formato `wNtNpN:UUID`).
- `KITTY_WINDOW_ID` e `KITTY_LISTEN_ON`; `WEZTERM_PANE`.
- `WARP_FOCUS_URL` (`warp://session/<hex>`, conforme o código do Warp).
- `TMUX` e `TMUX_PANE`; `ZELLIJ_SESSION_NAME` e `ZELLIJ_PANE_ID`.
- `WT_SESSION`; `KONSOLE_DBUS_WINDOW` (`/Windows/N`).

**`CLAUDE_CODE_ENTRYPOINT`** não é documentado. No binário 2.1.288 aparecem estes valores:
- `cli`, que vira `sdk-cli` em modo não interativo; `sdk-ts`, `sdk-py`; `mcp`; `claude-code-github-action`.
- `claude-vscode`; `claude-desktop`, `claude-desktop-3p`, `local-agent`.
- `remote`, `remote_desktop`, `remote_mobile`, `remote_projects`, `remote_trigger`, `remote_cowork`, `remote_cowork_trigger`, `remote_baku`.
- `claude-in-slack` e `claude_in_slack`, `claude-in-teams`.

Nesta sessão o valor observado foi `cli`. Use-o só como dica de estratégia:
- `cli`: há terminal.
- `claude-vscode`: focar o VS Code.
- `claude-desktop` ou `local-agent`: focar o app Desktop.
- `remote*`: não há nada local.

#### 1.4 Opções de transporte

| Opção | Cross-OS | Privacidade | Identidade do terminal | Veredito |
|---|---|---|---|---|
| **Helper nativo em exec form** (`claude-pet avisar <Evento>`) | Sim, com o mesmo `hooks.json` | Lista branca dentro do helper, como hoje | Lê `CLAUDE_PID`, tty, env, `AttachConsole` | **Recomendado** |
| HTTP hook (`type: "http"`) | Sim, sem nenhum binário | **Manda o JSON inteiro** (prompt, `tool_input`, `last_assistant_message`) para 127.0.0.1. Quem ocupar a porta recebe tudo | Headers com env via `allowedEnvVars` (`TERM_PROGRAM`, `ITERM_SESSION_ID`, `WT_SESSION`…), mas não `CLAUDE_PID` | Só como modo "sem instalar". É síncrono e sujeito a `allowedHttpHookUrls` |
| Mod (handlers JS dentro do Claude Code, v2.1.287+) | Sim, sem binário | Filtra dentro do processo e usa `$.http.fetch` no loopback | `$.env.get(...)`, `$.process.run([...])` | Promissor para uma v2. A API é nova e orgs podem bloquear (`allowManagedModsOnly`) |
| `avisar.sh` atual | Linux e macOS (degradado) | Ok | Não tem | Manter só como fallback no Linux |

#### 1.5 Receita de hooks e distribuição

```json
{"hooks":{
  "Stop":[{"hooks":[{"type":"command","command":"claude-pet","args":["avisar","Stop"],"async":true}]}],
  "SessionEnd":[{"hooks":[{"type":"command","command":"claude-pet","args":["avisar","SessionEnd"],"timeout":1}]}]
}}
```

**Distribuição — opção (a):** marketplace no GitHub com esse `hooks.json` genérico. O instalador do app (Homebrew, winget/MSI, pacotes Linux) põe `claude-pet` no PATH.

**Distribuição — opção (b):** fonte de plugin **`command`** do marketplace (`"source":{"source":"command","command":"claude-pet claude-plugin-path"}`).
- O app instalado imprime um diretório de plugin gerado para o SO, com o caminho absoluto do `.exe` ou do binário.
- A doc diz: "Claude Code runs the command through `sh`, or through `cmd.exe` on Windows". O modo `link` é recusado no Windows, então use `copy`.
- Admins podem desligar isso com `disableCommandPluginSources`.

**Cuidados:**
- Quando o Claude roda pelo app Desktop ou pela extensão do VS Code no macOS, o PATH pode ser mínimo. Prefira caminho absoluto, com a opção (b) ou um symlink em `/usr/local/bin`.
- Não use `bin/` no topo do plugin.

#### 1.6 Lista de sessões (a bolha)

- `claude agents --json` é, nas palavras da doc, "the supported way to read session state from outside Claude Code".
  - Campos: `cwd`, `kind` (`interactive` ou `background`), `startedAt`, `pid`, `status` (`busy`, `waiting`, `idle`), `waitingFor`, `sessionId`, `name`.
  - Use para reconciliar, já que os hooks podem se perder no fim da sessão.
- Existe `~/.claude/sessions/<pid>.json` (observado: pid, sessionId, cwd, name, status, waitingFor), mas o formato não é interface.
- `~/.claude/jobs/` é declaradamente instável.

### 2. Identificar e focar o terminal

**Princípio:** o hook roda em `SessionStart` e `UserPromptSubmit` e manda identidade opaca: `CLAUDE_PID`, o tty (Unix) e os IDs de env. No Windows manda também o HWND da janela do WT e o RuntimeId da aba. O pet resolve tudo no clique, em camadas:

1. aba ou pane exato;
2. janela;
3. app;
4. só a bolha.

Dentro do tmux, os IDs de env congelam no momento em que o pane nasce. No clique, use `tmux list-clients` e ache o terminal pelos ancestrais do cliente (é o que o peon-ping faz).

#### 2.1 macOS

**Identificação:**
- `ps -o tty= -p $CLAUDE_PID` dá algo como `ttys003`.
- Subir pelos ancestrais até achar um processo com bundle id: `lsappinfo info -only bundleid -app pid:N` ou `NSRunningApplication(processIdentifier:)`.
- Com o iTerm2, os shells são filhos do `iTermServer`, que pode ser reparentado ao launchd. Nesse caso use `ITERM_SESSION_ID`.

**Receitas por terminal:**

**Terminal.app** — o sdef tem `tab.tty` (leitura) e `tab.selected` (leitura e escrita), mais `index`, `frontmost` e `selected tab` na janela:
```applescript
tell application "Terminal"
  repeat with w in windows
    repeat with t in tabs of w
      if tty of t is "/dev/ttys003" then
        set selected tab of w to t
        set index of w to 1
        activate
        return
      end if
    end repeat
  end repeat
end tell
```

**iTerm2** — o AppleScript está marcado como "(Deprecated)", mas funciona:
- Comparar `unique id of s` com o UUID depois do `:` do `ITERM_SESSION_ID`, ou `tty of s`.
- Depois `select w`, `select t`, `select s` e `activate`. Pela doc, `select` na janela "gives the window keyboard focus and brings it to the front".
- Alternativa: Python API `Session.async_activate(select_tab=True, order_window_front=True)`, que exige ligar "Enable Python API".

**Ghostty ≥1.3.0** — AppleScript:
- `focus (item 1 of (every terminal whose working directory is "<cwd>"))`.
- As propriedades `tty` e `pid` entraram no main em 2026-04-20 e ainda **não** estão na v1.3.1. Com elas o casamento passa a ser exato.
- O usuário pode desligar o AppleScript com `macos-applescript = false`.

**Outros terminais:**

| Terminal | Como focar |
|---|---|
| Warp | `open "$WARP_FOCUS_URL"` foca o pane exato |
| kitty | `kitten @ --to "$KITTY_LISTEN_ON" focus-window --match id:$KITTY_WINDOW_ID`. Exige `allow_remote_control` e `listen_on`. Depois, ativar o app |
| WezTerm | `wezterm cli activate-pane --pane-id $WEZTERM_PANE`, depois ativar o app |
| tmux | `switch-client -c <tty> -t <pane> \; select-window -t <pane> \; select-pane -t <pane>` |
| VS Code, Cursor, JetBrains, Alacritty | Só o app ou a janela. O peon-ping registrou que o AXRaise não reordena de forma confiável janelas Electron múltiplas |

**Ativação:**
- No macOS 14, `activateIgnoringOtherApps` "is deprecated in macOS 14 and will have no effect".
- No clique, o pet fica ativo e usa `NSRunningApplication.activate(from:options:)` ou `NSApp.yieldActivation(to:)` + `activate()`.
- Fallback: `tell application id "<bundle>" to activate`.

**Permissões e empacotamento:**
- **Automation (Apple Events):** um prompt por app alvo. Exige `NSAppleEventsUsageDescription` e, com Hardened Runtime ou notarização, o entitlement `com.apple.security.automation.apple-events`.
- **Accessibility:** só se usar AXRaise.
- **Screen Recording:** só se ler títulos via CGWindowList.
- Distribua como **.app assinado**. Senão o prompt do TCC sai em nome do terminal ou do `osascript`.

#### 2.2 Windows

**Janela.** No helper do hook, que tem console oculto próprio, ou no pet:
```text
FreeConsole(); AttachConsole(CLAUDE_PID)
h   = GetConsoleWindow()               // em ConPTY: janela só "para fila de mensagens"
top = GetAncestor(h, GA_ROOTOWNER)     // dono = janela do WT que hospeda a aba
FreeConsole()
```
- O código do WT confirma esse caminho: `ConptyReparentPseudoConsole` vira `SetWindowLongPtrW(..., GWLP_HWNDPARENT, owner)`, "so GetConsoleWindow() ... is owned by the actual hosting terminal's HWND" (GH#2988).
- O dono é atualizado quando a aba muda de janela e também no defterm handoff.
- No conhost clássico, `h` já é a própria janela.
- Na extensão do VS Code e no node-pty não há reparent. Nesse caso, fallback: subir pela árvore com Toolhelp32 (`th32ParentProcessID`, conferindo o horário de criação) até `Code.exe`, enumerar as janelas e desempatar pelo título (pasta do projeto).
- A janela do WT tem a classe `CASCADIA_HOSTING_WINDOW_CLASS`.

**Aba do WT:**
- `wt -w <id|nome|0> focus-tab -t <índice>` existe. Mas não há como mapear sessão → id da janela ou → índice da aba.
- O time do WT **rejeitou** "Focus/Activate Tab by WT_SESSION" (#19783, not planned, jan/2026) e não mergeou o `focus-tab --session` (#19784). Preferiram notificações consentidas.
- **WT 1.26 Preview (02/10/2026):**
  - suporta OSC 777 (`compatibility.allowOSC777`, opt-in);
  - suporta `bellStyle: "notification"`;
  - o clique no toast acha a aba e o pane de origem, mesmo depois de a aba ser movida (#20010).
- **Gambiarra que funciona: UI Automation.**
  - O `TabViewItemAutomationPeer` do WinUI implementa `ISelectionItemProvider.Select()`.
  - O claude-code-notify (MIT, Rust, crate `windows` 0.61) faz assim:
    - no `UserPromptSubmit`, se o foreground é o WT, grava o RuntimeId do `TabItem` selecionado;
    - no clique, acha o `TabItem` com esse RuntimeId e chama `SelectionItemPattern.Select()`.
  - Melhoria possível: só gravar quando `GetForegroundWindow() == top`, com `top` vindo do `AttachConsole`.
  - Panes dentro da aba não dão para selecionar.

**Foreground:**
- `SetForegroundWindow` é permitido se, entre outras condições, "The calling process received the last input event". Por isso, chame dentro do handler de clique do pet, depois de `ShowWindow(SW_RESTORE)` se a janela estiver minimizada.
- Fallbacks usados pelos projetos existentes:
  - `AllowSetForegroundWindow(ASFW_ANY)` antes de disparar outros processos;
  - `AttachThreadInput`;
  - ALT simulado;
  - `SwitchToThisWindow` e `BringWindowToTop`.
- A doc avisa: "It is possible for a process to be denied ... even if it meets these conditions".

**WSL:**
- O hook roda no Linux.
- Em NAT (o padrão), o app do Windows não é alcançável por `localhost` a partir do WSL. Seria preciso o IP do host e bind em `0.0.0.0`, o que expõe à LAN.
- Com `networkingMode=mirrored` (Windows 11 22H2+), `127.0.0.1` funciona nos dois sentidos.
- Focar a partir do WSL exige rodar um `.exe` Windows de dentro do hook. O claude-code-notify faz isso.

#### 2.3 Linux

- **Base:** `/proc/$CLAUDE_PID` (cadeia de ppid, `environ` legível pelo mesmo usuário, `fd/0` → `/dev/pts/N`).
- **Conferido localmente:** `claude` (pts/0) → `bash` → `foot`, e `hyprctl -j clients` liga o pid do foot a `address:0x…`.

**Por compositor:**

| Ambiente | Como focar a janela |
|---|---|
| Hyprland 0.56.2 | `hyprctl dispatch focuswindow address:0x…`. O nome legado ainda está no "legacy translator" do `KeybindManager`. A wiki atual documenta a forma Lua `focus({ window })`. Testar as duas |
| sway | `swaymsg '[pid=N] focus'` ou `[con_id=N]` |
| X11 | `_NET_ACTIVE_WINDOW` com source=2 (pager) via xdotool ou wmctrl, mapeando por `_NET_WM_PID` |
| KDE Wayland | KWin scripting por D-Bus (kdotool) |
| GNOME Wayland | Só via extensão (Window Calls, activate-window-by-title) |
| Família wlroots, Hyprland, niri | `zwlr_foreign_toplevel_handle_v1.activate` também funciona, mas sem PID: casar por `app_id` e título |

**Terminais de processo único com várias janelas** (foot server, gnome-terminal-server, kitty single-instance, WezTerm, Ghostty GTK, Konsole) deixam o PID ambíguo. Nesses casos, use o IPC do próprio terminal:
- kitty, WezTerm, tmux;
- zellij: `zellij -s S action focus-pane-id ID`, desde a 0.44.1;
- Konsole: `setCurrentSession(int)` no objeto D-Bus de `KONSOLE_DBUS_WINDOW`;
- Warp: `xdg-open $WARP_FOCUS_URL`.

### 3. Como as ferramentas existentes fazem

| Projeto | macOS | Windows | Linux/outros |
|---|---|---|---|
| **peon-ping** (★5k) | Overlay JXA e terminal-notifier. `activateWithOptions`. iTerm2 por tty + AXRaise. Warp por `WARP_FOCUS_URL`. tmux por cliente e ancestrais | Toast com parentPid: sobe pela árvore até achar `MainWindowHandle`, depois `AttachThreadInput` + `SetForegroundWindow`. Só janela | Notas: AXRaise falha com Electron multi-janela; o iTermServer é reparentado |
| **agent-notifications** (★813, Go) | AX + Screen Recording. Ghostty por AppleScript. iTerm2 por Python API | Só janela (afirma que aba é impossível). BEL via `AttachConsole` ao console do Claude, que marca a aba certa no WT | GNOME com extensão, wlrctl, kdotool, xdotool. zellij `focus-pane-id` |
| **chuilishi/claude-code-notify** (MIT, Rust) | — | **Troca a aba do WT por UIA** (RuntimeId gravado no `UserPromptSubmit`) mais os truques de foreground | Funciona dentro do WSL chamando o `.exe` |
| **MioIsland** (★541) | iTerm2 e Terminal por AppleScript. Ghostty por `working directory`. kitty `@ focus-window --match cwd:`. yabai para tmux | — | — |
| **code-notify** (★289) | `terminal-notifier -activate <bundle>` a partir de `TERM_PROGRAM`. Só o app | WSL: wsl-notify-send com AppId do WT | — |
| **CCNotify** (★215) | `terminal-notifier -execute 'code "<cwd>"'` | — | — |
| **terminal-notifier 3.x** | `-activate BUNDLE_ID`: "Bring an application to the front". Só o app | — | — |

### 4. Itens não verificados — testar antes de prometer

1. Exec form no Windows com caminho absoluto sem `.exe` (por exemplo `${CLAUDE_PLUGIN_ROOT}/x/claude-pet`).
2. Se `terminalSequence` vale em hooks async.
3. Se os hooks async sobrevivem ao fim da sessão interativa.
4. Se o `ITERM_SESSION_ID` dá exatamente o `unique id`.
5. Se o `kitten @ focus-window` levanta a janela do SO.
6. Se o `WT_SESSION` chega dentro do WSL.
7. Se o owner da pseudo-janela no VS Code é nulo.
8. A ativação cooperativa no macOS 15 e no 26, inclusive com Spaces.

Fora deste tema, mas bloqueante para "open source": a arte atual vem de um pacote pago que proíbe redistribuição.

### Fatos conferidos

- Shell form (sem args): sh -c no macOS/Linux, Git Bash no Windows, PowerShell se o Git Bash nao estiver instalado; o campo shell aceita bash ou powershell e e ignorado quando ha args. *Evidência:* https://code.claude.com/docs/en/hooks.md (secao Exec form and shell form; tabela Command hook fields). *Confiança:* alta.
- Exec form (com args) resolve command no PATH e faz spawn direto, sem shell; placeholders como ${CLAUDE_PLUGIN_ROOT} e ${user_config.*} sao substituidos em command/args; no Windows exige um executavel real (.exe), nao .cmd/.bat; o exemplo oficial usa command node. *Evidência:* https://code.claude.com/docs/en/hooks.md (Exec form and shell form, Note sobre Windows). *Confiança:* alta.
- async: true so existe para type command; o timeout nao e imposto em hooks async; a saida chega no proximo turno; avisos de conclusao ficam ocultos sem --verbose; em -p hooks async sao mortos no teardown. *Evidência:* https://code.claude.com/docs/en/hooks.md (Run hooks in the background). *Confiança:* alta.
- Timeouts padrao: 600 s para command/http/mcp_tool, 30 s em UserPromptSubmit; SessionEnd tem orcamento de 1,5 s e timeouts de hooks de plugin nao aumentam esse orcamento. *Evidência:* https://code.claude.com/docs/en/hooks.md (Common fields; secao SessionEnd). *Confiança:* alta.
- No macOS/Linux os hooks rodam em sessao propria sem terminal controlador (sem /dev/tty); o campo de saida terminalSequence faz o Claude Code emitir OSC 0/1/2/9/99/777 ou BEL, so em sessao interativa; a doc nao diz se vale em hooks async. *Evidência:* https://code.claude.com/docs/en/hooks.md (Hook input and output; Emit terminal notifications). *Confiança:* alta.
- CLAUDE_PID (v2.1.214+) e posto pelo Claude Code com o proprio PID nos subprocessos, incluindo comandos de hook; CLAUDE_CODE_SESSION_ID casa com o session_id do JSON do hook. *Evidência:* https://code.claude.com/docs/en/env-vars.md; observado nesta maquina: CLAUDE_PID=1713473 no ambiente do Bash tool. *Confiança:* alta.
- Hooks de plugin recebem CLAUDE_PLUGIN_ROOT, CLAUDE_PLUGIN_DATA, CLAUDE_PROJECT_DIR e CLAUDE_PLUGIN_OPTION_<KEY>; no Windows os caminhos substituidos usam barras normais; o processo do hook herda o ambiente do pai (TERM_PROGRAM etc.). *Evidência:* https://code.claude.com/docs/en/plugins/manifest-reference.md (Environment variables); https://code.claude.com/docs/en/plugins/components.md; hooks.md linha sobre heranca de ambiente. *Confiança:* alta.
- O bin/ de um plugin entra so no PATH da ferramenta Bash, nao no dos hooks; claude.ai e Cowork nao instalam plugins com bin/ no topo. *Evidência:* https://code.claude.com/docs/en/plugins/components.md (Executables); manifest-reference.md (Standard layout). *Confiança:* alta.
- Git for Windows e opcional no Windows nativo; sem ele o Claude usa a ferramenta PowerShell e detecta pwsh.exe com fallback para powershell.exe. *Evidência:* https://code.claude.com/docs/en/setup.md (Set up on Windows); tools-reference.md. *Confiança:* alta.
- CLAUDE_CODE_ENTRYPOINT nao e documentado. No binario 2.1.288 aparecem os valores cli (vira sdk-cli sem interatividade), sdk-ts, sdk-py, mcp, claude-code-github-action, claude-vscode, claude-desktop, claude-desktop-3p, local-agent, remote, remote_desktop, remote_mobile, remote_projects, remote_trigger, remote_cowork, remote_cowork_trigger, remote_baku, claude-in-slack e claude-in-teams; nesta sessao o valor foi cli. *Evidência:* grep no binario ~/.local/share/mise/installs/claude/2.1.288/claude; env da sessao. *Confiança:* media.
- Hooks type http fazem POST do JSON completo do hook; headers aceitam interpolacao de variaveis de ambiente listadas em allowedEnvVars; async nao se aplica a eles; politicas allowedHttpHookUrls e httpHookAllowedEnvVars podem restringi-los. *Evidência:* https://code.claude.com/docs/en/hooks.md (HTTP hook fields; HTTP response handling; allowlists). *Confiança:* alta.
- claude agents --json e a forma suportada de ler o estado das sessoes de fora do Claude Code. Campos: cwd, kind (interactive ou background), startedAt, pid, status (busy, waiting, idle), waitingFor, sessionId e name. Ja ~/.claude/jobs nao e uma interface estavel. *Evidência:* https://code.claude.com/docs/en/agent-view.md (List sessions as JSON; Read session state from a script). *Confiança:* alta.
- Existe ~/.claude/sessions/<pid>.json com pid, sessionId, cwd, entrypoint, name, status e waitingFor; a doc so diz que guarda um arquivo pequeno por sessao em execucao, sem definir formato. *Evidência:* ~/.claude/sessions/1713473.json (lido); https://code.claude.com/docs/en/claude-directory.md. *Confiança:* media.
- Mods (v2.1.287+, ligados por padrao) sao handlers JS dentro do Claude Code com $.http.fetch, $.process.run, $.env e eventos turn/session; rodam no CLI, no Desktop, na extensao do VS Code e em -p; orgs podem restringir com allowManagedModsOnly. *Evidência:* https://code.claude.com/docs/en/plugins/mods/overview.md; https://code.claude.com/docs/en/plugins/mods/api.md; mods/reference.md. *Confiança:* alta.
- A fonte de plugin command do marketplace roda um comando, via sh ou cmd.exe no Windows, que imprime o diretorio do plugin; o modo link e recusado no Windows (usar copy); admins podem desligar com disableCommandPluginSources. *Evidência:* https://code.claude.com/docs/en/plugins/marketplace-reference.md (command plugin source). *Confiança:* alta.
- O dicionario AppleScript do Terminal.app tem tab.tty (somente leitura), tab.selected (leitura e escrita) e, na janela, index, frontmost e selected tab. *Evidência:* Copia do Terminal.sdef em github.com/JXA-userland/JXA packages/@jxa/types/tools/sdefs/Terminal.sdef. *Confiança:* alta.
- iTerm2: o AppleScript esta marcado como Deprecated mas expoe tty e unique id da sessao, e select em janela, aba e sessao. A Python API tem Session.async_activate(select_tab, order_window_front). O ITERM_SESSION_ID tem o formato wXtYpZ:UUID. *Evidência:* https://iterm2.com/documentation-scripting.html; https://iterm2.com/python-api/session.html; gnachman/iTerm2 it2cli APIClient.swift normalizeSessionId. *Confiança:* alta.
- Ghostty: o AppleScript chegou na 1.3.0 (terminal com id, name e working directory; comando focus). As propriedades tty e pid entraram no main em 2026-04-20 e nao estao na v1.3.1; o usuario pode desligar com macos-applescript = false. *Evidência:* https://ghostty.org/docs/features/applescript; ghostty-org/ghostty macos/Ghostty.sdef (main vs v1.3.1); commit 9a9002202. *Confiança:* alta.
- Warp exporta WARP_FOCUS_URL (esquema://session/<uuid-hex>) e WARP_TERMINAL_SESSION_UUID para os processos; abrir a URL foca a sessao exata. *Evidência:* warpdotdev/warp crates/warp_terminal/src/focus_env.rs; uso em PeonPing/peon-ping e 777genius/agent-notifications. *Confiança:* alta.
- kitty: KITTY_WINDOW_ID e KITTY_LISTEN_ON no env; kitten @ focus-window --match aceita id, pid, cwd etc.; controlar de fora exige allow_remote_control e listen_on; a implementacao chama set_active_window(switch_os_window_if_needed=True). WezTerm: wezterm cli activate-pane --pane-id, cujo padrao vem de WEZTERM_PANE. *Evidência:* https://sw.kovidgoyal.net/kitty/remote-control/; kitty/rc/focus_window.py; https://wezterm.org/cli/cli/activate-pane.html. *Confiança:* alta.
- macOS 14: activateIgnoringOtherApps esta deprecated e sem efeito; activate(from:options:) e yieldActivation(to:) implementam a ativacao cooperativa, e o pedido pode ser negado. *Evidência:* Apple docs JSON: appkit/nsapplication/activationoptions/activateignoringotherapps; nsrunningapplication/activate(from:options:); nsapplication/yieldactivation(to:). *Confiança:* alta.
- Enviar Apple Events a outros apps pede autorizacao do usuario (NSAppleEventsUsageDescription) e, com Hardened Runtime, o entitlement com.apple.security.automation.apple-events. *Evidência:* Apple docs: bundleresources/information-property-list/nsappleeventsusagedescription; entitlements/com.apple.security.automation.apple-events. *Confiança:* alta.
- SetForegroundWindow so funciona se, entre outras condicoes, o processo chamador recebeu o ultimo evento de input, e o primeiro plano ou foi iniciado por ele. AllowSetForegroundWindow so funciona se quem chama ja pode trocar o primeiro plano, e mesmo cumprindo as condicoes o pedido pode ser negado. *Evidência:* https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setforegroundwindow; nf-winuser-allowsetforegroundwindow. *Confiança:* alta.
- Num pseudoconsole, GetConsoleWindow devolve uma janela so para fila de mensagens; o WT define o dono dessa pseudo-janela como a janela que hospeda a aba (ConptyReparentPseudoConsole -> SetWindowLongPtrW GWLP_HWNDPARENT, GH#2988), inclusive ao mover a aba de janela e no defterm handoff. *Evidência:* MicrosoftDocs/Console-Docs getconsolewindow.md; microsoft/terminal src/winconpty/winconpty.cpp, src/interactivity/base/InteractivityFactory.cpp, src/cascadia/TerminalConnection/ConptyConnection.cpp. *Confiança:* alta.
- No Windows o hook nasce com um console oculto proprio (CREATE_NO_WINDOW); FreeConsole seguido de AttachConsole no console do Claude ancestral atinge o pane ConPTY certo (um BEL escrito ali marca a aba de origem no WT). *Evidência:* 777genius/agent-notifications docs/CLICK_TO_FOCUS.md (Terminal bell); AttachConsole em MicrosoftDocs/Console-Docs. *Confiança:* media.
- wt.exe: -w aceita o ID ou o nome da janela (0/last, -1/new) e focus-tab aceita -t <indice>; nao ha comando para mirar por sessao/WT_SESSION. *Evidência:* https://learn.microsoft.com/en-us/windows/terminal/command-line-arguments. *Confiança:* alta.
- O time do WT fechou Focus/Activate Tab by WT_SESSION (#19783, not planned, 2026-01-28) e nao mergeou a PR #19784 (focus-tab --session), preferindo OSC 777. O WT Preview 1.26.2734.0 (2026-10-02) traz OSC 777 (compatibility.allowOSC777) e bellStyle notification, ambos opt-in; a infraestrutura de toast acha a aba e o pane de origem. *Evidência:* github.com/microsoft/terminal issues 19783 e 19784, PRs 20010 e 20011, release v1.26.2734.0. *Confiança:* alta.
- O TabViewItemAutomationPeer do WinUI implementa ISelectionItemProvider.Select(); o claude-code-notify (MIT, Rust windows 0.61) troca a aba do WT gravando o RuntimeId do TabItem selecionado no UserPromptSubmit e chamando Select() no clique; a classe da janela do WT e CASCADIA_HOSTING_WINDOW_CLASS. *Evidência:* microsoft/microsoft-ui-xaml controls/dev/TabView/TabViewItemAutomationPeer.h; chuilishi/claude-code-notify src-rust/src/uiautomation.rs e activate.rs; microsoft/terminal IslandWindow.cpp. *Confiança:* alta.
- peon-ping: no macOS ativa com NSRunningApplication.activateWithOptions e foca a aba do iTerm2 por tty via JXA + AXRaise; resolve o tmux no clique via list-clients e ancestrais (os IDs de env ficam congelados). No Windows sobe parentPid ate MainWindowHandle e usa AttachThreadInput + SetForegroundWindow, so janela. Relata que o AXRaise nao reordena janelas Electron multiplas. *Evidência:* PeonPing/peon-ping scripts/local-focus.sh, scripts/mac-overlay.js, scripts/win-notify.ps1, docs/plans/2026-02-20-multi-window-focus.md. *Confiança:* alta.
- agent-notifications documenta: no WT so janela; no macOS AX + Screen Recording; no Linux extensao do GNOME, wlrctl, kdotool ou xdotool; zellij focus-pane-id desde a 0.44.1; o hook exec form chama sh com wrapper que baixa o binario sob demanda. *Evidência:* 777genius/agent-notifications docs/CLICK_TO_FOCUS.md, hooks/hooks.json, bin/hook-wrapper.sh. *Confiança:* alta.
- code-notify usa terminal-notifier -activate com o bundle id derivado de TERM_PROGRAM (so app) e, no WSL, wsl-notify-send com o AppId do WT; CCNotify usa -execute code <cwd>; o terminal-notifier 3.x documenta -activate BUNDLE_ID como Bring an application to the front. *Evidência:* mylee04/code-notify lib/code-notify/core/notifier.sh e click-through-store.sh; dazuiba/CCNotify ccnotify.py; julienXX/terminal-notifier README (3.0.0 e 3.1.0, ago/2026). *Confiança:* alta.
- MioIsland foca o iTerm2 por tty (AppleScript), o Ghostty por every terminal whose working directory contains, o kitty com @ focus-window --match cwd: e o tmux com yabai. *Evidência:* MioMioOS/MioIsland ClaudeIsland/Services/Window/TerminalJumper.swift; Resources/codeisland-state.py. *Confiança:* alta.
- No Hyprland 0.56.2 o KeybindManager ainda registra focuswindow pelo tradutor legado; a wiki atual documenta focus({ window }) com seletores exatos pid:, stableid: e address:0x. *Evidência:* hyprwm/Hyprland v0.56.2 src/managers/KeybindManager.cpp; https://wiki.hypr.land/Configuring/Basics/Dispatchers/. *Confiança:* media.
- Verificado localmente: claude (pid 1713473, pts/0) -> bash -> foot (4562) -> Hyprland; hyprctl -j clients liga o pid 4562 a address 0x5bbf4e6128f0; /proc/<CLAUDE_PID>/environ e legivel pelo mesmo usuario. *Evidência:* ps, readlink /proc/1713473/fd/0, hyprctl -j clients nesta maquina. *Confiança:* alta.
- sway aceita o criterio pid (Compare value against the window's process ID). No X11, _NET_ACTIVE_WINDOW com source indication 2 (pager) pede ativacao e o WM pode recusar. O wlr-foreign-toplevel-management tem activate sem PID. *Evidência:* man sway(5) em man.archlinux.org; EWMH spec specifications.freedesktop.org/wm/latest/ar01s03.html; wayland.app/protocols/wlr-foreign-toplevel-management-unstable-v1. *Confiança:* alta.
- WSL2 em NAT (padrao): apps do Windows nao sao alcancaveis por localhost a partir do Linux (use o IP do host e bind em 0.0.0.0). No modo mirrored (Windows 11 22H2+) 127.0.0.1 funciona nos dois sentidos. *Evidência:* MicrosoftDocs/WSL WSL/networking.md. *Confiança:* alta.
- Konsole exporta KONSOLE_DBUS_WINDOW=/Windows/N e o ViewManager tem o slot D-Bus setCurrentSession(int). *Evidência:* KDE/konsole src/ViewManager.h e src/ViewManager.cpp. *Confiança:* media.
- Em shell form o Git Bash pode carregar o perfil do usuario, e um echo no perfil corrompe o JSON de saida do hook. *Evidência:* https://code.claude.com/docs/en/hooks-guide.md. *Confiança:* alta.

### Recomendações

1. Trocar sh+jq+curl por um helper nativo em exec form, com o mesmo hooks.json nos 3 SOs: command claude-pet, args [avisar, <Evento>], async true. O helper le o stdin com serde_json, aplica a lista branca atual, faz POST em 127.0.0.1 com timeout proprio (conexao ~300 ms, total <=2 s), nunca imprime nada e sempre sai com 0.
2. Deixar o SessionEnd sincrono com timeout 1, porque o orcamento e de 1,5 s e timeouts de plugin nao o aumentam; reconciliar a lista de sessoes com claude agents --json para cobrir eventos perdidos.
3. Usar CLAUDE_PID como ancora (e nao $PPID, que no Git Bash e um PID do MSYS). Mandar a identidade opaca so no SessionStart e no UserPromptSubmit: CLAUDE_PID, tty (Unix) e IDs de env (TERM_PROGRAM, ITERM_SESSION_ID, KITTY_WINDOW_ID, KITTY_LISTEN_ON, WEZTERM_PANE, WARP_FOCUS_URL, TMUX, TMUX_PANE, ZELLIJ_*, WT_SESSION, KONSOLE_DBUS_*); no Windows mandar tambem o HWND do WT e o RuntimeId da aba.
4. Fazer o foco no processo do pet, no handler do clique, em camadas: aba ou pane exato, depois janela, depois app, por fim so a bolha. E onde o Windows permite SetForegroundWindow, onde o macOS 14+ permite ativacao cooperativa e onde o TCC atribui o prompt ao Zeca.
5. Distribuir o plugin pelo GitHub (marketplace) com hooks.json generico chamando claude-pet pelo nome; o instalador do app (Homebrew, winget/MSI, pacotes Linux, cargo) coloca claude-pet no PATH. Como alternativa, oferecer a fonte command (claude-pet claude-plugin-path, modo copy) que gera hooks com caminho absoluto por SO. Nao usar bin/ no topo do plugin.
6. Windows, janela: no helper fazer FreeConsole, AttachConsole(CLAUDE_PID), GetConsoleWindow e GetAncestor(GA_ROOTOWNER). Se nao houver dono (VS Code, outros), subir pela arvore com Toolhelp32 conferindo o horario de criacao, enumerar as janelas do processo GUI e desempatar pelo titulo (pasta do projeto).
7. Windows, aba: copiar o padrao do claude-code-notify (MIT). No UserPromptSubmit/SessionStart, se GetForegroundWindow() for a janela do WT vinda do AttachConsole, gravar o RuntimeId do TabItem selecionado; no clique, ShowWindow(SW_RESTORE) se minimizada, SetForegroundWindow e SelectionItemPattern.Select(). Opcional: hook sincrono que devolve terminalSequence BEL ou OSC 777 para quem usa WT 1.26+ (toast nativo que foca a aba certa).
8. macOS: distribuir como .app assinado e notarizado com NSAppleEventsUsageDescription e o entitlement com.apple.security.automation.apple-events; AppleScript por terminal (Terminal.app e iTerm2 por tty ou unique id, Ghostty por working directory e depois por tty); ativar com activate(from:options:) ou yieldActivation e cair para tell application id ... to activate. Pedir Accessibility so se for usar AXRaise.
9. Linux: manter Hyprland (CLAUDE_PID e ancestrais casados com hyprctl -j clients) e testar o despacho legado focuswindow contra a forma Lua focus({ window }) da 0.56. Somar sway, X11 (_NET_ACTIVE_WINDOW), KDE (KWin por D-Bus) e IPC dos terminais (kitty, WezTerm, tmux, zellij, Konsole, Warp). Para terminais de processo unico, desempatar por IPC ou titulo.
10. Para a bolha de sessoes: usar os hooks como fonte ao vivo e claude agents --json (interface suportada, com pid, status, waitingFor e name) para reconciliar sob demanda. Nao depender do formato de ~/.claude/sessions nem de ~/.claude/jobs.
11. Avaliar mods (v2.1.287+) como transporte v2 sem binario, filtrando dentro do processo e fazendo $.http.fetch no loopback. Nao adotar hooks type http, salvo como modo opcional sem instalar e com aviso de privacidade, porque enviam prompt e codigo inteiros ao loopback.
12. Montar uma matriz de testes antes de anunciar suporte: Windows 10 e 11 (WT 1.24, 1.25 e 1.26; conhost; VS Code; com e sem Git Bash; WSL NAT e mirrored), macOS 14, 15 e 26 (Terminal, iTerm2, Ghostty 1.3, kitty, WezTerm, VS Code; Spaces e tela cheia), Linux (Hyprland, sway, GNOME, KDE, X11).

### Riscos

- **Risco:** Nao esta claro se o exec form no Windows aceita caminho absoluto sem .exe (por exemplo ${CLAUDE_PLUGIN_ROOT}/x/claude-pet); a doc so garante que o .exe precisa existir e que um nome simples no PATH (node) funciona. **Mitigação:** Usar o nome simples no PATH ou a fonte command, que gera caminho absoluto com .exe no Windows; testar em Windows com e sem Git Bash.
- **Risco:** Quando o Claude e iniciado pelo app Desktop ou pela extensao do VS Code (sobretudo no macOS), o PATH pode ser minimo e claude-pet nao ser encontrado. **Mitigação:** Fonte command com caminho absoluto, ou symlink em /usr/local/bin; o helper ausente deve virar no-op silencioso, e o pet mostra no doutor que nao recebe eventos.
- **Risco:** Hooks async podem nao sobreviver ao fim da sessao, ou chegar fora de ordem, e o SessionEnd se perde. **Mitigação:** SessionEnd sincrono com timeout 1, carimbo ts no evento (ja existe) e reconciliacao periodica com claude agents --json.
- **Risco:** terminalSequence pode nao funcionar em hooks async, e so funciona em sessao interativa. **Mitigação:** Se usar o toast nativo do WT, iTerm2, Ghostty ou kitty, separar um hook sincrono minimo que so devolve o JSON com terminalSequence.
- **Risco:** CLAUDE_PID so existe a partir da v2.1.214, e PIDs podem ser reutilizados. **Mitigação:** Fallback para getppid ou ancestrais conferindo o nome (claude, claude.exe) e o horario de criacao; descartar identidade velha no SessionEnd.
- **Risco:** A troca de aba do WT por UI Automation nao e suportada pelo time do WT (que rejeitou focus por sessao); o RuntimeId pode mudar ao mover a aba de janela; panes nao sao selecionaveis; o heuristico do foreground no UserPromptSubmit falha com prompts vindos do Remote Control ou do celular. **Mitigação:** Tratar como melhor esforco: cair para foco de janela; regravar o RuntimeId a cada UserPromptSubmit so quando o foreground for a janela do WT certa; oferecer o toast nativo do WT 1.26 (OSC 777 ou bellStyle notification) como caminho preciso e consentido.
- **Risco:** As restricoes de primeiro plano do Windows: os truques AttachThreadInput e ALT simulado podem travar se a thread do primeiro plano estiver pendurada, ou abrir menus no app atual. **Mitigação:** Chamar SetForegroundWindow direto no handler de clique do pet (recebeu o ultimo input); usar os truques so como fallback, com timeout, e nunca a partir de um processo de hook.
- **Risco:** macOS: a ativacao cooperativa (14+) pode negar o pedido; Spaces e tela cheia atrapalham; o AXRaise nao reordena janelas Electron multiplas; o AppleScript do iTerm2 esta deprecated; cada terminal dispara um prompt de Automation que o usuario pode negar. **Mitigação:** App assinado, com Info.plist e entitlement corretos; tornar o pet ativo no clique antes de ceder a ativacao; degradar para ativar so o app; oferecer a Python API do iTerm2 como opcao; mostrar no doutor o estado das permissoes.
- **Risco:** Ghostty 1.3.x nao expoe tty: o casamento por working directory e ambiguo com varias sessoes na mesma pasta. **Mitigação:** Usar tty quando a versao do Ghostty tiver a propriedade (ja no main); senao desempatar por name (titulo) e, em ultimo caso, so ativar o app.
- **Risco:** Linux: terminais de processo unico com varias janelas (foot server, gnome-terminal-server, kitty single-instance, WezTerm, Ghostty GTK, Konsole) deixam o PID ambiguo; o GNOME Wayland nao tem API de foco; a sintaxe de dispatch do Hyprland esta migrando para Lua. **Mitigação:** Preferir o IPC do terminal (kitty, WezTerm, Konsole, tmux, zellij); titulo como desempate; extensao opcional no GNOME; testar os dois formatos de dispatch no Hyprland.
- **Risco:** WSL: em NAT o pet no Windows nao e alcancavel por 127.0.0.1, e bind em 0.0.0.0 expoe a LAN; os PIDs Linux nao servem para focar no Windows. **Mitigação:** Documentar o modo mirrored como requisito; chamar o claude-pet.exe do Windows a partir do hook no WSL (padrao do claude-code-notify) para que ele faca o AttachConsole e o POST localmente.
- **Risco:** Politicas corporativas podem desligar a integracao: disableAllHooks, allowManagedHooksOnly, allowManagedModsOnly, disableCommandPluginSources e allowedHttpHookUrls. **Mitigação:** O doutor do pet detecta a falta de eventos e explica; nao depender de um unico mecanismo.
- **Risco:** Privacidade e porta: HTTP hooks ou mods sem filtro mandariam conteudo ao loopback; outro processo local pode ocupar a porta 27380 antes do pet. **Mitigação:** Manter o helper com lista branca; opcionalmente segredo por usuario (arquivo 0600 ou credencial do SO) em header; no Windows e no macOS considerar named pipe ou socket Unix com checagem de peer.
- **Risco:** CLAUDE_CODE_ENTRYPOINT e ~/.claude/sessions nao sao documentados e podem mudar. **Mitigação:** Usar so como dica, com default seguro (tratar como cli); para listar sessoes usar claude agents --json.
- **Risco:** Fora deste tema mas bloqueante: a arte atual vem de um pacote pago que proibe redistribuicao, e o visual pode lembrar um personagem de terceiros e trazer risco de marca. **Mitigação:** Arte original no repositorio aberto e o pacote pago so como skin local do usuario (o sistema de skins-locais ja separa isso); pesquisa juridica separada.

## Achado 4 — Distribuição, assinatura e licença da arte

*Zeca para todo mundo: distribuição open source (Linux, macOS, Windows), licença da arte e risco de associação*

Pesquisa só de leitura, feita em 2026-10-03. Conferi as fontes primárias nesta sessão (lista no fim). Onde não deu para conferir, eu aviso. **Não é aconselhamento jurídico.**

Limite desta sessão: o orçamento de WebSearch já tinha acabado. Usei só WebFetch em URLs conhecidas. Fiverr, Upwork, Reddit, Arch Wiki/AUR e planalto.gov.br bloquearam ou recusaram o acesso.

### Respostas curtas

- **Windows dá? Sim, na parte de distribuição.** O `dist` gera zip, instalador PowerShell e MSI para `x86_64-pc-windows-msvc`, e dá para publicar no winget e no Scoop. O trabalho pesado está em dois outros pontos:
  - **Portar a camada de plataforma.** Hoje `crates/claude-pet/src/wl/` só funciona com Wayland e layer-shell. macOS e Windows precisam de uma janela overlay própria; o `pet-core`, que já é puro, ajuda.
  - **Assinar o binário.** Sem assinatura, o Windows mostra "O Windows protegeu o seu PC". Onde o **Smart App Control** estiver ligado, o app sem assinatura é bloqueado e não existe exceção por app.
  - A opção barata da Microsoft (Artifact Signing, US$ 9,99/mês) **só aceita pessoa física nos EUA e no Canadá**. Para o Renan, no Brasil, sobram: SignPath Foundation (grátis, com condições), Certum Open Source (a partir de € 69), certificado OV (US$ 150–300/ano) ou Microsoft Store com MSIX (grátis).
- **Caminho de custo mínimo (US$ 0):**
  - repositório público com `dist`: GitHub Releases, `curl | sh`, PowerShell, tap do Homebrew e atestados do GitHub;
  - pacote AUR `-bin`;
  - plugin `bichinho` num marketplace dentro do próprio repositório.
  - No macOS, quem instala por `curl | sh` ou pela fórmula do Homebrew não recebe quarentena, então o Gatekeeper não pergunta nada. Quem baixa pelo navegador precisa clicar em "Abrir Mesmo Assim".
  - Os US$ 99/ano da Apple passam a compensar quando o Zeca precisar de permissão de Acessibilidade ou Automação (para focar o terminal). Com assinatura ad-hoc, o macOS pede essa permissão de novo a cada atualização.
- **Arte: o Zeca de hoje é derivado do pack e não pode entrar no repositório público.** Há três saídas:
  - **(a) "Traga seu pack".** Cada usuário compra o pack (US$ 0,50) e o próprio binário monta o Zeca na máquina dele. Isso bate com a leitura literal da licença e tem precedente: o DevilutionX exige que o usuário traga o `DIABDAT.MPQ` do jogo comprado.
  - **(b) Encomendar um papagaio original** em CC BY 4.0 ou CC0 para ser o personagem público.
  - **(c) Pedir à exclusiveOlive, por escrito,** licença para embutir o Zeca nos binários oficiais, sem arquivos soltos no repositório. O modelo de mensagem está na seção 2.4.
  - Não encontrei nenhum papagaio CC0 pronto com as ~20 animações.
- **Associação com personagens de terceiros: o risco está na combinação, não nos elementos isolados.** Papagaio verde, chapéu-palheta, gravata-borboleta, malandro e o nome "Zeca", cada um sozinho, não são protegidos (seção 3).
  - Textos que citem um personagem de terceiros como referência de estilo são o agravante concreto; os do `DECISIONS.md` e do `PLANO.md` já foram reescritos (decisão 0035).
  - A pesquisa recomendou outro nome público; o Renan decidiu manter o Zeca (decisão 0035).

### 1. Distribuição

#### 1.1 `dist` (ex-cargo-dist)

**Está ativo:**
- v0.33.0 em 2026-09-11 (assinatura Windows por Azure Artifact Signing, só x86_64);
- v0.32.0 em 2026-05-21;
- v0.31.0 em 2026-02-23.

**O que ele gera:**
- o workflow do GitHub Actions;
- os arquivos por alvo, com SHA256;
- instaladores:
  - `shell` (`curl | sh`);
  - `powershell` (`irm | iex`);
  - `npm`;
  - `homebrew`: gera uma **fórmula** (não cask) num tap seu e exige o segredo `HOMEBREW_TAP_TOKEN`;
  - `msi` (WiX v3; o v4 ainda não é suportado).

**Extras:**
- `install-updater = true` instala o `claude-pet-update` (axoupdater). Só funciona nos instaladores shell e PowerShell.
- `github-attestations = true` liga os atestados. Exige repositório público (ou o plano Enterprise); o usuário confere com `gh attestation verify`.

**O que ele não gera:** `.deb`, `.rpm`, AppImage, `.dmg`/`.app`, cask, winget, Flatpak e imagem Docker. Todos estão na lista de pedidos do projeto.

**Assinatura:**
- Windows: pelo SSL.com eSigner ou pelo Azure Artifact Signing.
- macOS: codesign **experimental**, configurado por variáveis de ambiente. **Não há notarização** no changelog nem na referência de configuração; ela vira um passo extra no CI (`notarytool`).

**Atenção:** `crates/claude-pet/Cargo.toml` tem `publish = false`. O dist trata isso como "não distribuir", então é preciso pôr `dist = true` em `[package.metadata.dist]`.

Configuração ilustrativa (gere a real com `dist init`):

```toml
[dist]
cargo-dist-version = "0.33.0"
ci = "github"
targets = ["x86_64-unknown-linux-musl", "aarch64-unknown-linux-musl", "aarch64-apple-darwin", "x86_64-apple-darwin", "x86_64-pc-windows-msvc"]
installers = ["shell", "powershell", "homebrew"]   # "msi" depois do porte para Windows
tap = "<usuario>/homebrew-tap"
publish-jobs = ["homebrew"]
install-updater = true
github-attestations = true
```

**Alternativa se o macOS pedir um bundle `.app`** (ícone, item de login, nome legível no pedido de permissão): o **cargo-packager** (CrabNebula).
- Gera `.app` e `.dmg`, NSIS e MSI, `.deb`, AppImage e pacman.
- Aceita identidade Developer ID e notarização no macOS, e `signCommand` no Windows.
- A última versão é a 0.11.8, de 2025-11-27; o projeto anda mais devagar que o dist.

#### 1.2 macOS

| Situação | O que acontece |
|---|---|
| Binário arm64 "sem assinatura" | Não existe. No Apple Silicon todo executável precisa de assinatura, mas ad-hoc basta, e o linker já assina sozinho. |
| Instalado por script curl ou pela fórmula do Homebrew | A quarentena é posta pelo app que baixa, e só se ele optar por isso (`LSFileQuarantineEnabled`). curl e as fórmulas não põem, então o Gatekeeper não pergunta (média confiança: é uma inferência). |
| Baixado pelo navegador, sem notarização | Desde o macOS Sequoia, o atalho Ctrl-clique > Abrir não funciona mais. O usuário vai em Ajustes > Privacidade e Segurança > "Abrir Mesmo Assim", e o app fica salvo como exceção. Também dá para usar `xattr -d com.apple.quarantine`. |
| Cask no repositório oficial do Homebrew | O Homebrew 5.0 (nov/2025) depreciou casks sem assinatura e marcou para **setembro de 2026** o desligamento dos casks oficiais que falham no Gatekeeper. Também depreciou `--no-quarantine`. Uma fórmula num tap próprio (o que o dist gera) não é afetada. |
| Tap de terceiros | O Homebrew 6.0 (jun/2026) exige confiança explícita no tap. Instalar pelo nome completo (`brew install <usuario>/tap/<formula>`) confia só naquela fórmula. |
| Permissões TCC (Acessibilidade/Automação para o clique que foca o terminal) | Na assinatura ad-hoc, o requisito de identidade fica preso àquela versão exata, e o macOS pede a permissão de novo depois de cada mudança (TN3127). Com Developer ID, a permissão sobrevive às atualizações. |
| Notarização | Exige o Apple Developer Program (US$ 99/ano; a isenção não vale para pessoa física), Developer ID, hardened runtime e timestamp seguro. Um binário solto pode ser notarizado dentro de um zip, mas o ticket **não pode ser grampeado** nele nem no zip, só em `.pkg`, `.dmg` ou `.app`. |
| Macs Intel | O Homebrew 7.0 (set/2026) pôs o macOS Intel no Tier 3, com remoção em 2027-09-01. Dá para compilar x86_64 de forma cruzada num runner arm64. |

**Recomendação para o macOS:**
- Comece sem pagar, distribuindo por curl ou pela fórmula do Homebrew.
- Pague os US$ 99/ano quando a ação de focar o terminal chegar ao macOS (por causa do TCC), ou quando quiser oferecer um `.pkg`/`.dmg` para quem baixa pelo navegador.

#### 1.3 Windows

**Como o SmartScreen funciona:**
- Ele avalia o **editor** (o certificado) e o **hash** do arquivo.
- Sem assinatura: aparece "O Windows protegeu o seu PC" com a opção "Executar assim mesmo", e a reputação recomeça do zero a cada versão.
- Com assinatura: o aviso continua aparecendo no começo, mas mostra o nome do editor, e a reputação vai passando para as versões seguintes. A Microsoft fala em "semanas e centenas de instalações limpas".
- O certificado EV **não pula mais o SmartScreen** (isso mudou em 2024).

**Smart App Control (Windows 11):**
- Bloqueia executáveis sem assinatura quando a nuvem não tem uma previsão confiável.
- Vale para qualquer executável, não só os baixados da internet, e não há exceção por app.
- Atualizações recentes permitem religá-lo sem reinstalar o Windows.

**Opções de assinatura para o Renan (pessoa física, no Brasil):**

| Opção | Custo | Serve? |
|---|---|---|
| Azure Artifact Signing | US$ 9,99/mês no plano Basic (5.000 assinaturas); Premium US$ 99,99 | **Não serve.** Pessoa física só nos EUA e no Canadá. Organizações só em EUA, Canadá, UE, Reino Unido, Austrália, Nova Zelândia, Japão, Coreia do Sul, Singapura, Suíça, Noruega e Israel. Também não aceita assinatura Azure gratuita ou de teste. |
| SignPath Foundation | grátis | Serve, se cumprir as condições: licença OSI em **todos** os componentes e nada proprietário (o binário assinado não pode embutir o Zeca do pack), projeto ativo e já lançado, MFA, papéis de autor, revisor e aprovador, e uma página com a política de assinatura. O editor que aparece é "SignPath Foundation". |
| Certum Open Source Code Signing | a partir de € 69 (certificado, cartão criptográfico e leitora) | Serve. O nome sai como "Open Source Developer, <nome>". A chave fica num cartão físico, então a assinatura é feita localmente. |
| Certificado OV comercial | US$ 150–300/ano | Serve. Desde 2023 a chave precisa estar em HSM ou token. Certificados emitidos a partir de 2026-03-01 valem no máximo 460 dias. |
| Certificado EV | US$ 400+/ano | Não vale a pena: não pula mais o SmartScreen. |
| Microsoft Store (MSIX) | grátis (conta individual gratuita) | Serve. A Microsoft reassina o pacote e não há aviso do SmartScreen. Exige empacotar em MSIX e passar na certificação da loja. |

**Canais de instalação:**
- zip com instalador PowerShell e MSI, ambos pelo dist;
- winget: a política do repositório não exige assinatura, mas o `InstallerUrl` precisa ser o release do próprio autor;
- Scoop.

Scripts como `irm | iex` normalmente não marcam o arquivo com Mark of the Web, então o aviso de download costuma não aparecer (média confiança). Isso não vale para o Smart App Control, que continua checando.

**Hooks no Windows:**
- O Claude Code roda os hooks no Git Bash, se ele estiver instalado; senão, no PowerShell.
- O `~/Documents/claude-pet/plugin/scripts/avisar.sh` depende de `sh`, `curl` e `jq`. O Git for Windows traz `sh` e `curl`, mas não traz `jq`. Sem `jq`, o hook cai no payload mínimo, sem os ids de sessão. Sem Git Bash, ele não roda.
- **Solução portátil:** um hook na forma exec que chama o próprio binário, por exemplo `command: claude-pet` com `args: [hook, Stop]`, e a lista branca escrita em Rust. No Windows a forma exec exige um `.exe` de verdade, e o binário é um.
- Existem também hooks do tipo `http`, mas eles fazem POST do evento JSON **inteiro**, não aceitam `async` e esperam a resposta. Isso contraria a regra do projeto de mandar só metadados.

#### 1.4 Linux

- **Artefato principal:** o binário estático musl (x86_64 e aarch64) em tar, com `curl | sh` e uma unit `systemd --user`. O Docker passa a ser opcional.
- **AUR:** pacote `-bin` apontando para o release (grátis).
- **`.deb` e `.rpm`:** ficam fora do dist; dá para gerar com `cargo-deb`, `cargo-generate-rpm` ou nFPM num job à parte.
- **AppImage:** acrescenta pouco a um binário estático.
- **Flatpak:** encaixa mal, por causa da sandbox com o layer-shell, do socket do Hyprland e da porta no loopback.
- **Imagem no GHCR:** grátis para pacote público, mas nunca com `skins-locais`. O `~/Documents/claude-pet-m2/Dockerfile` copia `skins-locais/`; no CI público a pasta chega vazia, mas um push feito localmente vazaria a arte.
- **Escopo honesto para o README:** o overlay depende do `wlr-layer-shell`.
  - Funciona em Hyprland, KDE KWin, niri, Sway, river, labwc, Wayfire, Mir, Treeland e Jay.
  - **Não funciona no GNOME (Mutter) nem no Weston.**
  - Seguir o monitor focado hoje usa o socket de eventos do Hyprland.

#### 1.5 Plugin do Claude Code

- **Publicação:** o marketplace é o arquivo `.claude-plugin/marketplace.json` dentro do próprio repositório.
  - O usuário roda `/plugin marketplace add <dono>/<repo>` e depois `/plugin install bichinho@<marketplace>`.
  - Também dá para submeter o plugin ao diretório da Anthropic.
- **Atualizações:**
  - O auto-update de marketplace de terceiros vem **desligado**. O usuário liga em `/plugin` > Marketplaces, ou roda `/plugin marketplace update`.
  - Suba o campo `version` a cada release.

#### 1.6 Atualização automática

Em ordem de preferência:
1. O gerenciador de pacotes (brew, yay, winget, scoop).
2. O `claude-pet-update` do dist, para quem instalou por script.
3. No máximo, um aviso de "versão nova" que consulta a API de Releases, opcional e desligável.

Trocar o binário sozinho no macOS sem Developer ID reabre os pedidos de permissão TCC.

#### 1.7 Builds reprodutíveis e proveniência

**O que já ajuda:** imagens base fixadas por digest, `--locked`, `strip` e `panic=abort`.

**O que falta:**
- Fixar a versão exata do Rust no CI. O `~/Documents/claude-pet-m2/rust-toolchain.toml` diz `stable` por causa da decisão 0015; no CI, use a mesma 1.98.1 da imagem.
- Usar `--remap-path-prefix` para tirar `$HOME` e o caminho do repositório de dentro do binário. O `trim-paths` do Cargo ainda é instável.
- Fixar os timestamps dentro do tar e do zip.
- Conferir: dois builds devem dar o mesmo SHA256.

**Proveniência:** os atestados do GitHub dão SLSA v1.0 Build Level 2, registrados no Sigstore público. A assinatura de código no macOS e no Windows muda os bytes, então compare antes de assinar.

#### 1.8 Plano e custos

| Fase | O quê | Custo |
|---|---|---|
| 1 | Linux (Hyprland primeiro): repositório público novo, dist, AUR `-bin`, marketplace do plugin, skin padrão livre e o comando `skin instalar` | US$ 0 |
| 2 | Porte para macOS, distribuindo sem notarização por curl e fórmula do Homebrew | US$ 0 |
| 3 | Porte para Windows: um release sem assinatura para os primeiros usuários; depois SignPath (grátis) ou Certum | € 0 a 69+ |
| 4 | macOS com Developer ID e notarização, quando o TCC ou um `.dmg` ficarem necessários | US$ 99/ano |

O GitHub Actions é grátis em repositório público com os runners padrão (inclusive macOS e Windows). O GHCR é grátis para pacotes públicos.

### 2. Licença da arte

#### 2.1 O que a licença diz

Texto da página do pack, conferido hoje:
- "These assets can be used in both commercial and non-commercial projects."
- "These assets can be edited."
- "These assets cannot be redistributed or resold, even if the assets have been edited."
- "Credit is appreciated, but it is not necessary :)"

Outros dados da página:
- preço de US$ 0,50 ou mais; quadros de 48×48; 21 animações;
- **não há contato** da artista nem na página nem no perfil;
- os comentários estão vazios e o pack tem 1 avaliação.

**Como ler a cláusula:**
- Em packs do itch.io, a leitura comum é: pode distribuir o asset **dentro** do seu projeto, mas não pode redistribuir os **arquivos como asset**. Outro autor do itch.io escreve isso explicitamente: "cannot be resold or redistributed as a standalone game asset; must be integrated into a project".
- Um repositório open source, porém, publica os PNG e JSON de forma extraível. Isso é redistribuir o asset.
- Conclusão: a arte nunca vai para o repositório. Embutir no binário oficial só com confirmação por escrito da artista.

#### 2.2 Opções

| Opção | A favor | Contra |
|---|---|---|
| A. Traga seu pack | Já existe (`bin/pet skin-instalar`); respeita a licença; o Zeca fica idêntico ao de hoje | Cada usuário compra e baixa o pack à mão; depende de o pack continuar à venda e com o mesmo formato; o visual continua carregando o risco de associação |
| B. Licença para embutir nos binários oficiais | Todo mundo recebe o Zeca sem esforço | Depende de a artista responder; o binário deixa de ser 100% livre (o SignPath e as distros recusam); forks não herdam a permissão |
| C. Licença para pôr a folha de sprites no repositório (CC BY) | Tudo fica livre | É o pedido mais difícil de a artista aceitar |
| D. Papagaio original encomendado (CC BY 4.0 ou CC0) | Um personagem público só seu; afasta também o risco de associação | Tem custo e leva semanas |
| E. Skin CC0 como padrão | Grátis e imediata | Não existe papagaio CC0 com o vocabulário que o pet precisa |

**Recomendação:** A e E agora, D como meta, e B como pedido em paralelo (perguntar não custa nada).

#### 2.3 "Traga seu pack" é aceitável?

**Pela leitura literal, sim.**
- Quem compra o pack é o licenciado e edita para o próprio uso; nada derivado sai da máquina dele.
- O repositório só contém: código, as grades do chapéu e da gravata (nossas, MIT), códigos de cor em hex e coordenadas de encaixe.
- Conferi o histórico de todas as branches: nenhum arquivo do pack foi commitado; só há imagens de teste e a skin xadrez.
- O DevilutionX segue o mesmo desenho (exige que o usuário traga o `DIABDAT.MPQ`).

**O que precisa mudar para o lançamento:**
1. Levar o montador do Zeca para o binário distribuído, como `claude-pet skin instalar <zip>`. Hoje ele está em `~/Documents/claude-pet/xtask/src/zeca`, e o usuário final não tem o repositório nem o toolchain.
2. Nunca baixar o pack automaticamente do itch.io.
3. Validar o formato do pack e falhar com uma mensagem clara se ele mudar.
4. Mostrar só a skin livre no README e nas páginas de release. Foto ou GIF do Zeca, só com permissão da artista.
5. Manter os créditos e o link para o pack.

#### 2.4 Pedido à exclusiveOlive

**Canal:** o itch.io não tem mensagem privada e a artista não lista contato. O caminho é um comentário público na página do pack, oferecendo conversar em privado.

**O que pedir por escrito:**
- quais arquivos (o Parrot 2 editado nas duas variantes, com os nossos acessórios);
- onde podem ser distribuídos (GitHub Releases, Homebrew, AUR, winget, Docker);
- se a permissão vale para forks e empacotadores;
- que seja perpétua e irrevogável;
- o texto do crédito;
- o preço e o recibo;
- se pode haver capturas de tela e GIFs no README.

**Desfechos possíveis** (não há dado público sobre isso; baixa confiança):
- silêncio, que é comum em criadores pequenos sem contato listado (silêncio não é permissão);
- "sim, dentro do app", o mais provável entre as respostas positivas;
- "sim, no repositório", mais raro;
- pedido de pagamento;
- "não".

**Modelo da mensagem:**

> Hi exclusiveOlive! I bought Cute Parrots! and turned Parrot 2 into a small desktop pet that reacts to my coding sessions (I drew my own hat and bow tie on top). I'd like to release the app for free as open source (the code is MIT), and I want to respect your licence, so I'm asking before publishing:
> 1. May the official release builds include the edited parrot sprites inside the program? They would not be loose image files in the public source repository, and never offered as a standalone asset.
> 2. Would you consider licensing the edited sprite sheet so it can live in the public repository (for example CC BY 4.0 with credit to you), or under terms you prefer? I'm happy to pay for an extended licence; just tell me your price.
> 3. May I show screenshots/GIFs of the pet on the project page?
> If none of this is OK, no problem: I'll keep the current setup, where each user buys your pack and the app builds the skin on their own computer, and the project only links to your page. Either way I'll credit you and link to the pack. If you prefer to answer privately, tell me where to write. Thanks for the lovely art!

**Em português:** a mensagem pede para (1) embutir nos binários, (2) eventualmente pôr no repositório com CC BY, pagando se for o caso, e (3) mostrar capturas. Se nada disso for possível, mantém-se o "traga seu pack".

#### 2.5 Encomendar um papagaio original

**Escopo:**
- A folha do Zeca (`~/Documents/claude-pet-m2/skins-locais/zeca/sheet.json`) usa 20 animações nativas com **92 quadros**, que viram 166 células com as compostas.
- O mínimo que cobre os estados do pet fica em uns **55–60 quadros**: parado, sentar e levantar, piar, comer, dormir e acordar, decolar, voar, planar, pousar e susto.

**Preço — estimativa de baixa confiança:**
- Fiverr, Upwork e Reddit bloquearam a consulta.
- Os únicos dados públicos que consegui são do PixelJoint em 2026: um artista anunciando US$ 12/h, e outro cobrando de US$ 10 a 25 por personagem estático.

| Faixa | Por quadro (48×48) | ~55 quadros | ~92 quadros |
|---|---|---|---|
| Iniciante ou marketplace | US$ 3–8 | US$ 165–440 | US$ 280–740 |
| Indie experiente | US$ 10–25 | US$ 550–1.400 | US$ 920–2.300 |
| Profissional conhecido | US$ 30–60 | US$ 1.650–3.300 | US$ 2.760–5.500 |

Some o desenho do personagem. Licença CC0 ou cessão de direitos pode custar mais. Peça três orçamentos.

**Onde procurar:**
- fóruns "Job Offerings" e "Portfolios/Resumes" do PixelJoint (ativos em setembro de 2026);
- r/HungryArtists (as regras não foram conferidas);
- Fiverr e VGen;
- artistas do itch.io que tenham link para encomendas;
- artistas brasileiros (pagamento por Pix, e eles entendem o malandro sem que seja preciso citar referência nenhuma).

**O que pôr no briefing e no contrato:**
- **Briefing:** não citar personagem nem estúdio de terceiros, e não mandar o pack pago como referência. Mande só a lista de animações, as durações, células de 48×48, a paleta travada e o contorno de 1 px escuro.
- **Entregas:** o `.aseprite` com tags, o PNG e o JSON no formato json-array.
- **Licença:** por escrito, na fatura (CC BY 4.0 ou CC0).
- **Garantias:** obra original, sem IA generativa.
- **Etapas:** desenho do personagem, depois poses-chave, depois as animações; de 30% a 50% adiantado; número de revisões combinado.

#### 2.6 Skins livres para usar como padrão

- **MoikMellah, "Animated Birds (32x32)"** (OpenGameArt, CC0): um papagaio com só 5 quadros de voo. Serve como semente, não como personagem.
- **Pixel Frog, "Pixel Adventure 1"** (CC0, grátis) e **"Pixel Adventure 2"** (CC0, US$ 5+, com pássaros mas poucas animações).
- **pixel-boy, "Ninja Adventure"** (CC0, 16×16): ganhou animais em março de 2026, mas não confirmei se há pássaro.
- **ansimuz, "Sunny Land"** (CC0).
- **Kenney** (CC0), para os balões e emotes.

### 3. Risco de associação com personagens de terceiros

> Esta seção foi reescrita de forma neutra (decisão 0035): o projeto não
> cita personagem nem estúdio de terceiros em código, docs ou balões. O
> que fica é a análise de risco.

#### 3.1 O que é protegido

| Elemento | Protegido sozinho? | Por quê |
|---|---|---|
| Papagaio verde | Não | É uma ideia ou um animal (US Copyright Office, Circular 33) |
| Chapéu-palheta e gravata-borboleta | Não | São roupas comuns; o chapéu-palheta é ícone cultural do malandro brasileiro |
| Malandro | Não | É um personagem-tipo (cenas obrigatórias, *scènes à faire*) |
| Falas de malandro | Não | Frases curtas não têm proteção autoral |
| Nome "Zeca" | Não por direito autoral (Circular 33; Lei 9.610/98, art. 8º, VI). Por marca, depende do uso e da confusão causada | É um apelido brasileiro comum |
| A combinação e a expressão visual | Pode ser | Pelo teste de personagem distintivo (DC Comics v. Towle) e pela confusão de marca |
| O corpo em pixel art | É da exclusiveOlive | Arte comprada, com licença própria |

#### 3.2 Termômetro de risco

- **Mais alto:** papagaio verde com chapéu-palheta, gravata-borboleta e falas de malandro, com textos que citem um personagem de terceiros como referência de estilo. Era a situação dos textos do `DECISIONS.md` e do `PLANO.md` antes da decisão 0035 (o histórico do git do repositório privado ainda tem esses textos).
- **Médio:** papagaio com chapéu-palheta e gravata, nada de personagem de terceiros nos textos.
- **Baixo:** papagaio original sem o par chapéu-palheta e gravata.

**Pior caso realista:** uma notificação ao GitHub (DMCA ou marca). O GitHub dá cerca de 1 dia útil para alterar o conteúdo antes de desativar o repositório. Depois seria preciso tirar o pacote do Homebrew e do AUR, renomear e redesenhar.

#### 3.3 Como ficar seguro

- Não usar nomes, títulos ou marcas de personagens de terceiros em nomes, textos, tags ou posts; evitar também nomes de papagaios famosos de TV e cinema.
- Publicar a partir de um repositório novo, sem o histórico atual, com os textos limpos.
- Manter as cores próprias e nunca usar paletó, charuto ou guarda-chuva.
- Não pôr aviso de "sem afiliação" com estúdio nenhum: num design só inspirado, isso chama atenção para a associação.
- Pesquisar o nome escolhido no INPI e no USPTO.

#### 3.4 Domínio público não resolve agora

- Obras dos anos 1940 só entram em domínio público nos EUA no fim dos anos 2030, e só a versão original; as versões posteriores e as marcas continuam protegidas.
- Não conte com isso.

#### 3.5 Nome do produto

- O nome "claude-pet" já existe no GitHub: xtrimsystems/claude-pet, licença MIT, criado em 2026-02-14.
- Ele também usa a marca "Claude".
- Prefira um nome próprio (por exemplo "bichinho") e use "para o Claude Code" como descrição. Não conferi as diretrizes de marca da Anthropic.

### Fontes

- dist: https://github.com/axodotdev/cargo-dist/releases/tag/v0.33.0 · https://api.github.com/repos/axodotdev/cargo-dist/releases · https://axodotdev.github.io/cargo-dist/book/installers/index.html · …/installers/homebrew.html · …/installers/msi.html · …/installers/updater.html · …/supplychain-security/signing/windows.html · …/supplychain-security/attestations/github.html · https://raw.githubusercontent.com/axodotdev/cargo-dist/main/book/src/reference/config.md · https://raw.githubusercontent.com/axodotdev/cargo-dist/main/CHANGELOG.md
- cargo-packager: https://github.com/crabnebula-dev/cargo-packager · https://docs.crabnebula.dev/packager/configuration/
- Homebrew: https://brew.sh/2025/11/12/homebrew-5.0.0/ · https://brew.sh/2026/06/11/homebrew-6.0.0/ · https://brew.sh/2026/09/13/homebrew-7.0.0/ · https://docs.brew.sh/Taps · https://docs.brew.sh/Acceptable-Casks
- Apple: https://developer.apple.com/news/?id=saqachfa · https://support.apple.com/en-us/102445 · https://developer.apple.com/help/account/membership/fee-waivers/ · https://developer.apple.com/support/compare-memberships/ · https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution · https://developer.apple.com/documentation/security/customizing-the-notarization-workflow · https://developer.apple.com/documentation/technotes/tn3127-inside-code-signing-requirements · https://developer.apple.com/documentation/bundleresources/information-property-list/lsfilequarantineenabled · https://developer.apple.com/documentation/macos-release-notes/macos-big-sur-11_0_1-universal-apps-release-notes
- Microsoft: https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation · https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options · https://learn.microsoft.com/en-us/azure/artifact-signing/quickstart · https://learn.microsoft.com/en-us/azure/artifact-signing/faq · https://prices.azure.com/api/retail/prices?$filter=serviceName%20eq%20%27Trusted%20Signing%27 · https://support.microsoft.com/en-us/topic/what-is-smart-app-control-285ea03d-fa88-4d56-882e-6698afdb7003 · https://learn.microsoft.com/en-us/windows/package-manager/package/windows-package-manager-policies
- Assinatura: https://signpath.org/ · https://signpath.org/terms · https://shop.certum.eu/open-source-code-signing.html · https://raw.githubusercontent.com/cabforum/code-signing/main/docs/CSBR.md
- GitHub: https://docs.github.com/en/actions/concepts/security/artifact-attestations · https://docs.github.com/en/billing/concepts/product-billing/github-actions · https://docs.github.com/en/billing/concepts/product-billing/github-packages · https://docs.github.com/en/site-policy/content-removal-policies/dmca-takedown-policy
- Wayland: https://wayland.app/protocols/wlr-layer-shell-unstable-v1
- Claude Code: https://code.claude.com/docs/en/hooks · https://code.claude.com/docs/en/setup · https://code.claude.com/docs/en/plugin-marketplaces · https://code.claude.com/docs/en/plugins/host-marketplace
- Rust: https://doc.rust-lang.org/cargo/reference/unstable.html
- Arte: https://exclusiveolive.itch.io/cute-parrots-pixel-art-asset-pack · https://exclusiveolive.itch.io/ · https://exclusiveolive.itch.io/capybaras-pixel-art-asset-pack · https://pop-shop-packs.itch.io/pigeons-2d-pixel-asset-pack · https://opengameart.org/content/animated-birds-32x32 · https://pixelfrog-assets.itch.io/pixel-adventure-2 · https://pixel-boy.itch.io/ninja-adventure-asset-pack · https://ansimuz.itch.io/sunny-land-pixel-game-art · https://pixeljoint.com/forum/forum_topics.asp?FID=11 · https://pixeljoint.com/forum/forum_topics.asp?FID=20 · https://pixeljoint.com/forum/forum_posts.asp?TID=27797 · https://raw.githubusercontent.com/diasurgical/DevilutionX/master/README.md
- Direito autoral e marca: https://www.copyright.gov/circs/circ33.pdf · https://en.wikipedia.org/wiki/Copyright_protection_for_fictional_characters · Lei 9.610/98 (planalto.gov.br estava fora do ar; o texto foi citado de memória) · https://api.github.com/repos/xtrimsystems/claude-pet

### Fatos conferidos

- O dist (ex-cargo-dist) está ativo. A v0.33.0 saiu em 2026-09-11 e trouxe assinatura de Windows por Azure Artifact Signing, só para x86_64. Antes vieram a 0.32.0 (2026-05-21) e a 0.31.0 (2026-02-23). *Evidência:* https://api.github.com/repos/axodotdev/cargo-dist/releases e notas da v0.33.0: 'support for codesigning Windows binaries and installers using Azure Artifact Signing'; 'currently supports only x86_64 Windows targets'. *Confiança:* alta.
- O dist gera estes instaladores: shell, powershell, npm, homebrew (uma fórmula num tap próprio, não cask) e msi (WiX v3). Não gera .deb, .rpm, AppImage nem dmg; cask, winget, Flatpak e Docker estão só na lista de pedidos. *Evidência:* https://axodotdev.github.io/cargo-dist/book/installers/index.html; homebrew.html: 'Does not support Cask'; msi.html: 'WiX v4 isn't yet supported'. *Confiança:* alta.
- No dist, install-updater = true instala o programa <pacote>-update (axoupdater), só nos instaladores shell e PowerShell. github-attestations = true só funciona em repositório público ou no plano Enterprise, e o usuário confere com gh attestation verify. *Evidência:* https://axodotdev.github.io/cargo-dist/book/installers/updater.html; .../supplychain-security/attestations/github.html: 'only supports public repositories and private repositories of an organization with the GitHub Enterprise plan'. *Confiança:* alta.
- O codesign de macOS no dist é experimental e não há notarização embutida. O livro só documenta assinatura de Windows, e a referência de configuração não menciona macos-sign nem notarização. *Evidência:* Sumário do livro em https://axodotdev.github.io/cargo-dist/book/ (só 'Windows Signing'); o CHANGELOG cita 'experimental macOS codesigning'; config.md sem 'notar'. *Confiança:* media.
- O dist trata publish = false como sinal para não distribuir o pacote; dist = true no pacote força a distribuição. O crate claude-pet tem publish = false. *Evidência:* config.md do dist: 'dist = true ... in spite of signals like Cargo's publish = false'; ~/Documents/claude-pet-m2/crates/claude-pet/Cargo.toml. *Confiança:* alta.
- O Homebrew 5.0.0 (2025-11-12) depreciou casks sem assinatura de código, marcou para setembro de 2026 o desligamento dos casks oficiais que falham no Gatekeeper e depreciou --no-quarantine. O 6.0.0 (2026-06-11) reafirma o prazo. *Evidência:* https://brew.sh/2025/11/12/homebrew-5.0.0/ e https://brew.sh/2026/06/11/homebrew-6.0.0/. *Confiança:* alta.
- Desde o Homebrew 6.0.0, um tap de terceiros precisa de confiança explícita. Instalar pelo nome completo (usuario/repo/formula) confia só naquele item. *Evidência:* https://brew.sh/2026/06/11/homebrew-6.0.0/; https://docs.brew.sh/Taps: 'Install a fully qualified item to trust only that item'. *Confiança:* alta.
- O Homebrew 7.0.0 (2026-09-13) passou a rodar as operações de fórmula e cask em sandbox e pôs o macOS Intel no Tier 3, com remoção em 2027-09-01. *Evidência:* https://brew.sh/2026/09/13/homebrew-7.0.0/. *Confiança:* alta.
- No macOS Sequoia não dá mais para usar Ctrl-clique para passar pelo Gatekeeper. O usuário precisa ir em Ajustes do Sistema > Privacidade e Segurança > Abrir Mesmo Assim, e o app fica salvo como exceção. *Evidência:* https://developer.apple.com/news/?id=saqachfa; https://support.apple.com/en-us/102445. *Confiança:* alta.
- A quarentena é posta pelo app que cria o arquivo, e só se ele optar por isso (LSFileQuarantineEnabled). Por isso binários instalados por curl ou por fórmula do Homebrew normalmente não passam pelo diálogo do Gatekeeper. *Evidência:* https://developer.apple.com/documentation/bundleresources/information-property-list/lsfilequarantineenabled: 'whether the files this app creates are quarantined by default' (o comportamento do curl é inferência). *Confiança:* media.
- A notarização exige Developer ID, hardened runtime e timestamp seguro. Um binário solto pode ser notarizado dentro de um zip, mas não é possível grampear o ticket nele nem no zip. *Evidência:* https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution; customizing-the-notarization-workflow: 'Although tickets are created for standalone binaries, it's not currently possible to staple tickets to them'. *Confiança:* alta.
- O Apple Developer Program custa US$ 99/ano. A isenção da taxa não vale para pessoa física, e notarização exige ser membro do programa. *Evidência:* https://developer.apple.com/help/account/membership/fee-waivers/: 'Not be an individual, sole proprietor, or single-person business'; '99 USD'; compare-memberships: 'Mac software notarization' só para membros. *Confiança:* alta.
- No Apple Silicon todo executável precisa de assinatura, mas ad-hoc basta, e o toolchain (clang/ld) já assina sozinho. *Evidência:* Notas de lançamento do macOS Big Sur 11.0.1 (universal apps): 'a simple ad-hoc signature is sufficient'; 'the toolchain will now automatically sign your executables'. *Confiança:* alta.
- Em código assinado ad-hoc, o requisito de identidade fica preso àquela versão, então o macOS repete os pedidos de permissão TCC depois de cada mudança. *Evidência:* TN3127: 'Ad hoc signed code ... has a DR but it's tied to that specific version of the code ... If you tweak the code and run it again, macOS repeats that prompt'. *Confiança:* alta.
- No Azure Artifact Signing, pessoa física precisa estar nos EUA ou no Canadá. Organizações só em EUA, Canadá, UE, Reino Unido, Austrália, Nova Zelândia, Japão, Coreia do Sul, Singapura, Suíça, Noruega e Israel. O Brasil não está na lista. *Evidência:* https://learn.microsoft.com/en-us/azure/artifact-signing/quickstart (atualizado em 2026-09-29): 'Individual developers must be located in the United States or Canada'. *Confiança:* alta.
- O Artifact Signing custa US$ 9,99/mês no plano Basic (5.000 assinaturas) e US$ 99,99/mês no Premium (100.000), com excedente a US$ 0,005. Não aceita assinatura Azure gratuita ou de teste e não emite EV. *Evidência:* Azure Retail Prices API (Trusted Signing); https://azure.microsoft.com/en-us/pricing/details/artifact-signing/; FAQ: 'doesn't support free, trial, or sponsored Azure subscriptions'. *Confiança:* alta.
- O certificado EV não pula mais o SmartScreen desde 2024. A reputação vem do hash e do editor e leva semanas e centenas de instalações limpas. OV custa US$ 150–300/ano e EV US$ 400+; desde 2023 a chave precisa estar em HSM ou token. *Evidência:* https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation e .../code-signing-options (2026-08-29). *Confiança:* alta.
- O Smart App Control bloqueia apps sem assinatura quando a nuvem não tem previsão confiável, checa todo executável (não só os baixados) e não tem exceção por app. *Evidência:* https://support.microsoft.com/en-us/topic/what-is-smart-app-control-285ea03d-fa88-4d56-882e-6698afdb7003; smartscreen-reputation: 'Smart App Control signature checks apply to all executable files'. *Confiança:* alta.
- Publicar como MSIX na Microsoft Store é grátis (a conta de desenvolvedor também), a Microsoft reassina o pacote e não aparece aviso do SmartScreen. *Evidência:* https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options. *Confiança:* alta.
- A SignPath Foundation assina de graça projetos open source se: todos os componentes tiverem licença OSI e nada for proprietário, o projeto estiver ativo e já lançado, houver MFA, papéis de autor/revisor/aprovador e uma política de assinatura publicada. A SignPath Foundation aparece como editor. *Evidência:* https://signpath.org/terms: 'may not contain any proprietary, non open-source component'. *Confiança:* alta.
- O Certum Open Source Code Signing custa a partir de € 69, com cartão criptográfico e leitora inclusos, e o certificado traz o nome da pessoa precedido de 'Open Source Developer'. *Evidência:* https://shop.certum.eu/open-source-code-signing.html. *Confiança:* alta.
- Pelos requisitos do CA/B Forum (CSBR v3.11.0), certificados de assinatura de código emitidos a partir de 2026-03-01 valem no máximo 460 dias. *Evidência:* https://raw.githubusercontent.com/cabforum/code-signing/main/docs/CSBR.md. *Confiança:* alta.
- A política do repositório do winget não exige assinatura de código, mas o InstallerUrl precisa ser o local de release do próprio autor. *Evidência:* https://learn.microsoft.com/en-us/windows/package-manager/package/windows-package-manager-policies (sem cláusula de assinatura; 1.1.4). *Confiança:* media.
- O GitHub Actions é grátis em repositório público com os runners padrão (inclusive macOS e Windows), e o GitHub Packages/GHCR é grátis para pacotes públicos. *Evidência:* https://docs.github.com/en/billing/concepts/product-billing/github-actions; .../github-packages. *Confiança:* alta.
- O wlr-layer-shell não existe no GNOME Mutter nem no Weston. Existe em Hyprland, KWin, niri, Sway, river, labwc, Wayfire, Mir, Treeland e Jay. *Evidência:* https://wayland.app/protocols/wlr-layer-shell-unstable-v1 (tabela de compositores). *Confiança:* alta.
- No Windows, os hooks do Claude Code rodam no Git Bash se ele estiver instalado, senão no PowerShell. Na forma exec, o comando precisa ser um .exe real. Hooks http fazem POST do evento JSON inteiro, não aceitam async e não bloqueiam o Claude se falharem. *Evidência:* https://code.claude.com/docs/en/hooks; https://code.claude.com/docs/en/setup: 'Without Git for Windows, Claude Code runs shell commands via the PowerShell tool'. *Confiança:* alta.
- Um marketplace de plugins é um repositório com .claude-plugin/marketplace.json, adicionado com /plugin marketplace add dono/repo. O auto-update de marketplace de terceiros vem desligado, e dá para submeter o plugin ao diretório da Anthropic. *Evidência:* https://code.claude.com/docs/en/plugin-marketplaces; https://code.claude.com/docs/en/plugins/host-marketplace: 'Background auto-update is off for your marketplace by default'. *Confiança:* alta.
- A licença do pack diz: pode usar em projetos comerciais e não comerciais, pode editar, não pode redistribuir nem revender, mesmo editado; crédito é bem-vindo. Custa US$ 0,50 ou mais, tem quadros de 48x48 e 21 animações. Não há contato da artista na página nem no perfil, e os comentários estão vazios. *Evidência:* https://exclusiveolive.itch.io/cute-parrots-pixel-art-asset-pack e https://exclusiveolive.itch.io/. *Confiança:* alta.
- Outra licença do itch.io deixa explícito o sentido comum dessa cláusula: proíbe redistribuir como asset avulso, mas o asset precisa ser integrado a um projeto. *Evidência:* https://pop-shop-packs.itch.io/pigeons-2d-pixel-asset-pack: 'Cannot be resold or redistributed as a standalone game asset; Must be integrated into a project'. *Confiança:* media.
- A folha local do Zeca usa 20 animações nativas do pack com 92 quadros, que somam 166 células com as tags compostas. *Evidência:* ~/Documents/claude-pet-m2/skins-locais/zeca/sheet.json (meta.frameTags). *Confiança:* alta.
- O histórico do git (todas as branches) não tem nenhum arquivo do pack, só imagens de teste e a skin xadrez. Já o DECISIONS.md e o PLANO.md citavam um personagem de terceiros como referência de estilo (reescritos na decisão 0035). *Evidência:* git log --all --name-only em ~/Documents/claude-pet; ~/Documents/claude-pet-m2/DECISIONS.md linhas 13-16, 398, 422; PLANO.md linhas 18-20. *Confiança:* alta.
- O Dockerfile copia skins-locais/ para dentro da imagem. *Evidência:* ~/Documents/claude-pet-m2/Dockerfile: 'COPY skins-locais/ /opt/claude-pet/skins-locais/'. *Confiança:* alta.
- O hook atual depende de sh, curl e jq. Sem jq ele manda só o nome do evento. *Evidência:* ~/Documents/claude-pet/plugin/scripts/avisar.sh (cabeçalho) e plugin/hooks/hooks.json ('sh \"${CLAUDE_PLUGIN_ROOT}/scripts/avisar.sh\"'). *Confiança:* alta.
- Fato sobre um personagem de terceiros (estreia, visual e família) omitido por regra do projeto (decisão 0035). O que importa para o risco: o malandro com chapéu de palha e gravata-borboleta é um arquétipo, e a associação vem da combinação dos elementos, não de um deles. *Evidência:* omitida (decisão 0035). *Confiança:* alta.
- O nome de um personagem não é protegido por direito autoral, mas pode ser protegido como marca. Ideias também não são protegidas. *Evidência:* https://www.copyright.gov/circs/circ33.pdf: lista 'The name of a character'; 'may be protectable under federal or state trademark laws'. *Confiança:* alta.
- Personagem só é protegido quando é suficientemente delineado e especialmente distintivo (teste de DC Comics v. Towle). Personagens-tipo e arquétipos não são protegidos. *Evidência:* https://en.wikipedia.org/wiki/Copyright_protection_for_fictional_characters (fonte secundária). *Confiança:* media.
- A lei de marcas proíbe o uso que causa confusão sobre origem ou patrocínio, e as versões posteriores de um personagem continuam protegidas mesmo quando a original cai em domínio público. *Evidência:* estudo do Center for the Study of the Public Domain (Duke Law) sobre personagens que entram em domínio público. *Confiança:* alta.
- Pela Lei 9.610/98, art. 8º, VI, nomes e títulos isolados não são protegidos. *Evidência:* Lei 9.610/98 (planalto.gov.br estava fora do ar nesta sessão; texto citado de memória). *Confiança:* media.
- Ao receber um DMCA, o GitHub dá cerca de 1 dia útil para alterar o conteúdo antes de desativá-lo. Existe contranotificação, e o titular tem de 10 a 14 dias para entrar na Justiça. *Evidência:* https://docs.github.com/en/site-policy/content-removal-policies/dmca-takedown-policy. *Confiança:* alta.
- Precedente do modelo 'traga seu pack': o DevilutionX exige os dados do jogo original (DIABDAT.MPQ), comprados pelo usuário. *Evidência:* https://raw.githubusercontent.com/diasurgical/DevilutionX/master/README.md. *Confiança:* alta.
- Opções CC0 encontradas: o papagaio de 32x32 da MoikMellah tem só 5 quadros de voo. Pixel Adventure 2 é CC0 (US$ 5+), Ninja Adventure é CC0 em 16x16 e Sunny Land é CC0. Nenhuma tem um papagaio com o vocabulário que o pet precisa. *Evidência:* https://opengameart.org/content/animated-birds-32x32; https://pixelfrog-assets.itch.io/pixel-adventure-2; https://pixel-boy.itch.io/ninja-adventure-asset-pack; https://ansimuz.itch.io/sunny-land-pixel-game-art. *Confiança:* alta.
- Os únicos dados públicos de preço de pixel art que consegui (PixelJoint, 2026): um artista a US$ 12/h e outro com personagem a partir de US$ 10-25. *Evidência:* https://pixeljoint.com/forum/forum_topics.asp?FID=20; https://pixeljoint.com/forum/forum_posts.asp?TID=27797. *Confiança:* media.
- O trim-paths do Cargo ainda é instável. *Evidência:* https://doc.rust-lang.org/cargo/reference/unstable.html (Profile trim-paths option). *Confiança:* alta.
- Já existe um projeto chamado claude-pet no GitHub: xtrimsystems/claude-pet, licença MIT, criado em 2026-02-14 e com push em 2026-09-20. *Evidência:* https://api.github.com/repos/xtrimsystems/claude-pet. *Confiança:* alta.
- O cargo-packager gera .app/.dmg, MSI/NSIS e deb/AppImage/pacman, aceita identidade Developer ID, notarização e signCommand. A última versão é a 0.11.8, de 2025-11-27. *Evidência:* https://docs.crabnebula.dev/packager/configuration/; https://api.github.com/repos/crabnebula-dev/cargo-packager/releases. *Confiança:* alta.

### Recomendações

1. Antes de abrir o código, publique num repositório público NOVO, sem o histórico atual, ou reescreva os textos. O DECISIONS.md e o PLANO.md citavam um personagem de terceiros (reescritos na decisão 0035).
2. Escolha um nome de produto sem 'Claude' e sem colisão (por exemplo 'bichinho', descrito como 'para o Claude Code'). Já existe um xtrimsystems/claude-pet.
3. Recomendação da pesquisa, não adotada (o Renan manteve o nome Zeca, decisão 0035): um nome público que não remeta a personagem de terceiros, evitando nomes de papagaios famosos de TV e cinema.
4. No README e nas releases, mostre só a skin livre. Nunca publique fotos ou GIFs do Zeca derivado do pack sem permissão escrita da exclusiveOlive.
5. Leve o montador do Zeca (hoje em ~/Documents/claude-pet/xtask/src/zeca) para dentro do binário distribuído, como 'claude-pet skin instalar <zip>'. O comando deve validar o formato do pack, falhar com mensagem clara e nunca baixar do itch.io por conta própria.
6. Deixe uma skin livre como padrão já no primeiro lançamento (CC0, ou uma ave simples desenhada no pipeline de grades de texto) e trate o Zeca do pack como opcional.
7. Encomende um papagaio original em CC BY 4.0 ou CC0. Escopo de 55 a 60 quadros essenciais (cerca de 92 para a paridade com o pack), três orçamentos, licença por escrito na fatura, entrega do .aseprite, garantia de obra original e sem IA, e briefing sem citar personagem ou estúdio de terceiros nem enviar o pack.
8. Comente na página do pack da exclusiveOlive usando o modelo do relatório, pedindo permissão para embutir nos binários oficiais e, se possível, CC BY no repositório. Exija resposta por escrito: silêncio não é permissão.
9. Fase 1 de distribuição (US$ 0): dist com alvos musl x86_64/aarch64 e, depois dos portes, apple-darwin e windows-msvc; instaladores shell, powershell e homebrew; publish-jobs homebrew, install-updater e github-attestations; e dist = true no crate claude-pet, que tem publish = false.
10. Publique um pacote -bin no AUR, a imagem no GHCR como opcional (sem skins-locais; ajuste o COPY do Dockerfile com um build-arg) e uma unit systemd --user como forma principal no Linux.
11. Escreva no README que o overlay exige um compositor com wlr-layer-shell (Hyprland, KDE, Sway, niri…) e que GNOME e Weston não funcionam.
12. Troque o avisar.sh (sh, jq, curl) por um hook em forma exec que chama o próprio binário ('claude-pet hook <Evento>'), com a lista branca de metadados em Rust. Assim funciona no Windows sem Git Bash e no macOS sem jq. Não use hooks http, que mandam o JSON inteiro e são síncronos.
13. Publique o plugin pelo marketplace no próprio repositório (.claude-plugin/marketplace.json), suba o version a cada release e documente como ligar o auto-update.
14. macOS: lance sem assinatura (curl/sh e fórmula do Homebrew, com o comando de instalação pelo nome completo por causa da confiança em tap). Reserve US$ 99/ano para Developer ID e notarização quando o clique que foca o terminal precisar de TCC, ou quando for oferecer .pkg/.dmg pelo navegador.
15. Windows: depois do porte, faça um release sem assinatura para os primeiros usuários, com instruções. Em seguida peça a SignPath Foundation (grátis; o binário não pode embutir arte proprietária) ou compre o Certum Open Source (a partir de € 69). Avalie a Microsoft Store com MSIX (grátis). Não conte com o Azure Artifact Signing, indisponível para pessoa física no Brasil.
16. Reprodutibilidade: fixe a versão exata do Rust no CI (a mesma 1.98.1 da imagem), use --locked e --remap-path-prefix, fixe os timestamps dos arquivos tar/zip e confira o SHA256 entre dois builds. Publique os SHA256 junto com os atestados do GitHub e compare antes de assinar.
17. Antes de anunciar, pesquise o nome final no INPI e no USPTO. Se um dia houver dinheiro envolvido, consulte um advogado de propriedade intelectual.

### Riscos

- **Risco:** Associação com um personagem de terceiros pela combinação de papagaio verde, chapéu-palheta, gravata-borboleta, falas de malandro e textos que citassem esse personagem como referência de estilo. Pode levar a um DMCA ou reclamação de marca no GitHub, com o repositório desativado depois de cerca de 1 dia útil. **Mitigação:** Repositório novo e textos sem referência a personagem de terceiros (decisão 0035); skin padrão original sem o par chapéu-palheta e gravata, ou com visual bem distinto; o nome público ficou Zeca, por decisão do Renan.
- **Risco:** Vazamento da arte do pack: commit acidental, capturas no README, imagem Docker publicada com skins-locais/ ou GIFs nas releases. **Mitigação:** Manter o .gitignore atual (o histórico está limpo); build-arg para não copiar skins-locais/ em imagens publicáveis; README só com a skin livre; checagem no CI que falha se algum skin.json com redistribuivel=false entrar no repositório.
- **Risco:** A exclusiveOlive não responde ou nega. **Mitigação:** Seguir com o 'traga seu pack' e com a arte original encomendada. Não tratar silêncio como permissão.
- **Risco:** O pack muda de formato, sai de venda ou troca de licença, e o 'traga seu pack' para de funcionar para usuários novos. **Mitigação:** Validar o formato e os hashes das versões conhecidas, mostrar erro claro e manter uma skin livre como padrão.
- **Risco:** A arte encomendada sai cara, atrasa ou vem com licença ambígua. **Mitigação:** Três orçamentos, entrega em etapas (design, poses-chave, animações), licença CC BY 4.0 ou CC0 por escrito na fatura, entrega do .aseprite e garantia de obra original e sem IA.
- **Risco:** No macOS, o binário assinado ad-hoc faz o sistema pedir as permissões TCC de novo a cada atualização (para focar o terminal), e quem baixa pelo navegador encontra o bloqueio do Gatekeeper. **Mitigação:** Distribuir por curl e pela fórmula do Homebrew (sem quarentena); adotar Developer ID e notarização (US$ 99/ano) quando entrar alguma função que dependa de TCC.
- **Risco:** No Windows, avisos do SmartScreen, bloqueio sem exceção do Smart App Control e falsos positivos do Defender para um binário novo sem assinatura. **Mitigação:** Assinar com SignPath ou Certum; manter a mesma identidade de assinatura entre versões; enviar falsos positivos ao Microsoft WDSI; considerar a Microsoft Store com MSIX.
- **Risco:** O projeto fica inelegível para a SignPath se o binário oficial embutir arte proprietária. **Mitigação:** Deixar o Zeca do pack fora do binário assinado ('traga seu pack') ou usar um certificado Certum ou OV.
- **Risco:** Lacunas do dist: não gera .deb, .rpm, .dmg nem cask, não notariza, e o codesign de macOS é experimental. **Mitigação:** Jobs extras com cargo-deb/nFPM e notarytool; o cargo-packager se o macOS exigir um bundle .app.
- **Risco:** Atrito com a confiança em tap do Homebrew 6+ e auto-update de plugin desligado por padrão: usuários ficam presos em versões antigas. **Mitigação:** Documentar o 'brew install <usuario>/tap/<formula>' pelo nome completo e o caminho para ligar o auto-update do marketplace; subir o version do plugin a cada release.
- **Risco:** O hook atual não funciona no Windows sem Git Bash, e sem jq perde os ids de sessão. O 'date +%s%3N' é específico do GNU (há reserva no script). **Mitigação:** Hook em forma exec chamando o binário, com a lista branca em Rust.
- **Risco:** Usuários de GNOME e de X11 esperam que funcione no Linux. **Mitigação:** Dizer o escopo no README (compositores com layer-shell) e avaliar um fallback no futuro.
- **Risco:** O nome 'claude-pet' colide com o xtrimsystems/claude-pet e usa a marca Claude da Anthropic. **Mitigação:** Nome de produto próprio, com 'para o Claude Code' só como descrição.
- **Risco:** Macs Intel saindo de cena (Homebrew Tier 3 desde setembro de 2026, remoção em 2027): mais custo de CI por um público que encolhe. **Mitigação:** Compilar x86_64-apple-darwin de forma cruzada no runner arm64 e reavaliar em 2027.
