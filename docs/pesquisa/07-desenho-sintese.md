# Desenho: painel de três propostas e síntese

> Três planos (lean-robust, delight, evolvability) julgados em 2026-10-02. O PLANO.md do repo é a versão aprovada; isto é o material de origem.

## Notas do juiz

- **lean-robust** — requisitos 8, robustez 9, encanto 7, evolução 7, footprint 10, risco 7. Strongest engineering backbone. Its Hyprland source claims hold at v0.56.2: SHM buffers are dropped right after commit (Compositor.cpp dropCurrentBuffer); the texture is updated in place with accumulated damage (SurfaceState::updateSynchronousTexture); a NULL output places the layer on focusState()->monitor(); enter is sent on map; the viewport destination is used as layer geometry under fractional scale. It has the smallest footprint (static musl binary, about 25 MB image) and the fewest moving parts. There are no boot-fragile binds and the daemon never opens .socket.sock. Gaps: (1) No AskUserQuestion/ExitPlanMode attention signal. The user runs claude --dangerously-skip-permissions, so these are the real 'Claude needs you' moments. (2) The PostToolUse matcher is too narrow: a permission escalation on a non-mutating tool never clears until Stop. (3) prompt_id, background_tasks and UserPromptSubmit.source are ignored. (4) DND is sampled on only 3 events. (5) Unioning damage above 8 rects is too coarse for screen-wide confetti. (6) It passes a name to 'hyprctl output create headless BICHO', but the command accepts no name. (7) Thinner delight: no drag physics, no presence-based replay, no recipes for packs that lack states.
- **delight** — requisitos 9, robustez 6, encanto 10, evolução 6, footprint 5, risco 4. The richest attention and UX design. It also has the best-verified Claude Code facts: prompt_id, background_tasks, UserPromptSubmit.source and the StopFailure.error enum are all confirmed in the bundled 2.1.288 d.ts. It also saw that permission prompts are rare under --dangerously-skip-permissions. Downsides: (1) Largest scope and most moving parts: a Python supervisor, a GTK child, swayidle, and a hook-side PID walk that calls hyprctl and git. (2) It hooks 17 events, including every PreToolUse, and the container queries the request socket. (3) It bind-mounts ~/.local/state/omarchy read-only, which would also expose clipboard-history.json to the container. (4) GTK cairo crispness at scale 1.5 and full-output buffer memory are unverified, though gated in M0. (5) elicitation_response and elicitation_complete are not notification types in the 2.1.288 list. (6) The scope makes a usable v0.1 the least likely of the three.
- **evolvability** — requisitos 8, robustez 7, encanto 7, evolução 9, footprint 6, risco 6. The best development process: a pure reducer, golden scenarios recorded from real sessions, canary privacy tests, project skills, one verificar gate, and protocol and state versioning. Its claim that a local-directory marketplace loads in place is confirmed by the docs. Weaknesses: (1) The forwarder reads StopFailure.error_type, but 2.1.288 sends error (an enum), so error reactions would never fire. (2) The quota_* notification names do not exist; the real ones are quota_auto_resume_*. (3) It keeps one full-output GTK overlay mapped per monitor, including a 33 MB 4K buffer that may be double-buffered. (4) It spawns a Python process for every PreToolUse and PostToolUse. (5) GTK crispness is unverified. (6) It names the headless output in hyprctl output create, which the command does not accept. (7) The walking skeleton only arrives at M3 of M0-M10.

**Vencedor:** lean-robust

## Decisões da síntese

- **Engine: one static Rust binary on smithay-client-toolkit 0.21.1 (pure-Rust wayland-client), in an alpine:3.24.2 image whose only extra package is pulseaudio-utils. Plan B, only if the M1 GO/NO-GO gate fails, is gtk4-rs + gtk4-layer-shell; the core crate stays the same.** — Pixel crispness is a hard requirement and the user's art bar is high. Writing device pixels directly (D = round(N*s)) makes the art crisp at any scale, whereas GTK cairo at 1.5 is unverified. The Hyprland v0.56.2 behaviours this relies on were checked in source: SHM buffer dropped after commit, damage-only texture update, viewport-destination geometry. It has the smallest image and RSS on a 7.4 GiB laptop, can reconnect in-process, and needs no LD_PRELOAD or D-Bus neutralisation. The user already keeps a Rust workspace with a pure core + xtask, and rustc 1.98.1 is installed. Quickshell, ranked first by the research, was rejected because its GL path damages the full surface every frame on a full-output overlay and its image is about 1 GiB.
- **Process model: a single daemon process with threads (calloop main, ingress, hypr, watchdog) and a paplay child. Expected failures are handled in-process; bugs panic and abort, and a 60 s watchdog also aborts, leaving recovery to Docker's restart policy.** — SCTK can drop and rebuild the Wayland connection inside the process, so the Python supervisor plus renderer child used by the other plans is unnecessary. This is the fewest moving parts with crash-only recovery, and state persisted in /state covers restarts.
- **Lifecycle: restart: unless-stopped plus in-process discovery (newest hypr/<HIS> by its epoch field, read hyprland.lock, probe liveness with connect()). /run/user is bound read-only with rslave and create_host_path false; sockets are opened by absolute path. A systemd --user unit is documented in the README only for hosts that enable only docker.socket.** — On this host docker.service is enabled and Linger=yes, so the container runs from boot and survives logout/login, compositor crashes, suspend and hotplug without any session unit. Binding /run/user avoids the boot race and stale socket inodes that binding /run/user/1000 or individual sockets would cause.
- **Surface: a single fixed, full-output, transparent OVERLAY layer surface, only on the focused monitor (namespace bichinho, anchored to all 4 edges, exclusive_zone -1, keyboard NONE). It is created with a NULL output and a 1x1 transparent bootstrap buffer. The input region covers the hitbox plus the bubble, widened to the whole surface while the button is held. There is no per-monitor overlay set, no resizing and no subsurfaces.** — OVERLAY is drawn and hit-tested above fullscreen windows. Moving content inside one buffer avoids full-monitor damage and layout recalculation on every commit, avoids the stale-buffer stretch on resize (Omarchy's toast comment), and avoids the subsurface squish bug (#10515). Hyprland places a NULL-output layer on the focused monitor, which removes name-matching races. Keeping one surface instead of one per monitor avoids a second permanent buffer of about 33 MB on the 4K display.
- **Monitor following uses only socket2 focusedmonv2, debounced 300 ms, frozen while dragging, with at least 1.5 s between travels. Re-home on the layer's closed event after a 250 ms settle. Guard against FALLBACK and 0x0 outputs, and ignore configreloaded. The daemon never opens .socket.sock; hyprctl is used only by host-side test scripts.** — The request socket can execute commands on the host and freezes Hyprland for up to 5 s if a client stalls. Events plus Wayland are enough for every runtime need. The debounce absorbs the Omarchy screensaver's focus burst and quick mouse crossings.
- **Ingress: HTTP on 127.0.0.1:27380 published from the container. POST /v1/evento checks Host, Content-Type and X-Pet and caps the body at 8 KiB, then returns 204. The other endpoints are /v1/comando, /v1/estado, /saude, and /v1/debug/eventos (debug only).** — Async command hooks can reach it with curl, which is always present. Header checks defeat DNS rebinding and CSRF from browsers. http-type hooks were rejected because they add transcript noise when the pet is down and cannot be async. A unix socket was rejected because it needs a host directory that must exist before boot.
- **Claude Code plugin bichinho lives in the same repo with 13 async shell-form command hooks: SessionStart, UserPromptSubmit, PreToolUse (only AskUserQuestion and ExitPlanMode), PostToolUse (all tools), PostToolUseFailure, PermissionRequest, Notification (matcher-filtered), SubagentStart, PreCompact, PostCompact, Stop, StopFailure, SessionEnd. It is installed from the local clone as a directory marketplace.** — The user runs --dangerously-skip-permissions, so AskUserQuestion and ExitPlanMode are the real needs-input signals. An unfiltered PostToolUse is needed to clear escalations whatever the tool. Leaving SubagentStop unhooked means subagents never celebrate. The docs confirm a local-directory marketplace is read in place, so edits apply after /reload-plugins without a version bump. Shell form matches the official plugins on disk.
- **The forwarder is avisar.sh (POSIX sh + jq + curl -m 2) and always exits 0. It whitelists metadata only: event, ts, session id, prompt_id, an agente flag, tool name, notification type, the StopFailure error enum, source, reason, interrupt flag, stop_hook_active, background_tasks count, project basename, a sha256 prefix of edited paths, and DND read on the host.** — Hook payloads contain prompts, code and assistant text. Forwarding only allowlisted fields and hashing paths keeps all content on the host, and canary tests prove it. jq is a dependency of omarchy and curl of pacman. It uses 2.1.288's verified field names; the evolvability plan's error_type field does not exist.
- **Celebration intensity is proportional to the work in the turn. score = active minutes (user waits excluded) + 0.15 per mutating tool + 0.05 per other tool + 0.5 per unique file + 1.0 per subagent, capped at 20. Tiers: T0 silent nod, T1 below 4, T2 from 4 to 12, T3 at 12 or more (at most once per 10 min). Machine-started turns are capped at T1 and silent. Background-task Stops chain into the final one. A 0.8 s settle, a 3 s merge window and a per-turn Stop dedupe apply. Every knob is config and every score is logged.** — This implements the user's pending default: a big party for real work and a small hop for quick answers. It avoids celebrating paused background work or machine turns, and gives logged scores for the tuning week.
- **Needs-input escalation is bounded: L1 at 0 s, L2 at 30 s, L3 at 90 s, capped at L4 (silent) after 5 min, with one global clock and one sound per step. A global sound arbiter allows one sound at a time, at least 1.5 s apart, at most 6 per minute, with cooldowns. Sound is ON by default and can be muted, snoozed, silenced by Omarchy DND, or limited by quiet hours. idle_prompt, T0, subagents, drag and travel never make sound.** — Motion onset grabs attention without nagging. These rules avoid the annoyances documented by PeonPing users (repeated idle_prompt sounds, overlapping sounds, subagent noise) while honouring the user's 'sound on, but mutable'.
- **DND is sampled by the hook on every event (it reads ~/.local/state/omarchy/notifications.json on the host). That directory is never bound into the container.** — Verified: that directory also holds clipboard-history.json. Reading the file on the host keeps clipboard history out of the container and avoids a boot-fragile bind. Events are frequent enough while Claude works; mute and click-acknowledge cover the rare case of toggling DND mid-escalation.
- **Timezone: bind /etc/localtime read-only and never set TZ. Local time is used only once quiet hours ship (jiff reading /etc/localtime).** — The host runs America/New_York while the user's other projects use Sao_Paulo, so neither may be assumed. The bind always exists, so there is no boot risk, and it is ready for quiet hours and log times.
- **Configuration: config/bichinho.toml (gitignored, template exemplo.toml, bind-mounted read-only, hot-reloaded) for every tunable. Precedence: runtime commands persisted in /state, then PET_* env vars, then the file, then code defaults. /v1/estado shows each value and its source.** — Tuning weights, thresholds and timings is an iterative job; a visible file with precedence beats a pile of environment variables and needs no container recreation.
- **Skin = sheet.png + an Aseprite json-array sheet.json + skin.json. skin.json holds semantic-to-tag fallback chains, variants, key poses for recipes, hitbox, feet pivot, default scale, FX palette and licence. Built-in recipes synthesise missing states. xtask provides the importer (Aseprite and strips), lints, a coverage report, contact sheets with GIF previews, and a checkerboard QA skin. Packs that cannot be redistributed go in a gitignored skins-locais folder that is baked only into the local image.** — The character comes from a separate art track (most likely a professional pack). With recipes, a pack with only idle, jump and hurt covers every state. Contact sheets let the user approve the art, and the checkerboard is never shown as the character. Licences are respected without committing restricted art.
- **Presence comes from ext_idle_notifier_v1 bound inside the same Wayland session (Hyprland exposes it at version 2). It drives faster sleep while the user is away and a single welcome-back replay.** — This keeps delight's best attention idea, a reminder the user actually sees when they come back, without the extra swayidle process or any request-socket polling.
- **Testing: cargo tests (brain tables, score examples, arbiter properties, raster and golden frames, scenario-replay goldens with a fake clock, ingress, the hypr parser, discovery); privacy canary tests that run the real avisar.sh; a live scripts/verificar.sh (layer placement, 6x6-block grim crispness, hook round trip, budgets); an opt-in headless-output e2e that cleans up after itself; a manual checklist. bin/bichinho verificar is the single pre-commit gate.** — Behaviour changes show up as reviewable golden diffs, privacy is proven rather than assumed, and anything that touches the compositor runs only with consent.
- **Milestones go risk-first: M0 foundation; M1 crisp overlay (GO/NO-GO); M2 walking skeleton (hooks to reaction); M3 drag and follow; M4 full brain and sound; M5 real character (pulled forward when the art arrives); M6 delight and attention; M7 polish and v0.1.0. Each milestone is a branch and a PR that the user reviews.** — The highest technical risk (crispness and the Wayland plumbing) is settled first, a usable celebration loop arrives by M2, and human gates sit exactly where taste matters: crispness, art and feel.
- **Conventions: PT-BR docs, comments, commits, CLI and config; English Rust identifiers and semantic keys. Root-level DECISIONS.md in balcao format (NNNN, Problema / Escolha / Por quê), PROGRESS.md with one row per task, PLANO.md. Commits are plain PT-BR sentences citing (Tn.m) and (decisão NNNN) and end with the session's Co-Authored-By trailer. Project skills live in .claude/skills.** — Mirrors the user's most recent projects (balcao, agenda) and the-light's split between code and docs language. The project skills turn recurring workflows into repeatable recipes for future Claude Code sessions.

## Ideias enxertadas

- [from delight] AskUserQuestion and ExitPlanMode as needs-input triggers through a PreToolUse matcher limited to those two tools; essential because the user runs --dangerously-skip-permissions.
- [from delight] prompt_id as the turn key; agent_id used to count subagent tool calls without letting them change session state.
- [from delight] Stop.background_tasks chains turns: no 'done' while work is paused, a '…' badge, and the final Stop celebrates the summed score.
- [from delight] UserPromptSubmit.source caps machine-started turns (system, loop_wakeup, schedule_wakeup, poll_event) at T1 and silent.
- [from delight] The verified StopFailure.error enum, mapped to error, tired (rate_limit) and sad_key (auth or billing).
- [from delight] A 0.8 s settle after Stop, cancelled by more activity from the same session, to absorb stop_hook_active loops.
- [from delight] Active minutes exclude time spent waiting on the user; unique edited files counted through a sha256 prefix computed on the host.
- [from delight] Built-in motion recipes (pulo, salto_giro, danca, queda, pendurado, aceno) build missing states from a pack's key poses, plus a coverage report listing what is native, recipe, fallback or missing.
- [from delight] Drag physics: dangle spring with lean frames, plop with rebound, dust; fling to dizzy later.
- [from delight] T3 full-screen moments drawn inside the overlay (fogos, chuva de confete, holofote), with shuffle-bag variants and at least 1.5 s between monitor travels.
- [from delight] User-returned replay; implemented in-process with ext_idle_notifier_v1 instead of a swayidle child.
- [from delight, backlog] Ready-until-seen via a hook-side PID walk to the foot window plus socket2 activewindowv2; screen-share discreet mode; click-spam chain; mini-clones.
- [from evolvability] Scenario JSONL replays with expected-intent goldens, recorded from real sessions through /v1/debug/eventos and bin/bichinho eventos --salvar.
- [from evolvability] Privacy canary tests that run the real avisar.sh against fixtures carrying SEGREDO strings in every content field.
- [from evolvability] One pre-commit gate (bin/bichinho verificar) plus foto and simular, so Claude Code can check its own visual and behavioural work.
- [from evolvability] TOML config with precedence (runtime, env, file, defaults), hot reload, and value sources shown in /v1/estado.
- [from evolvability] Project skills in .claude/skills (nova-animacao, novo-evento-hook, conferir-na-tela) that encode recurring workflows.
- [from evolvability] Local-directory marketplace read in place (confirmed in the docs), and a thin, stable plugin whose hook set is fixed once in M2.
- [from evolvability] Walking skeleton (hooks to reaction) right after the overlay spike, and one PR per milestone as the user's review gate.
- [from evolvability] The balcao DECISIONS format (Problema / Escolha / Por quê) and plain Portuguese commit sentences citing task and decision ids.
- [new in synthesis] Damage sent as a list of up to 128 rects instead of a union, so full-screen confetti stays cheap; e2e reads the headless output name from hyprctl -j monitors because create takes no name; DND read on the host so clipboard-history.json never reaches the container.

## Plano sintetizado (original, em inglês)

# Bichinho: synthesized implementation plan

**Backbone: lean-robust.** Ideas grafted from the other plans are tagged **[D]** (delight) and **[E]** (evolvability).

Claude Code executes this plan task by task. Every milestone ends with an end-to-end verification. Milestones marked **(human gate)** also need explicit user approval.

---

## 0. Verdict

**Keep lean-robust's backbone:**
- **One static Rust binary.** Built on smithay-client-toolkit (SCTK) 0.21.1 with the pure-Rust wayland-client. The image is `alpine:3.24.2`, and its only extra package is `pulseaudio-utils`.
- **One surface.** A fixed, full-output, transparent OVERLAY layer surface on the focused monitor. The sprite moves inside it. Pixels are written directly in device pixels, so they stay crisp at any scale.
- **Hyprland events only.** The daemon reads `.socket2.sock`. It never opens the exec-capable `.socket.sock`.
- **Lifecycle.** `restart: unless-stopped`, plus discovery and reconnect inside the process, plus a watchdog.
- **Claude Code integration.** A plugin of async command hooks. Its script forwards **metadata only** to `127.0.0.1:27380`.

**Graft:**
- **[D] Attention signals that actually fire for this user.** The user runs `claude --dangerously-skip-permissions`, so the signals that matter are AskUserQuestion and ExitPlanMode, not permission prompts.
- **[D] Turn and error handling:**
  - `prompt_id` turns;
  - background-task chains;
  - machine-turn caps;
  - the verified `StopFailure.error` enum;
  - a short settle window after Stop.
- **[D] Animation richness:**
  - motion recipes that build missing states from a pack's key poses;
  - drag physics;
  - full-screen T3 moments;
  - "user returned" replay, done in-process with `ext_idle_notifier_v1` instead of a swayidle child.
- **[E] Process and testing:**
  - scenario goldens recorded from real sessions;
  - canary privacy tests;
  - one `verificar` gate;
  - a TOML config with visible precedence;
  - project skills that teach Claude Code the recurring workflows;
  - the in-place local marketplace;
  - one PR per milestone as the review gate.

---

## 1. Facts re-verified during judging (2026-10-02, read-only)

| Fact | Evidence |
|---|---|
| `BaseHookInput` has `prompt_id` (it links a prompt to all later events). `agent_id` is present only inside subagents. | bundled 2.1.288 `plugin-authoring/types/claude-code.d.ts` (BaseHookInput) |
| `StopFailure.error` is an enum (authentication_failed, oauth_org_not_allowed, account_on_hold, verification_required, billing_error, rate_limit, overloaded, invalid_request, model_not_found, server_error, unknown, max_output_tokens, cloud_credential_error). There is **no** `error_type`. | same d.ts (StopFailureHookInput, SDKAssistantMessageError) |
| `Stop` carries `stop_hook_active`, `last_assistant_message`, `background_tasks[]` and `session_crons[]`. | same d.ts |
| `UserPromptSubmit` carries `prompt`, `source` (user, sdk, system, loop_wakeup, schedule_wakeup, poll_event) and `session_title`. | same d.ts |
| `PostToolUseFailure` carries `error` (free text), `is_interrupt` and `duration_ms`. | same d.ts |
| The 2.1.288 notification types are: permission_prompt, idle_prompt, auth_success, elicitation_dialog, agent_needs_input, agent_completed, elicitation_url_dialog, worker_permission_prompt, push_notification, computer_use_enter, computer_use_exit, quota_auto_resume_fired, quota_auto_resume_stale, quota_auto_resume_disabled, model_refusal_fallback, auth_storage_failure. | string table in the binary |
| Hook mechanics:<br>- `async` exists only on command hooks;<br>- an async hook's timeout is not enforced, and its non-zero exits are not reported;<br>- hooks inherit the Claude process environment;<br>- an omitted matcher matches every occurrence;<br>- an exec form (`command` + `args`) also exists. | code.claude.com/docs/en/hooks |
| A marketplace added from a local directory reads a relative-path plugin in place. Edits apply at the next session or after `/reload-plugins`, with no version bump. | code.claude.com/docs/en/plugin-marketplaces, "Test an edit to a plugin" |
| The official plugins on disk use shell-form hooks (`sh "${CLAUDE_PLUGIN_ROOT}/..." arg \|\| true`) with `async`. Their marketplace.json has `$schema`. | `~/.claude/plugins/marketplaces/claude-plugins-official` |
| Hyprland v0.56.2 behaviour:<br>- an SHM buffer is dropped right after commit;<br>- the texture is updated in place from the accumulated damage;<br>- a NULL output puts the layer on `focusState()->monitor()`;<br>- `enter` is sent on map;<br>- under fractional scale, the viewport destination becomes the layer geometry. | Compositor.cpp, SurfaceState.cpp, LayerSurface.cpp at tag v0.56.2 (`gh api`) |
| Hyprland v0.56.2 globals: fractional-scale v1, viewporter v1, cursor-shape v2, ext-idle-notify v2, layer-shell v5. | ProtocolManager.cpp at tag v0.56.2 |
| `hyprctl output create <backend>` takes **no name**. The new output's name must be read from `hyprctl -j monitors`. | `hyprctl output --help` |
| `rust:1.98.1-alpine3.24` exists. | Docker Hub |
| Host rustc and cargo are 1.98.1 in `~/.cargo/bin`, which is **not on PATH**. | `~/.cargo/bin/rustc --version` |
| SCTK 0.21.1 default features are `calloop` and `xkbcommon`. `calloop` pulls in calloop-wayland-source. SCTK depends on wayland-protocols 0.32.9 with `staging`. | SCTK Cargo.toml @v0.21.1 |
| `jq` is required by the omarchy package; `curl` is required by pacman. The host has no pytest, Pillow or shellcheck. | `pacman -Qi`, imports |
| `~/.local/state/omarchy/` contains `clipboard-history.json` next to `notifications.json`, so that directory must **never** be bound into the container. `notifications.json` is absent, which means DND is off. | `ls` |
| Omarchy's toasts use a fixed-size full-screen overlay "so the compositor can't briefly scale a stale buffer". | `/usr/share/omarchy/shell/plugins/notifications/Service.qml` |
| User conventions:<br>- commits are plain PT-BR sentences (balcao, agenda);<br>- DECISIONS entries look like `NNNN — Título (data)` with Problema / Escolha / Por quê;<br>- PROGRESS has one table row per task. | `git log`, `~/Documents/balcao/docs/` |

---

## 2. Names, language, conventions

**Working name `bichinho`** (still pending with the user):
- Repo `butkeraites/bichinho`, private, cloned at `~/Documents/bichinho`.
- Cargo binary `bichinho`.
- Compose project `bichinho`, service `pet`, image `bichinho:local`, named volume `estado`.
- Layer namespace `bichinho`.
- Plugin and marketplace both `bichinho`; the install id is `bichinho@bichinho`.

**Rename-proof names.** Environment variables use the prefix `PET_`, and the ingress header is `X-Pet: 1`, so neither changes if the project is renamed.

**Renaming touches:**
- the repo;
- `Cargo.toml` (package and bin);
- compose `name` and `image`;
- the `NAMESPACE` constant;
- the plugin and marketplace names;
- the CLI symlink.

**Language:**
- Portuguese: docs, comments, commit messages, bubbles, CLI subcommands, config keys.
- English: Rust identifiers and skin semantic keys (`idle`, `done_small`, ...), following the-light.

**Docs at the repo root:**
- `CLAUDE.md`.
- `README.md`.
- `DECISIONS.md`: entries `## NNNN — Título (AAAA-MM-DD)` with **Problema / Escolha / Por quê**. Changes are appended, never rewritten.
- `PROGRESS.md`: one table `| Data | Tarefa | O quê | Commit |` with one row per task.
- `PLANO.md`: this plan in Portuguese, with task IDs, acceptance criteria and verification.

**Commits:**
- One plain Portuguese sentence, citing `(T2.3)` or `(decisão 0007)` when relevant.
- Ends with the trailer `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- PR bodies end with `🤖 Generated with [Claude Code](https://claude.com/claude-code)`.

---

## 3. Architecture

```
HOST (Hyprland 0.56.2, uid 1000)                           CONTAINER bichinho-pet-1 (alpine 3.24.2, uid 1000, ro rootfs)
claude (foot) + plugin bichinho                             PID1 docker-init (tini, init: true)
  async hook: sh avisar.sh <Evento>                          └ bichinho rodar   (one static process)
    jq whitelist -> curl -m 2 POST ---docker-proxy--->         main thread (calloop): brain, animator, scene, Wayland session, timers, store
      127.0.0.1:27380/v1/evento                                ingress thread: HTTP 0.0.0.0:27380
/run/user --bind ro, rslave, create_host_path:false-->         hypr thread: .socket2.sock reader (one per compositor session)
  1000/wayland-1                <- 1 OVERLAY layer surface     watchdog thread: heartbeat older than 60 s -> abort() -> Docker restarts
  1000/hypr/<HIS>/.socket2.sock <- events only                 paplay child: at most one sound
  1000/pulse/native             <- paplay (PULSE_SERVER)
/etc/localtime (ro), ./config -> /etc/bichinho (ro), volume estado -> /state
```

| Thread | Job | Failure behaviour |
|---|---|---|
| main (calloop) | brain, animator, scene, Wayland session (layer, pointer, cursor shape, fractional scale, viewporter, ext-idle-notify), timers, store | Wayland error or EOF: tear down, go to Waiting with backoff 1 s doubling to 30 s (reset after 60 s alive). Panic: abort. |
| ingress | HTTP; validation; `/saude` answered from atomics | 1 s per-connection timeouts; bounded channel of 256, 503 when full |
| hypr | parse socket2 lines, filter, track screensaver window addresses | EOF: report hypr lost; main retries every 2 s while Wayland is alive |
| watchdog | heartbeat older than 60 s: `abort()` | Docker restarts the container (backoff 100 ms doubling to 1 min) |
| paplay child | one sound | killed after 5 s; reaped by a 250 ms timer |

**Crash policy:**
- Expected failures are handled in-process: compositor gone, socket2 EOF, audio failure, a bad skin (fallback to `_teste`, warning in `/v1/estado`).
- Bugs `panic` with `panic = "abort"`, and Docker's restart policy brings the container back.
- `/healthz`-style health is informational only; Docker never restarts an unhealthy container.

**Latency target:** under 150 ms from hook to the first frame of the reaction.

---

## 4. Docker

### `Dockerfile`

```dockerfile
# syntax=docker/dockerfile:1
# Fixar @sha256 das duas bases no primeiro build (docker buildx imagetools inspect) e registrar em DECISIONS.md.
FROM rust:1.98.1-alpine3.24 AS build
RUN apk add --no-cache musl-dev
WORKDIR /src
# Sem rust-toolchain.toml aqui: usa o toolchain da imagem (nenhum download no build).
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY xtask ./xtask
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked -p bichinho \
 && install -Dm0755 target/release/bichinho /out/bichinho

FROM alpine:3.24.2 AS runtime
ARG APP_UID=1000
ARG APP_GID=1000
RUN set -eux; apk add --no-cache pulseaudio-utils; \
    addgroup -g "$APP_GID" pet; \
    adduser -D -H -u "$APP_UID" -G pet -h /state -s /sbin/nologin pet; \
    install -d -o "$APP_UID" -g "$APP_GID" -m 0750 /state
COPY --from=build /out/bichinho /usr/local/bin/bichinho
COPY assets/ /opt/bichinho/assets/
COPY skins/ /opt/bichinho/skins/
# Pacotes de arte não redistribuíveis: gitignored, mas entram só na imagem LOCAL (nunca no git).
COPY skins-locais/ /opt/bichinho/skins-locais/
ENV PET_HOST_RUNTIME=/host/run/user PET_ESCUTA=0.0.0.0:27380 PET_ASSETS=/opt/bichinho/assets \
    PET_SKINS=/opt/bichinho/skins:/opt/bichinho/skins-locais PET_ESTADO=/state PET_CONFIG=/etc/bichinho \
    HOME=/tmp XDG_RUNTIME_DIR=/tmp/xdg
USER ${APP_UID}:${APP_GID}
EXPOSE 27380
ENTRYPOINT ["/usr/local/bin/bichinho"]
CMD ["rodar"]
```

- **No entrypoint script.** The binary creates `/tmp/xdg` (mode 0700) and handles SIGTERM.
- **No Mesa and no `/dev/dri`.** We write pixels into SHM, and Hyprland's GPU does the compositing.
- **`.dockerignore`:** `target/`, `.git/`, `tmp/`, `docs/`, `cenarios/`, `*.md`. It must **not** exclude `skins-locais/`.

### `docker-compose.yml`

```yaml
name: bichinho

# Sem isto o json-file cresce sem limite.
x-log: &log
  logging:
    driver: json-file
    options: {max-size: "10m", max-file: "5"}

services:
  pet:
    <<: *log
    image: bichinho:local
    build:
      context: .
      args: {APP_UID: "${APP_UID:-1000}", APP_GID: "${APP_GID:-1000}"}
    # O binário espera o Hyprland e reconecta sozinho: vale desde o boot (docker.service + linger),
    # sobrevive a logout/login, crash do compositor, suspensão e hotplug, sem unit do systemd.
    restart: unless-stopped
    init: true
    user: "${APP_UID:-1000}:${APP_GID:-1000}"   # TEM que ser o dono de wayland-1 e hypr/ (0700)
    read_only: true
    cap_drop: [ALL]
    security_opt: ["no-new-privileges:true"]
    pids_limit: 64
    mem_limit: 128m
    cpus: 0.5
    stop_grace_period: 5s
    ports:
      - "127.0.0.1:${PET_PORTA:-27380}:27380"   # só loopback: é por aqui que os hooks chegam
    environment:
      PET_LOG: ${PET_LOG:-info}
      PET_PORTA_PUBLICA: ${PET_PORTA:-27380}     # para validar o cabeçalho Host
    tmpfs:
      - /tmp:size=16m,mode=1777
    volumes:
      # /run/user (NUNCA /run/user/1000 nem sockets avulsos): existe desde o boot; rslave propaga o
      # tmpfs do usuário; connect() funciona mesmo com bind somente-leitura.
      - type: bind
        source: /run/user
        target: /host/run/user
        read_only: true
        bind: {propagation: rslave, create_host_path: false}
      - type: bind                               # relógio do host (hoje America/New_York); nunca TZ
        source: /etc/localtime
        target: /etc/localtime
        read_only: true
        bind: {create_host_path: false}
      - type: bind                               # config/bichinho.toml (gitignored), relida a quente
        source: ./config
        target: /etc/bichinho
        read_only: true
        bind: {create_host_path: false}
      - estado:/state
    healthcheck:
      # "Esperando login" também é saudável. O Docker não reinicia unhealthy; o watchdog interno sim.
      test: ["CMD", "/usr/local/bin/bichinho", "saude"]
      interval: 60s
      timeout: 5s
      retries: 3
      start_period: 10s
      start_interval: 2s

volumes:
  estado:
```

### `docker-compose.dev.yml`

The file is deliberately **not** named `compose.override.yml`, so Compose never merges it automatically.

```yaml
# docker compose -f docker-compose.yml -f docker-compose.dev.yml up --build
services:
  pet:
    restart: "no"
    environment: {PET_LOG: debug, PET_RECARREGAR: "1", PET_DEBUG: "1"}   # WAYLAND_DEBUG: "1" quando preciso
    volumes:
      - {type: bind, source: ./skins, target: /opt/bichinho/skins, read_only: true, bind: {create_host_path: false}}
      - {type: bind, source: ./assets, target: /opt/bichinho/assets, read_only: true, bind: {create_host_path: false}}
```

### Native dev loop (fastest)

```sh
docker compose stop pet
PET_HOST_RUNTIME=/run/user PET_ESCUTA=127.0.0.1:27380 PET_ASSETS=./assets PET_SKINS=./skins:./skins-locais \
  PET_ESTADO=$HOME/.cache/bichinho-dev PET_CONFIG=./config PET_LOG=debug PET_DEBUG=1 \
  ~/.cargo/bin/cargo run -p bichinho -- rodar
```

`.env.example` lists `APP_UID`, `APP_GID`, `PET_PORTA` and `PET_LOG` with their defaults.

---

## 5. Discovery and lifecycle

### Discovery (main-loop timer, every 2 s, only while Waiting)

1. `base = $PET_HOST_RUNTIME/<getuid()>`.
2. List `base/hypr/*`. Sort newest first by the epoch field of `<hash>_<epoch>_<rand>`.
3. Read `hyprland.lock`. Line 2 is the Wayland socket name.
4. Accept the instance only if both `connect(dir/.socket2.sock)` and `connect(base/<wayland-N>)` succeed. Stale directories refuse connections and are skipped.
5. Connect by absolute path (`UnixStream::connect`, then `Connection::from_socket`). WAYLAND_DISPLAY and the instance signature never appear in compose.
6. Bind the globals:
   - **required:** `wl_compositor`, `wl_shm`, `zwlr_layer_shell_v1`;
   - **optional:** `wl_seat`, `wp_fractional_scale_manager_v1`, `wp_viewporter`, `wp_cursor_shape_manager_v1`, `ext_idle_notifier_v1`.
7. Spawn the hypr thread.
8. Create the surface (section 6).

### Lifecycle events

| Event | Behaviour | Recovery |
|---|---|---|
| Boot before login | Container up; `/saude` returns 200 with `tela: aguardando` | Pet visible within 2 s of `hypr/<HIS>` appearing |
| Logout or Hyprland crash | EOF: tear down, back to Waiting. The hypr thread exits when its fd is `shutdown()`. | Within 2 s of the next instance |
| Suspend and resume | Connections survive. A re-added output produces `closed`, which triggers a re-home. | About 0.3 s |
| HDMI unplug or Omarchy clamshell | `closed`, wait 250 ms to settle, then create on the focused (backup) monitor with a NULL output | About 0.3 s |
| Every output gone (FALLBACK) or a 0x0 output | Hide and wait for `focusedmonv2` or `monitoraddedv2` | n/a |
| Container crash or `kill -9` | Docker restarts it; positions, mute, snooze and sessions come back from `/state` | 1 to 2 s |
| `sudo systemctl restart docker` | Live-restore is off, so the restart policy starts it again | seconds |
| Pet down while Claude runs | Async hook; `curl -m 2` fails; script exits 0; no transcript noise | n/a |

On stock Omarchy, where only `docker.socket` is enabled, the README documents an optional `~/.config/systemd/user/bichinho.service` (PartOf/After `graphical-session.target`) that runs `docker compose up -d`. This host does not need it.

---

## 6. Surface, rendering, crispness

### Layer surface

- OVERLAY layer, namespace `bichinho`.
- Anchors TOP, BOTTOM, LEFT and RIGHT; size 0x0; `exclusive_zone -1`; keyboard NONE.
- Created with a **NULL output**, so Hyprland places it on the focused monitor.
- **Never** resized; no margin animation; no subsurfaces.

### Bootstrap (a NULL output only learns its scale at map time)

1. After the first `configure(w,h)`:
   - set the viewport destination to `(w,h)`;
   - attach a **1x1 transparent** buffer;
   - set an empty input region;
   - commit.
2. Hyprland then sends `enter` and `preferred_scale`.
3. Fallbacks:
   - no scale after 200 ms: `s = mode.width / logical.width` from SCTK OutputInfo;
   - no `enter` after 500 ms: retry with the explicit `wl_output` whose name equals the last `focusedmonv2`.

### Buffer

- `wl_shm` ARGB8888, premultiplied, of `(round(w*s), round(h*s))`.
- Viewport destination `(w,h)`; `buffer_scale` stays 1.
- **One** SlotPool buffer. If `canvas()` returns busy, allocate a second buffer and redraw it in full once.
- Never memset the whole buffer: a fresh memfd is already transparent.
- About 9 MB of SHM on eDP-1, about 33 MB on the 4K.

### Pixel scale

- `D = max(1, round(N*s))` device pixels per art pixel.
- N comes from config `aparencia.escala`, or else the skin's `escala_padrao` (default 4).
- Results: 6 at 1.5, 4 at 1.0, 5 at 1.25.
- D is always an integer, so blocks are always uniform; no even-N discipline is needed.
- Every position is an integer device pixel, and procedural offsets move in multiples of D.

### Blit and damage

- An art pixel becomes a DxD block:
  - alpha 0: skipped;
  - alpha 255: copied;
  - anything else: premultiplied over.
- Flip-x is an index flip.
- Damage is a **list** of rects (old and new bounds of each changed element). Send up to 128 `damage_buffer` rects; beyond that, send their union. This keeps screen-wide confetti cheap.

### Frame pacing

Draw only on change, with one frame in flight (frame callback). Deadlines come from per-frame Aseprite durations.

| State | Rate |
|---|---|
| Particles | 30 fps or less |
| Idle | 2 to 8 fps |
| Sleep | 2 fps or less |
| Deep sleep | zero commits |

### Hide

1. Draw transparent over the last bounds and commit.
2. Destroy on the next frame callback (or after 50 ms).

Hyprland's fade-out snapshot is then empty, so no ghost frame is left behind.

---

## 7. Following the active monitor

**Inputs:**
- socket2 `focusedmonv2>>NAME,WSID`;
- `monitoraddedv2` and `monitorremovedv2` (split with maxsplit);
- `openwindow` / `closewindow` for the screensaver;
- `fullscreen>>0/1`;
- `screencast>>` (later);
- the layer's `closed`;
- a drag released outside the output.

**Rules:**
- **Debounce** 300 ms and apply only the last value. This absorbs the screensaver's focus burst and quick mouse crossings.
- **Freeze** while dragging. Ignore the event if NAME is the current output.
- **[D] Spacing:** at least 1.5 s between travels. More than 3 travels in 20 s switches to a quick poof.
- **Re-home:**
  1. `poof_out` (250 ms or less; skipped with reduced motion);
  2. transparent commit;
  3. destroy;
  4. create with a NULL output;
  5. on `enter`: position = the saved fraction for that output name, or the default (right edge, feet at 40% of the height), clamped so the hitbox is visible;
  6. `poof_in`.
- **Animation state survives re-homes.** A celebration keeps playing on the new monitor.
- **`configreloaded` never re-homes.** Omarchy's modeless-recovery loop can emit it repeatedly.
- **Ignored events:** everything else, including Claude's title spinner (about 4 events/s). The reader drains continuously, because Hyprland drops a client after a 64-event backlog.
- **Screensaver:** an `openwindow` line containing `,org.omarchy.screensaver,` adds its address to a set; the matching `closewindow` removes it. With `aparencia.protetor = esconder` (default), the pet hides while the set is non-empty.

---

## 8. Drag, click, cursor

### Input region

- Normally: the skin hitbox (`toque`), plus the bubble while one is shown.
- Empty while hidden.
- Updated only when it changes.

### Cursor

`wp_cursor_shape_v1` shows `grab` on enter and `grabbing` while a button is held.

### Press inside the hitbox

1. Widen the input region to the **whole surface**, so the drag keeps working even when Hyprland's implicit grab does not engage (empty workspace).
2. Remember the grab offset.

### Drag and click

- **Drag** starts after moving 4 logical px or holding 250 ms:
  - pose `dangle`;
  - sprite position = `round(pointer*s) - grab`;
  - clamped so at least 50% of the hitbox stays visible.
- **Release after a drag:** `land` plus dust. Save the position as a fraction keyed by output name, and restore the input region.
- **Click** (under 250 ms and under 4 px):
  - `giggle` plus a heart;
  - acknowledges every ready badge and pending input;
  - snoozes escalation for 5 min.

### Cross-monitor drop

- **Released outside the surface** (implicit grab held):
  1. global point = output logical position + local coordinates;
  2. re-home immediately (with `mouse_move_focuses_monitor`, the focused monitor is the one under the pointer);
  3. place the sprite under the pointer.
- **`leave` during a drag:** treat it as a drop at the last in-bounds point; focus-follow then moves the pet.
- The sprite clips at the monitor edge while being dragged. This is accepted for v0.1.

### Later (M6, [D])

- A dangle spring with lean frames −2..+2.
- A squash plus rebound hop on landing.
- Fling at more than 1500 px/s: slide, bounce off the edges, then `dizzy`.

---

## 9. Ingress API and Claude Code plugin

### HTTP (hand-rolled, no dependencies)

- HTTP/1.1, `Connection: close`, 1 s timeouts.
- Headers at most 8 KiB; body at most 8 KiB; `Content-Length` required (chunked gets 411).
- Bodies are never logged.

| Route | Requirements | Response |
|---|---|---|
| `POST /v1/evento` | `Host` host part in {127.0.0.1, localhost} with port = `PET_PORTA_PUBLICA`; `Content-Type: application/json`; `X-Pet: 1` | 204. Errors: 400, 403, 413, 415, 411, 503 |
| `POST /v1/comando` | same | 200 JSON (`mudo`, `som`, `volume`, `soneca`, `acordar`, `tocar`, `esconder`, `mostrar`, `recarregar`, `posicao_padrao`) |
| `GET /v1/estado` | `X-Pet: 1` | metadata-only snapshot (below) |
| `GET /v1/debug/eventos` | `X-Pet: 1` and `PET_DEBUG=1` | the last 200 sanitized events, used to record scenarios [E] |
| `GET /saude` | none | 200 if the main-loop heartbeat is under 30 s old |

**`/v1/estado` contains:**
- `tela` (aguardando or ativa), the instance-signature prefix, `monitor`, `escala`, `d`;
- `sprite_global {x,y,w,h}` in global logical px;
- `base` and `ultima_reacao`;
- `sessoes` (8-character session-id prefix, project, state, counters);
- `turnos` (the last 20 scores with their components);
- `som` (ligado, mudo, soneca_ate, dnd, historico);
- `skin {id, avisos}`;
- `config` (each key's value and source);
- `reinicios`, `commits_por_min`, `presenca`.

### Wire format v1

```json
{"v":1,"e":"Stop","ts":1790020208123,"sid":"…","turno":"…","agente":true,"tool":"Edit","nt":"permission_prompt","err":"rate_limit","src":"user","reason":"logout","intr":true,"sha":true,"bg":0,"arq":"3fa2b19c04de","proj":"agenda-presidencial","dnd":false}
```

- Every field except `v` and `e` is optional.
- Unknown fields are ignored. Strings are length-capped and checked against a character class.
- `ts` (host clock) is used for durations only when |ts − now| ≤ 6 h; otherwise the receive time is used.

### `plugin/hooks/hooks.json` (shell form, as the official plugins on disk use)

```json
{
  "description": "bichinho: repassa só METADADOS dos eventos do Claude Code ao bichinho local (127.0.0.1). Assíncrono; nunca bloqueia; sempre sai 0.",
  "hooks": {
    "SessionStart":       [{"hooks": [{"type": "command", "async": true, "command": "sh \"${CLAUDE_PLUGIN_ROOT}/scripts/avisar.sh\" SessionStart || true"}]}],
    "UserPromptSubmit":   [{"hooks": [{"type": "command", "async": true, "command": "sh \"${CLAUDE_PLUGIN_ROOT}/scripts/avisar.sh\" UserPromptSubmit || true"}]}],
    "PreToolUse":         [{"matcher": "AskUserQuestion|ExitPlanMode",
                            "hooks": [{"type": "command", "async": true, "command": "sh \"${CLAUDE_PLUGIN_ROOT}/scripts/avisar.sh\" PreToolUse || true"}]}],
    "PostToolUse":        [{"hooks": [{"type": "command", "async": true, "command": "sh \"${CLAUDE_PLUGIN_ROOT}/scripts/avisar.sh\" PostToolUse || true"}]}],
    "PostToolUseFailure": [{"hooks": [{"type": "command", "async": true, "command": "sh \"${CLAUDE_PLUGIN_ROOT}/scripts/avisar.sh\" PostToolUseFailure || true"}]}],
    "PermissionRequest":  [{"hooks": [{"type": "command", "async": true, "command": "sh \"${CLAUDE_PLUGIN_ROOT}/scripts/avisar.sh\" PermissionRequest || true"}]}],
    "Notification":       [{"matcher": "permission_prompt|worker_permission_prompt|elicitation_dialog|elicitation_url_dialog|agent_needs_input|idle_prompt",
                            "hooks": [{"type": "command", "async": true, "command": "sh \"${CLAUDE_PLUGIN_ROOT}/scripts/avisar.sh\" Notification || true"}]}],
    "SubagentStart":      [{"hooks": [{"type": "command", "async": true, "command": "sh \"${CLAUDE_PLUGIN_ROOT}/scripts/avisar.sh\" SubagentStart || true"}]}],
    "PreCompact":         [{"hooks": [{"type": "command", "async": true, "command": "sh \"${CLAUDE_PLUGIN_ROOT}/scripts/avisar.sh\" PreCompact || true"}]}],
    "PostCompact":        [{"hooks": [{"type": "command", "async": true, "command": "sh \"${CLAUDE_PLUGIN_ROOT}/scripts/avisar.sh\" PostCompact || true"}]}],
    "Stop":               [{"hooks": [{"type": "command", "async": true, "command": "sh \"${CLAUDE_PLUGIN_ROOT}/scripts/avisar.sh\" Stop || true"}]}],
    "StopFailure":        [{"hooks": [{"type": "command", "async": true, "command": "sh \"${CLAUDE_PLUGIN_ROOT}/scripts/avisar.sh\" StopFailure || true"}]}],
    "SessionEnd":         [{"hooks": [{"type": "command", "async": true, "command": "sh \"${CLAUDE_PLUGIN_ROOT}/scripts/avisar.sh\" SessionEnd || true"}]}]
  }
}
```

**Event selection (13 events):**
- **PreToolUse** is limited to the two tools that open a user dialog.
- **PostToolUse** is unfiltered. It is needed for the "working" state, for counters, and to clear permission escalations whatever the tool. That costs about 10 to 20 ms of CPU per call, asynchronously.
- **SubagentStop** is not hooked, so subagents can never trigger a celebration.
- The hook set is defined **once in M2** and then rarely changes; new behaviour lives in the container [E].

### `plugin/scripts/avisar.sh`

```sh
#!/bin/sh
# avisar.sh EVENTO — hook assíncrono do plugin bichinho. Lê o JSON do hook (stdin) e envia SÓ
# metadados (nunca prompt, código, saída de ferramenta, texto de erro ou resposta). Não imprime
# nada e SEMPRE sai 0: um Stop hook que sai 2 impediria o Claude de parar.
ev=$1
case "$ev" in ''|*[!A-Za-z]*) ev=Desconhecido ;; esac
porta=${PET_PORTA:-27380}
case "$porta" in ''|*[!0-9]*) porta=27380 ;; esac

dnd=false   # Omarchy: {"version":3,"dnd":bool}; arquivo ausente = desligado
arq_dnd="${XDG_STATE_HOME:-$HOME/.local/state}/omarchy/notifications.json"
if [ -r "$arq_dnd" ] && [ "$(jq -r '.dnd == true' "$arq_dnd" 2>/dev/null)" = true ]; then dnd=true; fi

filtro='
def tok($n): if type == "string" and test("^[A-Za-z0-9_.:-]{1,\($n)}$") then . else null end;
def enum: if type == "string" and test("^[a-z_]{1,40}$") then . else null end;
({v: 1, e: $e, ts: (now * 1000 | floor), dnd: $dnd,
  sid: (.session_id | tok(80)), turno: (.prompt_id | tok(80)),
  agente: (if .agent_id then true else null end),
  tool: (.tool_name | tok(64)), nt: (.notification_type | enum),
  err: (if $e == "StopFailure" then (.error | enum) else null end),
  src: (.source | enum), reason: (.reason | enum),
  intr: (if .is_interrupt == true then true else null end),
  sha: (if .stop_hook_active == true then true else null end),
  bg: (if (.background_tasks | type) == "array" then (.background_tasks | length) else null end),
  proj: ((.cwd // "") | split("/") | map(select(length > 0)) | (last // "") | .[0:64])
 } | with_entries(select(.value != null)) | tojson),
(if (.tool_name // "") | test("^(Edit|Write|MultiEdit|NotebookEdit)$")
   then ((.tool_input.file_path // .tool_input.notebook_path // "") | @base64) else "" end)'

saida=$(jq -r --arg e "$ev" --argjson dnd "$dnd" "$filtro" 2>/dev/null) || saida=
corpo=; arq64=
{ IFS= read -r corpo; IFS= read -r arq64; } <<EOF
$saida
EOF
[ -n "$corpo" ] || corpo="{\"v\":1,\"e\":\"$ev\"}"
if [ -n "$arq64" ]; then   # caminho vira hash: conta arquivos únicos sem o caminho sair do host
  h=$(printf '%s' "$arq64" | base64 -d 2>/dev/null | sha256sum 2>/dev/null | cut -c1-12)
  case "$h" in ''|*[!0-9a-f]*) ;; *) corpo="${corpo%\}},\"arq\":\"$h\"}" ;; esac
fi
printf '%s' "$corpo" | curl -sS -m 2 -o /dev/null -H 'Content-Type: application/json' -H 'X-Pet: 1' \
  --data-binary @- "http://127.0.0.1:${porta}/v1/evento" >/dev/null 2>&1
exit 0
```

**Never forwarded:**
- `prompt`, `tool_input` (except the hashed path), `tool_response`;
- the error text of `PostToolUseFailure`;
- `error_details`, `message`, `title`, `session_title`, `last_assistant_message`;
- `transcript_path`, and full paths.

### Manifests

- **`plugin/.claude-plugin/plugin.json`:**
  ```json
  {"name": "bichinho", "version": "0.1.0", "description": "Repassa só metadados dos eventos do Claude Code ao bichinho pixel-art local (127.0.0.1:27380).", "author": {"name": "butkeraites"}, "repository": "https://github.com/butkeraites/bichinho", "license": "MIT", "keywords": ["pet", "hyprland", "wayland", "pixel-art", "omarchy"]}
  ```
  - The version lives **only** here.
  - No `userConfig` in v0.1. The port is overridden with `PET_PORTA` in the environment Claude Code inherits.
- **`.claude-plugin/marketplace.json`:**
  ```json
  {"$schema": "https://anthropic.com/claude-code/marketplace.schema.json", "name": "bichinho", "description": "Marketplace pessoal do bichinho", "owner": {"name": "butkeraites"}, "plugins": [{"name": "bichinho", "source": "./plugin", "description": "Bichinho pixel-art que reage ao Claude Code"}]}
  ```
- If `claude plugin validate --strict` flags any field, drop that field.

### Install on this machine (local directory, read in place, no git credentials needed)

```sh
claude plugin validate ~/Documents/bichinho --strict && claude plugin validate ~/Documents/bichinho/plugin --strict
claude plugin marketplace add ~/Documents/bichinho
claude plugin install bichinho@bichinho      # sessões abertas: /reload-plugins
```

- **Edits** to `plugin/` apply at the next session or after `/reload-plugins`. No version bump is needed.
- **Isolated hook experiments:** `claude --plugin-dir ~/Documents/bichinho/plugin`.
- **Other machines:**
  1. `gh auth setup-git` (it changes the global git credential helper, so ask the user first);
  2. `claude plugin marketplace add butkeraites/bichinho --sparse .claude-plugin plugin`;
  3. `claude plugin install bichinho@bichinho`;
  4. updates: `claude plugin marketplace update bichinho && claude plugin update bichinho@bichinho`, then restart Claude Code.
- **Uninstall:** `claude plugin uninstall bichinho@bichinho && claude plugin marketplace remove bichinho && docker compose down --rmi local`. Add `-v` only if the user explicitly asks.

---

## 10. Brain (pure, in `bichinho-core`, with injected clock and RNG)

### Records

- **Session:** `sid`, `proj`, `turn`, `ready_since`, `needs_input {kind, since, step, snooze_until}`, `error {kind, since}`, `last_stop`, `reminders_used`.
- **Turn:** `turno` (prompt_id), `t0`, `origem`, `espera_ms`, `ferramentas_mutantes`, `outras_ferramentas`, `arquivos` (set of hashes), `subagentes`, `falhas`, `bg_chain`.

### Events

| Event | Guard | Session effect | Reaction (class) |
|---|---|---|---|
| SessionStart | src ∈ {startup, resume} | create or refresh | `hello` (C). Sound `ola` only if the pet was asleep or nobody had been active for 10 min. |
| SessionStart | src ∈ {clear, compact, fork} | refresh | none |
| UserPromptSubmit | none | new turn (t0=ts, origem=src); clears this session's ready, needs_input and error | wake; base `thinking` |
| PreToolUse [D] | tool ∈ {AskUserQuestion, ExitPlanMode}, main agent | needs_input = pergunta or plano | escalation L1 (A); bubble `<proj>: pergunta pra você` or `<proj>: plano para aprovar` |
| PermissionRequest; Notification nt ∈ {permission_prompt, worker_permission_prompt, elicitation_dialog, elicitation_url_dialog, agent_needs_input} | dedupe 2 s per session | needs_input = permissao | L1 (A); bubble `<proj>: precisa de você` |
| PostToolUse | `agente` | counters only | none |
| PostToolUse | main agent | opens a turn if none (t0 = ts); counters (mutating = Edit, Write, MultiEdit, NotebookEdit, Bash; unique `arq` hashes); clears needs_input | base `working` |
| PostToolUseFailure | `intr` | turn cancelled (Stop will not come) | `shrug` (C) |
| PostToolUseFailure | otherwise | falhas++; clears needs_input | none in MVP (sweat later) |
| SubagentStart | none | subagentes++ | none |
| Notification idle_prompt | ready pending, ≥ 2 min since the last reminder, at most 3 per ready | none | silent hop (C). **Never a sound.** |
| PreCompact / PostCompact | none | base `compacting` (capped at 3 min) / restore | none |
| Stop | `sha` | turn stays open | none |
| Stop | dedupe per (sid, turno), plus 5 s | 0.8 s **settle** [D]: a Pre/PostToolUse or UserPromptSubmit from the session inside the window cancels it | After settle: if `bg > 0`, the chain stays open with a `…` badge and at most T1, silent [D]. Otherwise compute score and tier, close the turn, mark ready if tier ≥ 1, and play `done_<tier>` (B, merged). |
| StopFailure | err = rate_limit | tired for 60 s; close the turn | `tired` + `sono` |
| StopFailure | err ∈ {authentication_failed, oauth_org_not_allowed, account_on_hold, verification_required, billing_error, cloud_credential_error} | error for 60 s | `sad_key` (`login?`) + `bonk` |
| StopFailure | other values | error for 60 s | `error` + `bonk` |
| SessionEnd | reason ∉ {clear, resume} | remove the session after 5 s | `bye` (C) if no session remains |
| click | none | acknowledges every ready and pending input; snoozes escalation 5 min | `giggle` + heart (C) |

### Score and tiers

All weights and thresholds are config. Every Stop logs its components, which also appear in `/v1/estado.turnos`.

```
min_ativos = max(0, t_stop - t0 - espera_ms) / 60000          # waits for the user are not work [D]
score = min(20, 1.0*min_ativos + 0.15*ferramentas_mutantes + 0.05*outras_ferramentas
                + 0.5*arquivos_unicos + 1.0*subagentes)
T0  min_ativos < 15 s and no tools        -> nod, SILENT
T1  score < 4                             -> done_small + moeda
T2  4 <= score < 12                       -> done_medium + powerup + 12 confetti
T3  score >= 12, at most 1 per 10 min     -> done_big (shuffle-bag variant) + fanfarra + 30-40 confetti + rays, 2.5-4 s
```

**Caps and modes:**
- **Machine-started turns** (src ∈ {system, loop_wakeup, schedule_wakeup, poll_event}) are capped at T1 and silent [D]. `sdk` (`claude -p`) is treated normally.
- **`celebracao.modo`:** `proporcional` (default), `sempre_grande` (T3 with a 2-min cooldown), `discreta` (at most T1), `desligada`.

**Worked examples** (these become unit tests):

| Turn | Score | Tier |
|---|---|---|
| 8 s Q&A, no tools | n/a | T0 |
| 40 s: 1 Edit on 1 file, 1 Bash, 2 Read | 1.57 | T1 |
| 2 min: 15 Read, 2 WebFetch | 2.85 | T1 |
| 4 min: 10 Edit on 5 files, 8 Bash, 7 Read | 9.55 | T2 |
| 20 min: 40 Edit on 15 files, 30 Bash, 50 Read, 2 subagents | capped at 20 | T3 |

### Merging finishes

- The first finish plays immediately.
- Finishes within 3 s merge into the running celebration:
  - the tier becomes the maximum of the two;
  - the bubble shows `2 prontos: api, web`;
  - extra confetti;
  - a second sound plays only if the tier went up.

### Display priority ("loudest wins")

waiting > error > tired > ready > working > compacting > thinking > idle (with blink) > sleep.

Other sessions appear as badges: a `+N` count, a ready flag in the project colour (hash of the name, mapped to the skin's FX palette), and `…` for background work.

### Reaction classes

- **A (attention)** is never dropped while pending.
- **B (done or error)** has a 10 s TTL; while the pet is hidden it becomes a badge.
- **C (hello, bye, click, poof, micro)** has a 3 s TTL.
- A pre-empts B, which pre-empts C, at the next frame boundary.

### Needs-input escalation

There is one global clock, set by the oldest pending item; other items show as `+N`.

| Step | When | Visual | Sound |
|---|---|---|---|
| L1 | 0 s | anticipation, then `alert` + `!`, then the `waiting` loop; bubble | ping |
| L2 | +30 s | bursts every 6 s for 30 s | ping2 |
| L3 | +90 s, or the user returns while it is pending | bursts every 4 s for 20 s; `!!` blinking at 2 Hz or less | trinado |
| L4 (cap) | 5 min and later | `waiting` + badge pulse at 1 Hz; a silent burst every 60 s | none |

**Exits:** any main-agent event of that session other than the triggers themselves (PostToolUse, PostToolUseFailure, UserPromptSubmit, Stop, StopFailure, SessionEnd), or a click.

### Timers

| Timer | Value |
|---|---|
| working/thinking without events falls back to idle | 5 min (the turn stays open) |
| turn expiry | 2 h |
| error | 60 s |
| ready badge | until seen (click or a new prompt), at most 2 h |
| forgotten needs_input | 2 h |
| yawn | 3 min |
| sleep (2 fps or less) | 8 min |
| deep sleep (static frame, zero commits) | 30 min |

Events wake the pet softly; attention events use a startle wake.

### Presence [D, done in-process] (M6)

- `ext_idle_notifier_v1.get_input_idle_notification(90 s)` reports user idle or active.
- **User away:** the pet falls asleep after 3 min instead of 8.
- **User returns after ≥ 2 min** with something ready or pending: play `welcome_back` once. It shows a summary bubble (`voltou! api: pronto · web: precisa de você`) and plays one arbitrated sound. Pending attention jumps to an L3 burst.

### Sound arbiter

- **Allowed only when:** `som.ligado`, not muted, not snoozed, last-known `dnd` false, and outside quiet hours.
- **Rules:**
  - one sound at a time; attention kills a lower-priority sound;
  - at least 1.5 s between sounds (attention may wait up to 2 s);
  - at most 6 per minute (one L3 is always allowed);
  - cooldowns: done 5 s, error 10 s, ola/tchau 60 s; attention plays once per step.
- **Never a sound for:** T0, `idle_prompt`, subagents, micro reactions, drag, travel.

### Reduced motion

`aparencia.movimento = reduzido` means no particles, no flicker, no grow pulses, no travel animation, and attention capped at level 1.

---

## 11. Animation catalog

Each semantic state maps to a default tag chain in `skin.json`.

| Semantic | Chain (fallbacks) | Mode | Attention | Sound |
|---|---|---|---|---|
| idle | idle | loop, pingpong | 0 | none (`blink` overlay every 2.5–6 s, 15% double) |
| thinking | thinking → idle (+ `...` fx) | loop | 0 | none |
| working | working → idle | loop | 0 | none |
| waiting | waiting → idle (+ `?` fx) | loop | 1 | none |
| ready | ready → idle (+ flag badge) | loop | 1 | none |
| error / tired / sad_key | error → sad → idle | loop, 60 s or less | 1–2 | bonk / sono |
| yawn, sleep | yawn; sleep → idle (+ Z) | one-shot; loop at 2 fps or less | 0 | none |
| hello / bye | wave → recipe `aceno` → idle | one-shot | 1 | ola / tchau |
| nod (T0) | nod → recipe `aceno` | one-shot | 0 | none |
| done_small | done_small → recipe `pulo(h=8)` | one-shot, 1.2 s or less | 2 | moeda |
| done_medium | done_medium → recipe `salto_giro(h=12)` | one-shot, 2 s or less | 2–3 | powerup |
| done_big | variants (dance, victory, …) → recipe `danca` → done_medium | one-shot, 2.5–4 s | 3 | fanfarra |
| alert | alert → recipe `pulo(h=4)` (+ `!`) | one-shot / bursts | 2–3 | ping / ping2 / trinado |
| giggle | giggle → recipe `pulo(h=2)` (+ heart) | one-shot | 0 | none |
| dangle / land | dangle → recipe `pendurado`; land → recipe `queda` (+ dust) | loop / one-shot | 0 | none |
| poof_out / poof_in | fx only | one-shot, 250 ms or less | 0 | none |
| shrug, compacting, welcome_back | own tags → recipes | one-shot / loop | 0–2 | none / (arbitrated) |

**Recipes [D]** are built into the core. They need only `idle` plus the key poses `agacha`, `estica`, `ar` and `esmaga`, so a pack with just idle, jump and hurt covers every state:

| Recipe | What it does |
|---|---|
| `pulo(h)` | hop of height h |
| `salto_giro(h)` | flip-alternation "spin" |
| `danca` | flips plus 1-px hops on the beat |
| `queda` | plop |
| `pendurado` | `ar` pose plus lag |
| `aceno` | 1-px bob |

Recipe offsets always move in whole art pixels.

**MVP tags to request from the art track (about 19):** idle, blink, thinking, working, waiting, ready, error, sleep, yawn, wave, done_small, done_medium, done_big (at least 1 variant), alert, giggle, dangle, land, poof_in, poof_out.

**Procedural effects** (rects in art-pixel units, using the skin's `paleta_fx`):
- **Particle set:** confetti, sparkles, `!`, `?`, Z, heart, dust, poof, sunburst rays.
- **Physics:** 150–350 px/s within ±60°, gravity 600 px/s², lifetime 1.2–2 s, at most 40 particles (60 for T3 moments).
- **Bubble:** a procedural 9-slice (ink border, cream fill). Text uses the monogram atlas at `N_ui = N/2`; the bubble pops 60% → 110% → 100%.
- **Bubble text:** typed at 40 chars/s; NFC normalized; truncated to 18 characters with `..`.
- **Bubble timing:** done bubbles last 4–6 s; attention bubbles collapse to a badge after 15 s.

**T3 screen-wide moments [D]** stay inside the pet's overlay, and the input region stays on the pet:
- `fogos` (rockets with pixel-star bursts);
- `chuva_de_confete` (confetti across the top edge, at most 60 particles);
- `holofote` (sunburst plus a `PRONTO!` banner).

Flashing stays at 2 Hz or less, on small areas only.

---

## 12. Skin format and art pipeline (the character is a swappable skin)

### Locations

- `skins/<id>/`: redistributable skins, committed.
- `skins-locais/<id>/`: packs whose licence forbids redistribution. Gitignored, but baked into the **local** image only.

### `skin.json`

```json
{
  "formato": 1, "id": "padrao", "nome": "…", "autor": "…", "licenca": "…", "fonte": "https://…", "redistribuivel": true,
  "folha": "sheet.png", "dados": "sheet.json",
  "celula": [48, 48], "pe": [24, 45], "toque": [8, 14, 32, 32], "escala_padrao": 4, "espelhar": true,
  "contorno_externo": true, "paleta_fx": ["#FFD23F", "#4CC35A", "#3A86FF", "#E5394B", "#FF7AB6", "#FFF1D6"],
  "poses": {"agacha": "jump#1", "estica": "jump#2", "ar": "jump#4", "esmaga": "jump#7"},
  "estados": {"idle": ["idle"], "working": ["walk", "idle"], "done_big": ["dance", "receita:danca"]},
  "variantes": {"done_big": ["dance", "victory"]},
  "sons": {"done_big": "fanfarra.wav"}
}
```

### `sheet.json` (Aseprite json-array)

Fields read:
- `frames[].frame`, `duration`, `spriteSourceSize`, `sourceSize` (trimmed frames are supported);
- `meta.frameTags[] {name, from, to, direction, repeat}` (forward, reverse, pingpong);
- `meta.size`.

### xtask tools (Rust; `cargo xtask <cmd>`)

| Tool | What it does |
|---|---|
| `skin-importar` | Inputs: `--aseprite <json>`, or `--tiras <dir> --celula 48x48 --ms 100 [--tag-ms idle=300]`. Options: `--pe X,Y`, per-clip pivot alignment, `--contorno creme` (1-art-px cream outer ring made by dilating alpha, so the pet reads on hackerman `#0B0C16` and on light themes). Writes `sheet.png`, `sheet.json` and a `skin.json` skeleton. |
| `lint-skin` | **Errors:** JSON parse; image size ≠ `meta.size`; `sourceSize` ≠ `celula`; `idle` does not resolve; durations outside 16–5000 ms; `redistribuivel=false` under `skins/`. **Warnings:** unknown tags; one-shots over 4 s; more than 32 colours; partial alpha; bounding box jumping more than 3 art px between consecutive frames; feet off the pivot on non-airborne frames. |
| `cobertura` | [D] Writes `cobertura.md`. Each semantic state is marked nativo, receita, fallback or faltando. This becomes the to-do list for custom frames in the pack's style. |
| `contato` | Writes a contact-sheet PNG (every tag, frame durations, on dark `#0B0C16` and light `#EFF1F5` checkerboards, ×4) plus `previa/<tag>.gif`. Used for user approval, and Claude Code can Read it. |
| `skin-teste` | Generates `skins/_teste`, a checkerboard with frame digits that covers every semantic tag. It is a QA fixture and fallback only, **never shown as the character**. |
| `fonte` | Builds the monogram atlas (CC0, Portuguese diacritics) from `monogram-bitmap.json`. Fallback: Departure Mono (OFL) rasterized at 11 px. |
| `sfx` | Generates the sound effects (section 13). |
| `nitidez <png> --bloco 6` | Crispness check for grim captures. |

### Scale guidance

- A 32-px body in a 48-px cell uses N = 4: 128 logical px, 192 device px at 1.5.
- 64-px cells use N = 2.
- Aim for a body of about 96–160 logical px on the laptop.

### Acceptance gate

No skin becomes `padrao` until the user has approved both:
- the contact sheet; and
- a live demo (`bin/bichinho testar pequeno|medio|grande|alerta|erro`).

Licence and credits go into `CREDITS.md` in the skin folder and into `NOTICE.md`. Adding the outer ring counts as a modification, so check the pack licence first.

---

## 13. Sound

**Generation:** `cargo xtask sfx` writes deterministic 48 kHz mono s16 WAVs, licensed CC0, into `assets/sfx/` from parameters in `assets/sfx/*.toml`.
- Synth: a note sequencer (square/duty, triangle, sine, noise, ADSR, slide, vibrato, one-pole low-pass at about 7 kHz) plus an sfxr-style generator (jsfxr is Unlicense).
- Peak is normalized to −3 dBFS or lower, with a per-sound gain table.

| Sound | Use | Momentary max target |
|---|---|---|
| moeda | T1 | −24 LUFS |
| powerup | T2 | −21 LUFS |
| fanfarra | T3 (never louder than `complete.oga`) | −18 LUFS |
| ping, ping2, trinado | L1, L2, L3 | −21, −19, −18 LUFS |
| bonk | errors | −22 LUFS |
| sono | rate limit | −26 LUFS |
| ola, tchau | session start/end (only after sleep or absence) | −26 LUFS |

**Loudness check (host):**
```
ffmpeg -hide_banner -nostats -i f.wav -af apad=pad_dur=0.5,ebur128=peak=true -f null - 2>&1
```
Take the maximum `M:` value and the `True peak` line.

**Playback:**
- Command: `paplay --client-name=bichinho --property=media.role=event --volume=<round(volume*65536)> <wav>`.
- Environment: `PULSE_SERVER=unix:/host/run/user/<uid>/pulse/native` and `HOME=/tmp`. No cookie is needed.
- The sound is skipped if the socket is missing, and the process is killed after 5 s.
- A failure is logged once and never affects the visuals.

---

## 14. Configuration, persistence, CLI

### Precedence [E]

1. runtime commands (persisted in `/state`);
2. `PET_*` environment variables;
3. `config/bichinho.toml` (gitignored; template `config/exemplo.toml`; re-read on mtime change, 2 s poll);
4. code defaults.

`/v1/estado.config` shows each key's effective value and its source. Invalid values produce a warning and fall back to the default.

```toml
[som]
ligado = true               # pendente com o usuário; padrão pedido: LIGADO
volume = 0.6
silencio = ""               # ex. "22:00-08:00" (relógio do host); vazio = desligado

[celebracao]
modo = "proporcional"       # proporcional | sempre_grande | discreta | desligada
limiar_rapido_s = 15
faixas = [4.0, 12.0]
teto = 20.0
grande_intervalo_min = 10
mesclar_s = 3
assentar_ms = 800
[celebracao.pesos]
minutos = 1.0
ferramentas_mutantes = 0.15
outras_ferramentas = 0.05
arquivos = 0.5
subagentes = 1.0

[atencao]
degraus_s = [0, 30, 90, 300]
soneca_clique_min = 5

[aparencia]
skin = "padrao"
escala = 0                  # 0 = escala_padrao da skin
tela_cheia = "mostrar"      # mostrar | esconder (best-effort via fullscreen>>)
protetor = "esconder"       # esconder | mostrar
movimento = "normal"        # normal | reduzido

[monitor]
seguir_atraso_ms = 300
intervalo_min_viagem_ms = 1500

[sono]
bocejo_min = 3
dormir_min = 8
sono_profundo_min = 30
```

### Environment variables

| Group | Variables |
|---|---|
| Compose/user | `APP_UID`, `APP_GID`, `PET_PORTA`, `PET_LOG` |
| Internal | `PET_HOST_RUNTIME`, `PET_ESCUTA`, `PET_ASSETS`, `PET_SKINS`, `PET_ESTADO`, `PET_CONFIG`, `PET_PORTA_PUBLICA`, `PET_RECARREGAR`, `PET_DEBUG` |

### State file

`/state/estado.json`:
```json
{"versao": 1, "posicoes": {"eDP-1": {"x": 0.86, "y": 0.42}}, "mudo": false, "soneca_ate_epoch_ms": null, "sessoes": […], "ultima_festa_grande_ms": …}
```
- Writes are atomic (temp file + rename) and debounced 1 s.
- A corrupt file is renamed to `.corrompido-<ts>` and defaults are used.

### Binary subcommands

| Command | Purpose |
|---|---|
| `bichinho rodar` | default |
| `bichinho saude` | healthcheck |
| `bichinho ctl <cmd> [arg]` | send a command |
| `bichinho versao` | print the version |

### Host CLI `bin/bichinho` (bash + curl + jq)

It prepends `~/.cargo/bin` to PATH and reads `PET_PORTA` from the environment or `.env`.

| Command | What it does |
|---|---|
| `estado` | summary of `/v1/estado` |
| `mudo [on\|off]`, `som on\|off`, `volume <x>`, `soneca [30m]`, `acordar`, `esconder [30m]`, `mostrar` | runtime controls, persisted |
| `tocar <reacao>`, `posicao-padrao`, `recarregar` | direct actions |
| `testar <rapido\|pequeno\|alerta\|pergunta\|erro>` | synthetic hook payloads **through `plugin/scripts/avisar.sh`** (the real path) |
| `simular <cenario>` | replays `cenarios/<c>.jsonl` (wire events with backdated `ts`) [E] |
| `eventos [--salvar arquivo]` | turns `/v1/debug/eventos` into a new scenario fixture [E] |
| `foto` | grim of the sprite rect into `tmp/fotos/<ts>.png`, for Claude to Read [E] |
| `logs`, `subir`, `parar`, `reconstruir`, `dev` | container and compose shortcuts |
| `verificar` | the single pre-commit gate [E] (see below) |

`verificar` runs, in order:
- `cargo fmt --check`;
- `cargo clippy --all-targets -- -D warnings`;
- `cargo test`;
- `cargo xtask lint-skin` on every skin;
- `docker compose config -q`;
- `claude plugin validate . --strict`;
- `claude plugin validate ./plugin --strict`;
- shellcheck, if installed.

---

## 15. Repository layout

```
bichinho/
  CLAUDE.md README.md DECISIONS.md PROGRESS.md PLANO.md LICENSE NOTICE.md
  Cargo.toml Cargo.lock rust-toolchain.toml(channel "1.98.1" + clippy + rustfmt) rustfmt.toml .editorconfig
  crates/bichinho-core/src/{lib,config,event,brain,score,attention,arbiter,animator,recipes,skin,aseprite,raster,scene,particles,text,geometry,scenario}.rs
  crates/bichinho-core/tests/{brain_table,score,arbiter,skin,raster,golden,scenarios}.rs + tests/fixtures/
  crates/bichinho/src/{main,daemon,discovery,hypr,ingress,audio,store,watchdog}.rs
  crates/bichinho/src/wl/{mod,surface,fractional,input,shm,idle}.rs
  crates/bichinho/tests/{ingress,hypr_parser,discovery,avisar}.rs + tests/fixtures/{hooks,hypr}/
  xtask/src/main.rs        # sfx | skin-teste | skin-importar | lint-skin | cobertura | contato | fonte | nitidez
  assets/sfx/{*.toml,*.wav}  assets/fonte/monogram/{monogram-bitmap.png,monogram-bitmap.json,LICENSE}
  skins/_teste/  skins/padrao/ (from the art track)  skins-locais/.gitkeep
  cenarios/*.jsonl + *.esperado.jsonl
  config/exemplo.toml       # config/bichinho.toml gitignored
  .claude-plugin/marketplace.json
  plugin/.claude-plugin/plugin.json  plugin/hooks/hooks.json  plugin/scripts/avisar.sh
  bin/bichinho  scripts/verificar.sh  scripts/e2e-monitor.sh
  Dockerfile docker-compose.yml docker-compose.dev.yml .env.example .dockerignore .gitignore
  .claude/skills/{nova-animacao,novo-evento-hook,conferir-na-tela}/SKILL.md
  docs/{ARQUITETURA,TESTES,SKINS,HOOKS,SEGURANCA}.md
  .github/workflows/ci.yml
```

**Workspace settings:**
- Edition 2024, resolver 3.
- `[profile.release] lto = "thin", strip = true, codegen-units = 1, panic = "abort"`.

**Dependencies** (added with `cargo add`; `Cargo.lock` committed):

| Crate | Dependencies |
|---|---|
| core | `serde` (derive), `serde_json`, `toml`, `png` |
| binary | `smithay-client-toolkit = { version = "0.21", default-features = false, features = ["calloop"] }` (no xkbcommon or pkg-config, so a fully static build); `wayland-client 0.31`; `wayland-protocols 0.32` (`client`, `staging`: fractional-scale, cursor-shape, ext-idle-notify; viewporter is stable); `signal-hook`; `jiff` (only when quiet hours ship, reading `/etc/localtime`) |
| xtask | `png`, `gif`, `serde_json` |

No clap, no tokio. **`bichinho-core` must never depend on Wayland crates**; Cargo enforces this, which keeps tests fast and plan B cheap.

**`CLAUDE.md` (Portuguese) must state:**
- **What the project is.** The repository state, updated at each milestone.
- **Commands.** `~/.cargo/bin` is not on PATH; `bin/bichinho verificar` runs before every commit.
- **Architecture facts:**
  - one fixed full-output OVERLAY surface with no resize and no subsurfaces;
  - re-home through a NULL output;
  - a single SHM buffer with damage rects; D = round(N*s).
- **Golden rules:**
  - hooks are async, metadata only, always exit 0, and content is never logged;
  - the daemon never opens `.socket.sock` and never calls `hyprctl dispatch` or `keyword`;
  - Hyprland config changes go only through the omarchy skill, with the user's consent;
  - test or placeholder art is never shown as the character;
  - PROGRESS and DECISIONS are updated with every task.
- **Gotchas:**
  - the 64-event socket2 overflow;
  - `idle_prompt` re-fires about every 60 s;
  - Stop may not fire on Esc;
  - never name a dev file `compose.override.yml`;
  - never pass `GDK_SCALE` into the container;
  - `hyprctl output create` takes no name.

**Project skills [E]:**

| Skill | Recipe |
|---|---|
| `nova-animacao` | catalog entry, fallback, recipe, skin mapping, brain trigger, scenario golden, `tocar`, then `foto` and Read |
| `novo-evento-hook` | `avisar.sh` whitelist, canary fixture, `hooks.json`, wire doc, brain table, scenario, `/reload-plugins` reminder |
| `conferir-na-tela` | dev loop, `tocar`, `foto`, `nitidez`, layers query |

---

## 16. Tests and verification

**Unit tests (`bichinho-core`):**
- **Brain table:** every row of the section 10 events table. This includes:
  - AskUserQuestion and ExitPlanMode attention;
  - the background chain and the machine-turn cap;
  - the StopFailure enum mapping;
  - interrupt cancel and out-of-order events (PostToolUse before the prompt);
  - `sha` continuation, settle cancel, Stop dedupe and the cross-session merge;
  - escalation at 0, 30 and 90 s, the cap and its exits, and snooze on click;
  - DND mutes sound but keeps visuals; `idle_prompt` is always silent;
  - the yawn, sleep and deep-sleep schedule.
- **Score:** the worked examples above.
- **Arbiter properties:**
  - never more than 6 sounds per minute;
  - never two sounds less than 1.5 s apart;
  - at most one sound per escalation step;
  - none for subagents, T0 or `idle_prompt`.
- **Animator:** per-frame durations, pingpong, fallbacks, recipes, and no immediate repeat of a variant.
- **Skin:** Aseprite fixtures, trimmed frames.
- **Raster:** exact 6x6 blocks at D=6, premultiplied over, flip, clipping, the damage list.
- **Text:** ç, ã, õ; NFC; truncation.

**Goldens:**
- SHA-256 of rendered RGBA frames (`golden_idle_s15`, `golden_bubble_ptbr`); regenerate with `PET_ATUALIZAR_OURO=1`.
- **Scenario goldens [E]:** each `cenarios/*.jsonl` replayed with the fake clock must produce `*.esperado.jsonl` intents. Seed set: rapido, pequeno, medio, grande, dois-prontos, pergunta, permissao-escalada, idle-prompt-repetido, erro-limite, interrompido, segundo-plano, protetor-de-tela.
- After a strange live behaviour: `bin/bichinho eventos --salvar cenarios/<caso>.jsonl`, write the expected output, then fix.

**Binary tests:**
- **Ingress:** a valid request returns 204; `Host: evil.com` → 403; missing `X-Pet` → 403; `text/plain` → 415; 9 KiB → 413; chunked → 411; slow client → timeout.
- **hypr parser:** recorded lines, including commas inside descriptions and the screensaver burst.
- **Discovery:** temp dirs + `UnixListener`; the newest live instance wins; stale directories are skipped.

**Privacy canaries [E]** (`crates/bichinho/tests/avisar.rs`):
- Runs `sh plugin/scripts/avisar.sh <Evento>` for a fixture of every hooked event.
- A fake `curl` on PATH captures stdin.
- `SEGREDO-<n>` is planted in `prompt`, `tool_input`, `tool_response`, `error`, `error_details`, `message`, `title`, `session_title` and `last_assistant_message`.
- Assertions: no `SEGREDO` in the output; valid JSON; exit 0; nothing on stdout; still exits 0 when curl fails; the `arq` hash is present for Edit/Write.

**Live check `scripts/verificar.sh`** (host, stack running):
1. Health: `docker compose ps --format json | jq -e 'select(.Service=="pet") | .Health=="healthy"'` and `curl -fsS 127.0.0.1:27380/saude`.
2. Layer placement: `hyprctl -j layers | jq -r 'to_entries[]|select(any(.value.levels["3"][]?; .namespace=="bichinho")).key'` must equal `hyprctl -j monitors | jq -r '.[]|select(.focused).name'`. The layer's x/y/w/h must equal that monitor's logical rect (eDP-1: 640,0 1280x800).
3. Crispness:
   - `geo=$(curl -fsS -H 'X-Pet: 1' 127.0.0.1:27380/v1/estado | jq -r '.sprite_global|"\(.x),\(.y) \(.w)x\(.h)"')`;
   - `grim -g "$geo" "$tmp/b.png"`;
   - `cargo xtask nitidez "$tmp/b.png" --bloco 6`.
4. Hook round trip: pipe `printf '{"session_id":"t1","prompt_id":"p1","cwd":"/x/demo","hook_event_name":"UserPromptSubmit"}'` into `sh plugin/scripts/avisar.sh UserPromptSubmit`, then do the same for `Stop`. Expect `/v1/estado.ultima_reacao` = `nod` after the settle window.
5. Budgets:
   - image under 40 MB;
   - `docker stats --no-stream` memory under 64 MiB on eDP-1 (96 MiB on the 4K);
   - idle CPU under 1% (3 samples over 30 s);
   - `avisar.sh` under 50 ms while the pet is up.

**Opt-in end-to-end `scripts/e2e-monitor.sh`** (it changes compositor state, so it only runs with `--autorizo` after the user consents each time):
1. Record the original monitor list.
2. `hyprctl output create headless`.
3. Find the new name by diffing `hyprctl -j monitors`.
4. Focus it: `hyprctl dispatch "hl.dsp.focus({ monitor = \"$m\" })" || hyprctl dispatch focusmonitor "$m"`.
5. Assert that `/v1/estado.monitor` and the layers query show the pet there within 1 s.
6. Focus the original monitor again and assert the pet followed.
7. `hyprctl output remove "$m"`; assert the pet is home and the restart count is unchanged.

A `trap` always restores focus and removes the output.

**Manual checklist (`docs/TESTES.md`; the user runs these):**
- drag on a busy workspace and on an empty one;
- HDMI plug and unplug with the pet on HDMI;
- clamshell (lid closed);
- `systemctl suspend`;
- `omarchy-launch-screensaver force`;
- logout/login and reboot with autologin;
- `sudo systemctl restart docker`;
- toggling DND (`omarchy-toggle-notification-silencing`);
- sound levels.

---

## 17. Milestones

Each task gets one row in PROGRESS.md and one commit. Each milestone is a branch and a PR.

### M0 — Fundação

**Tasks:**
- **T0.1** Ask the open questions (section 20). Record the answers as decisões 0001+.
- **T0.2** Scaffold the repo:
  - `git init -b main`;
  - `CLAUDE.md`, `README`;
  - `DECISIONS.md`, seeded from this plan's decisions;
  - `PROGRESS.md`, `PLANO.md`, `LICENSE` (MIT), `NOTICE.md`;
  - ignore files and `.editorconfig`;
  - `.env.example`, `config/exemplo.toml`, `skins-locais/.gitkeep`.
- **T0.3** Workspace, toolchain pin, crate skeletons, xtask skeleton.
- **T0.4** Daemon skeleton:
  - config precedence;
  - ingress serving `/saude` and a `/v1/estado` stub;
  - watchdog; SIGTERM; `bichinho saude`;
  - the first core test.
- **T0.5** Dockerfile, both compose files, `bin/bichinho` (`subir`, `parar`, `logs`, `estado`, `verificar`).
- **T0.6** Private GitHub repo and push (section 18). Optional light CI (fmt, clippy, test, lint-skin).

**Verify (end to end):**
- `bin/bichinho verificar` is green.
- `docker compose up -d --build && docker compose ps` shows healthy.
- `curl -fsS 127.0.0.1:27380/saude`.
- `docker image inspect bichinho:local -f '{{.Size}}'` is under 40 MB.
- `gh repo view butkeraites/bichinho --json visibility -q .visibility` prints `PRIVATE`.

### M1 — Overlay nítido (engine GO/NO-GO) (human gate)

**Tasks:**
- **T1.1** Discovery and reconnect loop, with unit tests.
- **T1.2** Wayland session: globals, NULL-output OVERLAY layer, 1x1 bootstrap, fractional-scale and viewporter binding, the enter and scale fallbacks.
- **T1.3** Core raster and scene; `cargo xtask skin-teste`; golden frames.
- **T1.4** Single-SHM renderer with damage list, frame pacing, transparent hide.
- **T1.5** Input region; cursor shape; `/v1/estado` sprite rect; `bin/bichinho foto`.
- **T1.6** `scripts/verificar.sh` crispness and budget gates; `cargo xtask nitidez`. Record the measurements in a DECISIONS entry.

**Verify:**
- Live checks 2, 3 and 5 pass.
- Clicks next to the sprite reach the window below (manual).
- `docker compose restart pet`: back within 3 s, at the same place.
- `docker kill -s KILL $(docker compose ps -q pet)`: RestartCount goes up.
- `docker compose run --rm --no-deps -e PET_HOST_RUNTIME=/tmp/nada pet` logs `aguardando compositor`.

**Gate:**
- **NO-GO** (crispness, RSS, click-through or reconnect fails): switch `crates/bichinho/src/wl/` to plan B, gtk4-rs + gtk4-layer-shell on alpine:3.24. The core crate is kept.
- **GO:** open the PR with `foto` captures for the user.

### M2 — Esqueleto andante: hooks → reação

**Tasks:**
- **T2.1** Wire format v1; `/v1/evento`, `/v1/comando` and `/v1/debug/eventos` with validation tests.
- **T2.2** The plugin (both manifests, `hooks.json` with the 13 events, `avisar.sh`), plus canary tests. Both `validate --strict` runs pass.
- **T2.3** Minimal brain: sessions, Stop settle and dedupe, T0 `nod` versus T1 procedural hop on the test skin. `bin/bichinho testar rapido|pequeno`.
- **T2.4** Install the local marketplace (section 9).

**Verify:**
- `claude plugin list` shows `bichinho@bichinho` enabled.
- In a new terminal, `claude -p "responda só: ok"`: `/v1/estado.sessoes` shows the session and the reaction is `nod`.
- A `claude -p` prompt that edits a temporary file makes the pet hop.
- With `docker compose stop pet`, a real `claude -p` turn shows no hook error. `time (printf '{}' | sh plugin/scripts/avisar.sh Stop)` takes 2.1 s or less.
- `bin/bichinho subir`.

### M3 — Arrastar e seguir o monitor ativo

**Tasks:**
- **T3.1** Drag and click state machine (whole-surface input while pressed, thresholds, clamping, cursor shapes).
- **T3.2** hypr thread: socket2 reader, parser, reconnect, screensaver set.
- **T3.3** Focus follow: debounce, drag freeze, travel spacing, poof, `closed` settle, FALLBACK and 0x0 guards, `configreloaded` ignored.
- **T3.4** Cross-monitor drop; per-monitor fractional positions persisted in `/state`.
- **T3.5** Screensaver hide with welcome-back; `soneca`; the `tela_cheia` option.

**Verify:**
- Manual drag on a busy and an empty workspace.
- The position survives `docker compose restart pet`.
- `scripts/e2e-monitor.sh --autorizo` passes (only with the user's consent).
- `omarchy-launch-screensaver force` hides the pet, and it returns afterwards.
- The user-run HDMI, clamshell and suspend items are recorded in PROGRESS.

### M4 — Cérebro completo e som

**Tasks:**
- **T4.1** Full session model (`prompt_id` turns, the `agente` counters rule, background chains, machine-turn cap, interrupt cancel, timeouts, loudest wins, badges).
- **T4.2** Score, tiers, merge and cooldown modes; score logging in `/v1/estado.turnos`.
- **T4.3** Escalation L1 to L4 for every trigger; click acknowledgement and snooze; the silent `idle_prompt` reminder.
- **T4.4** `cargo xtask sfx`, the arbiter and paplay; `mudo`, `som`, `soneca`; DND; loudness check.
- **T4.5** Scenario goldens, `bin/bichinho simular` and `eventos --salvar`.

**Verify:**
- `cargo test -p bichinho-core` is green (tables, goldens, scenarios).
- `simular grande` gives `done_big` and **exactly one** fanfarra (`/v1/estado.som.historico`).
- `simular rapido` gives a silent `nod`.
- `simular dois-prontos` gives one merged celebration with `2 prontos`.
- `simular pergunta` gives ping at 0 s, ping2 at 30 s and trinado at 90 s, then silence (3 sounds at most).
- `simular idle-prompt-repetido` plays no sound.
- With DND toggled on by the user, the visuals play with no sound.
- The ebur128 targets are met within ±2 LU, with true peak at −3 dBFS or lower.

### M5 — Personagem real (depends on the art track; pull it forward as soon as the pack is chosen) (human gate)

**Tasks:**
- **T5.1** `skin-importar` (Aseprite and strips), pivot alignment, `--contorno creme`, `lint-skin`, the `redistribuivel` rule, `skins-locais`.
- **T5.2** Recipes and `cobertura`.
- **T5.3** `contato` contact sheet and GIF previews.
- **T5.4** Integrate the chosen character as `skins/padrao` (or under `skins-locais`), with `CREDITS.md` and `NOTICE.md`.

**Verify:**
- `cargo xtask lint-skin skins/padrao` reports zero errors.
- `cobertura.md` has no `faltando` among the MVP states.
- `verificar.sh` crispness passes with the real skin.
- **The user approves the contact sheet and the live demo** (`bin/bichinho testar pequeno|alerta|erro`, `simular medio|grande`).

### M6 — Encanto e atenção (human gate on feel)

**Tasks:**
- **T6.1** Particles and reduced motion.
- **T6.2** Bubbles (monogram atlas, 9-slice, pop overshoot, typewriter, PT-BR templates) and badges.
- **T6.3** Idle variety: blink, look around, fidget shuffle-bag, yawn, sleep, deep sleep, soft and startle wakes.
- **T6.4** Drag physics: dangle spring with lean frames, plop with rebound, dust.
- **T6.5** T3 screen-wide variants with variant shuffle-bag.
- **T6.6** Presence via `ext_idle_notifier_v1` and the welcome-back replay.

**Verify:**
- Every reaction checked with `tocar` + `foto`, reviewed by the user. Rubric: noticed in peripheral vision within 2 s; still pleasant on the 10th repetition; readable on dark and light themes; crisp.
- The golden bubble with `ç` and `ã` passes.
- T3 uses under 30% CPU for at most 4 s on eDP-1 (`docker stats` sampling).
- Deep sleep: `commits_por_min == 0`.
- After 90 s idle and then returning, the replay plays exactly once.

### M7 — Polimento e v0.1.0

**Tasks:**
- **T7.1** Complete CLI; config hot reload; quiet hours (using `jiff` and `/etc/localtime`).
- **T7.2** Optional Hyprland layer rule (section 19), only through the omarchy skill and only with consent.
- **T7.3** README with a GIF; `docs/SEGURANCA.md`; complete `docs/TESTES.md`; measurements in PROGRESS.
- **T7.4** Release:
  - `claude plugin tag plugin --dry-run`;
  - `git tag -a v0.1.0 -m "…"` and push;
  - `gh release create v0.1.0 --generate-notes`.
- **T7.5** Tuning week: read `/v1/estado.turnos`, adjust weights and thresholds in config, record the result in DECISIONS.

**Verify:**
- From a fresh clone in a temp dir: `docker compose up -d --build`, install the local marketplace, run `claude -p` → a celebration plays.
- After uninstalling, `claude plugin list`, `claude plugin marketplace list` and `docker images` are clean.
- `gh release view v0.1.0`.
- The tier distribution over the week is mostly T0/T1, with about 2 T3 per day or fewer.

---

## 18. GitHub and working process

1. **Pre-check:** `gh auth status` (account butkeraites with `repo` and `workflow` scopes).
2. **Create the repo:**
   ```
   gh repo create butkeraites/bichinho --private --source=. --remote=origin --description "Bichinho de pixel art que comemora quando o Claude Code termina (Hyprland/Omarchy, Docker)" --push
   ```
   The user asked for a GitHub repo, which covers creating and pushing it. At the start of execution, confirm once that per-task commits and per-milestone pushes and PRs are fine.
3. **Per milestone:**
   - branch `mN-<tema>`;
   - one commit per task, with its PROGRESS row (hash filled in or `—`);
   - at the end, `gh pr create`. The body lists changes, verification output and `foto` captures, and ends with the Claude Code line.
   - The user reviews (this is the human gate), then `gh pr merge --squash` and tag `v0.N.0`.
4. **Never commit:** `.env`, `config/bichinho.toml`, `skins-locais/*`, `tmp/`, `target/`.
5. **CI (optional, private-repo minutes):** fmt, clippy, test, lint-skin. The image is built only on tags.

---

## 19. Security posture (written up in DECISIONS.md and docs/SEGURANCA.md)

**The container is packaging, not a sandbox.**
- Access to `wayland-1` (virtual keyboard, screencopy) already amounts to running code as the user.
- The `/run/user` bind also exposes the session D-Bus, gpg-agent, the pulse microphone and Claude's `cc-socks`.

**What still matters:**
- uid 1000, `cap_drop ALL`, `no-new-privileges`, read-only rootfs, tmpfs `/tmp`, pid/memory/CPU limits;
- no docker.sock and no host network;
- loopback-only ingress with Host, Content-Type and `X-Pet` checks and an 8 KiB cap;
- the daemon opens only `wayland-N`, `.socket2.sock` and `pulse/native` (through paplay), never `.socket.sock` or `bus`;
- digest-pinned bases and `cargo build --locked`;
- hooks forward metadata only and content is never logged;
- DND is read **on the host by the hook**, so `~/.local/state/omarchy` (which holds the clipboard history) never enters the container.

---

## 20. Optional Hyprland rule (T7.2; only via the omarchy skill and only with consent)

```lua
hl.layer_rule({ name = "bichinho", match = { namespace = "^bichinho$" }, order = 1, no_anim = true })
```

- `order = 1` keeps the pet below Omarchy's own overlay popups (polkit, menus, toasts).
- `no_anim` removes Hyprland's ~180 ms fade-in on each re-home.

## 21. Backlog after v0.1.0 (M8+, each behind config)

- **[D] Ready-until-seen:** the hook walks the PID chain to the foot window via `hyprctl -j clients`, cached per session. socket2 `activewindowv2` is deduplicated. Focusing the finished session's terminal clears its flag with a silent thumbs-up. This also enables focus-aware sound suppression.
- **[D] Interaction extras:** click-spam chain (laugh → dizzy → furious → faint), petting by scroll, fling physics, subagent mini-clones, tool micro-reactions.
- **[D] Screen sharing:** discreet bubbles (icons only, no project names) during `screencast>>1`.
- **Menu:** a right-click pixel menu.
- **Function hooks:** an optional plugin using `turn.complete` (early-access API) for exact durations without spawning processes.
- **Cross-monitor drag:** draw the sprite on both outputs during the drag.

## Top risks

| Risk | Mitigation |
|---|---|
| Hand-written Wayland plumbing has bugs | SCTK covers most of it; M1 is a GO/NO-GO gate; plan B swaps only `wl/` |
| Annoyance | arbiter caps, bounded escalation, silent `idle_prompt`, merging, cooldowns, DND, mute, snooze, tuning week |
| Art quality | real pack + recipes + coverage report, mandatory approval gate, the test skin never shown as the character |
| Missed Stop on Esc | interrupt detection plus timeouts |
| Privacy | jq allowlist + canaries |
| Upstream drift in Claude Code fields or Hyprland events | tolerant parsers, scenario goldens, `validate --strict` after every Claude Code update |

### Critical files for implementation
- ~/Documents/bichinho/crates/bichinho/src/wl/surface.rs
- ~/Documents/bichinho/crates/bichinho-core/src/brain.rs
- ~/Documents/bichinho/crates/bichinho/src/daemon.rs
- ~/Documents/bichinho/plugin/scripts/avisar.sh (+ plugin/hooks/hooks.json)
- ~/Documents/bichinho/docker-compose.yml (+ Dockerfile)

