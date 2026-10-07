# Hyprland 0.56.2

> Pesquisa automática de 2026-10-02 (somente leitura), em inglês. Pode envelhecer: confira antes de confiar.

## Relatório

# Hyprland 0.56.2 specifics for the "claude-pet" overlay (on this Omarchy host)

Sources: read-only queries on this host (`hyprctl -j …`, raw sockets via Python), Omarchy Lua defaults in `/usr/share/omarchy/default/hypr/`, the installed Lua API stubs `/usr/share/hypr/stubs/hl.meta.lua`, the Hyprland source at tag **v0.56.2** (commit efb50993, same as the running binary: `hyprctl version` reports "Tag: v0.56.2"), streamed from GitHub, and the hyprland-wiki markdown sources. Nothing on the host was modified.

---
## 0. TL;DR
* **Use a `zwlr_layer_shell_v1` surface on the OVERLAY layer.** Settings: namespace `claude-pet`, `keyboard_interactivity = NONE`, **`exclusive_zone = -1`**, anchor to one corner (e.g. TOP|LEFT), position set through margins in logical px, input region limited to the sprite, and **an explicit `wl_output` chosen by connector name**.
  * OVERLAY is drawn and hit-tested above fullscreen windows and is never faded.
  * A layer surface belongs to an output, not a workspace, so it shows on every workspace and special workspace of that monitor.
* **"Follow the active monitor" is client-side work.**
  * Track `focusedmonv2>>MON,WSID` on `.socket2.sock`.
  * Debounce, and don't move during a drag.
  * Re-create the surface on the newly focused output.
  * Also re-home on `zwlr_layer_surface_v1.closed` (unplug, or Omarchy clamshell disabling eDP-1).
* **No Hyprland layer rule is required.** Blur and shadow are off globally, layers get no borders or rounding, and Omarchy's layer rules only match its own namespaces. Two rules are optional: `order = 1` keeps the pet under Omarchy's own overlay popups, and `no_anim = true` removes the compositor fades.
* **The xdg-toplevel window alternative is worse here.**
  * A floating, pinned window is drawn above fullscreen windows.
  * It cannot follow the focused monitor without IPC *dispatch* calls (which amount to full compositor control).
  * It is caught by Omarchy's global window rules (default opacity 0.985/0.96).
  * It counts as a window on every workspace.

---
## 1. IPC (verified in source and live)
**Paths:** `$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket.sock` is the request socket; `.socket2.sock` is the event socket.
* On this host: `/run/user/1000/hypr/efb50993780079460b0cbed1363e2166a2de1d9f_1790020208_1687561921/`.
* Permissions: `hypr/` and `<HIS>/` are mode 0700; both sockets are `srwxr-xr-x` owned by youruser. Connecting needs write permission, so **the container must run as UID 1000**.
* The same directory holds `hyprland.lock`, whose content is `1501\nwayland-1` (compositor PID, then the Wayland socket name). A container can use it to discover the instance; never hardcode the HIS or `wayland-1`. `hyprland.log` is also there.
* The HIS changes on every Hyprland start.

### Event socket (`.socket2.sock`)
* Wire format (`EventManager.cpp::formatEvent`): `EVENT>>DATA\n`. DATA is truncated to 1024 bytes, and any newlines inside DATA are replaced with spaces.
* **Back-pressure:** each client has a queue of at most 64 events. On overflow Hyprland logs "overflowed event queue, removing" and **disconnects the client**. The reader must drain the socket continuously (a dedicated thread or async task) and reconnect on EOF.
* Window addresses in events have **no `0x`** (`5bbf4e6128f0`), while the JSON from `hyprctl` uses `0x5bbf4e6128f0`.
* Split DATA with a maximum split count: workspace names, titles and descriptions can contain commas.

Relevant events (source file:line at v0.56.2):

| event | DATA | emitted from / semantics |
|---|---|---|
| `focusedmon` | `MONNAME,WORKSPACENAME` | FocusState.cpp:287 `rawMonitorFocus()`; only when the focused monitor actually changes (early return otherwise) |
| `focusedmonv2` | `MONNAME,WORKSPACEID` | FocusState.cpp:288 (WSID = the monitor's regular active workspace; -1 if none) |
| `workspace` / `workspacev2` | `NAME` / `ID,NAME` | Monitor.cpp:1505 (changeWorkspace) and WorkspacePlacementController.cpp:224; not emitted by mouse movement |
| `activespecial` / `activespecialv2` | `NAME,MON` / `ID,NAME,MON` (empty fields when closed: `,MON` / `,,MON`) | Monitor.cpp:1567, 1616, 1688 |
| `monitoradded` / `monitoraddedv2` | `NAME` / `ID,NAME,DESCRIPTION` | Monitor.cpp:387-388, the last step of onConnect |
| `monitorremoved` / `monitorremovedv2` | `NAME` / `ID,NAME,DESCRIPTION` | Monitor.cpp:397-398, in a scope guard, so emitted **last** in onDisconnect |
| `moveworkspace` / `moveworkspacev2` | `WSNAME,MON` / `WSID,WSNAME,MON` | WorkspacePlacementController.cpp:230-233, 376-377 |
| `fullscreen` | `0` or `1` (any mode ≠ NONE, including maximized); no window or monitor given; may repeat | FullscreenController.cpp:485 |
| `configreloaded` | empty (`configreloaded>>`) | lua/ConfigManager.cpp:854 |
| `openlayer` / `closelayer` | `NAMESPACE` | LayerSurface.cpp:218/227 (map/unmap) |
| `activewindow` / `activewindowv2` | `CLASS,TITLE` / `ADDR` | FocusState.cpp, and also re-emitted on title changes of the focused window (Window.cpp:1457) |
| `windowtitle` / `windowtitlev2` | `ADDR` / `ADDR,TITLE` | Window.cpp:1452 |
| `openwindow` / `closewindow` | `ADDR,WSNAME,CLASS,TITLE` / `ADDR` | Window.cpp:2369/2574 |
| `pin` | `ADDR,0/1` | ConfigActions.cpp:265 |
| `screencast` / `screencastv2` | `STATE,TYPE` / `STATE,TYPE,NAME` | ScreenshareSession.cpp:140-148 (TYPE encoding not verified) |

**Live capture (35 s passive listen on this host).** There were 148 events, all of this form:
```
windowtitle>>5bbf4e6128f0
windowtitlev2>>5bbf4e6128f0,◑ Personagem animado desktop Claude
activewindow>>foot,◑ Personagem animado desktop Claude
activewindowv2>>5bbf4e6128f0
```
* While working, Claude Code's terminal title alternated `◐`/`◑` about once per second, giving 4 events per change.
* The idle title seen earlier was `✳ …`.
* An earlier 8-second listen saw no events at all, so the cause of the spinner timing is unknown.
* This confirms the format and shows a real event stream the reader has to keep up with.
* The title glyphs are an undocumented heuristic, not a replacement for hooks.

### Request socket (`.socket.sock`)
Protocol: `[flags]/command args`. Flags are any of `j` (JSON), `r` (refresh), `a` (all), `c`. Batch form: `[[BATCH]]cmd1;cmd2`; replies are joined with `"\n\n\n"`. All of the following were verified live: `j/cursorpos`, `j/monitors`, `j/monitors all`, `j/layers`, `j/activeworkspace`, and `[[BATCH]]j/cursorpos;j/monitors;j/layers` (parsed with `json.JSONDecoder.raw_decode`). JSON `cursorpos` begins with `\n{`.

**Critical behaviour:**
* Hyprland `accept()`s and then `poll()`s for up to 5000 ms **synchronously in the compositor thread** (HyprCtl.cpp ~l.2264). The wiki says unclosed connections "will cause Hyprland to freeze until the five-second timeout".
* So: connect, write immediately, read until EOF, close. One request per connection. Never poll per frame (the wiki also says hyprctl calls are synchronous and spamming them causes slowdowns).
* There is no read-only mode. Peer credentials are only logged. Access to this socket means `dispatch` and `eval` of Lua, i.e. `hl.exec_cmd` → arbitrary host commands as the user.

```python
import os, socket, glob
RT = os.environ.get("XDG_RUNTIME_DIR", "/run/user/1000")
def hypr_dir():
    sig = os.environ.get("HYPRLAND_INSTANCE_SIGNATURE")
    if sig and os.path.exists(f"{RT}/hypr/{sig}/.socket2.sock"):
        return f"{RT}/hypr/{sig}"
    for d in sorted(glob.glob(f"{RT}/hypr/*/"), key=os.path.getmtime, reverse=True):
        try:
            with socket.socket(socket.AF_UNIX) as s: s.connect(d + ".socket2.sock")
            return d.rstrip("/")            # hyprland.lock line 2 = wayland socket name
        except OSError: pass
def request(cmd):                            # e.g. "j/monitors" or "[[BATCH]]j/cursorpos;j/monitors"
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as s:
        s.connect(hypr_dir() + "/.socket.sock"); s.sendall(cmd.encode())   # send at once!
        out = b""
        while (b := s.recv(65536)): out += b
    return out.decode()
def events():                                # reconnect on StopIteration/EOF
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as s:
        s.connect(hypr_dir() + "/.socket2.sock"); buf = b""
        while (b := s.recv(65536)):
            buf += b
            while b"\n" in buf:
                line, buf = buf.split(b"\n", 1)
                name, _, data = line.decode(errors="replace").partition(">>")
                yield name, data   # focusedmonv2: data.split(",",1); monitorremovedv2: data.split(",",2)
```
A single `j/monitors` query (the entry with `"focused": true`) is enough for initial state.

---
## 2. "Focused monitor" semantics and coordinates
The focused monitor is `Desktop::focusState()->monitor()`; `j/monitors` reports it as `focused: true`. It changes in these cases (all go through `rawMonitorFocus`, and all emit `focusedmon`/`focusedmonv2`):
1. **The mouse moves onto another monitor** (InputManager.cpp:346, `misc:mouse_move_focuses_monitor`; the host value is true, the default). This also happens **while a mouse button is held** (e.g. while dragging the pet).
2. **Keyboard or window focus moves** to a window on another monitor (FocusState.cpp:176; *pinned windows are excluded*). This also covers switchToWindow across monitors (ConfigActions.cpp:88).
3. **Focusing a workspace that lives on another monitor** (ConfigActions.cpp:957), the focus-monitor dispatcher (`tryMoveFocusToMonitor`, :124), or a non-silent move-to-workspace (:349).
4. **Any cursor warp** (PointerController.cpp:21/28). Omarchy has `cursor:warp_on_change_workspace=1` and `no_warps=false`, so keyboard navigation also moves the cursor and the "mouse monitor" stays consistent with the focused one.
5. **Unplug of the focused monitor**: focus moves to the backup monitor (Monitor.cpp:506). Focus is also set on first connect when nothing is focused (:348/:379).
6. Touch, foreign-toplevel activation, and workspace placement (Touch.cpp:38, ForeignToplevelWlr.cpp:63, WorkspacePlacementController.cpp:339).

Host values: `input:follow_mouse=1`, `misc:mouse_move_focuses_monitor=true`, `cursor:warp_on_change_workspace=1`, `binds:hide_special_on_workspace_change=true`, `input:special_fallthrough=false`.

**Special workspaces:**
* A special workspace opens on the focused monitor and does not change monitor focus.
* It emits `activespecial(v2)`. `focusedmon`, by contrast, always reports the regular workspace.
* Render order is: regular windows → special-dim/blur rect → special windows → pinned floats → TOP → OVERLAY. So **the overlay pet stays above, and is not dimmed by, an open special workspace** (Renderer.cpp:1179-1257).

**Coordinates** (HyprCtl.cpp getMonitorData; confirmed on the host):
* `monitors` → `width`/`height` are the physical mode in pixels; `x`/`y` are the position in **global logical** coordinates; `reserved` is `[left, top, right, bottom]` in logical px.
* Logical size = `(width, height) / scale`, with width and height swapped when `transform` is odd.
* This host:
  * eDP-1: 1920x1200, scale 1.5 → logical **1280x800 at (640, 0)**.
  * HDMI-A-1: 3840x2160, scale 1.5 → logical **2560x1440 at (0, -1440)**. **Global y is negative on the 4K monitor.**
* `cursorpos` = `getMouseCoordsInternal().floor()` → global logical integers, e.g. `{"x":1454,"y":697}`.
* `layers` → `x`/`y`/`w`/`h` are global logical geometry. Example: `omarchy-bar` at (640, -26, 1280x26); it is currently hidden off-screen, and `reserved` is `[0,0,0,0]`.
* On the Wayland side, `wl_output.name` (v4) is the same connector name as IPC (`eDP-1`/`HDMI-A-1`) (Output.cpp:30). `xdg_output` (v3) gives logical position and size. `wl_output.scale` is `ceil(1.5) = 2`.

---
## 3. Layer-shell behaviour in 0.56.2
**Protocol and output selection:**
* `zwlr_layer_shell_v1` version **5** (ProtocolManager.cpp:199).
* A NULL `output` means the **monitor focused at `get_layer_surface` time** (LayerSurface.cpp:26; the CLayerSurface is created inside `onGetLayerSurface`).
* With an explicit output, `preferred_scale` and transform are sent immediately (LayerShell.cpp:263-269). With NULL they are only sent at map (`updateSurfaceScaleTransformDetails`), so the first frame can be at the wrong scale. **Prefer explicit outputs.**
* A layer surface cannot span or move across monitors. Its image is clipped at its monitor's edge.

**Above fullscreen:**
* OVERLAY is rendered after windows (including the fullscreen path, special workspaces and pinned windows) and is never faded.
* For input, OVERLAY is hit-tested first ("overlays are above fullscreen", InputManager.cpp ~l.480).
* TOP layers are faded to alpha 0, and become non-interactive, while the active (or special) workspace has a real FSMODE_FULLSCREEN window. A maximized window does not trigger this (WorkspaceAnimationController.cpp:153-198; InputManager.cpp ~l.495-505).
* Quirk: `onMap` sets `m_aboveFullscreen = true`, so a TOP surface mapped *during* a fullscreen stays visible until the next workspace or fullscreen change. **TOP is therefore unreliable; use OVERLAY.**
* Live evidence: with Omarchy's screensaver fullscreen (`org.omarchy.screensaver`, fullscreen 2), `omarchy-bar` (TOP) showed alpha 0; later it showed alpha 1. **The overlay pet will be drawn over the Omarchy screensaver** and over fullscreen games and video.
* Session lock: layers without `above_lock` are not drawn, so the pet is hidden while locked (good).

**Animations (exact names):**
* Tree: `layers` → `layersIn`, `layersOut` control position and size. `fade` → `fadeLayers` → `fadeLayersIn`, `fadeLayersOut` control alpha.
* Styles from LayerSurfaceAnimationController.cpp:
  * `slide [top|bottom|left|right]` (defaults to the nearest edge),
  * `popin N%`,
  * anything else (e.g. `fade`) is alpha-only.
* Omarchy values (`hyprctl animations`; speed unit = 100 ms):

| animation | speed | curve | style |
|---|---|---|---|
| `layers` | 3.81 | easeOutQuint | – |
| `layersIn` | 4 | easeOutQuint | fade |
| `layersOut` | 1.5 | linear | fade |
| `fadeLayersIn` | 1.79 (≈179 ms) | almostLinear | – |
| `fadeLayersOut` | 1.39 (≈139 ms) | almostLinear | – |

* On unmap or destroy, a snapshot is taken (skipped with `no_anim`) and faded out.

**Margin and size changes jump; they don't animate.**
* `onCommit` → `arrangeLayersForMonitor` → `m_realPosition->setValueAndWarp()` unless a position animation is already running.
* With Omarchy's `fade` style, map starts no position animation. Assigning an equal goal is a no-op (hyprutils `operator=`).

**Cost of moving the surface.** Every commit that changes layer-shell state (margins, anchor, size) runs `arrangeLayersForMonitor`, which:
* stable-sorts the layer lists,
* calls `damageMonitor()` (**full-monitor damage**),
* calls `g_layoutManager->invalidateMonitorGeometries()`, which **recalculates every workspace layout on that monitor** (LayoutManager.cpp:357).

So: keep the surface still while idle, animate inside the buffer (only the damaged rect is re-composited), and change margins only for drags or short "run" moves.

**`exclusive_zone`:**
* With `0`, the surface is placed inside the *usable area*, which shifts when the bar reserves or releases space (bar toggle, fullscreen).
* With **`-1`**, it is placed relative to the full monitor area (Renderer.cpp:2601-2605). **Use -1.**

**Rounding at fractional scale:**
* `arrangeLayerArray` → `box.round()` gives integer **logical** geometry.
* At render: box × 1.5 → `CBox::round()`. x and y are rounded; w and h are recomputed to keep the right and bottom edges (hyprutils Box.cpp:51-63).
* An even logical size (e.g. 128 → 192 device px) maps 1:1 at any integer margin.
* If the buffer is off by ≤2 px, Hyprland switches to nearest-neighbour (`MISALIGNEDFSV1`, ElementRenderer.cpp:263-283). Otherwise filtering is linear.
* Hyprland sends `wl_surface.preferred_buffer_scale = ceil(1.5) = 2` **and** fractional scale 1.5 (WLSurface.cpp:211-218).
* **For crisp pixel art the client must render with `wp_fractional_scale_v1` plus `wp_viewporter` at exactly 1.5×.** Use even logical sizes; 1 art pixel = 2 logical px = 3 device px.
* There is **no nearest-neighbour layer rule**; the `nearest_neighbor` rule exists for windows only.

**Input:**
* The input region is honoured. An empty input region means clicks pass through (ViewHitTester.cpp:346-367).
* With `keyboard_interactivity NONE`, hovering or clicking never takes keyboard focus (InputManager.cpp:733), so the terminal keeps focus.
* While a button is held, pointer focus stays on the pressed surface (an implicit grab), so motion arrives with surface-local coordinates even outside the surface. This only happens when *some* keyboard-focused surface exists (InputManager.cpp ~l.404-412). Edge case: on an empty workspace a fast drag can lose the pointer.

**Stacking among overlays:**
* Order is insertion order; the newest is on top.
* Omarchy's menus, notifications, OSD, polkit, emojis, clipboard and reminders are **all OVERLAY**.
* The `order` rule stable-sorts descending (Renderer.cpp:2683), so **higher `order` means drawn earlier, i.e. lower z, and hit-tested later**. Default is 0.
* `order = 1` keeps the pet under Omarchy's popups. A negative value keeps it on top of them.

**Other effects:**
* Any OVERLAY surface blocks the solitary optimization (and with it direct scanout) for fullscreen apps on that monitor (`SC_OVERLAYS`, Monitor.cpp:1878). `render:direct_scanout` is 0 here anyway.
* `no_screen_share` on a layer paints a **black box** over it in captures (ScreenshareFrame.cpp:240-256). Avoid it; hide or unmap during `screencast>>1,…` instead.

**Layer rules in 0.56.2:**
* Effects: `no_anim`, `blur`, `blur_popups`, `ignore_alpha` (float 0-1), `dim_around`, `xray`, `animation` (string), `order` (int), `above_lock` (0-2), `no_screen_share`.
* Match key: `namespace`, as an RE2 **FullMatch**; a `negative:` prefix inverts it.
* Lua `hl.layer_rule{ name?, enabled?, match = {...}, <effects> }` returns a handle with `set_enabled()`/`is_enabled()`.
* Omarchy has no `o.layer` helper. It calls `hl.layer_rule` directly, e.g. `hl.layer_rule({ match = { namespace = "omarchy-bar" }, no_anim = true, animation = "none" })` (apps/omarchy-shell.lua, apps/screenshot-selection.lua).

Optional opt-in snippet (not required). It would live in `~/.config/hypr/claude_pet.lua`, loaded by adding `require("hypr.claude_pet")` at the end of `~/.config/hypr/hyprland.lua`; package.path includes `~/.config/?.lua`, and bootstrap.lua reloads `hypr.*` modules on reload:
```lua
-- Pet: stays above all windows (incl. fullscreen) but below Omarchy's own overlay popups.
hl.layer_rule({
  name    = "claude-pet",
  match   = { namespace = "^claude-pet$" },
  order   = 1,       -- higher order = drawn earlier = beneath default-order(0) overlays
  no_anim = true,    -- optional: pet owns its pop/hide animations; no 179ms fade-in / 139ms snapshot fade-out on re-home
})
```
Is a rule needed? **No.** Without one, the pet fades in about 0.18 s on map and out about 0.14 s on unmap, has no blur or shadow, and may cover newer Omarchy popups.

---
## 4. Alternative: xdg-toplevel window with window rules
Window-rule effects available in 0.56.2:
`float`, `tile`, `fullscreen`, `maximize`, `fullscreen_state`, `move`, `size`, `center`, `pseudo`, `monitor`, `workspace`, `no_initial_focus`, `pin`, `group`, `suppress_event`, `content`, `no_close_for`, `scrolling_width`, `rounding`, `rounding_power`, `persistent_size`, `animation`, `border_color`, `idle_inhibit`, `opacity`, `tag`, `max_size`, `min_size`, `border_size`, `allows_input`, `dim_around`, `decorate`, `focus_on_activate`, `keep_aspect_ratio`, `nearest_neighbor`, `no_anim`, `no_blur`, `no_dim`, `no_focus`, `no_follow_mouse`, `no_max_size`, `no_shadow`, `no_shortcuts_inhibit`, `opaque`, `force_rgbx`, `sync_fullscreen`, `immediate`, `xray`, `render_unfocused`, `no_screen_share`, `no_vrr`, `no_auto_hdr`, `tonemap`, `scroll_mouse`, `scroll_touchpad`, `stay_focused`, `confine_pointer`.

**There is no "keep above" effect.**

Omarchy-style rule:
```lua
o.window("^claude-pet$", { float = true, pin = true, no_initial_focus = true, no_focus = true,
  no_follow_mouse = true, border_size = 0, decorate = false, no_shadow = true, no_blur = true,
  no_dim = true, rounding = 0, no_anim = true, nearest_neighbor = true,
  tag = "-default-opacity", opacity = "1 1",          -- escape Omarchy's global 0.985/0.96 opacity
  size = { 128, 128 }, move = { "(monitor_w-window_w-40)", "(monitor_h*0.04)" } })
```

| requirement | OVERLAY layer | floating, pinned xdg-toplevel |
|---|---|---|
| above fullscreen | yes (drawn and hit-tested first) | yes: the "pinned always above" pass runs after the fullscreen and special passes; `isAllowedOverFullscreen` is true for pinned windows. But it sits below TOP and OVERLAY layers |
| all workspaces | yes (per-output, independent of workspace) | yes, but only for **its own** monitor (Monitor.cpp:1477-1481) |
| follows focused monitor | client re-creates the surface on another `wl_output` (Wayland only) | **not possible client-side**; needs `hyprctl dispatch 'hl.dsp.window.move({ monitor = "HDMI-A-1", window = "class:^claude-pet$" })'` (selector format not run) through the request socket, i.e. full compositor control |
| self-positioning / animation | margins | impossible in xdg-shell; needs IPC `move` |
| drag | client-side margins; cross-monitor drag needs a re-home on release | `xdg_toplevel.move` is native and can cross monitors (whether this works with `no_focus` was not verified) |
| Omarchy interference | none | global `default-opacity` tag rule, `suppress_event=maximize`, shows up in `clients` and workspace window counts on every workspace, focus-follows-mouse unless `no_focus` |
| pixel filtering | must render at 1.5× | `nearest_neighbor` rule available |
| Wayland sandbox (security-context) | **not allowed**: layer-shell is missing from the whitelist (ProtocolManager.cpp:360-398) | xdg-shell is allowed |

---
## 5. Omarchy launching, autostart and conflicts
**Start-up:**
* `default/hypr/autostart.lua` runs, inside `hl.on("hyprland.start", …)`:
  * `systemctl --user import-environment …`
  * `hl.exec_cmd("omarchy-launch-shell")` — a bash supervisor running `QS_DISABLE_FILE_WATCHER=1 QS_NO_RELOAD_POPUP=1 systemd-cat -t omarchy-shell -- quickshell -n -p "$OMARCHY_PATH/shell"`, relaunched on non-zero exit up to 5 times per minute.
  * `hl.exec_cmd(o.launch("omarchy-hyprland-monitor-watch"))`, udiskie, and `sleep 2 && omarchy-hook post-boot`.
* Helpers (`default/hypr/helpers.lua`):
  * `o.launch(cmd)` = `"uwsm-app -- " .. cmd`; this produces `app-Hyprland-<cmd>-<id>.scope` units.
  * `o.exec_on_start(cmd)` = `hl.on("hyprland.start", function() hl.exec_cmd(cmd) end)`.
  * `o.launch_on_start(cmd)` = `o.exec_on_start(o.launch(cmd))`. It runs once per Hyprland start, not on reload.
  * `o.window(match, rules)`: a string match becomes `match.class`.

**User autostart:** `~/.config/hypr/autostart.lua` (currently only the placeholder `-- o.launch_on_start("my-service")`). The user config loads, in order: omarchy defaults → `hypr.monitors`, `hypr.input`, `hypr.bindings`, `hypr.looknfeel`, `hypr.autostart` → `default.hypr.toggles`.

**systemd user units:** Omarchy also uses units such as `/usr/lib/systemd/user/omarchy-fcitx5.service`:
```ini
[Unit]
After=graphical-session.target
PartOf=graphical-session.target
ConditionEnvironment=WAYLAND_DISPLAY
[Service]
Type=simple
Restart=always
RestartSec=2
[Install]
WantedBy=graphical-session.target
```
A pet launcher unit, e.g. `ExecStart=docker compose up` / `ExecStop=docker compose down`, would follow session restarts (new HIS) automatically. On this host uwsm 0.26.7 is installed, and `wayland-wm@hyprland.desktop.service` and `graphical-session.target` are active.

**Conflicts and interactions:**
1. **Global window rules** (default-opacity tag, suppress maximize) affect windows only, not layers.
2. **Layer rules** match only Omarchy namespaces and `selection`.
3. **Blur** (`decoration:blur:enabled=false`) and **shadow** are disabled globally.
4. **Clamshell:** `omarchy-hyprland-monitor-watch` (socat on socket2) runs `omarchy-hyprland-monitor-clamshell`. Lid closed plus external monitor → it writes `~/.local/state/omarchy/toggles/hypr/internal-monitor-clamshell.lua` = `hl.monitor({ output = "eDP-1", disabled = true })` and runs `hyprctl reload`. That leads to `MonitorRuleManager` → `onDisconnect()` → **`closed` for the pet on eDP-1** plus `monitorremoved>>eDP-1`. Opening the lid reverses it (`monitoradded`).
5. **Modeless monitor:** `recover_modeless` can loop `hyprctl reload` (backing off to 60 s) while a monitor reports no mode, producing repeated `configreloaded`. Never recreate the surface on `configreloaded`; only re-validate geometry.
6. **Screensaver:** `omarchy-launch-screensaver` **focuses each monitor in turn** (`hyprctl dispatch "hl.dsp.focus({ monitor = \"$m\" })"`), opens a fullscreen `org.omarchy.screensaver` on each, then restores focus. That is a burst of `focusedmon` events, so **debounce**. The pet is drawn over the screensaver; consider sleeping on `openwindow>>…,org.omarchy.screensaver,…` until the matching `closewindow`.
7. **Bar toggle** changes the usable area. With `exclusive_zone = -1` the pet is unaffected.
8. **hyprsunset** tints the pet at night (cosmetic).
9. `~/.config/hypr/monitors.lua` sets `hl.env("GDK_SCALE", "2")` for processes Hyprland spawns. Don't pass it into the container.
10. Omarchy overlay popups (including polkit) can be covered by a newer pet surface; this is what `order` addresses.

---
## 6. Hotplug and re-homing
**Unplug or disable** (Monitor.cpp:392-520), in order:
1. `preRemoved`.
2. Workspace recorded.
3. **`sendClosed()` on every layer surface of that monitor (all 4 levels)**, then the lists are cleared.
4. Cursor warped to the backup monitor's centre.
5. Workspaces moved: `moveworkspace(v2)`.
6. `rawMonitorFocus(backup)` → `focusedmon(v2)`, if the removed monitor was focused.
7. At scope exit: `monitorremoved` + `monitorremovedv2`. The `wl_output` global is removed on the same bus event (ProtocolManager.cpp:151-156).

On the Wayland socket the client therefore sees `closed` before `global_remove`.

* If the client ignores `closed`, the surface becomes a zombie: it is not in any monitor list and never rendered.
* If every monitor goes away, a headless `FALLBACK` monitor is created. Never home there.
* A monitor that is connected but 0x0 is skipped by `arrangeLayersForMonitor`, so the surface would never be configured. Check `width/height > 0 && !disabled`.

**Plug:** `onConnect` → remembered workspaces moved back → `monitoradded(v2)`. A new `wl_output` global is created on the same bus event. Arrival order across the IPC socket and the Wayland socket is not guaranteed. Focus does not move to the new monitor unless nothing was focused.

**gtk4-layer-shell 1.3.0** (installed):
* Since 1.3, `respect_close` defaults to **FALSE**, so `closed` is silently swallowed (layer-surface.c:101-107).
* To handle it: `gtk_layer_set_respect_close(win, TRUE)` and connect `close-request` returning TRUE, then call `gtk_layer_set_monitor(win, new_monitor)`. `set_monitor` remaps the surface on the new output (layer-surface.c:274-281).
* `gtk_layer_set_monitor(win, NULL)` lets the compositor choose (the focused monitor).

**Recommended re-home algorithm:**
1. **Initial state:** `j/monitors` → the entry with `focused: true`. Alternatively, create the surface with a NULL output and read `wl_surface.enter`.
2. **Focus changes:** on `focusedmonv2`, debounce about 250-500 ms and ignore it while dragging. If the target has a `wl_output` with that name, is not a mirror, not disabled, not `FALLBACK`, and has size > 0, recreate the surface there. Keep the corner-relative position, clamped to the new logical size.
3. **On `closed` or `monitorremoved` for the current output:** recreate on the focused monitor, or on any remaining output. If none, wait for `monitoradded` or a new `wl_output` global.
4. **On `configreloaded` / `monitoradded`:** re-read geometry (via `xdg_output`, or `j/monitors`) and clamp.
5. **On drag end:** if the pointer ended on another monitor, re-home there. Global position = monitor logical position + margins + surface-local pointer.

## 7. Hyprland Lua API notes
* Lua events (`HL.EventName`): `monitor.focused`, `monitor.added`, `monitor.removed`, `monitor.layout_changed`, `layer.opened`, `layer.closed`, `window.fullscreen`, `workspace.active`, `workspace.special_active`, `screenshare.state`, `config.reloaded`, `hyprland.start`, …
* Queries: `hl.get_active_monitor()`, `hl.get_layers({namespace=…})`, whose results include `above_fullscreen`. They could push state to the pet, but socket2 is simpler and decoupled.

## Fatos-chave

- **[high]** The running Hyprland is exactly tag v0.56.2 (commit efb50993); config is Lua with installed API stubs at /usr/share/hypr/stubs/hl.meta.lua  
  _Evidência:_ `hyprctl version`: 'Tag: v0.56.2', commit efb50993...; ~/.config/hypr/.luarc.json points to /usr/share/hypr/stubs
- **[high]** Sockets are $XDG_RUNTIME_DIR/hypr/$HIS/.socket.sock (requests) and .socket2.sock (events). Dirs are 0700 and sockets srwxr-xr-x owned by UID 1000, so a container must run as UID 1000. hyprland.lock contains 'PID\nwayland-N'  
  _Evidência:_ ls -la /run/user/1000/hypr/*/; cat -A hyprland.lock → '1501$ wayland-1$'
- **[high]** Event format is EVENT>>DATA\n; data is truncated to 1024 bytes and newlines in it become spaces. Each client has a 64-event queue, and on overflow Hyprland disconnects the client  
  _Evidência:_ src/managers/EventManager.cpp formatEvent/postEvent (MAX_QUEUED_EVENTS = 64) at v0.56.2
- **[high]** focusedmon>>MON,WSNAME and focusedmonv2>>MON,WSID are emitted only when the focused monitor actually changes  
  _Evidência:_ src/desktop/state/FocusState.cpp:273-292 rawMonitorFocus (early return if same)
- **[high]** The focused monitor changes when the mouse enters another monitor (including while a button is held), when focus moves to a non-pinned window on another monitor, when a workspace on another monitor is focused, on any cursor warp, and on unplug of the focused monitor (→ backup monitor)  
  _Evidência:_ InputManager.cpp:346; FocusState.cpp:176; ConfigActions.cpp:88,124,349,957; PointerController.cpp:21,28; Monitor.cpp:506
- **[high]** monitoradded(v2) and monitorremoved(v2) payloads are NAME and ID,NAME,DESCRIPTION. monitorremoved is emitted last in onDisconnect, after layer surfaces receive closed  
  _Evidência:_ src/output/Monitor.cpp:387-398 (CScopeGuard), 463-469 (sendClosed loop)
- **[high]** On monitor unplug, or when a monitor is disabled by a rule (Omarchy clamshell), every layer surface on it gets zwlr_layer_surface_v1.closed  
  _Evidência:_ Monitor.cpp onDisconnect sendClosed for 4 levels; MonitorRuleManager.cpp:187 m_disabled ? onDisconnect()
- **[high]** The OVERLAY layer is rendered and hit-tested above fullscreen windows and is never faded. TOP layers get alpha 0 and are not interactive under FSMODE_FULLSCREEN (but not under maximized)  
  _Evidência:_ Renderer.cpp:1179-1257; WorkspaceAnimationController.cpp:153-198; InputManager.cpp ~480-505; live: omarchy-bar alpha 0 while screensaver fullscreen=2
- **[high]** Layer animations are layersIn/layersOut (position/size; styles slide [edge], popin N%, otherwise alpha-only) and fadeLayersIn/fadeLayersOut (alpha). Omarchy sets fade style with fadeLayersIn 1.79 (~179 ms) and fadeLayersOut 1.39 (~139 ms)  
  _Evidência:_ LayerSurfaceAnimationController.cpp; `hyprctl animations`; /usr/share/omarchy/default/hypr/looknfeel.lua
- **[high]** Margin and size changes are applied instantly (setValueAndWarp) unless a position animation is already running. Each such commit triggers arrangeLayersForMonitor → full monitor damage and a recalculation of all workspace layouts on that monitor  
  _Evidência:_ LayerSurface.cpp onCommit ~l.357-368; Renderer.cpp:2673-2702; LayoutManager.cpp:357-364
- **[high]** Layer geometry is rounded to integer logical px. At render it is scaled ×1.5 and CBox::round keeps the right/bottom edges. Hyprland uses nearest-neighbour only when the fractional-scale buffer is off by ≤2 px. preferred_buffer_scale is ceil(1.5)=2 and the fractional scale is 1.5  
  _Evidência:_ Renderer.cpp:2660 box.round(); ElementRenderer.cpp:251-283; hyprutils Box.cpp:51-63; WLSurface.cpp:211-218
- **[high]** Layer rule effects in 0.56.2 are no_anim, blur, blur_popups, ignore_alpha, dim_around, xray, animation, order, above_lock, no_screen_share. Matching is on namespace with RE2 FullMatch  
  _Evidência:_ LayerRuleEffectContainer.cpp; LuaBindingsConfigRules.cpp:1277-1388; RegexMatchEngine.cpp; wiki layer-rules.md
- **[high]** The `order` rule sorts each layer list in descending order, so a higher order is drawn earlier (lower z) and hit-tested later. The default is 0, and all of Omarchy's popups (menus, notifications, OSD, polkit) are OVERLAY layers  
  _Evidência:_ Renderer.cpp:2681-2684 stable_sort a.order > b.order; LayerRuleApplicator.hpp order default 0; grep of /usr/share/omarchy/shell QML (15× WlrLayer.Overlay)
- **[high]** exclusive_zone -1 positions a layer relative to the full monitor area; with 0 it is positioned inside the usable area, which shifts with the bar  
  _Evidência:_ Renderer.cpp arrangeLayerArray (PSTATE->exclusive == -1 → full_area)
- **[high]** A NULL output in get_layer_surface puts the surface on the monitor focused at creation time. The scale is only sent at map in that case, whereas an explicit output sends it immediately  
  _Evidência:_ LayerSurface.cpp:26 create(); LayerShell.cpp:221-270; layer-shell protocol version 5 (ProtocolManager.cpp:199)
- **[high]** Layer surfaces with keyboard_interactivity NONE never take keyboard focus on click or hover. The input region is honoured, and an empty region means click-through  
  _Evidência:_ InputManager.cpp:733; ViewHitTester.cpp:346-367
- **[high]** There is no 'keep above' window rule. A floating pinned window is drawn above fullscreen windows but stays on its own monitor, and only follows workspace changes there  
  _Evidência:_ WindowRuleEffectContainer.cpp list; Renderer.cpp 'pinned always above' and shouldRenderWindow; Monitor.cpp:1477-1481
- **[high]** The request socket is handled synchronously: after accept, Hyprland polls up to 5000 ms in the compositor thread, so an unclosed or slow client freezes Hyprland. There is no read-only mode (peer credentials are only logged)  
  _Evidência:_ HyprCtl.cpp hyprCtlFDTick ~l.2240-2275; wiki ipc/_index.md note
- **[high]** Raw requests 'j/cursorpos', 'j/monitors', 'j/monitors all', 'j/layers', 'j/activeworkspace' and '[[BATCH]]j/cursorpos;j/monitors;j/layers' work over .socket.sock; batch replies are separated by \n\n\n  
  _Evidência:_ Python socket test on host; HyprCtl.cpp getReply flags (j,r,a,c) and dispatchBatch DELIMITER
- **[high]** cursorpos returns global logical coordinates floored to ints. In monitors JSON, width/height are physical mode px and x/y are logical. On this host eDP-1 is logical 1280x800 at (640,0) and HDMI-A-1 is 2560x1440 at (0,-1440), so y is negative on the 4K monitor  
  _Evidência:_ HyprCtl.cpp cursorPosRequest/getMonitorData; `hyprctl -j monitors`; ~/.config/hypr/monitors.lua
- **[high]** wl_output.name (v4) and xdg_output.name equal the Hyprland monitor name (eDP-1, HDMI-A-1); xdg_output provides logical position and size  
  _Evidência:_ src/protocols/core/Output.cpp:30; XDGOutput.cpp:79-125
- **[high]** Since 1.3, gtk4-layer-shell ignores zwlr_layer_surface_v1.closed by default (respect_close=FALSE), and gtk_layer_set_monitor remaps the surface on a new output  
  _Evidência:_ gtk4-layer-shell v1.3.0 include/gtk4-layer-shell.h docs; src/layer-surface.c:101-107, 274-281
- **[high]** Omarchy launches its shell with omarchy-launch-shell (supervised quickshell via systemd-cat) and other daemons with uwsm-app. User autostart uses o.launch_on_start in ~/.config/hypr/autostart.lua, which runs once per Hyprland start  
  _Evidência:_ /usr/share/omarchy/default/hypr/autostart.lua, helpers.lua, bin/omarchy-launch-shell, ~/.config/hypr/autostart.lua
- **[high]** Omarchy's screensaver focuses every monitor in turn and opens a fullscreen org.omarchy.screensaver window on each, which produces a burst of focusedmon events  
  _Evidência:_ /usr/share/omarchy/bin/omarchy-launch-screensaver (hl.dsp.focus({ monitor = ... }) loop)
- **[high]** Omarchy's clamshell flow disables eDP-1 through a generated toggle file plus `hyprctl reload`, and the modeless-recovery loop can issue repeated reloads (configreloaded)  
  _Evidência:_ /usr/share/omarchy/bin/omarchy-hyprland-monitor-clamshell, omarchy-hyprland-monitor-watch
- **[high]** Any OVERLAY layer surface on a monitor blocks the solitary optimization (and with it direct scanout) for fullscreen apps on that monitor  
  _Evidência:_ Monitor.cpp:1878-1882 SC_OVERLAYS
- **[high]** no_screen_share on a layer draws a black rectangle in screen captures instead of hiding it  
  _Evidência:_ src/managers/screenshare/ScreenshareFrame.cpp:240-256
- **[high]** zwlr_layer_shell_v1 is not in Hyprland's security-context whitelist, so a client using a wp_security_context sandbox socket cannot create overlay layers  
  _Evidência:_ ProtocolManager.cpp:360-398 ALLOWED_WHITELIST
- **[medium]** Live: while Claude Code worked, its foot terminal title alternated ◐/◑ about once per second. Each change produced 4 socket2 events (windowtitle, windowtitlev2, activewindow, activewindowv2): 148 events in 35 s. An earlier 8 s listen saw none  
  _Evidência:_ Background passive listen on .socket2.sock (task output)
- **[high]** Host options: follow_mouse=1, mouse_move_focuses_monitor=true, warp_on_change_workspace=1, blur and shadow disabled, enforce_permissions=false, animations enabled  
  _Evidência:_ `hyprctl -j getoption ...` queries

## Recomendações

- Render the pet as a zwlr_layer_shell OVERLAY surface with namespace 'claude-pet', keyboard_interactivity NONE, exclusive_zone -1, anchored to one corner, positioned with margins in logical px, and an input region limited to the sprite (click-through elsewhere).
- Bind the surface to an explicit wl_output found by wl_output.name == Hyprland monitor name (so the scale is known before the first frame), rather than a NULL output.
- Follow the focused monitor using only .socket2.sock focusedmonv2 events, debounced by about 250-500 ms (Omarchy's screensaver emits focus bursts) and ignored during a drag. Get the initial state with a single j/monitors request (focused:true), or from wl_surface.enter after creating with a NULL output.
- Handle re-homing on zwlr_layer_surface_v1.closed and on monitorremoved for the current output. With gtk4-layer-shell 1.3: gtk_layer_set_respect_close(TRUE), a close-request handler that returns TRUE, then gtk_layer_set_monitor(new). Never home onto FALLBACK, mirrors, disabled outputs, or 0x0 outputs; clamp the position to the new logical size.
- Keep the surface stationary and animate inside the buffer; avoid continuous margin animation, since each margin commit causes full-monitor damage plus a layout recalculation. For big celebrations, temporarily use a larger or full-output transparent surface with the input region restricted to the sprite.
- For crisp pixel art at scale 1.5, require wp_fractional_scale_v1 + wp_viewporter rendering at exactly 1.5×. Use even logical sizes (1 art pixel = 2 logical px = 3 device px). Do not pass GDK_SCALE=2 (set in ~/.config/hypr/monitors.lua) into the container.
- Treat Hyprland config as opt-in. No layer rule is needed; if wanted, add `hl.layer_rule({ name='claude-pet', match={namespace='^claude-pet$'}, order=1, no_anim=true })` via a ~/.config/hypr/claude_pet.lua required from hyprland.lua. Use order=1 to stay below Omarchy overlay popups such as polkit and menus, or a negative order to stay above them.
- IPC hygiene: drain socket2 continuously in its own thread and reconnect on EOF (64-event overflow disconnects). For socket1, use one-shot connect/write/read-to-EOF/close and never poll per frame. Parse addresses without 0x and split DATA with a maxsplit. Discover the HIS and Wayland socket through hyprland.lock / glob instead of hardcoding them.
- Prefer not to mount .socket.sock into the container (it gives arbitrary host exec through dispatch/eval). Socket2 plus the Wayland socket are enough. If the request socket is needed, document the risk.
- Lifecycle: start the container from a systemd user unit with PartOf=graphical-session.target and ConditionEnvironment=WAYLAND_DISPLAY (the Omarchy pattern, e.g. omarchy-fcitx5.service), or from o.launch_on_start in ~/.config/hypr/autostart.lua. This way a Hyprland restart (new HIS, possibly a new wayland-N) recreates the container.
- Optional behaviours from IPC: go to sleep while org.omarchy.screensaver windows exist (openwindow/closewindow), stay quiet during screencast>>1, and react to fullscreen>>1 by using less intrusive animations.
- Use Claude Code hooks as the authoritative 'finished' signal. The terminal-title spinner (◐/◑ while working, ✳ idle) seen on socket2 is only an undocumented heuristic or fallback.
- For a container healthcheck, query [[BATCH]]j/layers;j/monitors and verify that namespace 'claude-pet' is on level 3 of the focused monitor (or watch openlayer>>claude-pet).

## Riscos

- The event reader stalls (GC, docker pause, a blocked UI thread) and more than 64 events queue up. Claude Code's title spinner alone produced about 4 events/s. Hyprland then drops the socket2 client and the pet stops following focus. → **Read socket2 in a dedicated thread/async task that only parses and enqueues; auto-reconnect with backoff on EOF and re-query state (j/monitors) after reconnecting.**
- Holding a .socket.sock connection open, or writing slowly, freezes the whole compositor for up to 5 s. → **Connect only when the request is ready; send it in one write, read to EOF, close; no per-frame polling (use events and Wayland pointer events instead).**
- After an HDMI unplug or Omarchy clamshell (eDP-1 disabled), gtk4-layer-shell 1.3 silently swallows `closed`, so the pet disappears and leaves a zombie surface. → **Enable respect_close, intercept close-request, and call gtk_layer_set_monitor on a valid output; also react to monitorremoved for the current output and to GdkMonitor invalidation.**
- Focus bursts (Omarchy screensaver focusing each monitor, modeless-recovery reload loops, rapid mouse crossings) make the pet bounce between monitors or recreate its surface repeatedly. → **Debounce focusedmon by about 250-500 ms, apply only the last value, ignore configreloaded for re-homing, and add hysteresis.**
- A single layer surface cannot cross monitors. A drag toward the HDMI monitor clips the sprite at the edge, and the implicit pointer grab only works if some keyboard-focused surface exists (on an empty workspace a fast drag can lose the pointer). → **Re-home on button release to the monitor under the pointer (computed from the surface origin plus local coordinates, or j/cursorpos once). Optionally use a full-output transparent surface during drags.**
- Continuous movement through margins causes full-monitor damage and a recalculation of all workspace layouts on every commit, wasting GPU/CPU and battery (4K monitor at 60 fps). → **Animate inside the buffer; move the surface only during drags or short scripted moves; keep celebrations time-limited.**
- Blurry pixel art: if the client renders at integer buffer scale 2 (Hyprland's preferred_buffer_scale = ceil(1.5)) or at 1×, Hyprland rescales it with linear filtering. Layers have no nearest-neighbour rule. → **Use a toolkit or path that supports wp_fractional_scale_v1 + viewporter; even logical sizes; verify visually on both monitors.**
- The pet, being the newest overlay surface, covers Omarchy popups (polkit password prompt, menus, notifications), and is drawn over the screensaver and fullscreen games or video (it also blocks the solitary/direct-scanout path). → **Optional layer rule order=1; sleep/hide while the screensaver or a fullscreen window is active if desired; a small sprite with a tight input region.**
- Security: .socket.sock allows dispatch/eval and therefore arbitrary host commands. The Wayland socket allows virtual-keyboard and screencopy. Docker is not a security boundary here, and security-context sandbox sockets cannot be used because layer-shell is not whitelisted. → **Mount only what is needed (socket2 plus the Wayland socket), run as UID 1000 with no extra capabilities, read-only rootfs, and document the trust assumption.**
- A Hyprland restart changes the HIS (and possibly wayland-N). File bind-mounts of sockets go stale, and a hardcoded HYPRLAND_INSTANCE_SIGNATURE in compose breaks after reboot. A boot-time `restart: unless-stopped` start happens before the session exists. → **Mount directories (e.g. /run/user/1000/hypr, or the runtime dir with rslave propagation) and discover sockets through hyprland.lock; tie container lifecycle to graphical-session.target (PartOf) or Hyprland autostart; make the pet wait/retry when sockets are missing.**
- Using a NULL output gives one wrong-scale first frame, because scale is sent only at map. A newly plugged monitor's wl_output and the IPC monitoradded can arrive in either order. → **Use explicit wl_outputs; when focusedmon names an output not yet known, wait for its wl_output global (match by name).**

## Perguntas em aberto

- Should the pet sit below Omarchy's own overlay popups (order=1: menus, notifications, polkit, OSD) or always above them (negative order)? This needs a user preference.
- Should the pet hide or sleep during the Omarchy screensaver, screen sharing (screencast events) and fullscreen apps, or keep celebrating on top?
- On a focus change, should the pet jump immediately (with debounce) and keep a per-monitor or corner-relative position, or play a 'walk-in' animation on the new monitor?
- Should dragging the pet onto the other monitor be supported (re-home on release), given that the sprite clips at the monitor edge during the drag?
- Will the container get .socket.sock at all (convenient queries, but host-exec power), or only .socket2.sock plus the Wayland socket?
- Unverified: the TYPE encoding in screencast/screencastv2 (numeric enum vs string), and whether grim screenshots also trigger screencast events.
- Unverified (toolkit side): whether GTK4 4.22 on Wayland honours GDK_SCALE and uses fractional scaling by default for layer surfaces; needs testing by the toolkit research.
- Unverified (only if the xdg-toplevel alternative is chosen): whether xdg_toplevel.move works for a window with no_focus=true, and the exact window selector syntax for hl.dsp.window.move in 0.56.2.
- Unverified: why the Claude Code title spinner events appeared in one listen window and not in an earlier 8 s window. The title-based heuristic should not be relied on.
