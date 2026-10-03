# Revisões adversariais da síntese

> Dois revisores (viabilidade e produto), 2026-10-02. As correções foram incorporadas ao PLANO.md.

## feasibility

The plan is feasible on this host. I found no blockers. These core mechanics check out:
- **Rust build:** SCTK 0.21.1 with only the `calloop` feature is pure Rust (no pkg-config, xkbcommon or system libwayland), so a static musl build works. rust-version 1.86 is below the host's 1.98.1.
- **Docker 29.7.2 mount:** it emits `rro` plus `rslave` for the read-only `/run/user` bind without rejecting the combination.
- **Boot order:** docker.service has DefaultDependencies=no, but it requires docker.socket, which is ordered after sysinit.target. So `/home` (a separate btrfs subvolume) and `/run/user` exist before containers start.
- **Hyprland v0.56.2 SHM and fractional scale:**
  - SHM buffers are released right after commit.
  - Textures are updated one damage rect at a time.
  - With a NULL output, the fractional scale only arrives at map (the explicit-output path sends it at creation), which justifies the 1x1 bootstrap at startup.
- **socket2:** Hyprland writes non-blocking and only queues events once the kernel buffer is full, so a dedicated reader thread is more than enough.
- **Audio:** pipewire-pulse is active. Alpine's pulseaudio-utils/libpulse dependencies fit the 40 MB image budget.
- **Toolchain and plugin CLI:**
  - wayland-protocols ≥0.32.6 ships ext-idle-notify v2.
  - gh has the `repo` and `workflow` scopes and is already the git credential helper.
  - The `claude plugin validate --strict`, `tag --dry-run` and `marketplace add --sparse` flags exist.
  - The hook field names (prompt_id, background_tasks, stop_hook_active, is_interrupt) are in the 2.1.288 binary.

**Two major corrections before M1:**
1. **Full-monitor repaints.** On every commit of a mapped layer, Hyprland damages the layer's whole geometry. Every pet frame therefore repaints the entire monitor. The premise that one full-output surface is cheap is false, and the budgets only measure the container.
2. **Crispness gate.** The check uses `grim -g` on a logical rect. That captures through a bilinear filter at half-pixel offsets, accepts only integer geometry, and mixes in background pixels. The M1 engine GO/NO-GO could fail for reasons unrelated to the renderer.

**Minor fixes:**
- Damage-rect overflow collapses to a bounding box.
- The SlotPool can reuse stale pixels across re-homes, and memory is under-counted.
- The `docker kill` restart test is wrong.
- Plan B (GTK) is not a drop-in replacement.
- The watchdog heartbeat, backoff and drag fail-safe need tightening.
- The toolchain pin would trigger a hidden rustup install.
- Monitor layout moves are not handled.
- The e2e timing and `claude -p` permissions are wrong.

### [major] Compositor cost of the permanent full-output OVERLAY (eDP-1 and the 4K)

The architecture's central premise is false on Hyprland 0.56.2. Moving the sprite inside one full-output buffer avoids layout recalculation, but not full-monitor damage. Every commit of a mapped layer surface damages its entire geometry, so each pet frame makes Hyprland repaint the whole monitor (wallpaper, windows, bar, overlay): idle at 2–8 fps, particles at 30 fps, drags at up to 60 Hz, and 8.3 Mpx per frame on the 4K. In addition, a permanent transparent full-screen texture is blended into every repaint caused by other apps, such as video or scrolling. The budgets (docker stats CPU/RSS) cannot see any of this; it is spent in Hyprland and on the iGPU.

**Evidência:** - **Unconditional damage:** Hyprland v0.56.2 `src/desktop/view/LayerSurface.cpp`, `CLayerSurface::onCommit` (l.289–312), runs `CBox geomFixed = {m_geometry.x, m_geometry.y, m_geometry.width, m_geometry.height}; g_pHyprRenderer->damageBox(geomFixed);` on every mapped commit, before and independent of the `committed != 0` check that triggers arrange.
- **Geometry is global:** `m_geometry` is in global coordinates (`Renderer.cpp` `arrangeLayerArray` l.2588: `full_area = {pMonitor->m_position.x, ...}`). `damageBox` (`Renderer.cpp` l.2794) adds it to every monitor.
- **Still on master:** current master has the same code (l.322–323).
- **Only uploads are damage-limited:** texture upload uses the buffer damage (`SurfaceState.cpp` `updateSynchronousTexture`, `GLTexture.cpp` l.146–153).
- **Omarchy precedent differs:** Omarchy maps its full-screen toast overlay only while toasts exist (`Service.qml:959` `visible: popupModel.count > 0`). `hyprctl -j layers` level 3 is empty right now.
- **Measurement is possible without installs:** intel_gpu_top, powertop and pidstat are not installed, but `/sys/class/drm/card1/gt/gt0/rc6_residency_ms` and `/proc/1501/stat` (Hyprland) are readable.

**Correção:** - **Correct the rationale:** fix the Surface decision and the CLAUDE.md architecture fact.
- **Add a Hyprland-side budget to the M1 gate:**
  - Measure for 30 s each with the pet hidden, idle, working, dragging and in T3, on eDP-1 and on a 4K headless output (with consent).
  - Hyprland CPU = delta of utime+stime in `/proc/$(pgrep -x Hyprland)/stat`.
  - GPU busy = 1 − Δrc6_residency_ms/Δt.
- **Cap commits per state:**
  - idle: mostly a static pose with blink/breath frames, at most about 2 commits/s on average;
  - deep sleep: 0;
  - sprite and FX coalesced into one commit per frame;
  - log commits per minute.
- **Fallback if the budget fails:** a hybrid of two surfaces.
  - A small fixed-size 'rest' surface (sprite + bubble + hop headroom, never resized) for idle, working and waiting.
  - A full-output 'stage' mapped only during drag, travel, particles and T3, like Omarchy's toasts.
  - The `no_anim` layer rule then becomes necessary for clean handoffs.
- **Fullscreen:** consider defaulting `tela_cheia` to hiding while a fullscreen video plays.
- **Optional:** report upstream that `m_geometry` only needs damage when `committed != 0`.

### [major] M1 crispness gate (scripts/verificar.sh step 3, cargo xtask nitidez, bin/bichinho foto)

The crispness check will produce false failures, and it sits inside the engine GO/NO-GO gate. A NO-GO would switch to plan B for no reason.

**Evidência:** - **Bilinear resampling:** grim 1.5.0 (`render.c`) captures whole outputs and composites them into the `-g` region. The transform is translate(output.logical.x − g.x)·scale with `PIXMAN_FILTER_BILINEAR`. It is grid-aligned (an exact `OP_SRC` copy) only when (output.x − g.x)·1.5 is an integer, i.e. for even logical offsets. Otherwise every edge blurs by half a pixel.
- **Integer geometry only:** `box.c` `parse_box` uses `strtol`, so a fractional `sprite_global` cannot even be passed.
- **Unknown block phase:** the sprite sits at arbitrary integer device pixels (D=6), so its logical rect is generally fractional, and the phase of its 6×6 blocks relative to any crop is arbitrary.
- **Background bleed:** transparent cell pixels show whatever is behind them (terminal text, wallpaper), which breaks a naive block-uniformity test.

**Correção:** - **Capture the whole output:** `grim -o "$mon" full.png`. The geometry is then the output rect, the transform is identity, and the copy is exact.
- **Crop in device pixels:** expose a device-pixel rect relative to the output in `/v1/estado` (e.g. `sprite_disp {x,y,w,h}` plus `monitor`) and crop with it.
- **Compare only opaque pixels:** compare against the expected frame from a debug endpoint (e.g. `/v1/debug/quadro` returning RGBA plus origin) only where the expected alpha is 255, requiring an exact RGB match. Alternatively, run the 6×6 check with the known phase on the opaque mask.
- **`foto`:** use the same method.

### [minor] Damage reporting for particles and T3 on the 4K

The plan sends up to 128 `damage_buffer` rects and collapses anything beyond that into their 'union' to keep screen-wide confetti cheap. Taken as a bounding box, scattered T3 confetti (old and new bounds of 30–60 particles plus sprite, bubble and rays) covers roughly the whole 3840×2160 buffer. That forces a full ~33 MB synchronous texture upload in Hyprland's render thread on every frame at 30 fps (~1 GB/s), exactly during the celebration. This is the opposite of cheap.

**Evidência:** - **Per-rect uploads:** Hyprland v0.56.2 `GLTexture.cpp` l.146–153 uploads each damage rect with its own `glTexSubImage2D` and never collapses them.
- **Accumulated damage is passed through:** `SurfaceState.cpp` `updateSynchronousTexture` → `texture->update(..., accumulateBufferDamage())`.
- **Plan text:** section 6, "beyond that, send their union".

**Correção:** - **Never use the bounding box:** either send every rect (20-byte requests that Hyprland iterates cheaply), or bin rects into a coarse tile grid (e.g. 64×64 device px) capped at a few hundred tiles.
- **Observe it:** report rect count and damaged area per frame in `/v1/estado`.
- **Test it:** add a 4K T3 frame-time check to M6.

### [minor] SHM buffer and pool lifecycle; memory accounting

Two problems:
- **Stale pixels:** 'A fresh memfd is already transparent; never memset' is true only for a fresh pool. SCTK's SlotPool reuses freed regions without zeroing and grows by at least doubling. Reusing one pool across re-homes or monitor sizes can show stale pixels (a ghost of the previous frame) on the first frame, and the pool's memory keeps growing.
- **Hidden memory:** Hyprland reads the entire buffer on the first commit of each surface and keeps a same-size GL texture. The real cost is about 2× the buffer outside the container's cgroup: ~18 MB on eDP-1, ~66 MB on the 4K. `docker stats` will not show it.

**Evidência:** - **Pool reuse:** SCTK v0.21.1 `src/shm/slot.rs` keeps a free list with no zeroing, and the pool "resizes like Vec::reserve, always at least doubling".
- **Full read on size change:** Hyprland v0.56.2 `Compositor.cpp` `setAttach` forces full damage when the buffer size changes. `SurfaceState.cpp` `updateSynchronousTexture` then calls `createTexture` on the whole buffer.
- **Pages charged to Hyprland:** shmem read faults allocate pages, so they are charged to Hyprland's cgroup.
- **Plan text:** section 6 only counts about 9 MB / 33 MB of SHM.

**Correção:** - **Fresh pool per surface:** create a new SlotPool (a fresh memfd) for each layer surface and drop it on destroy. Never reuse a pool across monitors or sizes.
- **Clear reused slots:** if any slot is reused or duplicated (the 'busy' case), clear it or full-copy it before the first commit.
- **Budget Hyprland's share too:** measure Hyprland's RSS delta (`ps -o rss= -p $(pgrep -x Hyprland)`) around a map on each monitor, and add it to the documented budget.

### [minor] M1 restart-policy verification

The M1 step "`docker kill -s KILL $(docker compose ps -q pet)`: RestartCount goes up" will always fail. `docker kill` marks the container as manually stopped and cancels its restart manager, so no restart policy brings it back. That can mislead the gate, or prompt a needless rework of the lifecycle.

**Evidência:** - **moby docker-v29.7.2 `daemon/kill.go` l.80–96:** when the image sets no StopSignal (and always for SIGKILL), `killWithSignal` calls `container.ExitOnNext()`, which is `RestartManager().Cancel()`, and sets `HasBeenManuallyStopped = true`.
- **`container.go` `ShouldRestart`** honours that flag.

**Correção:** Simulate a crash without going through the Docker API, then assert that RestartCount increments and the pet is back within about 3 s. Options:
- from the host, `kill -9 $(pgrep -u "$(id -u)" -f '/usr/local/bin/bichinho rodar')` (same uid, and the pid is visible);
- `docker compose exec pet kill -9 <child pid>` (busybox);
- a PET_DEBUG-only `bichinho ctl abortar` that calls `abort()`.

### [minor] Plan B (gtk4-rs + gtk4-layer-shell in the container)

Plan B is not a drop-in replacement for `wl/`, and the NO-GO triggers don't match it:
- **Lifecycle:** GTK owns a GLib main loop and kills the process when the compositor goes away, so the in-process reconnect design (calloop daemon with threads) cannot carry over.
- **Gate triggers:** GTK is equal or worse on every NO-GO criterion (crispness, RSS, click-through, reconnect).
- **Budgets:** its image and RSS would fail the same 40 MB / 64 MiB budgets.

**Evidência:** - **GTK exits on compositor loss:** GTK 4.22.4 `gdk/wayland/gdkeventsource.c` l.164–190 and 316–318 call `_exit(1)` on flush, read or dispatch errors ("Lost connection to Wayland compositor.").
- **Load-order requirement:** gtk4-layer-shell v1.3.0 `linking.md` says libgtk4-layer-shell must load before libwayland-client (via LD_PRELOAD or link order). Any direct wayland-client use breaks this, e.g. ext-idle-notify through gdk4-wayland.
- **`respect_close`:** defaults to FALSE in 1.3.
- **Container environment:** without Mesa, GTK needs GSK_RENDERER=cairo and GDK_DISABLE=gl,vulkan, plus GTK_A11Y=none, GDK_DEBUG=no-portals, GSETTINGS_BACKEND=memory, XDG_CACHE_HOME and a font.
- **Size:** research estimates the Alpine GTK image at about 150–250 MB and RSS at about 60–120 MB.

**Correção:** - **Reframe M1:** treat crispness, RSS, click-through and reconnect failures as bugs to fix in `wl/`.
- **Narrow plan B:** make it a time-boxed escape hatch only for "the Wayland plumbing is not converging".
- **If plan B is kept, write down now:**
  - its process model: a thin GTK renderer child that exits on compositor loss, supervised by the Rust daemon, which keeps ingress, brain and state;
  - its environment variables;
  - its own budgets.

### [minor] Watchdog, reconnect backoff and drag fail-safe

Three gaps:
- **Spurious watchdog aborts:** the watchdog aborts when the main-loop heartbeat is older than 60 s. Deep sleep has zero commits and no animation timers, and frame callbacks stop while the session is locked or the screen is off (DPMS off). Unless an always-armed main-loop timer feeds the heartbeat, the pet aborts and restarts in a loop. The thread that runs the 2 s config poll is unspecified.
- **Backoff contradicts the 2 s promise:** reconnect backoff caps at 30 s, while the lifecycle table promises the pet within 2 s of a new `hypr/<HIS>`.
- **Input region can stick:** while the button is held, the input region covers the whole monitor. If a release or leave is lost, or the main thread stalls, every click on that monitor is eaten until the 60 s watchdog fires.

**Evidência:** - **Plan sections:** 3 (watchdog >60 s, `/saude` <30 s), 5 (backoff 1→30 s; discovery only while Waiting), 6 (deep sleep = zero commits), 8 (input widened on press).
- **Hyprland frame callbacks:** they are sent only when the monitor renders.

**Correção:** - **Heartbeat:** arm an unconditional 5 s calloop heartbeat timer, independent of rendering, frame callbacks and brain state.
- **Backoff:** apply it only when retrying the same signature after a client-side failure. Try a new signature (newer epoch) immediately.
- **Drag input region:**
  - widen it only once the drag threshold is crossed;
  - restore the hitbox after about 5 s without pointer events;
  - also restore it on `closed`, hide, teardown and before any re-home.

### [minor] Host toolchain pin (rust-toolchain.toml)

Pinning `channel = "1.98.1"` silently changes system state. The only installed toolchain is `stable` (which is 1.98.1), and rustup treats "1.98.1" as a different toolchain. With auto-install on, the first `cargo` call in the repo (including `bin/bichinho verificar`) downloads and installs a second full toolchain plus clippy and rustfmt.

**Evidência:** - **Installed toolchains:** `~/.rustup/toolchains` contains only `stable-x86_64-unknown-linux-gnu`; `rustup show` reports rustc 1.98.1.
- **Auto-install:** in rustup 1.29.1, `set auto-install` defaults to `enable`, and `~/.rustup/settings.toml` does not override it.

**Correção:** Choose one:
- use `channel = "stable"` with `components = ["clippy", "rustfmt"]` (the Docker build already pins the compiler through `rust:1.98.1-alpine3.24`);
- make `rustup toolchain install 1.98.1 -c clippy -c rustfmt` an explicit setup step in T0.3 that the user consents to.

### [minor] Re-home triggers: monitor layout moves and the startup fallback

Two gaps:
- **Layout moves:** Hyprland leaves a mapped layer surface at its old global position when its monitor moves in the layout. The plan only re-homes on `closed`, focus changes and FALLBACK/0x0.
- **Startup fallback:** before any `focusedmonv2` has been seen, the 500 ms "retry with the explicit output named by the last focusedmonv2" fallback has no name to use.

**Evidência:** - **Omarchy hit this:** `/usr/share/omarchy/shell/Ui/ScreenMoveRemap.qml` describes the bug and fixes it with a 200 ms settle plus a 50 ms unmap pulse.
- **This host:** positions are explicit (`~/.config/hypr/monitors.lua`: HDMI-A-1 at 0x-1440, eDP-1 at 640x0). But the catch-all `hl.monitor({ output = "", position = "auto" })` covers any other monitor, including work displays and the e2e headless output.
- **No startup state:** focusedmonv2 is only emitted on change, and the daemon never opens `.socket.sock`.

**Correção:** - **Layout moves:** watch xdg_output logical_position and size changes for the current output (SCTK `OutputHandler::update_output`). Re-home after a 200 ms settle, as for `closed`.
- **Startup:** when no output name is known yet, fall back to the first `wl_output` that is not FALLBACK and has a non-zero size.

### [minor] E2E and live verification details

Two test steps will misfire:
- **e2e return trip:** in `e2e-monitor.sh`, the minimum travel spacing (≥1.5 s) plus the 300 ms debounce and ≤250 ms poof can legitimately delay the return trip by about 1.5–2 s. The 1 s "pet followed" deadline is therefore flaky.
- **M2 edit check:** the `claude -p` edit runs with no permission mode. Print mode cannot answer the Edit prompt, so the edit is denied, no mutating PostToolUse arrives and the pet does not hop.

**Evidência:** - **Timing:** plan sections 7 (spacing, debounce, poof) and 16 (assert within 1 s).
- **Permissions:** `~/.claude/settings.json` has no `permissions.defaultMode`. `claude --help` lists `--permission-mode` with acceptEdits, bypassPermissions, etc.

**Correção:** - **e2e:** use deadlines ≥ debounce + spacing + poof + 0.5 s (about 2.5 s), or wait 2 s between steps.
- **M2:** run `claude -p --permission-mode acceptEdits '…'` from inside a `mktemp -d` directory.

## product

The plan is not ready as written. The Wayland, Docker and payload-allowlist foundations are sound, but the Claude Code turn model and the attention policy don't match how this user actually works: `--dangerously-skip-permissions` plus plan mode, Opus at effort max, and background shells and workflows.

Fix these before M2 freezes the hook wire format and before M4 and M6 add sounds and bubbles:
- **Critical:** the background-task handling either suppresses celebrations or downgrades them.
- **Major:** the end-to-end gates rely on `claude -p`, which kills async hooks when it exits, and headless or non-terminal sessions are not filtered out.
- **Major:** sounds and escalation fire even when the user is already looking at the Claude terminal.
- **Major:** the time-based T0/T1 split makes almost every Opus-max answer play a coin sound.
- **Major:** the checkerboard test skin would be the on-screen character from M1 until M5, and again whenever a skin fails to load.
- **Major:** project names leak into screen shares, and the obvious fix (the `no_screen_share` layer rule) would black out the whole shared monitor.

The minor issues below are cheap to fix and mostly add scenario goldens.

### [critical] Brain / hook wire format: background tasks

Forwarding `background_tasks` as a bare count (`bg`) breaks celebrations in two ways.
(a) Any running background task makes every later Stop in that session take the 'chain open, at most T1, silent' path for the rest of the session. That includes a dev server, the plan's own native dev loop (`cargo run -p bichinho -- rodar`), `docker compose logs -f`, a Monitor, and Claude Code's internal tasks.
(b) When an agent or workflow chain really finishes, Claude Code wakes the session with a machine-injected prompt (UserPromptSubmit with source=system). The brain table turns that into a NEW turn with origem=system, and machine turns are capped at silent T1. So long background work never gets its T2/T3.
(c) The 0.8 s settle is cancelled by any Pre/PostToolUse 'from the session'. Background subagents keep streaming such events (they carry agent_id), so they cancel it.
(d) The 2 h turn expiry drops long workflows.

**Evidência:** - In the 2.1.288 d.ts, `StopHookInput.background_tasks` is 'In-flight background work (running/pending + backgrounded)'.
- The binary's filter `gg()` keeps every task whose status is running or pending.
- The type labels (`k3n`) are: shell, subagent, workflow, monitor, 'MCP task', teammate, dream, 'auto-mode scan', 'memory import', 'cloud session'.
- The `UserPromptSubmit.source` docs say `system` = 'machine-injected turns (… task notifications, auto-continuation)'. I saw these `<\task-notification>` turns in this session when background commands finished.
- Plan §9: `avisar.sh` forwards only the length of `background_tasks`.
- Plan §10: 'UserPromptSubmit → new turn (t0=ts, origem=src)'; machine-started turns, including `system`, are capped at silent T1; Stop with 'bg > 0 … at most T1, silent'.
- The plan's native dev loop is a daemon, so Claude has to start it in the background.

**Correção:** Fix this in the M2 wire format, since the plan freezes the hook set there.
- **New fields:** forward `bgt`, the task types normalised to `[a-z_]` (2.1.288 emits labels like 'MCP task' and 'auto-mode scan'), and `bgi`, the opaque task ids (tok-validated, at most 16).
- **Brain:**
  - Only agent-like work (subagent, workflow, teammate, cloud_session) opens or extends a chain.
  - Shells and monitors never suppress a Stop.
  - Ignore dream, auto_mode_scan and memory_import.
  - A `src=system` UserPromptSubmit while a chain is open is a continuation: keep t0 and origem=user, and accumulate counters. Attribute subagent tool calls to the turn active at their SubagentStart (via agent_id).
  - Apply the machine cap only to chains rooted in loop_wakeup, schedule_wakeup or poll_event, or in `system` with no chain open. Loop turns with no mutating tools are T0 and set no ready flag.
  - Only main-thread events whose ts is later than the Stop cancel the settle.
  - Chain expiry is 12 h for agent-like tasks.
- **Goldens:** `servidor-em-segundo-plano` (normal celebrations while a dev server runs) and `workflow-longo` (T3 on the final Stop of the system-sourced turn).

### [major] Verification / headless and non-terminal sessions

The M2 and M7 end-to-end gates are built on `claude -p`. In -p mode, async hooks are killed when the process exits, so delivery of Stop and SessionEnd races the exit. The gates also exercise a path the user doesn't use: no AskUserQuestion or ExitPlanMode dialogs, no idle_prompt.

The brain also treats every non-interactive or non-terminal session like a terminal session. That includes nested `claude -p` runs started by Claude or by skills, and IDE or desktop entrypoints. Expected results:
- flaky gates;
- bursts of hello/working events;
- ghost sessions whose final Stop or SessionEnd may never arrive, leaving the pet stuck in 'working' for up to 5 min and a +N badge for up to 2 h.

**Evidência:** - Hooks docs, 'Run hooks in the background': 'In non-interactive mode with the -p flag, async hooks are killed when the process exits'.
- Plan M2 verify: `claude -p "responda só: ok"` → nod, and a `claude -p` edit → hop. Plan M7: fresh clone, then `claude -p` → celebration.
- Plan §10: 'sdk (claude -p) is treated normally'.
- Binary: `CLAUDE_CODE_ENTRYPOINT` becomes `sdk-cli` for non-interactive runs (it is `cli` in this session). The docs say hooks inherit the environment.
- skill-creator's `run_eval.py` runs `claude -p` in a ProcessPoolExecutor.
- tmux 3.7 is installed.

**Correção:** - Forward `ent` (`$CLAUDE_CODE_ENTRYPOINT`, enum-validated) in wire v1.
- Default `sessoes.origens = ["cli"]`. Ignore other entrypoints, or treat them as silent T0 that expires after 2 min without events.
- Make the gates interactive:
  1. `tmux new-session -d -s e2e -c ~/Documents/bichinho 'claude --dangerously-skip-permissions "responda só: ok"'`, then poll `/v1/estado` for nod.
  2. Send a second prompt that edits a temp file and expect a hop.
  3. `tmux send-keys -t e2e /exit Enter`.
- Run the gate in a directory the user has already trusted, so the trust prompt doesn't block it.
- Keep `claude -p` only for the smoke test 'pet down → exit 0 and no hook-error notice'.

### [major] Attention / sound arbiter: presence

Sounds and escalation ignore whether the user is already looking at the Claude terminal. In bypass mode the needs-input triggers are AskUserQuestion and ExitPlanMode, which mostly appear while the user is at the terminal. A long plan in the approval dialog gets ping at 0 s, ping2 at 30 s and trinado at 90 s while the user reads it. Presence detection (`ext_idle_notifier_v1`) is scheduled for M6, and there it only drives sleep and welcome-back.

**Evidência:** - Plan §10 escalation table: L1/L2/L3 sounds at 0/30/90 s.
- Plan §10 arbiter gates: ligado, mudo, soneca, dnd, silencio. There is no presence or focus gate.
- `ps`: `claude --dangerously-skip-permissions`, and this session is in plan mode.
- Claude Code titles its terminal '✳ <title>' when idle and alternates '◐'/'◑' every 960 ms while working (binary: `U5=["◐","◑"],W5="✳",H9e=960`).
- `hyprctl -j activewindow` returns `{"class":"foot","title":"✳ Personagem animado desktop Claude"}`.
- socket2 already delivers `activewindow>>CLASS,TITLE`, which plan §7 discards as 'title spinner' noise.

**Correção:** - **M4 presence signal:** `olhando_claude` is true when the focused window title starts with '✳ ', '◐ ' or '◑ '. Keep only that boolean. Combine it with input activity from `ext_idle_notifier_v1` (move that binding from M6 to M4).
- **While looking with recent input:**
  - escalation stays at L1, visual only, with no ping;
  - T1 is silent;
  - a ready flag is acknowledged after about 10 s of focus when only one session is ready.
- L2/L3 sounds play only when focus is elsewhere or input has been idle for 60 s or more.
- **Optional:** forward a short hash of `session_title` so the daemon can match the exact window. It already sees titles through socket2, but check first that the terminal title equals `session_title`.
- **Config:** `som.presenca = consciente|sempre`.
- **Goldens:** `plano-lido-no-terminal` and `pergunta-ausente`.

### [major] Celebration tiers: calibration

T0 requires BOTH < 15 s and no tools, and wall-clock minutes dominate the score. With Opus at high effort, plain answers often take 20–90 s, so they land in T1 (hop plus coin sound). A 4–5 min thinking-heavy explanation lands in T2 (powerup plus confetti). The user asked for a 'small hop for quick answers'; this produces a coin after almost every answer, the sound fatigue PeonPing users complained about.

**Evidência:** - Plan §10: T0 is 'min_ativos < 15 s and no tools'; T1 is 'score < 4 → done_small + moeda'; time weight 1.0 per minute.
- This session runs with `CLAUDE_EFFORT=max`, and settings.json has `"model": "opus"`.
- `PostToolUseHookInput` has `duration_ms` ('Tool execution time … Excludes permission-prompt and hook time').

**Correção:** Classify by work, not wall time.
- **T0** (nod, silent): no mutating tools, no subagents and no edited files, for any duration up to about 5 min.
- **T1** (small hop): silent by default; it plays a sound only when presence says the user is away.
- **T2/T3:** keep their sounds.
- **Score:** forward `dur` (PostToolUse `duration_ms`) and score active time as the summed tool time plus subagent time, not thinking time.
- **Tests:** add the cases '40 s answer with no tools → T0' and '4 min explanation → T0/T1' to the worked examples.

### [major] Art / skins: placeholder exposure

The checkerboard `_teste` skin would be the on-screen character for weeks, despite the plan's rule that it never is.
(a) `padrao` doesn't exist until M5, the fallback for a bad skin is `_teste`, and the container runs `restart: unless-stopped` from M0. So from M1 to M4, the always-on-top pet that reacts to every Claude turn is a numbered checkerboard.
(b) A non-redistributable pack lives in the gitignored `skins-locais`. So every fresh clone, including the M7 fresh-clone check, and every other machine falls back to the checkerboard.

**Evidência:** - Plan §12: `skin-teste` is a 'QA fixture and fallback only, never shown as the character'.
- Plan §3: 'a bad skin (fallback to `_teste` …)'.
- Plan §14: default `skin = "padrao"`.
- Plan M5 'depends on the art track'.
- Compose sets `restart: unless-stopped`, and `.gitignore` excludes `skins-locais/*`.
- Plan M7 verify: 'From a fresh clone … a celebration plays'.
- The user rejected crude sketches as 'muito feias'.

**Correção:** - Render `_teste` only when `PET_DEBUG=1`, or during an explicit `tocar`/`testar`/`foto` demo that hides itself after about 10 s.
- With no approved skin, keep the pet hidden in a state `sem_personagem`, shown by `bin/bichinho estado` (and `doutor`).
- On approval, snapshot the skin into `/state`. A broken skin falls back to that snapshot, or the pet hides; never the checkerboard.
- Move T5.1/T5.4 to right after M1, so M2+ dogfooding never shows the placeholder.
- Add `bin/bichinho skin-instalar <zip|dir>` and a README step for packs that can't be redistributed, and include that step in the M7 fresh-clone check.

### [major] Privacy: screen sharing

Bubbles show project names: done, attention, '2 prontos: api, web', and the welcome-back summary. They are drawn on an OVERLAY above everything, including a monitor being shared in a meeting. Screen-share discretion is deferred until after v0.1.0, and the obvious quick fix is a trap.

**Evidência:** - Plan §10/§11 bubble templates include `<proj>`.
- Plan §21 backlog: 'discreet bubbles … during screencast>>1'.
- The user has a project at `~/Documents/agenda-presidencial`.
- Hyprland v0.56.2 posts `screencast>>1,<type>` and `screencastv2` (`ScreenshareSession.cpp` L140-141).
- `no_screen_share` is a valid layer rule (`LayerRuleEffectContainer.cpp`), but `ScreenshareFrame.cpp` L240-255 paints a BLACK box over the layer's whole geometry. For the full-output overlay, that is the entire monitor.
- grim, used by `bin/bichinho foto` and Omarchy screenshots, also opens managed screenshare sessions (`Screencopy.cpp` → `getManagedSession`).

**Correção:** - Ship this in M6 together with bubbles.
- Enter discreet mode once a screencast has been active for more than 2 s, so one-shot grim captures don't trigger it:
  - bubbles show only an icon or session colour, with no project names;
  - no welcome-back summary text;
  - attention sounds only.
- Add the option `aparencia.compartilhando = discreto|esconder|normal`.
- Add a CLAUDE.md gotcha: never put `no_screen_share` on the bichinho layer rule, because it blacks out the whole shared monitor.
- Add a screencast on/off scenario golden.

### [minor] Hooks: needs-input dedupe

One AskUserQuestion or ExitPlanMode dialog probably produces two needs-input triggers of different kinds: PreToolUse (pergunta/plano) and PermissionRequest (permissao). The plan dedupes only within the PermissionRequest/Notification group. Expect a bubble that flips from 'pergunta pra você' to 'precisa de você', a possible double L1 ping, and a '+1' for the same dialog.

**Evidência:** - Plan `hooks.json` registers PreToolUse with matcher `AskUserQuestion|ExitPlanMode` and an unfiltered PermissionRequest; plan §10 maps them to different kinds.
- Anthropic's claude-security plugin hooks `PermissionRequest` with matcher `AskUserQuestion` to deny an unanswered question after 60 s (`claude-plugins-official/plugins/claude-security/hooks/hooks.json`, `hooks.py unanswered()`). So PermissionRequest fires for the question dialog.

**Correção:** - Keep one needs_input slot per session.
- Make L1 idempotent: a second trigger within 5 s only refines the kind. When the tool is AskUserQuestion or ExitPlanMode, pergunta/plano wins over permissao.
- Golden `pergunta-dupla`: PreToolUse and PermissionRequest arrive 30 ms apart → exactly one ping and one bubble.

### [minor] Brain: stop_hook_active

The rule 'Stop with sha → turn stays open, no reaction' is backwards. Suppose another plugin's Stop hook blocks, as ralph-loop does:
1. The first Stop (sha=false) celebrates after the 0.8 s settle, because the model needs longer than that to issue its next tool call.
2. The real final Stop arrives with stop_hook_active=true and is ignored.
The pet then shows 'working' until the 5-min fallback, and the real end is never marked.

**Evidência:** - d.ts `StopHookInput.stop_hook_active`.
- Docs: 'A Stop hook can block the turn from ending, keeping Claude in the agentic loop'.
- The official marketplace's ralph-loop registers a blocking Stop hook (`plugins/ralph-loop/hooks/hooks.json`).

**Correção:** - Treat every Stop as a candidate completion.
- A sha=true Stop for a prompt_id that already celebrated re-scores the whole prompt and merges in an upgrade; play a second sound only if the tier rose. Then close the turn.
- Golden: `stop-bloqueado`.

### [minor] Brain: session lifecycle

Two sources of ghost and stale state:
- SessionEnd with reason clear or resume keeps the old sid, even though the process continues under a new session id. After every /clear, the dead session lingers in the +N and ready badges for up to 2 h.
- Esc during streaming fires neither Stop nor PostToolUseFailure, so 'working/thinking' sticks for up to 5 min.

**Evidência:** - d.ts `SessionEndInput`: '`clear` is how a hook sees a /clear: the conversation ends, the process goes on under a new session id'.
- Plan §10 SessionEnd row: 'reason ∉ {clear, resume} → remove'.
- Docs: Stop 'doesn't fire when the user presses Ctrl+C to interrupt Claude mid-response'.
- Plan timer: 'working/thinking … 5 min'.

**Correção:** - On every SessionEnd, drop that sid's ready, needs_input and turn. Only the `bye` reaction depends on the reason.
- Close an open turn without celebrating when idle_prompt arrives for that session.
- Also close it when the focused Claude terminal's title flips from ◐/◑ to ✳ and exactly one session is working.

### [minor] Sound UX: muting

The user wants sound on but mutable, yet there is no direct mute on the pet. Mute exists only in the CLI and config. DND is sampled only when a hook fires, and during a needs-input escalation Claude is blocked, so no events arrive. A user who toggles Omarchy DND to stop ping2 or trinado (for example, on joining a call) is ignored until the next event.

**Evidência:** - Plan §14: CLI commands `mudo`, `som`, `soneca`.
- Plan §8: a click acknowledges and snoozes for 5 min.
- Plan §21: the right-click menu is backlog.
- Decision: 'DND is sampled by the hook on every event … mute and click-acknowledge cover the rare case'.

**Correção:** - In M3, right-click on the pet toggles a persisted mute and shows a visible 🔇 badge.
- Middle-click or long-press snoozes for 30 min.
- Document that DND takes effect at the next Claude event, while click and right-click take effect immediately.

### [minor] Privacy beyond hook payloads

The privacy proof covers only `avisar.sh`.
(a) Through socket2 (activewindow, windowtitlev2, openwindow), the hypr thread receives every window title on the desktop: browser tabs, mail subjects, chat previews. The dev compose logs at debug level into json-file (5×10 MB), and nothing forbids logging raw lines.
(b) `bin/bichinho eventos --salvar` writes real session ids, prompt ids and project basenames into `cenarios/*.jsonl`, which are committed and pushed to GitHub.

**Evidência:** - Plan §7: the hypr reader drains all events.
- Plan §4 dev compose sets `PET_LOG: debug`.
- Plan §16: canaries cover only `avisar.sh`.
- Plan §14/§16: `eventos --salvar cenarios/<caso>.jsonl` is the workflow for recording a strange behaviour.

**Correção:** - The hypr parser keeps only event names, monitor names, window addresses and the first glyph of the title, and never logs payloads.
- Add a canary that injects `activewindow>>firefox,SEGREDO-T` and `windowtitlev2>>…,SEGREDO-T`, then asserts no SEGREDO appears in the logs, `/v1/estado` or `/v1/debug/eventos`.
- `eventos --salvar` pseudonymises sid, turno, proj and arq (s1, t1, proj-a, f1) before writing.

### [minor] Animation: attention economy

Constant motion wears down the motion-onset contrast that makes the pet noticeable.
- `working` loops for the whole turn, often 10–30 min, and the skin.json example maps it to `walk`.
- `ready` (attention 1) loops until seen, for up to 2 h, and outranks sleep. The pet never yawns, sleeps or reaches zero-commit deep sleep while anything is unacknowledged.

**Evidência:** - Plan §12: skin.json example `"working": ["walk", "idle"]`.
- Plan §10: display priority puts ready > working > … > sleep; the ready badge lasts up to 2 h; deep sleep starts only after 30 min of sleep.
- Research digest: 'attention = motion ONSET bursts (not constant motion)'.

**Correção:** - working and thinking use near-static poses: at most 4 fps and at most 1 art-pixel of movement, plus micro-actions from a shuffle-bag every 10–30 s. Never walk.
- After about 2 min, `ready` decays to a static flag badge, and the pet can yawn and sleep with the badge still shown.
- Keep big motion for state onsets.
- Add a golden budget for `commits_por_min` over a 20-min working state.

### [minor] Placement: default home and per-monitor memory

The default home is right edge, feet at 40% of the height, body 96–160 logical px. That floats in the middle of the single full-screen foot window Omarchy tiles on the 1280x800 laptop, right over the text the user is reading.

Each re-home also destroys and recreates the surface, which makes it the newest OVERLAY layer. It then stacks above Omarchy's notification toasts (top-right) and menus until the optional T7.2 order rule lands.

Per-monitor memory is keyed by connector name. That breaks on docks, where DP-N names change between plugs.

**Evidência:** - `hyprctl -j clients`: foot at [640,0], size [1280,800], covering all of eDP-1.
- omarchy-notifications is a full-output Overlay with toasts anchored top-right (`Service.qml` L961-985).
- Plan §7 step 5 sets the default; plan §20 makes the layer rule optional.
- `hyprctl -j monitors` gives a description: 'Chimei Innolux Corporation 0x1459'.

**Correção:** - Default to standing on a 'floor' at the bottom-right with an 8–16 px margin, which also fits the land/queda physics.
- Use a smaller default N on outputs no wider than 1440 logical px.
- Propose the layer rule (order=1, no_anim) in M3, with the user's consent.
- Key saved positions by output description (make/model/serial), falling back to the connector name.

### [minor] Verification: test isolation

`simular` and `testar` inject into the live daemon, which shares one brain and one global sound arbiter with the real Claude session running the test. That session's own events arrive during the 90 s escalation check, and its end-of-turn Stop triggers a real celebration. Assertions on `som.historico`, such as 'exactly one fanfarra' or '3 sounds at most', become flaky. Synthetic sessions (sid t1) never get a SessionEnd, so their ready flags and escalations linger for up to 2 h during development.

**Evidência:** - Plan §14: `testar` sends 'synthetic hook payloads through plugin/scripts/avisar.sh', and `simular` replays into the daemon.
- Plan M4 verify asserts on `/v1/estado.som.historico`.
- Plan §10: the arbiter is global.

**Correção:** - `simular` runs scenarios in `bichinho-core` with the fake clock and its own brain and arbiter. The live daemon only renders the resulting intents (`/v1/comando demo`).
- Synthetic events carry `teste:true` and a 60 s TTL, and are never merged or escalated with real sessions.
- Live assertions filter `som.historico` by source.

### [minor] Plugin install: in-place coupling

The local-directory marketplace reads `plugin/` in place. Every Claude session on the machine therefore runs whatever is in the repo's working tree, on whichever milestone branch is checked out. A half-edited `hooks.json`, or a WIP `avisar.sh` with a privacy regression not yet caught by commit-time canaries, reaches sessions in other projects at their next start or `/reload-plugins`.

**Evidência:** - Docs, 'Test an edit to a plugin': with a local-directory marketplace, 'Claude Code reads the plugin's files directly … Your edits take effect at the next session start or when you run /reload-plugins'.
- Plan §18: one branch per milestone, worked in `~/Documents/bichinho`.

**Correção:** - Register the marketplace from a stable worktree that tracks main (`git worktree add ~/.local/share/bichinho/estavel main`).
- Refresh it after each merge with `bin/bichinho plugin-atualizar`, which also reminds the user to run `/reload-plugins`.
- Validate WIP hook changes only through the `avisar.sh` fixture/canary harness before merging.

### [minor] Multi-session acknowledgement

One click acknowledges every ready badge and every pending input in all sessions, and snoozes escalation for 5 min. Clicking to dismiss session A's celebration therefore swallows session B's unanswered AskUserQuestion. B's escalation never runs, and its question is easy to miss.

**Evidência:** - Plan §8: a click 'acknowledges every ready badge and pending input; snoozes escalation for 5 min'.
- Plan §10 has the same row for click.

**Correção:** - A click acknowledges and snoozes only the item currently shown (the loudest, whose bubble names its project).
- Other sessions' pending inputs stay as +N and resume escalating from L1 when they become the loudest.
- needs_input is cleared only by that session's own events.

### [minor] Onboarding: first 5 minutes

After `docker compose up -d`, which includes a multi-minute first Rust build, the pet just idles: it yawns at 3 min and sleeps at 8. Nothing says whether the plugin is installed, whether open sessions have run `/reload-plugins`, or whether events are arriving. A missing plugin looks the same as a quiet Claude.

**Evidência:** - Plan §10: reactions come only from hook events; the SessionStart hello fires only for new sessions.
- Plugin install is a separate M2 step, and open sessions need `/reload-plugins`.
- `bin/bichinho estado` is a raw dump.
- `claude plugin list --json` exists in 2.1.288.

**Correção:** - On first map, play hello with an 'oi! me arraste' bubble.
- If no hook event has arrived 5 min after start, show one hint bubble: 'sem sinal do Claude Code — rode bin/bichinho doutor'.
- Add `bin/bichinho doutor`. It checks:
  - container health;
  - the Wayland and hypr connections;
  - that `bichinho@bichinho` is enabled in `claude plugin list --json`;
  - the age of the last event per session;
  - DND and the paplay sink;
  - the skin approval state.

