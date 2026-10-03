# Arte, animação e prior art

> Pesquisa automática de 2026-10-02 (somente leitura), em inglês. Pode envelhecer: confira antes de confiar.

## Relatório

# Research report: character, animation, pixel-art and sound direction for the Claude Code pet (Omarchy/Hyprland, Docker)

All checks were read-only. Sources are listed in section 8. Facts I could not verify are marked **(unverified)**.

---

## 0. TL;DR: recommendations

1. **Use an original character.** Do not copy Anthropic's "Clawd" (a trademark that many fan pets reuse), Pokémon PMD sprites (used by qs-vpets), or community Codex/Petdex pets (fan-art IP risk).
   - Default pick: **capivara com laranja**. It has the most distinctive silhouette, strong meme value in Brazil, and free secondary motion from the orange on its head.
   - **Slime** is the cheapest to animate well. **Kitten** reads best at small sizes. **Robot** can show status icons on its face screen.
2. **Geometry**
   - Draw the body on a **32-px grid** and package every frame in a **48×48 cell**. The extra room is for squash/stretch, hops of 12 px or less, and effects attached to the body.
   - Render at **N = 4 logical px per art px**, which is **6 device px at scale 1.5**. Body ≈ 128 logical px, cell = 192 logical / 288 device px.
   - Draw the UI layer (bubble, text, badges) at **N_ui = 2** (3 device px).
   - The rule is that `scale × N` must be an integer. At 1.5 only even N stays crisp (2, 4, 6, 8). N = 4 is also crisp at scales 1, 1.25, 1.75 and 2.
   - Snap sprite positions to multiples of 2 logical px (better: 4).
3. **Surface**
   - Use a **fixed-size, full-output transparent overlay**. Set the input `mask`/region to cover only the sprite and the bubble; during a drag, set it to the whole surface.
   - Omarchy's own toast window does exactly this (`/usr/share/omarchy/shell/plugins/notifications/Service.qml`, PanelWindow `omarchy-notifications`). Its source comment explains why: resizing the surface lets the compositor "briefly scale a stale buffer … which is what stretched/squished the cards".
   - **Never resize the surface for celebrations.**
   - **Do not use wl_subsurfaces.** Hyprland squishes subsurfaces that overflow their parent instead of cropping them (hyprwm/Hyprland#10515, still OPEN).
4. **Pipeline**
   - Source of truth: **text grids + palette + TOML manifest**, kept in git.
   - A Python/Pillow build (Docker build stage) produces:
     - PNG sheets;
     - **Aseprite `json-array`** data (frames with durations, frameTags, slices);
     - a runtime sidecar `anim.json`;
     - optionally, generated `.aseprite` files;
     - GIF previews and a contact sheet;
     - lint reports.
5. **Attention model**
   - 4 levels, from 0 (ambient) to 3 (celebration).
   - Use **motion-onset bursts separated by stillness**, not continuous motion. Abrams & Christ 2003 showed that the *onset* of motion captures attention, while ongoing motion does not.
   - "Needs input" escalation is bounded: 0 s → 30 s → 90 s → capped at 5 min.
   - A **"Ready until seen"** badge (copied from Codex Pets).
   - "Done" celebrations are tiered by a work score.
   - A 3-s coalescing window, a global sound arbiter, Omarchy DND support, focus-awareness, hiding during fullscreen and the screensaver, and **≤ 3 flashes/s** (WCAG 2.3.1).
6. **Sound**
   - Procedural 8-bit SFX generated in the repo: a note sequencer plus a Python port of jsfxr (jsfxr is **Unlicense**, original sfxr is MIT, ZzFX is MIT).
   - Output 48 kHz mono 16-bit WAV.
   - Loudness −30…−18 LUFS (momentary max), true peak ≤ −3 dBFS.
   - Play with `pw-play --media-role=Notification --volume=…` (both flags exist in the host's PipeWire 1.6.8).
7. **Fonts**
   - **monogram (CC0)** for bubble text. It explicitly lists "português" and ships `monogram-bitmap.png` + `monogram-bitmap.json`.
   - OFL alternatives: **Tiny5**, **Departure Mono**, Pixelify Sans, Press Start 2P, Silkscreen.
   - NFC-normalize all labels.

---

## 1. Prior art (2025–2026)

### 1.1 Pets and notifiers driven by Claude Code or other agents

| Project | Platform / stack | Event ingress | Ideas worth noting | License |
|---|---|---|---|---|
| [xtrimsystems/claude-pet](https://github.com/xtrimsystems/claude-pet) | Linux Wayland. Python + GTK4 + **gtk4-layer-shell** (Hyprland, sway, river) | Hooks write `/tmp/claude-pet-state`, which the pet polls | Closest to our stack. Uses Shimeji packs; Claude states live at sprite indices 38–50. States: Thinking, Working, Attention, Celebrating, "Doubling" (compaction), Idle after 60 s. One pet for all sessions; starts with the first session and exits with the last. Drag + throw. `--size 128` | MIT |
| [DecoudJuan/Claude-pet](https://github.com/DecoudJuan/Claude-pet) | Electron | Per-session JSON polled every 600 ms. Hooks: SessionStart, UserPromptSubmit, Notification, Stop/StopFailure, SessionEnd. **No PreToolUse**, to avoid "spawning dozens of Node processes per tool call" | Bubble shows the real permission text ("Claude needs your permission to use Bash"). Completion bubble shows project + duration + number of remaining sessions. Priority: waiting > working > idle | MIT |
| [matias-mn/hookling](https://github.com/matias-mn/hookling) | macOS Swift + a C hook | C hook → Unix socket with minimal JSON `{"event":"work_started","session":"<FNV hash>"}` (privacy) | 8 states, priority-ordered with timeouts: waiting 30 min, failed 60 s, working 3 min, review 90 s, sleep after 15 min. Wanders every 8–20 s. Reads Codex sheets | MIT |
| [devnomad-byte/petdex-cc](https://github.com/devnomad-byte/petdex-cc) | Electron | HTTP POST to 127.0.0.1:17321 | Rapid-click easter egg escalating at 2→4→6→9→12→15 clicks. Bubbles auto-dismiss after 3–8 s. 8 levels. 2-min AI-speech cooldown | MIT |
| [shigure0110/clawd-pet](https://github.com/shigure0110/clawd-pet) | Electron (Windows) | HTTP 127.0.0.1:31126 | **43 animation rows generated procedurally in Python/Pillow** (`assets/make_clawd_sprites.py`: 48×52 logical cells ×4, rectangles + letter-legend bitmaps). QA harness (contract checks, contact sheets). Per-frame durations and fallbacks in `renderer/clawd_meta.json`. "A run longer than 15 s ends with 'Ready to move on'… quick chat turns stay silent." Dangle, squash, fling → dizzy. Hides at the screen edge during fullscreen. "Mischief" toggle | MIT |
| [Door3172/clawd-desktop-pet](https://github.com/Door3172/clawd-desktop-pet) | Claude Code plugin; Node + tkinter | Plugin hooks | Types while Claude works, jumps with confetti on finish, waves "Needs your OK!". DND disables pop-ups and sounds. Hide for 10/30/60 min. Visible only while Claude is in front. Drag/fling → dizzy. Ctrl+wheel resizes | MIT (fan work) |
| [almathkurali/octo-pet](https://github.com/almathkurali/octo-pet) | macOS | Claude Code **mods / function hooks** (`turn.start`, `tool.call`, `tool.check`, `turn.complete`, `session.end`) | "The loudest wins: needs input → blocked → ready → running." "+N" badge. Click deep-links into the session. Respects Reduce Motion | MIT |
| [isr431/desk-pet](https://github.com/isr431/desk-pet) | Claude Code function-hooks plugin | `session.start`, `prompt.submit`, `turn.start`, `turn.complete`, `tool.call`, `session.compact`, `classic.Notification` | Tool-specific reactions: reading, hammering for edits, typing for shell, binoculars for web, celebrate/hide on test results, yawn after 5 min idle, gift on completion. **Sound off by default; chime synthesized in code** | MIT |
| [pablodelucca/pixel-agents](https://github.com/pablodelucca/pixel-agents) | VS Code extension + CLI | Hooks (SessionStart, PreToolUse, PermissionRequest, Stop) with a transcript-heuristic fallback | Pixel-art office of agents; optional chimes on finish/permission | MIT |
| [xiangking/agent-pet](https://github.com/xiangking/agent-pet) | Tauri | Watches local activity files; WebSocket on ws://127.0.0.1:8765 | Codex-sheet compatible | MIT |
| Claude Code `/buddy` (1 Apr 2026, [pilot-shell](https://pilot-shell.com/blog/claude-buddy)) | Inside the CLI | — | 18 ASCII species including **capybara ("Special" rarity)**. 5 lines × 12 chars, 3 frames. `/buddy mute` and `/buddy off`. **Not present in the installed 2.1.288** (the binary has no "capybara"/"buddy" strings) | — |
| OpenAI **Codex Pets** (May 2026) | Codex overlay | Internal | Atlas 1536×1872 = **8 columns × 9 rows of 192×208** (rows: idle, running-right, running-left, waving, jumping, failed, waiting, running, review). Spec and QA rules in [openai/skills hatch-pet SKILL.md](https://github.com/openai/skills/blob/main/skills/.curated/hatch-pet/SKILL.md): no size popping, no detached FX inside cells. Statuses: Running / Needs input / **Ready = "finished, but not viewed yet"** / Blocked. Reduce-motion. Users want per-state `durationMs` / `lastFrameDurationMs` ([openai/codex#20863](https://github.com/openai/codex/issues/20863)) | — |
| [PeonPing/peon-ping](https://github.com/PeonPing/peon-ping) | Sound packs for many agents | Hooks mapped to CESP categories | `silent_window_seconds: 10`; `suppress_subagent_complete: true`; user-spam rule "3 prompts in 10 s"; categories session.start, task.acknowledge, task.complete, task.error, input.required, resource.limit, user.spam, session.end | — |

### 1.2 Wayland / Hyprland pets

| Project | Notes |
|---|---|
| [CluelessCatBurger/wl_shimeji](https://github.com/CluelessCatBurger/wl_shimeji) (C, GPL-2.0) | No-input fullscreen layer-shell overlay with **one subsurface per mascot**. 7 mascots ≈ **0.9% CPU / 56 MiB** vs ~19% / 880 MiB for Java Shimeji. **Hyprland officially unsupported**: wl_subsurface squish (#17 sprites squished at the edges; tracked upstream in hyprwm/Hyprland#10515, still OPEN); #24 mascot not repainted where it overlaps a bar; drag released after 1 px on Hyprland (comments in #17/#19); #52 transparency rounding |
| [kyzmapiratov/Menagerie](https://github.com/kyzmapiratov/Menagerie) (Tauri/Rust, wraps wl_shimeji) | Hyprland "working with limitations": mascots get sliced at window borders and panels |
| [pixelomer/Shijima-Qt](https://github.com/pixelomer/Shijima-Qt) | Issues: multi-monitor geometry (#73), "Window mask slowdown" (#55), "Weird grab behaviour" (#45) |
| [jesperls/qs-vpets](https://github.com/jesperls/qs-vpets) (**Quickshell**, MIT) | Verified in source (`components/PetWindow.qml`): `PanelWindow` anchored to all 4 edges on `WlrLayer.Overlay`, `keyboardFocus: None`, `exclusionMode: Ignore`, `mask: dragArea.dragging ? null : petMask` (a Region around the pet). Timer ticks every 16 ms while moving and every 250 ms otherwise. Cursor position comes from polling `hyprctl cursorpos` every 2 s (10 s when idle) (`services/InputTracker.qml`); Hyprland raw events count as user activity. Animation fallback chains. `idleTimeout` = 300 s. Retreats from fullscreen. Uses Pokémon PMD sprites (IP) |
| wayneko ([page](https://leon_plickat.srht.site/wayneko.html), GPL-3.0) | Lives on the bottom edge only, because "Wayland does not support input spying" |
| koneko (KWin script), neko-mangowm, [hyprglaze](https://github.com/slastra/hyprglaze) (sprite "buddy" drawn inside a shader wallpaper) | Minor references |

### 1.3 Ideas worth copying

- **One pet, many sessions.** Pick the body state with "loudest wins" (needs-input > error > ready > working > thinking > idle > sleep) and show the other sessions as a "+N" badge (octo-pet, Hookling).
- **State timeouts.** A working state with no events for 3 min goes back to idle; waiting expires after 30 min (Hookling). This protects against missed Stop events.
- **"Ready = finished but not viewed yet"** as a persistent quiet badge (Codex Pets).
- **Turn-length threshold.** No fanfare for quick answers (Claw'd: > 15 s; PeonPing: 10 s). **Suppress subagent completions** (PeonPing).
- **Focus-aware suppression.** Stay quiet when the user is already looking at that terminal (wmedia.es, martinoyovo, PeonPing #255).
- **Rich bubbles.** Show the project name, duration, permission/tool name, and the number of sessions still running (DecoudJuan).
- **Drag physics.** Dangle while held, squash on landing, dizzy after a fling (Claw'd, clawd-desktop-pet).
- **Tool-specific micro-reactions** (desk-pet).
- **Subagent "clones"** (claude-pet "Doubling", Claw'd "summon").
- **Per-frame durations and fallback chains stored as data** (Claw'd meta, qs-vpets, Codex #20863).
- **Procedural sprite generation with a QA harness and contact sheets** (Claw'd).
- **Sound off or soft by default, synthesized in code** (desk-pet).
- **Mute, hide and snooze controls** (`/buddy mute|off`, clawd-desktop-pet's hide for 10/30/60 min), plus Reduce Motion (octo-pet, Codex).

### 1.4 What users complain about

These are real issues, mostly from PeonPing.

- **#486:** the completion sound repeats about every 60 s. Claude Code re-fires `Notification`/`idle_prompt` while the terminal is unfocused, and it was not de-duplicated against Stop. Users want "exactly one sound per completion".
- **#340:** sound bites from many agents overlap ("50 voice lines all at once").
- **#536, #306, #289:** noise from subagents.
- **#284:** users *want* reminders for decisions, but with a bounded number of repeats.
- **#255:** users want silence when the tab that produced the event is focused. Focus detection is fragile: #254 (iTerm2), #472 (alacritty/zellij), #541 (tabs sharing a cwd).
- **#320:** an overlay steals focus.
- **#303, #589:** overlay processes stuck at about 65–68% CPU.
- **#528:** sounds play while muted.
- **#378:** critical urgency is annoying.
- **Clippy lesson:** Horvitz's Bayesian "when to interrupt" model was replaced by a cruder trigger so the assistant would appear more often. Interruptions must be earned.
- **Hyprland pets:** rendering and drag bugs (section 1.2).
- **Electron/Java pets:** heavy CPU and RAM.
- **Codex Pets:** IP and moderation issues with user-generated characters.

---

## 2. Attention without annoyance

### 2.1 Principles and what they mean here

- **Motion onset captures attention; continuous motion does not.** Keep idle almost still: breathing at about 0.7 Hz and blinks. Express urgency as **bursts** separated by stillness, never as constant shaking.
- **Twelve animation principles** (Thomas & Johnston), applied to pixel art:
  - **Anticipation:** a crouch of 1–2 frames (≈ 100–140 ms) before any jump, shout or wave.
  - **Squash & stretch, area-preserving:** rest 24×20 (480 px²), squash 28×17 (476), stretch 20×24 (480). Hand-draw these frames; never resample.
  - **Overshoot:** bubble size 60% → 110% → 100%; "!" drawn big then normal; landing squash followed by a small rebound hop.
  - **Follow-through / overlapping action:** ears, tail, antenna and the capybara's orange lag the body by 1–2 frames.
  - **Secondary motion:** orange wobble, tail swish, slime jiggle.
  - **Holds:** 160 ms at the top of a hop; 400–1000 ms on victory poses; a comedic "take" hold of about 300 ms before a reaction.
  - **Arcs and slow-in/slow-out:** jump offsets follow a parabola.
  - **Smear frames:** at most 1 per fast move, opaque palette colors only.
  - **Sub-pixel tricks:** for breathing, shift highlight pixels instead of moving the whole body.
- **Calm Technology** (Amber Case): use the least attention possible, use the periphery, inform and create calm.
- **WCAG 2.3.1:** no more than 3 flashes in any 1 s. Keep blinking badges at 2 Hz or less and small (well under 25% of a 10° field, i.e. under 341×256 px at 1024×768).

### 2.2 Attention levels (used by the catalog)

| Level | Name | What happens |
|---|---|---|
| 0 | Ambient | Motion stays inside the sprite; no bubble, no sound |
| 1 | Notice | One short motion onset and/or a badge; bubble ≤ 3 s; no sound (or very soft) |
| 2 | Alert | Burst + bubble with project name + **one** sound |
| 3 | Celebration / alarm | Large motion, particles beyond the body, louder sound, optional power-up "grow" flicker N 4↔6 |

Levels 2–3 are rate-limited.

### 2.3 "Claude needs input" escalation

Triggers: `PermissionRequest`, or `Notification` with `notification_type` = `permission_prompt` / `elicitation_dialog` / `agent_needs_input`. De-duplicate per session within 2 s, because PermissionRequest and Notification can both fire for the same prompt.

| Time since request | Level | Visual | Sound |
|---|---|---|---|
| 0 s | **L1**, level 2 | `alert_L1`: anticipation + pop + "!", then the `waiting` loop with a hop burst every 8 s. Bubble: "<projeto>: precisa de você" / "permissão: Bash" | ping, once |
| +30 s | **L2**, level 2 | Jump + two-arm wave; burst every 6 s for 30 s | ping×2 (+2 dB), once |
| +90 s, or the user comes back after ≥ 2 min away with something pending | **L3**, level 3 | Bounce + shake + "!!" blinking at ≤ 2 Hz; burst every 4 s for 20 s; optional grow flicker ×3 | 3-note chirp, once |
| ≥ 5 min | **L4**, level 1 (cap) | `waiting` loop + slow badge pulse at 1 Hz; burst every 30 s | None. Optional reminder sound every 10 min, **off by default** |

- **Exit escalation** on any of: UserPromptSubmit; PostToolUse or PostToolUseFailure in that session (permission was resolved); PermissionDenied; Stop; SessionEnd; a click on the pet (acknowledge: "ok!" nod, snooze 5 min); or focus moving to that session's terminal.
- **"User returned"** = cursor movement or a Hyprland activewindow/workspace event after ≥ 2 min idle (the qs-vpets technique). Playing the burst at that moment gives reminders that are actually seen, without the unbounded repeats of PeonPing #486.
- **Use one global escalation clock** (the oldest pending item) so several sessions do not stack escalations. Show "+N" for the rest.

### 2.4 Scaling "done" by the amount of work

Each session keeps counters between UserPromptSubmit and Stop. Read only metadata, never content.

- `dur_min`: Stop time minus prompt time.
- `tools`: PostToolUse + PostToolUseFailure.
- `files`: unique `tool_input.file_path` for Edit/Write/MultiEdit/NotebookEdit.
- `subagents`: SubagentStart count.
- `tasks`: TaskCompleted count.
- `fails`: PostToolUseFailure count.

Score and tiers:

```
score = dur_min*1.0 + tools*0.15 + files*0.5 + subagents*1.0 + tasks*0.5   (cap 20)
tier 0 (nod, silent): dur < 15 s and tools == 0
tier 1 (done_small):  score < 3
tier 2 (done_medium): 3 ≤ score < 8
tier 3 (done_big):    score ≥ 8   (at most one per 10 min; otherwise downgrade to tier 2)
bonus: tests red→green inside the turn → "phew" prefix and +1 tier; git push / gh pr create → rocket micro
```

Log the score so the thresholds can be tuned. All values are config.

### 2.5 Debounce, coalescing and cooldowns

- **Coalescing window: 3 s.** If 2 or more sessions finish together, play one medium/big celebration with the bubble "2 prontos: api, web".
- **Stop de-duplication:** 5 s per session (`stop_hook_active` loops).
- **SubagentStop:** never celebrates; it only feeds the counters.
- **`idle_prompt`:** **never plays sound.** At most one silent hop every 2 min, used as a "user away" hint.
- **Reaction queue:** priority + TTL. Attention preempts done, done preempts micro-reactions. Drop non-attention reactions older than 3 s.
- **Sound arbiter:**
  - one sound at a time;
  - at least 1.5 s between sounds;
  - at most 6 sounds per minute;
  - per-category cooldowns: done 5 s, error 10 s, attention once per escalation step;
  - global mute;
  - silent while DND, quiet hours or reduced-sound mode is on;
  - no completion sound when the session's terminal is focused and the user was active in the last 30 s. Visuals still play, since the user wants to see the celebration.
- **Optional easter egg:** 3 prompts in 10 s (PeonPing `user.spam`) → "annoyed" reaction.

### 2.6 Speech bubble rules (PT-BR)

- **Project label**
  - Source: basename of the git toplevel or `cwd`. Fallback: `session_title`, which arrives with UserPromptSubmit in 2.1.288.
  - Apply NFC normalization. Cap at 18 chars, ellipsis "..".
  - Give each project a fixed color (hash → one of 6 palette colors) for the flag/dot.
- **Templates**
  - Start: "oi! <projeto>"
  - Done: "<projeto>: pronto!" with optional second line "6 arquivos · 8 min"
  - Needs input: "<projeto>: precisa de você" / "permissão: Bash" (tool_name only, never the command)
  - Error: "<projeto>: erro (overloaded)" / "limite atingido…"
  - Several sessions: "2 prontos: api, web" / "+2"
- **Timing**
  - The bubble pops with overshoot, then text appears typewriter-style at 40 chars/s (optional per-character blip, off by default).
  - Done bubbles stay 4–6 s. Attention bubbles stay; after 15 s they collapse into a small "!" badge to reduce occlusion.
  - Clicking the bubble or the pet acknowledges.

### 2.7 Idle variety

- **Breathing:** `idle` loop, pingpong.
- **Blinks:** random every 2.5–6 s; 15% of them are double blinks.
- **`look_around`:** every 20–45 s, or when the cursor stays within about 300 logical px for more than 1 s. Cursor position: Hyprland IPC `cursorpos`, polled at 2 Hz or less, only while idle.
- **Fidgets:** pool of 3–4 per character, every 45–120 s, weighted, never the same twice in a row.
- **Rare easter eggs (2%)**
  - Capybara: a bird lands on its head, or it balances 3 oranges.
  - Brazilian touches: "cafezinho" in the morning; sunglasses + "sextou!" on Friday after 17:00 when done_big fires.
- **Sleep cycle:** `yawn` after 3–5 min idle; sleep after 8 min with no session working (qs-vpets uses 300 s, Hookling 15 min, Claw'd 2.5–8 min). While asleep: Z particles every 2 s and a render rate of 2 fps or less.
- **Waking:** soft wake for normal events; startle wake for attention events.

### 2.8 Click and drag reactions

- **Hover:** ear twitch / look at the cursor.
- **Click** (shorter than 250 ms, moved less than 4 px): giggle squish + heart; also acknowledges pending items.
- **Click-spam escalation:** laugh → dizzy → furious puff → faint.
- **Drag:** `pickup` (2 frames), then the `dangle` loop. Legs kick; the lean frame is chosen by horizontal velocity (hysteresis ±40 px/s); the orange/tail/antenna lags.
- **Drop:** squash → rebound hop → settle, with a dust puff. The pet stays exactly where it was dropped; the bounce is a sprite offset only.
- **Optional fling:** above about 1500 px/s, slide with friction, bounce off the monitor edges, land dizzy.
- **Right-click:** later (menu: DND, snooze, size, character, quit).

### 2.9 Do-not-disturb, presence and placement

- **Omarchy DND, verified on this host**
  - State is persisted in `~/.local/state/omarchy/notifications.json` as `{"version":3,"dnd":true|false}`. The file is absent here, which means DND off.
  - It can also be queried with `omarchy-shell notifications isDnd` (returns "on"/"off").
  - Watch the **directory**, not the single file: the file may be replaced on write.
  - While DND is on: mute all sounds, cap visuals at level 1, keep badges. A `dnd_on` animation (sleep mask / headphones) is optional.
- **Fullscreen and screensaver:** hide or pause when the active window is fullscreen (`hyprctl -j activewindow` field `fullscreen`). The Omarchy screensaver is a fullscreen **foot** window with app-id `org.omarchy.screensaver` (observed live). Pause sounds and rendering while the session is locked (lock detection unverified).
- **Quiet hours:** default 22:00–08:00, no sound.
- **Snooze:** 30/60 min. Reduced motion: no particles, no flicker, attention capped at level 1.
- **Placement**
  - Omarchy toasts are on the Overlay layer anchored top-right; the bar is on the Top layer at the top.
  - Claude Code's input box sits at the bottom of a tiled terminal.
  - So the default home is the **right edge at about 40% of the height**. The user-dragged position is remembered per monitor as relative coordinates, snapped to 4 logical px.
- **Monitor change:** `poof_out` on the old monitor, `poof_in` + landing on the new one.
- **Focus mapping:** the default terminal is foot (`~/.config/xdg-terminals.list` = foot.desktop), running as standalone processes (`pgrep` shows `foot --working-directory=…`). The PID chain hook → claude → shell → foot can therefore be matched to `hyprctl clients` pids for focus-aware muting and click-to-focus. HTTP hooks carry no PID; command hooks can send `$PPID`.

---

## 3. Pixel-art pipeline without a human artist

### 3.1 Canvas and scale

| Option | Logical size | Device size at 1.5 | Share of laptop (1280×800 logical) | Verdict |
|---|---|---|---|---|
| 32-px body, N=4 (cell 48×48 → 192 logical) | body ≈ 128 | 192 (cell 288) | 10% of width, 16% of height | **Recommended** |
| 48-px body, N=4 | 192 | 288 | 24% of height | Too big and occluding |
| 48-px body, N=2 | 96 | 144 | — | Loses the chunky look and the attention value |
| N=3 or N=5 | — | 4.5 / 7.5 px | — | **Uneven pixels / blur; avoid** |

- On the 4K at 1.5 (2560×1440 logical), the body is 5% of the width. Allow `scale` ∈ {2, 4, 6, 8} per monitor if desired.
- **Feet anchor** at y ≈ 45 of the 48-px cell. Small offsets (≤ 8 px) are baked into frames so the sheet is WYSIWYG in Aseprite. Big jumps, flight and drag are runtime offsets.
- **QA check for crispness:** take a `grim` screenshot of a checkerboard test sprite and assert that every art pixel is a uniform 6×6 device block. GTK 4.22 / Qt 6.11 fractional-scale rendering on this setup is **unverified**.

### 3.2 Palette "pet18": `.` = transparent, 17 colors, 1-bit alpha

```toml
"." = "transparent"
"k" = "#1B1427"  # outline (ink)
"v" = "#3B2A4A"  # violet shadow
"b" = "#5C3A2E"  # dark brown
"B" = "#8C5A3C"  # brown
"t" = "#C08A5A"  # caramel
"s" = "#E3B97F"  # sand
"c" = "#FFF1D6"  # cream (sticker border / bubble fill)
"w" = "#FFFFFF"  # white (eye glint, sparkles)
"o" = "#FF8C1A"  # orange
"O" = "#C85A12"  # dark orange
"y" = "#FFD23F"  # yellow (stars, confetti)
"g" = "#4CC35A"  # green (leaf, success)
"u" = "#3A86FF"  # blue (tear, robot screen)
"r" = "#E5394B"  # red (error, heart)
"p" = "#FF7AB6"  # pink (blush, heart)
"m" = "#9AA3B5"  # grey (robot, Z, dust)
"M" = "#5A6275"  # dark grey
```

Add a 3-step ramp for the slime, e.g. greens `#9BE66B` / `#4CC35A` / `#2E7D46`. Hue-shift the ramps: cooler, more violet shadows; warmer highlights.

**Contrast check (WCAG ratio, computed here)**

| Color | hackerman bg #0B0C16 (current theme, dark) | white | catppuccin-latte #EFF1F5 | mid grey #808080 |
|---|---|---|---|---|
| ink #1B1427 | **1.09** (invisible) | 17.86 | 15.79 | 4.52 |
| cream #FFF1D6 | **17.44** | 1.12 (invisible) | 1.01 | 3.54 |
| brown #8C5A3C | 3.38 | 5.76 | 5.10 | 1.46 |
| orange #FF8C1A | 8.36 | 2.33 | 2.06 | 1.70 |

**Conclusion: use a "sticker" double outline** — a 1-px ink inner outline plus a 1-px cream outer border. It reads on any wallpaper, including the dark hackerman theme and the light themes Omarchy ships.
- The build script generates the outer border by dilating the alpha mask, so the artist only draws fills and the ink line.
- Emit variants: `sticker`, `ink-only`, `none`.
- Optionally pick the variant from `~/.local/state/omarchy/current/theme/colors.toml` (`mode = "dark"|"light"`). An Omarchy hook directory exists for theme changes: `~/.config/omarchy/hooks/theme-set.d`.

### 3.3 Source formats

- **Grid file:** one frame per block, one character per pixel, legend from the palette. Patch frames only overwrite the pixels they set.

```text
== idle_0  (48x48)
................................................
...................ggg..........................
..................kooOk.........................
... (48 rows)
== eyes_closed  patch 6x2 @ (18,20)
kkkkkk
......
```

- **Manifest `art/capivara/anim.toml`:**

```toml
[character]
id = "capivara"
cell = [48, 48]
pivot = [24, 45]          # feet
hitbox = [8, 14, 32, 32]  # clickable region / input mask
palette = "../palette.toml"

[tags.idle]
direction = "pingpong"
loop = true
attention = 0
frames = [ { grid = "idle_0", ms = 400 }, { grid = "idle_1", ms = 300 } ]

[tags.done_small]
loop = false
attention = 2
sfx = "coin"
then = "ready"
frames = [
  { grid = "crouch",  ms = 140 },
  { grid = "stretch", ms = 90,  dy = -2 },
  { grid = "air",     ms = 90,  dy = -6 },
  { grid = "air",     ms = 160, dy = -8, fx = ["sparkle@head"] },
  { grid = "air",     ms = 90,  dy = -6 },
  { grid = "squash",  ms = 90 },
  { grid = "crouch",  ms = 110 },
  { grid = "idle_0",  ms = 240 },
]
```

- **Transform ops:** `dx/dy`, `flip = "x"`, palette `swap = { o = "r" }` (moods / shiny), `patch` overlays.
- **Key poses** (crouch / squash / stretch / air / dangle) are hand-authored once and reused by many tags.
- **Hybrid approach** (Claw'd does this): the body can be drawn by code from primitives (rounded rectangles, no anti-aliasing), parameterized by squash; faces, props and particles stay as grid "stamps".

### 3.4 Build outputs (Python 3 + Pillow, Docker build stage)

```
dist/<char>/sheet.png      # cells 48x48, 1 row per tag
dist/<char>/sheet.json     # Aseprite json-array
dist/<char>/anim.json      # runtime semantics: loop/then/attention/sfx/fx cues/offsets/fallback
dist/<char>/<char>.aseprite  # optional, for human editors
dist/<char>/preview/<tag>.gif + contact.png   # checkerboard light/dark background, 4x
dist/fx/sheet.png+json     # shared particles
dist/ui/sheet.png+json     # 9-slice bubble, tail, icons, badges
dist/font/monogram-atlas.png+json
dist/sfx/*.wav
```

**Aseprite JSON fields**, verified against `aseprite/src/app/doc_exporter.cpp`:
- `frames[]`: `filename`, `frame{x,y,w,h}`, `rotated`, `trimmed`, `spriteSourceSize`, `sourceSize`, `duration` (ms).
- `meta`: `app`, `version`, `image`, `format` ("RGBA8888"), `size`, `scale` ("1"), `frameTags[]`, `layers[]`, `slices[]`.
- `frameTags[]`: `name`, `from`, `to`, `direction` ("forward" / "reverse" / "pingpong" / "pingpong_reverse" — the last string is medium-confidence), `repeat` **as a quoted string**, plus userData `color` / `data`.
- `slices[]`: keys with `frame`, `bounds`, optional `center` (**9-slice**) and `pivot`.

```json
{"frames":[{"filename":"capivara idle 0","frame":{"x":0,"y":0,"w":48,"h":48},"rotated":false,"trimmed":false,
  "spriteSourceSize":{"x":0,"y":0,"w":48,"h":48},"sourceSize":{"w":48,"h":48},"duration":400}],
 "meta":{"app":"https://www.aseprite.org/","version":"pet-build","image":"sheet.png","format":"RGBA8888",
  "size":{"w":384,"h":1152},"scale":"1",
  "frameTags":[{"name":"idle","from":0,"to":1,"direction":"pingpong"},
               {"name":"done_small","from":2,"to":9,"direction":"forward","repeat":"1"}],
  "layers":[{"name":"body","opacity":255,"blendMode":"normal"}],
  "slices":[{"name":"pivot","color":"#0000ffff","keys":[{"frame":0,"bounds":{"x":20,"y":44,"w":8,"h":2},"pivot":{"x":4,"y":1}}]},
            {"name":"hitbox","color":"#ff0000ff","keys":[{"frame":0,"bounds":{"x":8,"y":14,"w":32,"h":32}}]}]}}
```

**`.aseprite` writer:** feasible from the documented chunks in `aseprite/docs/ase-file-specs.md`:
- header `0xA5E0`, colour depth 32;
- per-frame header `0xF1FA` with duration in ms;
- Layer `0x2004`;
- Cel `0x2005` type 2 (zlib RGBA);
- Tags `0x2018` (from, to, direction 0–3, repeat, name);
- optional Slice `0x2022` (flag 1 = 9-patch, 2 = pivot).

Keep behaviour data in the `anim.json` sidecar, not in Aseprite user data. LibreSprite (1.1 lineage) probably lacks tag user data and slices.

### 3.5 Round trip for a future human artist

- **Aseprite:** paid, or compile from source. CLI: `aseprite -b capivara.aseprite --sheet sheet.png --data sheet.json --format json-array --list-tags --list-slices`.
- **LibreSprite 1.2:** in Arch `extra`, GPL-2.0, "based on the last GPL2 commit of Aseprite". JSON tag export likely works but is unverified.
- **Pixelorama** (MIT): reportedly opens `.aseprite`.
- **Back-conversion:** add a `png2grid` importer that quantizes to the palette, so text grids can remain the source of truth. Alternatively, switch the source of truth to `.aseprite` once an artist joins.
- The runtime only ever consumes PNG + JSON, so the art source does not matter to it.

### 3.6 Lints and QA

Codex and Claw'd QA rules, adapted:
- exact grid size; unknown legend characters; total colors ≤ 24; **no partial alpha**;
- orphan single pixels (warning);
- bounding-box popping inside a tag: at most 2–3 px unless the tag is flagged `squash` or `jump`;
- feet stay on the pivot except on airborne frames;
- one-shot tags ≤ 4 s;
- palette contrast checks;
- regenerate GIFs and the contact sheet in CI for PR review;
- `grim` crispness test.

### 3.7 Particles and effects (shared `fx` sheet)

Drawn at N=4, 1-bit alpha, ink outline. Physics in logical px; draw positions snapped to the art grid.

| Effect | Size (art px) | Frames @ ms | Behaviour |
|---|---|---|---|
| confetti a–d | 2×3 / 3×3 | 2 @ 80 (flip) | Launch speed 150–350 px/s in a ±60° upward cone, gravity 600 px/s², lifetime 1.2–2 s, max 40 at once |
| sparkle | 7×7 | 4 @ 70 | dot → plus → star → dot |
| heart | 7×6 | 2 @ 200 | Rises 30 px/s with ±4 px sway, 1.5 s |
| sweat drop | 3×5 | 3 @ 200, 200, 150 | — |
| Z | 5×5 → 6×6 → 7×7 | — | Rises diagonally over 2.5 s |
| "!" | 3×9 | 2 (normal / bright, ≤ 2 Hz) | — |
| "?" | 5×9 | — | — |
| music note | 5×7 | — | — |
| "..." dots | 9×3 | 3 @ 300 | — |
| poof | 12×12 | 4 @ 60 | — |
| dust | 6×4 | 3 @ 70 | — |
| firework | rocket 1×3 + burst 17×17 | burst 5 @ 70 | — |
| sunburst rays | 40×40 | 4 @ 90 | Drawn behind the pet |
| small icons (check, cross, star, key, gear, magnifier, pencil, globe, rocket, stamp) | ~7×7 | — | — |
| mini-pet (subagent clone) | 12×12 | 4 @ 150 | — |

### 3.8 9-slice speech bubble (UI scale N_ui = 2)

- **Source:** 12×12 px with rounded corners (corner pixels transparent), 1-px ink border, cream fill, 1-px sand shadow on the bottom/right.
- **Slice:** `bubble` with `center {x:4,y:4,w:4,h:4}`.
- **Tail:** separate 7×5 sprite; it flips side near screen edges.
- **Padding:** 4 px horizontal, 3 px vertical. Up to 24 chars per line, 2 lines.
- **Variants:** neutral (cream), alert (yellow `#FFD23F` fill), error (red border), success (green border). Use icons from `ui`, not emoji.
- **Pop animation:** render the 9-slice at integer sizes 60% → 110% → 100% (50 ms each) for a crisp overshoot.

### 3.9 Fonts covering á â ã à é ê í ó ô õ ú ç (and their capitals)

| Font | License | Coverage evidence | Grid | URL |
|---|---|---|---|---|
| **monogram** (datagoblin) | **CC0 1.0** | itch page lists "english, español, русский, português, ελληνικά…"; ships `monogram.ttf`, `monogram-bitmap.png`, `monogram-bitmap.json` (12 rows per glyph as bitmasks) | 6×12 cell, 5-px glyphs; TTF crisp at size 16 × k (community notes, medium confidence) | https://datagoblin.itch.io/monogram |
| **Tiny5** (Stefan Schmidt) | OFL | google/fonts METADATA subsets: latin, latin-ext, cyrillic(-ext), greek | 5 px tall (medium confidence) | https://fonts.google.com/specimen/Tiny5 |
| **Departure Mono** (Helena Zhang) | OFL | v1.500, 1,186 glyphs, "Basic Latin, Latin-1, Latin Extended-A" | "set the font size to increments of 11px" | https://departuremono.com · https://github.com/rektdeckard/departure-mono |
| Pixelify Sans / Press Start 2P / Silkscreen | OFL | subsets include latin + latin-ext | Pixelify is variable 400–700 (less grid-exact); Press Start 2P is 8×8 and wide (titles only) | fonts.google.com |

- The Google Fonts "latin" subset implies **GF Latin Core**. Verified: that glyph list contains atilde, otilde, ccedilla, acircumflex, ecircumflex, ocircumflex, agrave, aacute, eacute, iacute, oacute, uacute, Atilde, Otilde, Ccedilla.
- **Recommendation:** monogram at N_ui = 2 (12×24 logical px per character). Build the atlas from `monogram-bitmap.json`.
- For OFL fonts, ship `OFL.txt`; a renamed derivative is needed if a Reserved Font Name applies to a converted atlas.
- **Text pipeline:** NFC normalization, then a missing-glyph fallback (NFKD with combining marks stripped), then "?".

### 3.10 Renderer gotchas

- **Qt:** `AnimatedSprite.interpolate` **defaults to true** (it blends frames) and has **no per-frame durations**. Use a custom frame stepper + `Image { smooth: false; mipmap: false; sourceClipRect: … }`.
- **GTK:** `gtk_snapshot_append_scaled_texture(…, GSK_SCALING_FILTER_NEAREST, …)` is available since GTK 4.10.
- **Rendering cadence:** render only when the frame changes. Idle 4–8 fps, particles up to 30 fps, sleep ≤ 2 fps.
- **Input mask:** update it only when the bounding box changes (Shijima-Qt #55: window masks are slow).

---

## 4. Sound

### 4.1 Approach

- Sounds are generated at build time by an in-repo synth: Python stdlib `wave` + `math`, deterministic seed. The resulting WAVs are the project's own work (license them CC0).
- **Two generators**
  1. **Note sequencer** for melodic jingles: square wave (duty 12.5/25/50%), triangle, sine, noise; ADSR envelope with punch; pitch slide/vibrato; one-pole low-pass at about 6–8 kHz to soften the chiptune edge.
  2. **sfxr port** for noisy effects. Parameters: wave_type, p_env_attack/sustain/punch/decay, p_base_freq, p_freq_limit/ramp/dramp, p_vib_strength/speed, p_arp_mod/speed, p_duty/duty_ramp, p_repeat_speed, p_pha_offset/ramp, p_lpf_freq/ramp/resonance, p_hpf_freq/ramp, sound_vol. Presets: pickupCoin, laserShoot, explosion, powerUp, hitHurt, jump, blipSelect, synth, tone, click, random.
- jsfxr is **Unlicense** and the original sfxr is MIT, so porting the algorithm is license-clean. Designers can tweak sounds at sfxr.me and commit the parameter JSON. **ZzFX** (MIT; 21 parameters from volume to filter) is an alternative.

```python
import math, random, struct, wave
SR = 48000
def tone(f0, ms, w="square", duty=0.25, slide=0.0, vol=0.5, atk=5, rel=40):
    n=int(SR*ms/1000); ph=0.0; out=[]
    for i in range(n):
        f=f0*(1+slide*i/n); ph=(ph+f/SR)%1.0
        s={"square":1.0 if ph<duty else -1.0,"tri":4*abs(ph-0.5)-1,
           "noise":random.uniform(-1,1)}.get(w, math.sin(2*math.pi*ph))
        t=i*1000/SR; env=min(1,t/atk)*min(1,(ms-t)/rel)
        out.append(s*env*vol)
    return out
def write(path, x, peak_dbfs=-3.0):
    g=10**(peak_dbfs/20)/max(1e-9,max(map(abs,x)))
    with wave.open(path,"wb") as w:
        w.setnchannels(1); w.setsampwidth(2); w.setframerate(SR)
        w.writeframes(b"".join(struct.pack("<h",int(max(-1,min(1,v*g))*32767)) for v in x))
write("coin.wav", tone(987.8,60)+tone(1318.5,140))   # B5 -> E6
```

### 4.2 Which sound for which event

| SFX | Event | Design | Length | Target M_max (LUFS) |
|---|---|---|---|---|
| hello | session start (only if the pet was asleep or absent) | E5 → A5, triangle | 0.2 s | −26 |
| coin | done_small | B5 60 ms → E6 140 ms, square 25% | 0.2 s | −24 |
| powerup | done_medium | C5-E5-G5-C6 arpeggio (70 ms each, last 160), light vibrato | 0.45 s | −21 |
| fanfare | done_big | Noise "pop" 40 ms, then G4-C5-E5-G5 (90 ms each), rest, E5, G5 held 360 ms with 6 Hz vibrato, triangle bass an octave below | 1.0–1.2 s | −18 (never louder than the system `complete.oga`) |
| ping | attention L1 | A5 → E6, triangle + square blend | 0.22 s | −21 |
| ping2 | attention L2 | ping twice, second +2 semitones | 0.5 s | −19 |
| chirp3 | attention L3 | 3 rising slides (+40%, 90 ms each) | 0.6 s | −18 |
| bonk | error (StopFailure) | square 330 → 165 Hz slide + noise tick | 0.25 s | −22 |
| sleepy | rate limit | triangle 523 → 392 Hz slide with vibrato | 0.5 s | −26 |
| bye | session end | A5 → E5, triangle | 0.25 s | −26 |
| giggle / boop / thud / boing | click / pickup / drop / fling (optional, off by default) | Short blips; low-passed noise for thud | ≤ 0.25 s | −30 to −28 |

Use **no sound** for working, thinking, tool micro-reactions, monitor changes and `idle_prompt`. Never loop a sound.

### 4.3 Loudness

- **Reference norms:** EBU R128 is −23 LUFS / −1 dBTP. Sony ASWG-R001 is −24 LUFS ±2 for console games and −18 LUFS for mobile. Short UI sounds have no formal standard.
- **Measured on this host** with `ffmpeg 9.0.1` and the `ebur128` filter, on freedesktop sounds:

| File | Integrated | Momentary max | True peak |
|---|---|---|---|
| `complete.oga` | −17.1 LUFS | −13.6 LUFS | −1.4 dBFS |
| `message-new-instant.oga` | −30.4 | −26.9 | −15.4 |
| `dialog-warning.oga` / `window-attention.oga` | −27.6 | −27.6 | −20.3 |
| `bell.oga` (0.14 s) | −70 (gated) | — | −10.3 |

- `bell.oga` shows that **integrated LUFS is meaningless for clips under 400 ms**. So: pad each clip to ≥ 400 ms, measure the momentary max, keep a per-sound dB gain table tuned by ear, peak-normalize to −3 dBFS or lower, and use attack ≥ 5 ms and release ≥ 20 ms.
- The proposed −30…−18 LUFS band sits between the system's quiet message sound and its loud "complete" sound.
- Defaults: master volume 0.6; mute toggle; quiet hours.

### 4.4 Playback

- Command: `pw-play --media-role=Notification --volume=0.6 file.wav`. `pw-play --help` on host 1.6.8 shows `--media-role` (default "Music"), `--media-category`, `--volume`, `-P`.
- PulseAudio-protocol equivalent: `paplay --property=media.role=event` (libcanberra uses the event role).
- One player at a time, governed by the arbiter (section 2.5).

---

## 5. Animation catalog

Frames are 48×48 cells. ms values are per-frame durations. Attention levels are from section 2.2. Timings are partly calibrated against Claw'd's shipped `clawd_meta.json`: hop [140,90,90,160,90,90,110,240], yawn [220,160,200,450,300,140,220,500], dance 8×125, dangle loop 4×130.

| # | Tag | Trigger | Frames | Timing (ms) | Mode | Attn | Sound |
|---|---|---|---|---|---|---|---|
| 1 | idle | default | 4 | 400, 300, 400, 300 (pingpong) | loop | 0 | – |
| 2 | blink | random every 2.5–6 s (15% double) | 3 (eye patch) | 50, 90, 50 | one-shot overlay | 0 | – |
| 3 | look_around | 20–45 s, or cursor near | 6 | 120, 900, 120, 120, 900, 120 | one-shot | 0 | – |
| 4 | fidget_* (3–4 per character) | idle 45–120 s, no repeats | 6–10 | 100–150 + holds 300–500 | one-shot | 0 | – |
| 5 | rare_* (easter eggs, 2%) | idle | 10–16 | 100–150 | one-shot | 0 | – |
| 6 | yawn | idle ≥ 3 min | 8 | 220, 160, 200, 450, 300, 140, 220, 500 | one-shot | 0 | – |
| 7 | sleep_in → sleep | idle ≥ 8 min, nothing running | 6 → 2 (+Z every 2 s) | 150×4, 300, 500 → 800, 800 | one-shot → loop (≤ 2 fps) | 0 | – |
| 8 | wake_soft / wake_startle | any event / attention event | 5 / 5 (+"!") | 150, 120, 120, 200, 300 / 60, 60, 80, 120, 250 | one-shot | 0 / 1 | – |
| 9 | hello (wave) | SessionStart startup/resume; spawn | 8 | 120, 80, 120, 120, 120, 120, 100, 200 (≈ 0.9 s) | one-shot | 1 | hello (if the pet was asleep) |
| 10 | bye (wave → walk off / nap) | SessionEnd (not clear/resume) | 8 + 6 | 100 each (≈ 1.4 s) | one-shot | 1 | bye |
| 11 | thinking | UserPromptSubmit; gap > 4 s without tool events | 4 (+dots fx) | 220 | loop | 0 | – |
| 12 | working (typing on a tiny laptop) | PreToolUse / PostToolUse | 4 | 100 | loop, 3-min timeout | 0 | – |
| 13 | tool_read / edit / bash / web / test_ok / test_fail / commit / push | PostToolUse(Failure) by tool_name / command pattern | 2–6 | 70–200 (≤ 0.9 s) | one-shot overlay | 0 | – |
| 14 | subagent_spawn + mini clone | SubagentStart / SubagentStop | poof 4 + mini 4 | 60 / 150 | one-shot + loop | 0 | – |
| 15 | compact | PreCompact → PostCompact | 6 | 120 | loop (cap 60 s) | 0 | – |
| 16 | alert_L1 | PermissionRequest; Notification permission_prompt / elicitation_dialog / agent_needs_input | 6 | 120, 60, 60, 160, 100, 150 | one-shot → waiting | 2 | ping |
| 17 | waiting (holds a "?" sign / taps foot) | after L1 | 4 + hop burst every 8 s | 250 | loop | 1 | – |
| 18 | alert_L2 | +30 s unresolved | 10 | 80–100, bursts every 6 s × 30 s | burst | 2 | ping2 (once) |
| 19 | alert_L3 (+ optional grow flicker N 4↔6 ×3) | +90 s, or user returns while pending | 12 | 70–90, bursts every 4 s × 20 s | burst | 3 | chirp3 (once) |
| 20 | alert_L4 | ≥ 5 min | waiting + badge pulse 1 Hz | 500; burst every 30 s | loop | 1 | – |
| 21 | ack ("ok!" nod) | click / prompt submitted | 3 | 80, 80, 200 | one-shot | 0 | – |
| 22 | done_nod | Stop tier 0 | 3 | 100, 150, 200 | one-shot | 0 | – |
| 23 | done_small (hop + sparkles) | Stop tier 1 | 8 | 140, 90, 90, 160, 90, 90, 110, 240 (≈ 1 s) | one-shot → ready | 2 | coin |
| 24 | done_medium (jump + 360° spin + paw up + 12 confetti) | Stop tier 2 | 14 | 140, 80, 70×4, 90, 160, 90, 90, 120, 400… (≈ 1.6 s) | one-shot → ready | 2–3 | powerup |
| 25 | done_big_{dance, fireworks, victory, doublespin} (rotating, no repeats) + rays + 30–40 confetti | Stop tier 3 (cooldown 10 min) | 16–24 + fx | dance 8×125 ×3; fireworks 3 bursts × 5×70 staggered 250; victory hold 1 s (2.5–4 s total) | one-shot → ready | 3 | fanfare |
| 26 | ready (flag badge + project color) | after done tier ≥ 1, until seen | 2 | 500 | loop | 1 | – (idle_prompt → at most 1 silent hop per 2 min) |
| 27 | phew (wipes sweat) | tests red → green in the turn | 6 | 110 | prefix to done | +1 tier | – |
| 28 | error_dizzy → sad loop | StopFailure overloaded / server_error / unknown / invalid_request / max_output_tokens | 8 ×2 → 4 | 110 → 200 | one-shot → loop until next prompt | 2 | bonk |
| 29 | tired (clock + Zz) | StopFailure rate_limit; Notification quota_* | 6 | 200 | loop | 1–2 | sleepy |
| 30 | sad_key ("login?") | StopFailure authentication_failed / billing_error / oauth_org_not_allowed / account_on_hold | 6 | 150 | loop | 2 | bonk |
| 31 | sweat → nervous | PostToolUseFailure; ≥ 3 in a row | 3 → 8 | 200 → 125 | overlay → loop | 0 | – |
| 32 | hover_notice | pointer enters | 2 | 120 | one-shot | 0 | – |
| 33 | click_giggle (+heart) | click | 6 | 60, 60, 80, 80, 80, 200 | one-shot | 0 | giggle (optional) |
| 34 | click_spam chain (laugh → dizzy → furious → faint) | 4 / 7 / 10 / 14 clicks within 2-s windows | 6 / 8 / 6 / 8 | 90–125, faint hold 1.5 s | one-shot chain | 0 | – |
| 35 | pickup → dangle | drag | 2 → 4 kick + 3 lean frames | 60, 80 → 130 | one-shot → loop | 0 | boop (optional) |
| 36 | drop_land (+dust) | release | 7 | 60, 80, 60, 70, 70, 120, 200 | one-shot | 0 | thud (optional) |
| 37 | thrown → dizzy | release faster than ~1500 px/s (optional physics) | tumble 4 + dizzy 8 | 60 / 110 | one-shot | 0 | boing |
| 38 | poof_out / poof_in | Hyprland focusedmon change; fullscreen / screensaver | 4 + 4 | 60 | one-shot | 0 | – |
| 39 | dnd_on (sleep mask / headphones) | Omarchy DND on | 6 | 120 | one-shot | 0 | – |

**MVP subset** (≈ 18 tags): 1, 2, 6–12, 16–20, 22, 23, 25 (one variant), 26, 28, 33, 35, 36, 38. Escalation levels L2/L3 reuse jump frames plus runtime offsets.

**Frame budget per character:** about 120 frames in the MVP, but only about 45–60 unique grids, thanks to patches, flips and offsets.

---

## 6. Candidate characters

| | Capivara + laranja | Kitten | Slime / blob | Little robot |
|---|---|---|---|---|
| Silhouette at 32 px | Wide, low barrel, blunt snout, tiny ears. The orange on top breaks the silhouette, so it is recognizable even as a black blob | Ears + tail make it the most readable animal shape (classic oneko sprites are 32×32) | Simple dome; readable but generic; depends on eyes and color | Boxy head + antenna; the face screen can show ?, !, ✓ or a loading bar |
| Easy | Idle and sleep ("chill" meme); chewing; orange wobble / fall-and-return; waiting loop in a hot-tub bucket with steam; bird-on-head easter egg; deadpan comedy (stoic while confetti explodes, then the orange pops up) | Tail and ear secondary motion; grooming; stretch; pounce; curled sleep; typing on the keyboard (meme) | Squash & stretch for everything; goo-stretch while dragged; melts to sleep; splits into minis for subagents; palette-swap moods (green ok / blue sad / red error / gold win) | Status icons on the face; antenna LED as low-motion attention; sparks/smoke on error; rocket jump; "low battery" sleep |
| Hard | Waving / typing / holding a sign (no arms: needs stubby forepaws or props); 360° spin of a long body (4 views); expression with 1–2 px eyes (use eyelids, blush, particles) | Keeping front and side views consistent; can look generic (lots of prior art) | Gestures need pseudopods; personality without limbs | Rigid body resists squash (use springs / antenna); appeal (needs round shapes and big eyes) |
| Canvas | Body ~34×22, cell 48×48 (orange needs headroom) | Body ~24×24 + tail, cell 48×48 (32×32 for a minimal oneko style) | Rest 24×18 / squash 32×12 / stretch 18×30, cell 48×48 | ~22×30 including antenna, cell 48×48 |
| Palette | 3 browns, sand, cream, 2 oranges, leaf green | Orange tabby (2 oranges + cream) or grey tabby | 3-step ramp of one hue + white glint | 2 greys + blue screen + orange/yellow LEDs |
| IP note | Do not resemble Bandai's Kapibara-san | Generic | Generic | An orange, rectangular robot could look like Clawd; avoid |

---

## 7. Hook mapping, verified on the installed Claude Code 2.1.288

Verification: grep of `~/.local/share/mise/installs/claude/2.1.288/claude` plus the docs at code.claude.com.

| Hook | Payload fields (2.1.288) | Pet behaviour |
|---|---|---|
| SessionStart | `source` | hello |
| UserPromptSubmit | `prompt`, `session_title` (the binary has `prompt:…`, not `prompt_text`) | thinking; clears ready / attention; resets counters |
| PreToolUse / PostToolUse | `tool_name`, `tool_input` | working + micro-reactions + counters |
| PostToolUseFailure | — | sweat / nervous; counters |
| PermissionRequest | — | alert_L1 |
| Notification | `message`, `title`, `notification_type` | `permission_prompt`, `elicitation_dialog`, `agent_needs_input` → alert_L1. `idle_prompt` fires after `messageIdleNotifThresholdMs:60000` when the user appears away and can re-fire → no sound. `quota_*` → tired |
| Stop | `stop_hook_active`, `last_assistant_message` | done tier |
| StopFailure | `error_type` (e.g. rate_limit, overloaded, authentication_failed, billing_error, server_error, max_output_tokens, unknown) | error variants |
| SubagentStart / SubagentStop | — | mini clones; counters |
| TaskCompleted | — | check micro; counters |
| PreCompact / PostCompact | — | compact |
| SessionEnd | `reason` (clear, resume, logout, prompt_input_exit, other) | bye (skip clear / resume) |
| PermissionDenied | — | shrug micro |
| PostToolBatch, MessageDisplay | — | Exist in 2.1.288; not needed |

- The in-process "mods / function hooks" event strings (`turn.complete`, `tool.call`) also exist in 2.1.288. desk-pet and octo-pet use them; they avoid spawning a process per event.
- **Privacy:** payloads include prompt text, tool inputs and the last assistant message. Strip them to metadata before they reach the pet; never log content. HTTP hooks forward the full payload.

---

## 8. Sources

**Claude Code and agent pets**
- Claude Code hooks: https://code.claude.com/docs/en/hooks.md
- Claude Code terminal config: https://code.claude.com/docs/en/terminal-config
- Prior-art repos (all on github.com): xtrimsystems/claude-pet, DecoudJuan/Claude-pet, matias-mn/hookling, devnomad-byte/petdex-cc, shigure0110/clawd-pet, Door3172/clawd-desktop-pet, almathkurali/octo-pet, isr431/desk-pet, pablodelucca/pixel-agents, xiangking/agent-pet, MisonL/petdex, PeonPing/peon-ping (+ https://mintlify.wiki/PeonPing/peon-ping/features/sound-events)
- Codex hatch-pet spec: https://github.com/openai/skills/blob/main/skills/.curated/hatch-pet/SKILL.md
- Codex per-state timing request: https://github.com/openai/codex/issues/20863
- Codex Pets statuses: https://penchan.co/en/ai/coding/codex-pets/
- Claude /buddy: https://pilot-shell.com/blog/claude-buddy

**Wayland / Hyprland pets**
- https://github.com/CluelessCatBurger/wl_shimeji (issues 17, 19, 24, 52)
- Hyprland subsurface squish: https://github.com/hyprwm/Hyprland/issues/10515
- https://github.com/kyzmapiratov/Menagerie
- https://github.com/pixelomer/Shijima-Qt (issues 45, 55, 73)
- https://github.com/jesperls/qs-vpets
- https://leon_plickat.srht.site/wayneko.html

**Aseprite**
- JSON exporter source: https://raw.githubusercontent.com/aseprite/aseprite/main/src/app/doc_exporter.cpp
- File format spec: https://github.com/aseprite/aseprite/blob/main/docs/ase-file-specs.md
- CLI: https://www.aseprite.org/docs/cli/

**Fonts**
- https://datagoblin.itch.io/monogram
- Google Fonts metadata: google/fonts `ofl/{tiny5,pixelifysans,silkscreen,pressstart2p}/METADATA.pb`
- GF Latin Core glyph list: googlefonts/glyphsets `GF_Latin_Core.txt`
- https://github.com/rektdeckard/departure-mono

**Sound**
- https://github.com/chr15m/jsfxr
- https://drpetter.se/project_sfxr.html
- https://github.com/KilledByAPixel/ZzFX

**Attention, accessibility, renderers**
- Motion onset: https://pubmed.ncbi.nlm.nih.gov/12930472/
- Calm Technology: https://calmtech.com/
- WCAG 2.3.1: https://www.w3.org/WAI/WCAG22/Understanding/three-flashes-or-below-threshold.html
- Qt AnimatedSprite: https://doc.qt.io/qt-6/qml-qtquick-animatedsprite.html
- GTK scaled texture: https://docs.gtk.org/gtk4/method.Snapshot.append_scaled_texture.html

**Local files inspected (read-only)**
- `/usr/share/omarchy/shell/plugins/notifications/Service.qml`
- `/usr/share/omarchy/bin/omarchy-toggle-notification-silencing`
- `~/.local/state/omarchy/current/theme/colors.toml`
- `/usr/share/sounds/freedesktop/stereo/*.oga`
- `pw-play --help`
- the Claude 2.1.288 binary strings

## Fatos-chave

- **[high]** Several open-source Claude Code desktop pets already exist (2025-2026). The closest to our stack is xtrimsystems/claude-pet: Python + GTK4 + gtk4-layer-shell on Wayland (Hyprland, sway, river). Its hooks write a state file that the pet polls; it uses Shimeji sprites at a default size of 128; MIT license.  
  _Evidência:_ https://github.com/xtrimsystems/claude-pet (README fetched)
- **[high]** OpenAI Codex Pets (May 2026) use a 1536x1872 atlas: 8 columns x 9 rows of 192x208 cells. Rows are idle, running-right, running-left, waving, jumping, failed, waiting, running, review. The QA rules forbid size popping and detached effects. Statuses include 'Ready = finished but not viewed yet'.  
  _Evidência:_ https://github.com/openai/skills/blob/main/skills/.curated/hatch-pet/SKILL.md ; https://penchan.co/en/ai/coding/codex-pets/
- **[high]** Claw'd (shigure0110/clawd-pet) generates 43 animation rows procedurally in Python/Pillow, using rectangles plus letter-legend bitmaps. It ships per-frame durations, for example hop [140,90,90,160,90,90,110,240], yawn [220,160,200,450,300,140,220,500] and dance 8x125 ms. It only announces runs longer than 15 s.  
  _Evidência:_ gh api repos/shigure0110/clawd-pet contents assets/make_clawd_sprites.py and renderer/clawd_meta.json; README
- **[high]** PeonPing users report concrete annoyances: (1) the completion sound repeats about every 60 s because the Notification idle_prompt hook re-fires and was not de-duplicated against Stop (#486); (2) sounds from many agents overlap (#340); (3) subagent noise (#536, #306, #289). They want bounded reminders (#284) and silence when the tab that produced the event is focused (#255). PeonPing's defaults: silent_window_seconds 10 and suppress_subagent_complete.  
  _Evidência:_ gh issue list/view -R PeonPing/peon-ping; https://mintlify.wiki/PeonPing/peon-ping/features/sound-events
- **[high]** Hyprland squishes wl_subsurfaces that overflow their parent instead of cropping them. The upstream issue hyprwm/Hyprland#10515 is still OPEN. Because of this, wl_shimeji does not officially support Hyprland (its #17 shows sprites squished at the edges, and #24 shows a mascot not repainted where it overlaps a bar).  
  _Evidência:_ gh issue view 10515 -R hyprwm/Hyprland (state OPEN); gh issue view 17/24 -R CluelessCatBurger/wl_shimeji
- **[high]** Omarchy's own notification popups use one fixed-size, full-screen transparent PanelWindow per screen on the Overlay layer, with mask: Region { item: popupColumn } for click-through. A source comment says resizing the surface lets the compositor briefly scale a stale buffer, which stretched/squished the cards.  
  _Evidência:_ /usr/share/omarchy/shell/plugins/notifications/Service.qml lines ~955-990 (namespace omarchy-notifications)
- **[high]** qs-vpets (Quickshell, MIT) renders pets in a full-screen PanelWindow on WlrLayer.Overlay with keyboardFocus None and exclusionMode Ignore. Its input mask is a Region around the pet, set to null while dragging. Its timer ticks every 16 ms while moving and every 250 ms otherwise. It polls hyprctl cursorpos every 2 s (10 s when idle).  
  _Evidência:_ gh api repos/jesperls/qs-vpets contents components/PetWindow.qml and services/InputTracker.qml
- **[high]** At scale 1.5, pixel art stays crisp only if logical-px-per-art-px N is even (2, 4, 6, 8). N=4 gives 6 device px per art px and is also an integer at scales 1, 1.25, 1.75 and 2. A 32-px body at N=4 is 128 logical px: 10% of the laptop's 1280-px logical width and 16% of its 800-px height.  
  _Evidência:_ Arithmetic using host facts (eDP-1 1920x1200 @1.5 = 1280x800 logical; HDMI 4K @1.5 = 2560x1440 logical)
- **[high]** The active Omarchy theme is 'hackerman' (dark, background #0B0C16). Against it, a near-black outline (#1B1427) has a WCAG contrast of only 1.09:1, while a cream border (#FFF1D6) has 17.44:1. On white the numbers flip (17.86 vs 1.12). An inner ink outline plus an outer cream 'sticker' border is therefore needed to read on both dark and light themes.  
  _Evidência:_ ~/.local/state/omarchy/current/theme.name = hackerman; colors.toml background=#0B0C16; contrast computed with python3 WCAG formula
- **[high]** Omarchy DND is persisted in ~/.local/state/omarchy/notifications.json as {version:3, dnd:bool}. The file is currently absent, meaning DND off. It can also be queried with 'omarchy-shell notifications isDnd', which returns on/off. The toggle command is omarchy-toggle-notification-silencing.  
  _Evidência:_ /usr/share/omarchy/shell/plugins/notifications/Service.qml (settingsPath, IpcHandler target notifications: dndState/toggleDnd/setDnd/isDnd); /usr/share/omarchy/bin/omarchy-toggle-notification-silencing
- **[high]** The Omarchy screensaver runs as a fullscreen foot window with app-id org.omarchy.screensaver. The default terminal is foot, run as standalone processes, so mapping a session to its window by PID ancestry via hyprctl clients is feasible.  
  _Evidência:_ hyprctl -j activewindow -> class org.omarchy.screensaver, fullscreen 2; pgrep -a foot; ~/.config/xdg-terminals.list = foot.desktop
- **[high]** In Claude Code 2.1.288, the hook inputs are: UserPromptSubmit has 'prompt' (not prompt_text) and session_title; Stop has stop_hook_active and last_assistant_message; Notification has message, title and notification_type. The idle threshold is messageIdleNotifThresholdMs:60000. StopFailure, PostToolUseFailure, PostToolBatch, TaskCompleted, PermissionRequest and MessageDisplay all exist.  
  _Evidência:_ grep -a on ~/.local/share/mise/installs/claude/2.1.288/claude (e.g. hook_event_name:"Stop",stop_hook_active:s,last_assistant_message:ye)
- **[medium]** The /buddy terminal companion (April 2026; 18 ASCII species including capybara at 'Special' rarity) is not present in the installed Claude Code 2.1.288 binary.  
  _Evidência:_ grep -a -c capybara / "buddy" on the 2.1.288 binary = 0; https://pilot-shell.com/blog/claude-buddy
- **[high]** Aseprite json-array export writes: per frame filename, frame, rotated, trimmed, spriteSourceSize, sourceSize, duration; meta app, version, image, format, size, scale; frameTags name, from, to, direction, repeat (as a quoted string) plus userData color/data; slices whose keys have bounds and optional center (9-slice) and pivot.  
  _Evidência:_ https://raw.githubusercontent.com/aseprite/aseprite/main/src/app/doc_exporter.cpp
- **[high]** A .aseprite file can be written directly from the documented spec: header magic 0xA5E0, frame header 0xF1FA carrying the duration, Layer chunk 0x2004, Cel chunk 0x2005 type 2 (zlib), Tags chunk 0x2018 (direction 0-3 plus repeat), Slice chunk 0x2022 (flag 1 = 9-patch, 2 = pivot).  
  _Evidência:_ https://github.com/aseprite/aseprite/blob/main/docs/ase-file-specs.md
- **[high]** LibreSprite 1.2 is in Arch extra (GPL-2.0-only, based on the last GPL2 commit of Aseprite). Aseprite and Pixelorama are not in the official repos.  
  _Evidência:_ pacman -Si libresprite / aseprite / pixelorama
- **[medium]** monogram is CC0 1.0. It lists Portuguese among its charsets and ships monogram.ttf, monogram-bitmap.png and monogram-bitmap.json. Its cell is reportedly 6x12 and the TTF renders crisp at size 16 x k.  
  _Evidência:_ https://datagoblin.itch.io/monogram (+ community comments)
- **[high]** Tiny5, Pixelify Sans, Silkscreen and Press Start 2P are OFL fonts whose subsets include latin and latin-ext. GF Latin Core contains every Portuguese diacritic glyph (atilde, otilde, ccedilla, the circumflex vowels, and capitals). Departure Mono (OFL, v1.500) covers Latin-1 and Latin Extended-A and renders crisp at multiples of 11 px.  
  _Evidência:_ raw google/fonts ofl/*/METADATA.pb; googlefonts/glyphsets GF_Latin_Core.txt; github.com/rektdeckard/departure-mono releases
- **[high]** jsfxr is Unlicense and has headless APIs (toWave/toBuffer); original sfxr is MIT; ZzFX is MIT with 21 parameters. Sounds generated in-repo can therefore be released license-free.  
  _Evidência:_ https://github.com/chr15m/jsfxr ; https://drpetter.se/project_sfxr.html ; https://github.com/KilledByAPixel/ZzFX
- **[high]** Measured freedesktop reference sounds on this host: complete.oga I=-17.1 LUFS, momentary max -13.6, true peak -1.4 dBFS; message-new-instant.oga -30.4 / -26.9 / -15.4; dialog-warning.oga -27.6 / TP -20.3. bell.oga (0.14 s) is gated to -70 LUFS, showing integrated loudness is useless for clips under 400 ms.  
  _Evidência:_ ffmpeg -af ebur128 on /usr/share/sounds/freedesktop/stereo/*.oga (ffmpeg 2:9.0.1-4)
- **[high]** The host's pw-play (PipeWire 1.6.8) supports --media-role (default Music), --media-category, --volume and -P properties. paplay supports --property.  
  _Evidência:_ pw-play --help; paplay --help
- **[high]** Motion onset captures attention, while ongoing motion per se does not (Abrams & Christ 2003). WCAG 2.3.1 limits flashing to 3 times per second.  
  _Evidência:_ https://pubmed.ncbi.nlm.nih.gov/12930472/ ; https://www.w3.org/WAI/WCAG22/Understanding/three-flashes-or-below-threshold.html
- **[high]** Qt AnimatedSprite.interpolate defaults to true, which blends frames, and per-frame durations are not supported. GTK provides gtk_snapshot_append_scaled_texture with GSK_SCALING_FILTER_NEAREST since 4.10.  
  _Evidência:_ https://doc.qt.io/qt-6/qml-qtquick-animatedsprite.html ; https://docs.gtk.org/gtk4/method.Snapshot.append_scaled_texture.html
- **[medium]** Hookling's state timeouts are: waiting 30 min, failed 60 s, working 3 min, review 90 s, sleep after 15 min. octo-pet picks the displayed state as 'loudest wins' (needs input > blocked > ready > running).  
  _Evidência:_ https://github.com/matias-mn/hookling ; https://github.com/almathkurali/octo-pet

## Recomendações

- Use an original character, defaulting to 'capivara com laranja'. Keep the slime as the low-effort fallback. Avoid Clawd, Pokémon PMD sprites and community Codex/Petdex art.
- Design on a 32-px body grid inside 48x48 frame cells. Render at N=4 logical px per art px (6 device px at scale 1.5) and draw the UI at N_ui=2. Snap positions to multiples of 2 logical px (better: 4). Only allow even N.
- Render on a fixed-size, full-output transparent overlay (the same approach as Omarchy's toasts and qs-vpets). Limit the input mask to the sprite and bubble, and make it full-surface while dragging. Never resize the surface for effects, and do not use wl_subsurfaces (Hyprland #10515 is still open).
- Keep text grids + palette.toml + anim.toml in git as the source of truth. A Python/Pillow build stage should emit PNG sheets, Aseprite json-array data, an anim.json sidecar, optional .aseprite files, GIF previews/contact sheets and lint reports.
- Use a sticker double outline: 1-px ink #1B1427 inside and 1-px cream #FFF1D6 outside, generated automatically by dilating the alpha mask. Use only 1-bit alpha. Use the 17-color pet18 palette and verify its contrast against both dark and light themes.
- Put particles (confetti, sparkle, heart, sweat, Z, !, ?, poof, dust, firework, rays, icons, mini-clones) in a shared fx sheet. Put the 9-slice bubble (12x12 source, center 4,4,4,4) and badges in a ui sheet.
- Use monogram (CC0) for bubble text at N_ui=2, building a bitmap atlas from monogram-bitmap.json. NFC-normalize labels. Keep Tiny5 or Departure Mono (OFL) as alternatives.
- Implement attention levels 0-3 using motion-onset bursts. Bound the needs-input escalation: L1 at 0 s, L2 at 30 s, L3 at 90 s, capped at L4 after 5 min, with one sound per step. Replay the burst once when the user returns after 2 or more minutes away.
- Tier 'done' animations by a work score built from duration, tools, files, subagents and tasks. Stay silent for turns under 15 s. Rotate done_big variants with a 10-min cooldown, and keep a 'ready' badge until the session is seen.
- Coalesce finishes within 3 s across sessions. De-duplicate Stop per session for 5 s. Never celebrate SubagentStop. Never play a sound on idle_prompt.
- Use a global sound arbiter: one sound at a time, at least 1.5 s apart, at most 6 per minute, with per-category cooldowns. Mute during DND, quiet hours, and for completion sounds when the session's terminal is focused.
- Honor Omarchy DND by watching ~/.local/state/omarchy/notifications.json, or by calling omarchy-shell notifications isDnd. Hide during fullscreen windows and the org.omarchy.screensaver window. Provide snooze and reduced-motion options. Keep flashing at 2 Hz or below.
- Generate SFX procedurally in the repo with a note sequencer plus a jsfxr port (48 kHz mono 16-bit). Target -30 to -18 LUFS momentary max and a true peak of -3 dBFS or lower. Play via pw-play --media-role=Notification with volume 0.6.
- Copy proven ideas: 'loudest wins' state priority with timeouts, the Ready-until-seen badge, tool micro-reactions, subagent mini-clones, dangle/squash/dizzy drag physics, per-frame durations with fallback chains, and a QA contact sheet.
- Start with an MVP of about 18 tags (idle, blink, yawn, sleep, wake, hello, bye, thinking, working, alert L1-L4 plus waiting, done_nod, done_small, one done_big, ready, error, click_giggle, dangle, drop_land, poof). Add the rest iteratively.

## Riscos

- Pixel art authored by an AI as text grids may look amateurish or inconsistent across frames. → **Use simple geometric construction (as Claw'd does), hand-authored key poses reused through patches and offsets, and lints for bounding-box popping, orphan pixels and colour count. Review GIF previews and contact sheets in PRs, and keep the Aseprite/LibreSprite round-trip path so an artist can take over later.**
- Pixel art becomes blurry or has uneven pixels under fractional scale 1.5, or the toolkit renders at 2x and downsamples. → **Allow only even N, snap positions to even logical px, use nearest filtering (Qt smooth:false and interpolate:false, GTK GSK_SCALING_FILTER_NEAREST), and add an automated grim screenshot test asserting 6x6 device-pixel blocks.**
- Hyprland compositing quirks: squished subsurfaces, stale-buffer scaling when a surface resizes, missing repaints where the pet overlaps bars, and drag release problems. → **Use no subsurfaces and a fixed-size overlay surface; move the sprite inside the surface. Test drag and overlap with omarchy-bar on 0.56.2, and keep the sprite fully inside the output.**
- Annoyance: repeated sounds, pile-ups from many agents, subagent noise, nagging escalation. → **Global sound arbiter with caps and cooldowns, coalescing window, Stop de-duplication, no sound on idle_prompt, subagent suppression, a bounded escalation cap, and DND, quiet hours and snooze.**
- Photosensitivity and motion discomfort from attention-grabbing effects. → **Keep flashing to 3 per second or fewer (target 2 Hz or less) over small areas, offer a reduced-motion mode with no particles and attention capped at 1, and avoid full-screen flashes.**
- Privacy: hook payloads contain prompt text, tool inputs and last_assistant_message. → **Strip content in the hook so only metadata (event, session hash, tool_name, counts, cwd basename) is forwarded, and never log content. Note that HTTP hooks forward the full payload, so discard it immediately in memory.**
- IP or trademark problems from using Clawd, Pokémon or Codex community sprites, or from resembling Kapibara-san. → **Use an original design, CC0/OFL fonts with their license files, and sounds generated in-repo (CC0). Keep a NOTICE file.**
- Stop may not fire when the user interrupts Claude (unverified), leaving the pet stuck in 'working'. → **Add state timeouts (working: 3 min without events; waiting: 30 min) and reset on UserPromptSubmit or SessionEnd.**
- Mapping a session to its terminal window (for focus-aware muting or click-to-focus) is fragile: HTTP hooks carry no PID, and tmux or zellij break the PID chain. → **Use a command hook that sends the PPID chain. On this host foot runs as standalone processes, so the PID chain can be matched against hyprctl clients; fall back to non-focus-aware behaviour when no match is found.**
- CPU and RAM use (Electron and Java pets, plus overlay processes spinning at about 65% CPU in other projects). → **Render only when the frame changes, use adaptive timers (sleep at 2 fps or less), cap particles at about 40, update the input mask only when the bounding box changes, and stay under roughly 1% CPU idle and 60 MB RAM.**
- Accented project names render wrongly (NFD sequences, or glyphs missing from the font). → **NFC normalization, then an NFKD-strip transliteration fallback, then '?'. Verify the monogram atlas covers Latin-1.**

## Perguntas em aberto

- Does 'ficar no topo da tela' mean always-on-top (z-order) or docked at the top edge of the screen? This changes the default home position. Omarchy toasts sit top-right and Claude's input box sits at the bottom of the terminal.
- Which character should be default: capivara com laranja, kitten, slime or robot? Should multiple skins be supported from day one?
- Should sound be on by default, and at what volume? Should there be quiet hours (default 22:00-08:00)?
- How big may done_big be: celebration effects local to the pet, or screen-wide confetti? Should the pet ever move toward the cursor or the screen centre during attention level L3?
- Is it acceptable to show project name, file count and duration in bubbles? Should the bubble language be PT-BR, and should the tone be cute or jokey (e.g. 'Sextou!')?
- Should a click or double-click on the pet focus the Claude terminal? That requires hyprctl dispatch from the container plus PID mapping through command hooks.
- Should the pet react to subagents (mini clones) and to every tool call? Doing so means more hook traffic (PreToolUse/PostToolUse on every call).
- Should completion sounds be muted when the user is already looking at that session's terminal? Should visuals also be reduced in that case?
- Is a later Aseprite or LibreSprite round trip expected (generate .aseprite files), or are PNG + JSON enough for now?
