# Motor gráfico (Quickshell, GTK4, Rust)

> Pesquisa automática de 2026-10-02 (somente leitura), em inglês. Pode envelhecer: confira antes de confiar.

## Relatório

# Overlay renderer for a Dockerized pixel-art pet on Hyprland 0.56.2 (scale 1.5): research report

Everything here was done read-only. Sources were the installed `.qmltypes`, GIR and gtk-doc files, Omarchy's QML under `/usr/share/omarchy`, `pacman`/`pactree` queries, `strings` on the installed binaries, and upstream source at the exact installed tags via `gh api`: quickshell v0.3.1, Hyprland v0.56.2, qtdeclarative/qtbase 6.11, gtk4-layer-shell v1.3.0, GTK 4.22.4. Nothing was built or run.

## TL;DR: ranked recommendation

1. **Quickshell 0.3.1 (QML), running in an `archlinux:base` container.** Confidence: medium-high.
   - All the APIs needed are present and verified: layer-shell props, the `mask: Region`, Hyprland focus, `SocketServer`, `IpcHandler`.
   - Omarchy's own shell already uses every pattern needed, and the files are on disk to copy from.
   - QML hot reload makes iterating with Claude Code very fast.
   - The Qt-specific costs can be avoided with a known design:
     - keep the surface small in steady state;
     - grow it to the full monitor only while dragging or celebrating;
     - drive idle frames from a `Timer`, never a continuously running `AnimatedSprite`.
2. **GTK 4 + PyGObject + gtk4-layer-shell.** Confidence: medium.
   - There is a working precedent built on Omarchy/Hyprland: `xtrimsystems/claude-pet` (MIT).
   - GTK repaints and damages only the changed rectangle, so a permanent full-monitor overlay is cheap.
   - Downsides: no hot reload, the `LD_PRELOAD`/`CDLL` requirement, hand-written animation and Hyprland IPC, and `set_monitor` does a full unrealize/realize.
3. **Rust + smithay-client-toolkit 0.21.1.** Confidence: medium as a v1, high as a later engine.
   - Smallest footprint by far (about 5–15 MB RSS, image of about 5–10 MB on `scratch`) and total control over commits and damage.
   - Pixel-exact by construction, and golden-frame tests run headless.
   - Slowest to iterate, and the most Wayland plumbing to write (fractional scale has to be bound by hand).

**Component layout (Quickshell, recommended):**
- **Host side:**
  - Claude Code hooks call `claude-pet-notify`, a non-blocking script that always exits 0.
  - The script writes one JSON line to a UNIX socket at `$XDG_RUNTIME_DIR/claude-pet/events.sock`, using `socat` or `python3`. Alternative: `curl` to an HTTP bridge.
- **Container `claude-pet`:** a single `quickshell` process.
  - `SocketServer` receives events.
  - `PetModel.js` is the pure state machine (event → animation queue).
  - One `PanelWindow` (layer Overlay or Top) follows `Hyprland.focusedMonitor`.
  - `IpcHandler` provides `ping`/`play`/debug, used by the healthcheck and by `docker exec`.
  - `FileView` + `JsonAdapter` persist per-monitor positions to `/data`.
- **Lifecycle:** a `systemd --user` unit with `PartOf=graphical-session.target` runs `docker compose up`/`down`, so the container always gets the current session's `HYPRLAND_INSTANCE_SIGNATURE`/`WAYLAND_DISPLAY`.

---

## A. Quickshell 0.3.1: verified on this host

**Package**
- `pacman -Si quickshell` → `Repository: extra`, `0.3.1-1`.
- Hard dependencies include `mesa`, `libglvnd`, `qt6-base`, `qt6-declarative`, `qt6-svg`, `qt6-wayland`, `libpipewire`, `polkit`, `pam`, `cpptrace` and `jemalloc`.
- The QML plugins are compiled into `/usr/bin/quickshell` (`qmldir`: `optional plugin`, `prefer :/qt/qml/...`).
- `/usr/bin/qs` is a symlink to it.
- Repology: 0.3.1 is also in Debian forky/sid, Void and Chimera. Debian trixie-backports has 0.3.0. It is not in Alpine.

### Exact types and names, from `/usr/lib/qt6/qml/Quickshell/**/*.qmltypes`

**`WlrLayershell`** (`Quickshell.Wayland._WlrLayerShell`), used through attached properties on `PanelWindow`:
- `layer`: `WlrLayer.{Background, Bottom, Top, Overlay}`.
- `namespace`.
- `keyboardFocus`: `WlrKeyboardFocus.{None, Exclusive, OnDemand}`.
- `anchors`, `exclusiveZone`, `margins` (int left/right/top/bottom), `aboveWindows`, `focusable`.
- `exclusionMode`: `ExclusionMode.{Normal, Ignore, Auto}`.

**`QsWindow` / `ProxyWindowBase`:**
- `screen` (read/write `ShellScreen`), `mask` (a `Region`), `color`.
  - Per the docs, `color` **defaults to white**, so you must set `color: "transparent"`.
- `devicePixelRatio` (read-only), `surfaceFormat.opaque`, `updatesEnabled`, `visible`, `implicitWidth`/`implicitHeight`.
- Signals: `closed`, `windowConnected`.

**`Region`:**
- `item`, `x`/`y`/`width`/`height`.
- `shape`: `RegionShape.{Rect, Ellipse}`.
- `intersection`: `Intersection.{Combine, Subtract, Intersect, Xor}`.
- `regions` (nested children).

**`Hyprland` singleton:**
- `focusedMonitor` (`HyprlandMonitor`: `name`, `x`, `y`, `width`, `height`, `scale`, `focused`, `activeWorkspace`).
- `focusedWorkspace` (`hasFullscreen`, `monitor`, …), `monitors`, `workspaces`, `toplevels`.
- `requestSocketPath`, `eventSocketPath`, `usingLua`.
- `rawEvent(event)`, `dispatch()`, `monitorFor(screen)`, `refreshMonitors()`.

**`Quickshell.Io`:**
- `Process`: `command`, `running`, `environment`, `stdout`/`stderr` parsers, `stdinEnabled`, `write()`, `signal()`, `exec()`, `startDetached()`.
- `SplitParser`: `splitMarker`, `onRead`.
- `StdioCollector`: `text`, `waitForEnd`, `streamFinished`.
- `Socket`: `path`, read/write `connected`, `write()`, `flush()`, `parser`.
- `SocketServer`: `active`, `path`, `handler: Socket {}`. It is `Reloadable`.
- `IpcHandler`: `target`, `enabled`; functions must be typed (`string`/`int`/`bool`/`real`/`color`).
- `FileView` + `JsonAdapter`: `watchChanges`, `atomicWrites`.

**`Quickshell` singleton:**
- `screens`, `watchFiles` (default true), `reload(hard)`, `reloadCompleted`/`reloadFailed`, `inhibitReloadPopup()`.
- `env()`, `shellDir`, `dataDir`/`stateDir`/`cacheDir`, `execDetached()`.

**CLI** (from `quickshell --help`):
- `qs -p <file|dir>` (env `QS_CONFIG_PATH`), `-n` (no duplicate), `-d` (daemonize).
- `qs ipc [-p|--id|--pid|--any-display] {show|call|wait|listen|prop}`, `qs log|list|kill`.
- `qs ipc` assumes the "default" config unless `-p` or `QS_CONFIG_PATH` is given. Set `QS_CONFIG_PATH=/app/ui` in the container so `qs ipc call pet ping` finds the instance.

**Pragmas** (upstream `src/launch/launch.cpp:93-116`):
- `//@ pragma Env VAR=VALUE` (spaces trimmed).
- `//@ pragma DefaultEnv VAR=VALUE` (only sets the variable if it is unset).
- Others: `NativeTextRendering`, `ShellId`, `AppId`, `DataDir`/`StateDir`/`CacheDir`, `IconTheme`, `UseQApplication`, `DropExpensiveFonts`.

**Environment variables seen in the binary:**
- Quickshell: `QS_NO_RELOAD_POPUP`, `QS_DISABLE_FILE_WATCHER`, `QS_DISABLE_CRASH_HANDLER`, `QS_DROP_EXPENSIVE_FONTS`, `QS_DISABLE_DMABUF`, `QS_NO_BUFFER_REUSE`, `QS_APP_ID`, `QS_ICON_THEME`.
- Qt scene graph: `QSG_NO_VSYNC`, `QSG_NO_DEPTH_BUFFER`, `QSG_NO_STENCIL_BUFFER`.
- If `WAYLAND_DISPLAY` is present but `QT_QPA_PLATFORM` is not wayland, Quickshell warns. Set `QT_QPA_PLATFORM=wayland` in the container.

### Upstream v0.3.1 behaviours that matter

**Screen switch remaps cleanly** (`src/window/proxywindow.cpp`, `src/wayland/wlr_layershell/wlr_layershell.cpp`):
- `WlrLayershell::deleteOnInvisible()` returns `true`, so `visible:false` destroys the `QQuickWindow` and its layer surface.
- `ProxyWindowBase::setScreen()` on a live window does `setVisibleDirect(false)` → `setVisibleDirect(true)`. The window is recreated on the new `QScreen`.
- The QML content item (`mContentItem`) is only reparented, so item and animation state survive.
- `screen: null` sets `compositorPicksScreen`, which passes `output=NULL` to `get_layer_surface`.
- Hyprland places such a surface on `Desktop::focusState()->monitor()` (`src/desktop/view/LayerSurface.cpp:26`).
- Omarchy's comment in `Bar.qml` measures an unmap/remap rebuild at about 150 ms versus about 20 ms to tear down.

**Runtime margin changes:**
- Setting margins, anchors, layer or keyboardFocus calls `onStateChanged` → `schedulePolish` → `contentItem.polish()`.
- On the next frame, `onPolished` → `bridge->commitState()` → `LayerSurface::commit()`, which sends only the changed `set_margin`/`set_anchor`/… requests.
- Those are applied by that frame's `wl_surface.commit`.
- Qt 6's threaded render loop "always completes and presents a frame" once one is requested (comment in `qsgthreadedrenderloop.cpp` 6.11). Qt 5 used to abort, so margin-only changes do get committed in Qt 6.
- Margins are `int`, converted with `QHighDpi::toNativePixels`, which is the identity on Wayland unless `QT_SCALE_FACTOR` is set.

**Click-through:**
- `onPolished` turns the `Region` into a `QRegion` and applies it with `QWindow::setMask`.
- A non-null **empty** `Region {}` sets `Qt::WindowTransparentForInput`, making the whole surface click-through.
- `Region { item: x }` tracks `x`'s own x/y/width/height through `mapToScene`, so make the **moving** item the mask item.

**Socket handling:**
- `SocketServer::enableServer()` calls `QFile::remove(path)` before `listen()`, so a stale socket left by a crash is not a problem.
- Hyprland socket resolution (`src/wayland/hyprland/ipc/connection.cpp:41-65`):
  - It uses `$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE`, falling back to `/tmp/hypr/$HIS`.
  - The request socket is `.socket.sock` and the event socket `.socket2.sock` (opened `ReadOnly`).
  - Startup refresh uses `j/monitors`, `j/workspaces` and `j/clients` on the request socket.

**Runtime registry and collision risk:**
- Quickshell writes `$XDG_RUNTIME_DIR/quickshell/{by-id/<id>/{instance.lock,ipc.sock,log.qslog,log.log}, by-pid/<pid>, by-path/<hash>, by-shell/<hash>/<id>, vfs/<hash>/tooling.lock}`. This structure was observed live under `/run/user/1000/quickshell` next to Omarchy's instance.
- If the container shares the host runtime dir:
  - The container instance would register in the same tree, and `by-pid` would hold container-namespace PIDs.
  - The container would also see `bus`, `pipewire-0`, `pulse/native` and `gnupg` in that directory.
- **Use a private tmpfs `XDG_RUNTIME_DIR`** and bind only the sockets the pet needs.

### Patterns already in Omarchy (`/usr/share/omarchy/shell`)

- **`plugins/osd/Osd.qml`:** a full-screen Overlay `PanelWindow` with keyboard `None`, `ExclusionMode.Ignore` and `mask: Region {}`. It sets **no `screen:`**, so the compositor picks the focused monitor. It has `IpcHandler { target: "osd" }`.
- **`plugins/notifications/Service.qml:947-980`:** `Variants { model: Quickshell.screens }` with `mask: Region { item: popupColumn }`. The surface is click-through except over the toasts.
- **`plugins/bar/Bar.qml:1397-1500`:**
  - `DragGhostPanel` and `BarMoveGhostPanel` are full-screen Overlay windows with `mask: Region {}`, drawn only while dragging.
  - Drag coordinates come from the source `MouseArea` under the implicit grab.
  - The bar hides by **parking at a negative margin** instead of unmapping, which keeps the surface alive (the ~150 ms vs ~20 ms comment).
- **`Ui/ScreenMoveRemap.qml`:**
  - "Hyprland leaves an already-mapped layer surface at its old global position when its monitor moves within the layout."
  - Fix: on screen x/y change, wait 200 ms to settle, then pulse `visible` false for 50 ms.
- **`Ui/KeyboardPanel.qml`:** keyboard focus `Exclusive` "makes Hyprland route every pointer event to the exclusive surface". Keep the pet at `None`.
- **`Bar.qml:716`:** maps focus to a screen with `Hyprland.focusedMonitor.name === window.screen.name`.
- **Pure logic is kept in plain JS files** (`OsdModel.js`, `BarModel.js`, `NotificationLogic.js`). This is a testable pattern to copy.
- **Hyprland Lua config:**
  - `looknfeel.lua:83-87` enables the `layers`, `layersIn` (style fade), `layersOut` (fade), `fadeLayersIn` and `fadeLayersOut` animations.
  - `apps/omarchy-shell.lua` disables them per namespace with `hl.layer_rule({ match = { namespace = "omarchy-bar" }, no_anim = true, animation = "none" })`.
  - `input.lua` sets `follow_mouse = 1`.

### Renderer inside the container (Qt 6.11)

**OpenGL (RHI) with `/dev/dri/renderD128`:**
- The device is `crw-rw-rw-`, so no group is needed.
- Mesa in the Arch image (26.2.2, iris) matches the host exactly.
- You get ShaderEffect, MultiEffect and `QtQuick.Particles`.
- **Cost:** QtWayland's GL path calls plain `eglSwapBuffers` with no damage rects (`qtbase/src/plugins/platforms/wayland/plugins/hardwareintegration/wayland-egl/qwaylandglcontext.cpp:496-497`). Every rendered frame therefore damages the **whole surface**, and Hyprland recomposites all of it.
- So keep GL surfaces small in steady state. A full-monitor surface is fine transiently.
- If `/dev/dri` is not passed, Mesa falls back to llvmpipe (expected; not tested here).

**`QT_QUICK_BACKEND=software`:**
- Qt raster with partial updates.
- QtWayland's shm path commits only the flushed region (`qwaylandshmbackingstore.cpp:301-336` `safeCommit(buffer, region)`; `qwaylandwindow.cpp:803-828` `damage_buffer`).
- shm buffers are sized at the fractional `scale()` (`qwaylandshmbackingstore.cpp:395`).
- **No ShaderEffect and no particles** (Qt docs).
- `QSGSoftwareSpriteNode` is present in `libQt6Quick.so.6`, so `AnimatedSprite` works.
- This makes a permanent full-monitor stage cheap, at the price of 2 full-size shm buffers: 2×9.2 MB on eDP-1, 2×33 MB on the 4K monitor.
- Not tested with Quickshell; it needs a spike.

### Fractional scale and pixel art

**Qt side:**
- QtWayland 6.11 binds `wp_fractional_scale_manager_v1`, `wp_viewporter` and `wp_cursor_shape_manager_v1` (from `strings libQt6WaylandClient.so.6`). The window DPR is 1.5.
- Because of cursor-shape-v1, no cursor theme is needed in the image.
- `Image { smooth: false }` gives nearest-neighbour sampling (`qquickimage.cpp:43`).
- `AnimatedSprite` does `node->setFiltering(smooth() ? Linear : Nearest)` (`qquickanimatedsprite.cpp:931`), and `interpolate` blends frames by default. Set `interpolate: false`.
- **`AnimatedSprite::updatePaintNode` calls `maybeUpdate()` every frame while running** (lines 796-797). That forces vsync-rate (60 Hz) rendering whatever `frameRate` is.
- For a low-power idle, step frames with a `Timer` on a clipped sprite-sheet `Image`. Then Qt renders only when the frame changes.

**Hyprland side** (`src/render/ElementRenderer.cpp:251-283`, v0.56.2):
- The surface box is `scale(monitorScale)` then `.round()`-ed to integer device pixels.
- If a fractional-scale buffer is off by 1–2 px, Hyprland forces nearest-neighbour and fixes the UVs.
- So an odd *surface* margin never blurs; it only rounds by half a device pixel.
- *In-surface* content offsets are rendered by Qt or GTK at ×1.5, so items must sit at even logical px.

**Rules:**
- An art pixel is 4 logical px, which is 6 device px. For example, a 32×32 sprite becomes 128×128 logical and 192×192 device.
- All item x/y/width/height and the surface size must be even (`snap2 = v => Math.round(v/2)*2`).
- Do not scale or rotate pixel art with transforms; draw extra frames instead.
- Avoid `layer.enabled` on the sprite (it creates an FBO with linear filtering).

### Receiving external events in Quickshell

**Option 1: `SocketServer` (recommended).** This is the doc example:

```qml
SocketServer {
  active: true
  path: "/run/pet/events.sock"   // bind-mounted from host $XDG_RUNTIME_DIR/claude-pet/
  handler: Socket { parser: SplitParser { onRead: line => PetModel.handle(root, line) } }
}
```

The host writes with `socat - UNIX-CONNECT:...` or `python3`. `socat` is explicitly installed on this host but no package requires it. `curl` is required by pacman and git, so it is always present. Choose HTTP (curl) only if you add a small bridge.

**Option 2: `Process` + `SplitParser`** reading the stdout lines of a bridge program.
- Drawback: on every hot reload the Process is recreated, so the bridge restarts.
- Prefer a bridge whose lifecycle is independent and which talks to `SocketServer`.

**Option 3: `IpcHandler` + `qs ipc call`.**
- The call has to run inside the container (same private runtime dir), e.g. `docker exec claude-pet qs ipc call pet play celebrate`.
- Good for the healthcheck (`qs ipc call pet ping`) and for manual testing. Not for hooks, because of the docker exec latency and the dependency on the docker CLI.

### Footprint (estimates, not measured)

- **Arch installed sizes** (`pactree` + `pacman -Qi` on this host):
  - `base` closure: 511 MiB.
  - `base ∪ quickshell`: 1090 MiB.
  - `base ∪ quickshell ∪ python`: 1164 MiB.
  - `base ∪ python-gobject ∪ gtk4-layer-shell ∪ python-cairo`: 1220 MiB.
  - The largest single dependency is `llvm-libs` at 163.7 MiB, pulled in by mesa.
  - Expect an image of about 0.9–1.1 GB after `pacman -Scc`.
- **RAM:**
  - Omarchy's full Quickshell instance on this host right now: 316 MB RSS (208 MB anon, 25 threads). That is an upper bound, since it runs bar, background and notifications.
  - A minimal pet config is probably about 70–150 MB with GL and lower with the software backend. Unverified; measure it.

---

## B. GTK 4.22.4 + gtk4-layer-shell 1.3.0 + PyGObject 3.56.3

**API from the installed GIR and gtk-doc:**
- `init_for_window` must run before realize.
- `set_layer`, `set_namespace`, `set_anchor`, `set_exclusive_zone(-1)`, `set_keyboard_mode` (default `NONE`; the docs say "To control mouse/touch interactivity use input regions").
- `set_monitor`: "If the window is currently mapped, it will get remapped". Upstream `gtk_layer_surface_remap` does `gtk_widget_unrealize` + `gtk_widget_map`, which recreates the surface, renderer and GL context.
- `set_margin` sends `set_margin`, then fakes an xdg configure to force GTK to commit a frame (`layer_surface_needs_commit`).
- `set_respect_close` exists since 1.3.

**Python loading requirement** (upstream `linking.md`): either do
```python
from ctypes import CDLL
CDLL('libgtk4-layer-shell.so')
```
before importing `gi`, or use `LD_PRELOAD`. `xtrimsystems/claude-pet` re-execs itself with `LD_PRELOAD`.

**Input region:**
- `Gdk.Surface.set_input_region(region)`: "Mouse events which happen while the pointer position corresponds to an unset bit in the mask will be passed on to the surface below".
- PyGObject's `_gi_cairo` links `cairo_region_*`, so `cairo.Region(cairo.RectangleInt(x,y,w,h))` works.
- GTK 4.22 only rewrites the input region for client-decorated windows with shadows (`gtk/gtkwindow.c:4214-4243`: `if (!priv->client_decorated || !priv->use_client_shadow) return;`). Custom regions on undecorated layer windows stick.

**Fractional scaling** (GTK `NEWS`):
- 4.12: "Use fractional scales on Wayland with cairo".
- 4.14: "allow fractional scaling by default with gl".
- 4.17.5: "!8229 Remove GDK_DEBUG=gl-no-fractional". So on 4.18+ it is always on, and `strings` confirms there is no fractional flag left in 4.22.
- 4.17.6: Vulkan fractional fixes (#7314), plus "The Wayland cursor shape protocol is supported".

**Pixel-exact drawing:**
- Use a custom widget whose `do_snapshot` does `snapshot.push_clip(frame_rect)`, then `snapshot.append_scaled_texture(sheet_texture, Gsk.ScalingFilter.NEAREST, graphene_rect)` (GTK 4.10+, confirmed in `Gtk-4.0.gir`), then `pop()`.
- Prefer this over cairo `DrawingArea` + `FILTER_NEAREST`. How a cairo node is rasterized at 1.5 was not verified.

**Damage-aware rendering:**
- The GTK binary contains `EGL_EXT_buffer_age` and `eglSwapBuffersWithDamage{EXT,KHR}`, so only changed regions are redrawn and damaged. That is why a permanent full-output overlay is cheap; xtrimsystems reports about 1% idle CPU with a 60 fps GLib timer.
- Timing comes from `Gtk.Widget.add_tick_callback` or `GLib.timeout_add`.

**GTK gotcha** (xtrimsystems `pet_window.py`): "Hiding or moving the widget away leaves its last frame on screen: with nothing left to draw, GTK skips the frame and the compositor keeps the old buffer." Paint the spot transparent instead.

**Packages:**
- Arch: gtk4 4.22.4, gtk4-layer-shell 1.3.0.
- Debian trixie: libgtk-4-1 4.18.6+ds-2, gtk4-layer-shell 1.0.4-2 (`gir1.2-gtk4layershell-1.0`, `libgtk4-layer-shell0`; lacks the 1.3 API).
- Debian forky/sid, Ubuntu 26.04 and Alpine 3.23+ have 1.3.0.

---

## C. Rust + SCTK

**Crate:**
- `smithay-client-toolkit` 0.21.1 (2026-07-23) has `shell::wlr_layer`, `shm::slot::SlotPool`, `seat::pointer::cursor_shape` and an `image_viewporter` example.
- It has **no fractional-scale helper.** You must bind `wp_fractional_scale_manager_v1` yourself (from `wayland-protocols` with the "staging" feature) and set the `wp_viewport` destination to the logical size.
- Default features are `["calloop", "xkbcommon"]`. Use `default-features = false, features = ["calloop"]` to avoid libxkbcommon.
- With the pure-Rust wayland backend you can build a static musl binary and ship it `FROM scratch`.

**What you get:**
- You commit `set_margin` immediately, with no wait for vsync.
- Damage covers only the changed rect, and the art-pixel → 6×6 device-px blit is integer math.
- Golden PNG frame tests run without any compositor.

**Precedent:** `saatvik333/wayland-bongocat` (C, 481★) uses layer-shell + fractional-scale-v1 + viewporter + shm + Hyprland IPC for fullscreen hiding, and claims about 8 MB RAM.

---

## Cross-cutting questions

### (1) Dragging
- **No configure on a pure move:** Hyprland sends a layer `configure` only when the size changes (`src/render/Renderer.cpp:2666`). A margin-only move needs no configure round-trip.
- **Position warps at commit unless animating:** `LayerSurface.cpp:356-366` warps the position at commit unless a Hyprland animation is already running.
  - With Omarchy's `fade` style, `animateIn` keeps `pos.from == pos.to` (`LayerSurfaceAnimationController.cpp:115-131`), so only alpha animates on map.
  - A `no_anim` layer rule is optional.
- **The real jitter source is the moving coordinate frame:**
  - Pointer coordinates are surface-local, relative to the compositor's current box (`InputManager.cpp` ~425 `getSurfaceBoxGlobal`).
  - In Qt, the GUI thread updates `margins` immediately, but the commit lands up to one vsync later.
  - Motion events generated against the old origin then get counted twice, which shows up as overshoot and jitter on fast drags. This is inferred from the code.
  - `Ryoku-dev/ryoku` `Pin.qml` uses that plain `slotX += m.x - px` approach anyway.
- **Implicit grab:** `InputManager.cpp:410-413` says "if we are holding a pointer button, and we're not dnd-ing, don't refocus. Keep focus on last surface". Motion keeps flowing to the pressed surface even outside it, and even across monitors (also observed in Omarchy's bar drag and in xtrimsystems).
  - **But** the condition also requires `Desktop::focusState()->surface()`, the keyboard-focus surface, to be mapped.
  - On an **empty workspace** the lock may not engage. This is inferred and must be tested.
- **Recommended for Quickshell:** the "grow-on-hold stage" pattern from `dhrruvsharma/shell/.../WidgetWindow.qml`:
  - While held, `anchors.right/bottom: held` makes the surface full-monitor, and the sprite moves inside it with plain Qt coordinates (no compositor round-trip).
  - The sprite switches to stage coordinates only once `full` is true, so nothing jumps.
  - On release the surface shrinks back, with margins set to the snapped final position.
  - Also widen the `mask` to the whole stage while held, so the drag does not depend on the grab lock.
- **Alternatives:**
  - Use absolute cursor coordinates from Hyprland: a `Socket` on `Hyprland.requestSocketPath` writing `j/cursorpos`. Each request needs a new connection. This requires the exec-capable request socket.
  - Use a permanent full-output overlay plus an input region (the GTK/xtrimsystems approach).
  - In Rust, commit right after `set_margin`.
- **Snapping:** keep x/y on even logical px. Hyprland rounds the surface box anyway; the content offsets are what must be even.

### (2) Following the focused monitor
- Hyprland emits `focusedmon>>NAME,WSNAME` and `focusedmonv2>>NAME,WSID` on every monitor-focus change (`src/desktop/state/FocusState.cpp:287-288`), including pointer crossing, since `follow_mouse = 1`.
- **Quickshell:** `screen: screenByName(Hyprland.focusedMonitor?.name)`, which gives a clean remap (destroy and recreate).
  - Freeze the binding while dragging.
  - Reuse the `ScreenMoveRemap` logic for layout moves and hotplug.
  - Handle `closed` when HDMI-A-1 disappears.
- **Optional instant switch:** one small `PanelWindow` per screen (`Variants`), with inactive ones *parked* off-screen through negative margins (Omarchy's bar trick). Switching is then a margin change instead of a remap.
- **GTK:** parse socket2 with Gio, then `LS.set_monitor(win, gdk_monitor_by_connector)`, or keep per-monitor windows.
- **Rust:** watch socket2 as a calloop fd source and recreate the layer surface on the `wl_output` whose `name` matches.

### (3) Crisp pixel art at 1.5
- The rules are in the Fractional scale and pixel art section above.
- Verify with `grim -s 1.5 -g "X,Y WxH"` using the geometry from `hyprctl -j layers`. `grim -s` takes a factor; that it accepts fractional values was not verified.
- Then assert that every opaque sprite pixel's RGB is in the sprite palette (no interpolated colours) and that 6×6 blocks are uniform.

### (4) Cost of an always-animating ~200×200 logical overlay
That is a 300×300 device buffer, about 360 KB. All figures below are estimates except the cited claims.

| Stack | RSS | Idle CPU | Steady-state damage |
|---|---|---|---|
| Quickshell GL, small surface, Timer-stepped 6–8 fps | ~70–150 MB | <1% | 300×300 per frame |
| Quickshell GL with a running AnimatedSprite | same | 60 fps renders, ~2–5% | 300×300 at 60 Hz |
| Quickshell software backend, full stage | lower (no Mesa loaded) | <1% | dirty rects only |
| GTK/Python, per-monitor full overlay | ~60–120 MB | ~1% (xtrimsystems claim) | dirty rects only |
| Rust/SCTK shm | ~5–15 MB (bongocat ~8 MB claim) | ≪1% | dirty rect only |

### (5) Iteration speed
- **Quickshell:** `watchFiles` defaults to true. A soft reload reuses windows (`reload(hard=false)`; `reloadableId`), and the reload popup shows QML errors (disable it in prod with `QS_NO_RELOAD_POPUP=1`).
  - Bind-mount `./ui:/app/ui:ro` for development; inotify works across bind mounts.
  - Watching across editor atomic-rename saves was not verified; `QS_DISABLE_FILE_WATCHER` exists.
- **GTK/Python:** restart the process (~1 s) plus a remap flicker.
- **Rust:** recompile (seconds to tens of seconds). Only data (sprites/JSON) can be hot-reloaded, e.g. with the `notify` crate.

### (6) Testability
- **Logic:** keep the state machine in plain `.js` (as Omarchy does) and test it with `/usr/lib/qt6/bin/qmltestrunner` (qt6-declarative 6.11.2, under `QT_QPA_PLATFORM=offscreen`) or with Node. Quickshell's own types are not importable in qmltestrunner because they are statically linked into `/usr/bin/quickshell`.
- **Live checks:**
  - `docker exec claude-pet qs ipc call pet play celebrate`
  - `hyprctl -j layers | jq '.. | objects | select(.namespace?=="claude-pet")'` for monitor and geometry (global logical coords plus `pid`).
  - `grim -s 1.5 -g ...` for crispness.
  - `docker stats --no-stream` and `ps -o rss` for cost.
- **GTK:** pytest for logic.
- **Rust:** `cargo test` with headless golden frames.

---

## Recommended Quickshell skeleton (API names verified against the installed qmltypes)

```qml
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Hyprland
import "logic/PetModel.js" as PetModel

ShellRoot {
  id: root
  readonly property string focusedName: Hyprland.focusedMonitor ? Hyprland.focusedMonitor.name : ""
  property string petScreenName: ""
  onFocusedNameChanged: if (!win.held) petScreenName = focusedName
  function screenByName(n) { const s = Quickshell.screens; for (let i = 0; i < s.length; i++) if (s[i].name === n) return s[i]; return null }
  function snap2(v) { return Math.round(v / 2) * 2 }

  PanelWindow {
    id: win
    screen: root.screenByName(root.petScreenName)          // null → compositor picks focused monitor
    visible: !(Hyprland.focusedWorkspace && Hyprland.focusedWorkspace.hasFullscreen) || pet.celebrating
    color: "transparent"                                    // default is white!
    WlrLayershell.namespace: "claude-pet"
    WlrLayershell.layer: WlrLayer.Overlay                   // Top = auto-hidden under fullscreen
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
    exclusionMode: ExclusionMode.Ignore                     // margins relative to output edges, can cover bar
    property bool held: false
    readonly property bool stage: held || pet.celebrating
    readonly property bool full: screen !== null && width >= screen.width - 1 && height >= screen.height - 1
    property int restX: 400; property int restY: 300        // even; persisted per monitor name
    anchors { top: true; left: true; right: stage; bottom: stage }
    margins { left: stage ? 0 : restX; top: stage ? 0 : restY }
    implicitWidth: 160; implicitHeight: 160                 // even → 240x240 device buffer
    mask: Region { item: win.held ? stageHit : sprite }
    Item { id: stageHit; anchors.fill: parent }
    Item {
      id: sprite; width: 128; height: 128; clip: true       // 32 art px × 4
      x: win.full ? pet.stageX : 16; y: win.full ? pet.stageY : 16
      Image { source: "sprites/pet.png"; smooth: false; mipmap: false
              x: -pet.frame * 128; y: -pet.row * 128
              width: sourceSize.width * 4; height: sourceSize.height * 4 }
      MouseArea { anchors.fill: parent /* press: win.held=true, pet.stageX/Y = rest+16; move (only when win.full): p = mapToItem(stageHit, mouse.x, mouse.y) → snap2(p - grab); release: restX/Y = snap2(stage - 16), held=false */ }
    }
  }
  Timer { interval: Math.round(1000 / pet.fps); running: pet.playing; repeat: true; onTriggered: pet.advance() }
  SocketServer { active: true; path: "/run/pet/events.sock"
                 handler: Socket { parser: SplitParser { onRead: line => PetModel.handle(pet, line) } } }
  IpcHandler { target: "pet"
               function ping(): string { return "ok" }
               function play(name: string): void { pet.play(name) } }
}
```

## Docker specifics (compose sketch, following the user's conventions)

```yaml
x-logging: &logging { driver: json-file, options: { max-size: "10m", max-file: "5" } }
services:
  pet:
    image: claude-pet:local
    build: { context: ., args: { APP_UID: "${APP_UID:-1000}", APP_GID: "${APP_GID:-1000}" } }
    user: "${APP_UID:-1000}:${APP_GID:-1000}"   # must match the socket owner (srwxr-xr-x barbaruiva)
    restart: "no"                                # lifecycle owned by systemd --user (see risks)
    read_only: true
    network_mode: none                           # if ingress is the UNIX socket
    environment:
      TZ: America/Sao_Paulo
      QT_QPA_PLATFORM: wayland
      WAYLAND_DISPLAY: /wayland/wayland-1        # absolute path: libwayland accepts it
      XDG_RUNTIME_DIR: /run/user/1000            # private tmpfs → private quickshell registry
      HYPRLAND_INSTANCE_SIGNATURE: ${HYPRLAND_INSTANCE_SIGNATURE:?start from the Hyprland session}
      QS_CONFIG_PATH: /app/ui                    # used by both `quickshell` and `qs ipc`
      HOME: /home/pet
      # QT_QUICK_BACKEND: software               # optional (partial damage; no shaders/particles)
    tmpfs:
      - /run/user/1000:mode=0700,uid=1000,gid=1000
      - /home/pet:mode=0700,uid=1000,gid=1000
    volumes:
      - { type: bind, source: "${XDG_RUNTIME_DIR}/${WAYLAND_DISPLAY}", target: /wayland/wayland-1, bind: { create_host_path: false } }
      - { type: bind, source: "${XDG_RUNTIME_DIR}/hypr", target: /tmp/hypr, read_only: true, bind: { create_host_path: false } }   # QS falls back to /tmp/hypr/$HIS
      - { type: bind, source: "${XDG_RUNTIME_DIR}/claude-pet", target: /run/pet, bind: { create_host_path: false } }
      - ./ui:/app/ui:ro
      - ${HOME}/.local/state/claude-pet:/data
    devices: [ "/dev/dri/renderD128:/dev/dri/renderD128" ]
    healthcheck: { test: ["CMD", "qs", "ipc", "call", "pet", "ping"], interval: 30s, timeout: 5s, retries: 3 }
    logging: *logging
```

**Dockerfile:**
- `FROM archlinux:base`, pinned with a dated tag or an `archive.archlinux.org/repos/YYYY/MM/DD` mirrorlist.
- `pacman -Syu --noconfirm quickshell && pacman -Scc --noconfirm`, add a font only if text is used, create the user with `APP_UID`/`APP_GID`.
- `ENTRYPOINT ["quickshell"]`; it reads `QS_CONFIG_PATH`.

**Starting it:** a `systemd --user` unit with `PartOf=graphical-session.target` and `After=graphical-session.target docker.service`:
- `ExecStartPre=mkdir -p %t/claude-pet`
- `ExecStart=docker compose -f … up --force-recreate`
- `ExecStop=docker compose … down`
- The user manager already holds `WAYLAND_DISPLAY` and `HYPRLAND_INSTANCE_SIGNATURE` (exported by uwsm).

## Sources

- Installed files:
  - `/usr/lib/qt6/qml/Quickshell/**`
  - `/usr/share/omarchy/shell/**`, `/usr/share/omarchy/default/hypr/**`
  - `/usr/share/gir-1.0/{Gtk4LayerShell-1.0,Gtk-4.0,Gsk-4.0,Gdk-4.0}.gir`
  - `/usr/share/gtk-doc/html/gtk4-layer-shell/`
- Upstream source at the installed tags, via `gh api`:
  - `quickshell-mirror/quickshell@v0.3.1`, `hyprwm/Hyprland@v0.56.2`
  - `qt/qtdeclarative@6.11`, `qt/qtbase@6.11`
  - `wmww/gtk4-layer-shell@v1.3.0`, `GNOME/gtk@4.22.4`
- Quickshell docs (quickshell.org/docs/v0.3.0): Region, QsWindow, PanelWindow, Quickshell, Reloadable, SocketServer, IpcHandler.
- GTK blog: https://blog.gtk.org/2023/04/05/gtk-4-11-1/
- Qt Quick Software Adaptation docs.
- Repology: quickshell, gtk4-layer-shell.
- packages.debian.org: trixie gtk4-layer-shell, libgtk-4-1.
- crates.io: smithay-client-toolkit.
- Docker Compose services reference (volumes).
- Precedent repos:
  - https://github.com/xtrimsystems/claude-pet
  - https://github.com/dhrruvsharma/shell
  - https://github.com/Ryoku-dev/ryoku
  - https://github.com/saatvik333/wayland-bongocat
  - https://leon_plickat.srht.site/wayneko.html
  - https://github.com/pixelomer/Shijima-Qt (archived; its author says Qt was "not the right framework" because of the hacks needed in a Qt Widgets cross-platform app)

## Fatos-chave

- **[high]** Quickshell 0.3.1 is in Arch's official [extra] repo, so an archlinux image can `pacman -S quickshell`. It hard-depends on mesa, libglvnd, qt6-base, qt6-declarative, qt6-svg, qt6-wayland, libpipewire, polkit, pam, cpptrace and jemalloc. Its QML plugins are compiled into /usr/bin/quickshell.  
  _Evidência:_ `pacman -Si quickshell` → Repository: extra, Version 0.3.1-1; `pacman -Ql quickshell`; /usr/lib/qt6/qml/Quickshell/qmldir ('optional plugin', 'prefer :/qt/qml/Quickshell/')
- **[high]** WlrLayershell exposes these properties: layer (Background/Bottom/Top/Overlay), namespace, keyboardFocus (None/Exclusive/OnDemand), anchors, exclusiveZone, exclusionMode (Normal/Ignore/Auto), margins (int), aboveWindows and focusable. On PanelWindow they are set as attached properties, e.g. `WlrLayershell.layer: WlrLayer.Overlay`.  
  _Evidência:_ /usr/lib/qt6/qml/Quickshell/Wayland/_WlrLayerShell/quickshell-wayland-layershell.qmltypes; /usr/lib/qt6/qml/Quickshell/_Window/quickshell-window.qmltypes; usage in /usr/share/omarchy/shell/plugins/osd/Osd.qml:131-137
- **[high]** QsWindow `color` defaults to white, so the pet must set `color: "transparent"`. With alpha below 255, Quickshell requests an alpha buffer (a non-opaque surface).  
  _Evidência:_ quickshell.org/docs/v0.3.0/types/Quickshell/QsWindow ('Defaults to white'); upstream proxywindow.cpp ensureQWindow: opaque = mColor.alpha() >= 255
- **[high]** For layer shells, setting `visible: false` destroys the QQuickWindow and its layer surface (deleteOnInvisible() returns true). Changing `screen` on a live window hides it and re-shows it on the new QScreen, a clean remap. The QML content item is only reparented, so its state survives.  
  _Evidência:_ quickshell v0.3.1 src/wayland/wlr_layershell/wlr_layershell.cpp deleteOnInvisible(); src/window/proxywindow.cpp setScreen()/setVisibleDirect()/completeWindow()
- **[high]** `screen: null` makes Quickshell pass output=NULL to get_layer_surface. Hyprland 0.56.2 then places the surface on the currently focused monitor (Desktop::focusState()->monitor()).  
  _Evidência:_ quickshell v0.3.1 surface.cpp LayerSurface ctor (compositorPickesScreen); Hyprland v0.56.2 src/desktop/view/LayerSurface.cpp:26
- **[high]** Margin, anchor and layer changes in Quickshell are deferred to the next Qt frame: polish → bridge->commitState() → set_margin etc. They are applied by that frame's wl_surface.commit. Qt 6's threaded render loop always presents a frame once one is requested, so margin-only changes do commit.  
  _Evidência:_ quickshell v0.3.1 wlr_layershell.cpp onStateChanged/onPolished; surface.cpp LayerSurface::commit(); qtdeclarative 6.11 qsgthreadedrenderloop.cpp comment 'In Qt 6 this function always completes and presents a frame'
- **[high]** In Hyprland 0.56.2, a margin-only move sends no configure (configure is sent only when the box size changes). The position change warps instantly at commit unless the surface is already animating.  
  _Evidência:_ Hyprland v0.56.2 src/render/Renderer.cpp:2666 `if (Vector2D{box.width, box.height} != OLDSIZE) ls->m_layerSurface->configure(...)`; src/desktop/view/LayerSurface.cpp:356-366
- **[high]** Omarchy's layer animations use style 'fade'. In that style animateIn keeps pos.from == pos.to, so only alpha animates on (re)map and drags still warp. Omarchy disables animations per namespace with hl.layer_rule({ match = { namespace = ... }, no_anim = true, animation = "none" }).  
  _Evidência:_ /usr/share/omarchy/default/hypr/looknfeel.lua:83-87; /usr/share/omarchy/default/hypr/apps/omarchy-shell.lua:5,10; Hyprland v0.56.2 LayerSurfaceAnimationController.cpp:115-131
- **[medium]** While a pointer button is held (and no DnD is active), Hyprland keeps pointer focus on the last surface, including layer surfaces. This lock also requires a mapped keyboard-focus surface (focusState()->surface()). On an empty workspace with nothing focused, the lock may not engage.  
  _Evidência:_ Hyprland v0.56.2 src/managers/input/InputManager.cpp:410-433; FocusState.hpp m_focusSurface/rawSurfaceFocus (the edge case is inferred, not tested)
- **[high]** Hyprland scales each surface box by the monitor scale and rounds it to integer device pixels before drawing. It forces nearest-neighbour sampling when a fractional-scale buffer is off by 1–2 px. So odd surface margins do not blur; content offsets inside the client must still be on even logical px at scale 1.5.  
  _Evidência:_ Hyprland v0.56.2 src/render/ElementRenderer.cpp:251-283 (windowBox.scale(...); windowBox.round(); MISALIGNEDFSV1 → useNearestNeighbor)
- **[high]** Hyprland emits focusedmon>>NAME,WSNAME and focusedmonv2>>NAME,WSID on every monitor-focus change, including pointer crossing. Omarchy sets follow_mouse = 1.  
  _Evidência:_ Hyprland v0.56.2 src/desktop/state/FocusState.cpp:287-288; /usr/share/omarchy/default/hypr/input.lua:57
- **[high]** Quickshell finds Hyprland's sockets at $XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket.sock and .socket2.sock, falling back to /tmp/hypr/$HIS. At startup it refreshes state with j/monitors, j/workspaces and j/clients on the request socket.  
  _Evidência:_ quickshell v0.3.1 src/wayland/hyprland/ipc/connection.cpp:41-92; `strings /usr/bin/quickshell` shows HYPRLAND_INSTANCE_SIGNATURE, XDG_RUNTIME_DIR, /tmp/hypr/, /.socket.sock, /.socket2.sock
- **[high]** Quickshell registers each instance under $XDG_RUNTIME_DIR/quickshell/{by-id/<id>/(instance.lock, ipc.sock, log.qslog, log.log), by-pid, by-path, by-shell, vfs}. Sharing the host runtime dir would mix the container instance with Omarchy's registry and expose bus, pipewire and pulse to the container.  
  _Evidência:_ `find /run/user/1000/quickshell` (live Omarchy instance 1709085 run as `quickshell -n -p /usr/share/omarchy/shell`); binary strings 'by-pid', 'by-path', 'by-shell', 'ipc.sock'
- **[high]** Quickshell CLI: `qs -p <file|dir>` (env QS_CONFIG_PATH), -n (no duplicate), -d (daemonize), and `qs ipc {show,call,wait,listen,prop}` with -p/--id/--pid/--any-display. `qs ipc` targets the 'default' config unless -p or QS_CONFIG_PATH is given.  
  _Evidência:_ `quickshell --help`, `quickshell ipc --help` on host
- **[high]** Pragma syntax: `//@ pragma Env VAR=VALUE` (whitespace trimmed) and `//@ pragma DefaultEnv VAR=VALUE`. Also available: NativeTextRendering, ShellId, AppId, DataDir/StateDir/CacheDir and IconTheme.  
  _Evidência:_ quickshell v0.3.1 src/launch/launch.cpp:93-116
- **[high]** `mask: Region {}` (empty) makes a window fully click-through (Qt::WindowTransparentForInput). `Region { item: X }` tracks X's own x/y/width/height through mapToScene.  
  _Evidência:_ quickshell v0.3.1 proxywindow.cpp onPolished(); src/core/region.cpp setItem()/build(); Omarchy osd/Osd.qml:137 and notifications/Service.qml:978
- **[high]** SocketServer deletes any existing file at its path before listening, so a stale socket left by a crash does not block startup. It is Reloadable and its handler must create a Socket (doc example uses SplitParser.onRead).  
  _Evidência:_ quickshell v0.3.1 src/io/socket.cpp:156-182; quickshell.org/docs/v0.3.0/types/Quickshell.Io/SocketServer
- **[high]** Omarchy already uses the patterns the pet needs:
- full-screen click-through overlays without `screen:`, so the compositor picks the focused monitor (OSD);
- per-screen overlays with `mask: Region { item: popupColumn }` (notifications);
- full-screen drag-ghost overlays with an empty mask (bar);
- parking surfaces at negative margins instead of unmapping, measured at about 150 ms vs 20 ms;
- ScreenMoveRemap, which remaps when a monitor moves in the layout.  
  _Evidência:_ /usr/share/omarchy/shell/plugins/osd/Osd.qml; plugins/notifications/Service.qml:947-980; plugins/bar/Bar.qml:1230-1240,1397-1500; Ui/ScreenMoveRemap.qml
- **[high]** While running, AnimatedSprite requests a new frame every vsync (updatePaintNode calls maybeUpdate()), whatever its frameRate. Its filtering is Nearest when smooth is false, and `interpolate` (default true) blends frames.  
  _Evidência:_ qtdeclarative 6.11 src/quick/items/qquickanimatedsprite.cpp:778-800 and :929-931
- **[high]** QtWayland's GL path presents with plain eglSwapBuffers (no damage rects), so Qt Quick RHI frames damage the whole surface. The shm/software path commits only the flushed region and sizes shm buffers at the fractional scale.  
  _Evidência:_ qtbase 6.11 src/plugins/platforms/wayland/plugins/hardwareintegration/wayland-egl/qwaylandglcontext.cpp:496-497; qwaylandshmbackingstore.cpp:301-336,395; qwaylandwindow.cpp:803-828
- **[high]** The Qt Quick software adaptation cannot render ShaderEffect or particles but supports sprites: QSGSoftwareSpriteNode is present in the installed libQt6Quick.  
  _Evidência:_ Qt docs 'Qt Quick Software Adaptation'; `strings /usr/lib/libQt6Quick.so.6` → QSGSoftwareSpriteNode, QSG_SOFTWARE_RENDERER_FORCE_PARTIAL_UPDATES
- **[high]** QtWayland 6.11 and GTK 4.22 both bind wp_fractional_scale_manager_v1, wp_viewporter and wp_cursor_shape_manager_v1, so neither needs a cursor theme inside the container.  
  _Evidência:_ `strings /usr/lib/libQt6WaylandClient.so.6`; `strings /usr/lib/libgtk-4.so.1`
- **[high]** gtk4-layer-shell 1.3.0 behaviour:
- set_monitor on a mapped window remaps it (gtk_widget_unrealize + gtk_widget_map);
- set_margin forces a commit via a fake xdg configure;
- keyboard mode defaults to NONE;
- the docs point to input regions for pointer interactivity.  
  _Evidência:_ /usr/share/gir-1.0/Gtk4LayerShell-1.0.gir docs; wmww/gtk4-layer-shell v1.3.0 src/layer-surface.c:82-85,339-349; src/gtk4-layer-shell.c:143-151
- **[high]** In Python, libgtk4-layer-shell must load before libwayland-client: either `from ctypes import CDLL; CDLL('libgtk4-layer-shell.so')` before importing gi, or LD_PRELOAD.  
  _Evidência:_ wmww/gtk4-layer-shell v1.3.0 linking.md; xtrimsystems/claude-pet main.py ensure_layer_shell_preloaded()
- **[high]** PyGObject 3.56.3 marshals cairo.Region (so Gdk.Surface.set_input_region(cairo.Region(...)) works). GTK 4.22 only overrides the input region for client-decorated windows with shadows.  
  _Evidência:_ `strings /usr/lib/python3.14/site-packages/gi/_gi_cairo*.so` → cairo_region_copy/destroy/reference; GNOME/gtk 4.22.4 gtk/gtkwindow.c:4214-4243
- **[high]** GTK fractional scaling history:
- 4.12: cairo uses fractional scales;
- 4.14: GL (ngl) uses them by default;
- 4.17.5: GDK_DEBUG=gl-no-fractional removed, so it is always on from 4.18;
- 4.17.6: Vulkan fractional fixes and the cursor-shape protocol.
GTK 4.22 does damage-aware swaps.  
  _Evidência:_ GNOME/gtk 4.22.4 NEWS lines 4003-4022, 4834-4858, 2677 (!8229), 2516-2536; `strings libgtk-4.so.1` → EGL_EXT_buffer_age, eglSwapBuffersWithDamageKHR/EXT; blog.gtk.org 2023/04/05 gtk-4-11-1
- **[high]** GtkSnapshot.append_scaled_texture (since GTK 4.10) with Gsk.ScalingFilter.NEAREST gives nearest-neighbour texture scaling. Gtk.Widget.add_tick_callback exists for frame-clock driven animation.  
  _Evidência:_ /usr/share/gir-1.0/Gtk-4.0.gir:133495 (version="4.10"), :176566; Gsk-4.0.gir:10319-10342
- **[high]** Debian trixie ships GTK 4.18.6 and gtk4-layer-shell 1.0.4 (gir1.2-gtk4layershell-1.0, libgtk4-layer-shell0). Debian forky/sid, Ubuntu 26.04 and Alpine 3.23+ ship gtk4-layer-shell 1.3.0. Arch ships gtk4 4.22.4 and gtk4-layer-shell 1.3.0.  
  _Evidência:_ packages.debian.org/source/trixie/gtk4-layer-shell; packages.debian.org/trixie/libgtk-4-1; repology gtk4-layer-shell; `pacman -Q gtk4 gtk4-layer-shell`
- **[high]** smithay-client-toolkit 0.21.1 (2026-07-23) provides wlr_layer, the SlotPool shm allocator, cursor_shape and a viewporter example, but no fractional-scale helper. Its default features (calloop, xkbcommon) pull in libxkbcommon.  
  _Evidência:_ crates.io API; Smithay/client-toolkit repo tree; Cargo.toml @v0.21.1; GitHub code search for 'fractional' returned nothing
- **[high]** Prior art on this exact OS: xtrimsystems/claude-pet is a GTK4 + PyGObject + gtk4-layer-shell Claude Code pet developed on Hyprland/Omarchy.
- one full-output overlay per monitor, namespace claude-pet;
- input region limited to the sprite via set_input_region;
- GestureDrag with implicit grab;
- LD_PRELOAD re-exec;
- about 1% idle CPU;
- documents a GTK 'ghost frame' gotcha.  
  _Evidência:_ github.com/xtrimsystems/claude-pet CLAUDE.md and pet_window.py (MonitorOverlay, park(), _apply_input_region, _attach_input)
- **[high]** Two real Quickshell drag implementations exist:
- dhrruvsharma/shell grows the surface to full screen while held, so the widget moves with plain Qt coordinates without waiting on the compositor, and keeps the surface small otherwise because repaints cost the whole surface;
- Ryoku pins use plain margin-delta dragging.  
  _Evidência:_ github.com/dhrruvsharma/shell quickshell/modules/desktopwidgets/WidgetWindow.qml header comment and code; github.com/Ryoku-dev/ryoku ryoku/shell/quickshell/ryopin/Pin.qml
- **[high]** Host socket permissions: wayland-1 and the Hyprland .socket.sock/.socket2.sock are srwxr-xr-x owned by the user (UID 1000), and /run/user/1000 and its hypr directory are 0700. The container process must run as UID 1000 to connect.  
  _Evidência:_ `ls -la /run/user/1000/wayland-1 /run/user/1000/hypr/*/`; `stat /run/user/1000`
- **[high]** Docker Compose's long-syntax bind option `create_host_path` defaults to true (and short syntax always creates a missing source as a directory). It must be set to false for runtime-dir sockets.  
  _Evidência:_ docs.docker.com/reference/compose-file/services/#volumes
- **[medium]** Image size estimates from Arch closures (installed MiB):
- base: 511;
- base ∪ quickshell: 1090;
- base ∪ quickshell ∪ python: 1164;
- base ∪ PyGObject/gtk4-layer-shell/pycairo: 1220.
The largest single dependency is llvm-libs at 163.7 MiB (via mesa).  
  _Evidência:_ pactree -u -l + pacman -Qi sums on host
- **[medium]** Reference memory on this host: Omarchy's full Quickshell instance (bar, background, notifications, plugins) uses about 316 MB RSS (208 MB anon) with 25 threads. A minimal pet config should be well below that; this is an estimate, not a measurement.  
  _Evidência:_ ps -o rss and /proc/1709085/status
- **[high]** `hyprctl -j layers` reports per-monitor levels with namespace, global logical x/y/w/h and pid, which is usable for test assertions. `grim -g` takes layout coordinates and `-s` sets the image scale factor.  
  _Evidência:_ `hyprctl -j layers` output (omarchy-background at 640,0 1280x800; omarchy-bar parked at y=-26); `grim -h`, `man grim`
- **[high]** Hook-side tools on the host: curl is required by pacman/git (always present); socat is only explicitly installed (removable). jq, python3 and grim are present.  
  _Evidência:_ `command -v ...`; `pacman -Qi socat` (Required By: None, Install Reason: Explicitly installed); `pacman -Qi curl` Required By list
- **[high]** Shijima-Qt (a Qt Widgets Shimeji port) is archived. Its author says Qt was not the right framework because of the hacks it needed, and its wayland-layer-shell branch diverged. This is about cross-platform Qt Widgets, not Quickshell or layer shell.  
  _Evidência:_ github.com/pixelomer/Shijima-Qt README (archived=true)

## Recomendações

- Pick Quickshell 0.3.1 (QML) in an `archlinux:base` image pinned to a dated tag or the Arch Linux Archive, for exact parity with the host's quickshell, Qt 6.11.2 and Mesa 26.2.2. Run it as `quickshell` with `QS_CONFIG_PATH=/app/ui` and `QT_QPA_PLATFORM=wayland`, with UID/GID 1000.
- Keep the steady-state surface small (for example 160×160 logical, even numbers) on WlrLayer.Overlay. If Hyprland should hide the pet under fullscreen windows, use Top instead. Use keyboardFocus None, ExclusionMode.Ignore, `color: "transparent"` and namespace 'claude-pet'.
- Drag with the 'grow-on-hold stage' pattern (dhrruvsharma/shell WidgetWindow.qml):
- on press, set anchors to all edges;
- move the sprite inside with Qt coordinates only once `full` is true;
- widen the `mask` to the whole stage while held;
- on release, shrink back and set the margins to the snapped final position.
Avoid plain margin-delta dragging (fast-drag overshoot) and avoid needing Hyprland's exec-capable request socket for cursorpos.
- Make every pixel-art position and size even in logical px. Use art pixel = 4 logical = 6 device px, `Image { smooth: false }` or `AnimatedSprite { smooth: false; interpolate: false }`, no scale/rotation transforms, and no `layer.enabled` on the sprite.
- Drive idle animation with a `Timer` stepping a clipped sprite-sheet `Image` at 4–8 fps. Reserve AnimatedSprite, NumberAnimation and particles for short celebrations, because AnimatedSprite forces 60 fps redraws while running.
- Use the full monitor only for attention-grabbing celebrations (run across the screen, confetti, flashes). Reuse the same grow-to-stage mechanism with the mask restricted to the pet, then shrink back. With GL, every frame damages the whole surface, so keep these bursts to a few seconds.
- Follow focus by binding `screen` to the ShellScreen whose name matches `Hyprland.focusedMonitor.name`, compared by name as Omarchy's Bar.qml does:
- freeze the binding while dragging;
- add Omarchy's ScreenMoveRemap guard for HDMI hotplug and layout moves;
- optionally park per-screen surfaces off-screen for instant switching instead of remapping.
- Also gate visibility on `Hyprland.focusedWorkspace.hasFullscreen` (configurable). This also restores direct scanout for games and video.
- Ingress: a `SocketServer` on /run/pet/events.sock, bind-mounted from host $XDG_RUNTIME_DIR/claude-pet (created by the systemd unit). Hooks write one JSON line with socat or python3 and always `|| true`. If HTTP is preferred (curl is always present), add a tiny bridge that forwards to the socket.
- Put all event→animation logic in pure JS modules (`logic/PetModel.js`, the Omarchy *Model.js style) and unit-test them with /usr/lib/qt6/bin/qmltestrunner (offscreen) or Node. Keep QML thin.
- Add an `IpcHandler { target: "pet" }` with ping/play/debug for the Docker healthcheck (`qs ipc call pet ping`) and for manual triggering (`docker exec claude-pet qs ipc call pet play celebrate`).
- Container hardening: private tmpfs XDG_RUNTIME_DIR (0700, uid 1000) and HOME; bind only the wayland socket (WAYLAND_DISPLAY as an absolute path) and the hypr directory (at /tmp/hypr, read-only), each with `create_host_path: false`; `read_only: true`, `network_mode: none`, `devices: /dev/dri/renderD128`; json-file logging 10m×5; TZ America/Sao_Paulo.
- Own the lifecycle with a systemd --user unit (PartOf/After graphical-session.target) running `docker compose up --force-recreate` and `down`, instead of `restart: unless-stopped` at boot. This avoids Docker creating /run/user/1000 paths before login and stale HYPRLAND_INSTANCE_SIGNATURE values.
- Run a short spike before committing to a renderer:
- measure RSS and CPU with `docker stats`/`ps` for GL vs QT_QUICK_BACKEND=software, small surface vs permanent stage;
- verify crispness with `grim -s 1.5 -g` plus a palette or 6×6-block check;
- test dragging on an empty workspace and across monitors.
- Keep the GTK4/PyGObject design (per-monitor full-output overlay + set_input_region, as in xtrimsystems/claude-pet) as plan B if the Quickshell spike shows unacceptable cost. Keep Rust/SCTK as a possible later 'lightweight engine' if footprint becomes the priority.

## Riscos

- Qt Quick on GL presents full-surface damage every frame (plain eglSwapBuffers). A large or always-on overlay makes Hyprland recomposite the whole monitor at the animation frame rate, which costs battery and GPU. → **Keep the steady-state surface small and grow to full monitor only transiently. Or use QT_QUICK_BACKEND=software (partial damage; no shaders or particles) and measure.**
- AnimatedSprite and running NumberAnimations force vsync-rate (60 Hz) rendering even for low-fps pixel art, so the idle pet burns CPU and GPU continuously. → **Drive idle frames with a Timer on a clipped sprite-sheet Image. Run continuous animations only during short celebrations. Stop timers when the pet 'sleeps'.**
- Plain margin-delta dragging jitters or overshoots on fast drags, because pointer coordinates are surface-local and margins commit up to one vsync after the GUI thread sets them. → **Use the grow-on-hold full-monitor stage, where the sprite moves inside a stationary surface. Alternatively use Hyprland j/cursorpos absolute coordinates, or commit immediately (Rust).**
- On an empty workspace (no keyboard-focused surface), Hyprland's held-button focus lock may not engage. The drag could then stop when the pointer leaves the sprite's input region. This is inferred from source. → **Widen the mask to the whole stage while held (`mask: Region { item: held ? stageHit : sprite }`), and test explicitly on an empty workspace.**
- focusedmon can fire mid-drag (pointer crossing monitors). If the pet's screen binding follows it, the window is destroyed mid-drag. → **Freeze the screen binding while held. On release, reconcile with the focused monitor, ending the drag or placing the pet at the entry edge.**
- Monitor hotplug or layout changes (HDMI-A-1 is intermittent) leave mapped layer surfaces at stale positions, or close them (zwlr_layer_surface_v1.closed). → **Port Omarchy's ScreenMoveRemap (200 ms settle + 50 ms unmap pulse). Handle `closed`/screen destruction by re-showing on the focused screen. Store positions per monitor name and clamp to the current size.**
- Docker bind mounts of runtime-dir sockets: Compose's create_host_path defaults to true, so a boot-time start (restart: unless-stopped) can create root-owned /run/user/1000/... directories. HYPRLAND_INSTANCE_SIGNATURE also changes every session, so a long-lived container's env goes stale. → **Set `bind: { create_host_path: false }`. Start and stop the container from a systemd --user unit bound to graphical-session.target, recreating it each session. Use restart: "no" (or on-failure inside the session).**
- Security: the mounted Hyprland request socket (.socket.sock) accepts `dispatch exec`, so the container can run arbitrary host commands. The Wayland socket allows screencopy. A shared runtime dir would also expose the session D-Bus bus, pipewire and pulse. → **Use a private runtime dir and bind only the needed sockets. Document the risk. Least-privilege variant: mount only .socket2.sock, parse focusedmon events yourself, and use a compositor-picked output (screen: null) for the initial placement.**
- An Overlay-layer pet stays visible above fullscreen games and video, and blocks direct scanout. → **Hide when Hyprland.focusedWorkspace.hasFullscreen (configurable), or use WlrLayer.Top, which Hyprland fades out under fullscreen. Freshly mapped surfaces start above fullscreen, so keep the gate as well.**
- Blurry pixel art at scale 1.5 if any item offset or size is an odd logical px, or if transforms or layers are used. → **Enforce snap2() on all positions and sizes, use smooth:false and interpolate:false, use 4-logical-px art pixels, and run an automated grim palette/6×6-block check in tests.**
- Quickshell is pre-1.0. APIs can change between minor releases, and Arch's rolling updates can bump it unexpectedly. → **Pin the image to a dated archlinux tag or the Arch Linux Archive snapshot. Keep logic in pure JS. Re-verify the .qmltypes on upgrades.**
- Running Quickshell inside Docker is uncommon. Details are unverified (fonts/fontconfig warnings, AT-SPI D-Bus attempts, Mesa cache dirs, file-watcher behaviour with atomic-rename saves). → **Prototype early. Set HOME and XDG_RUNTIME_DIR to writable tmpfs. Silence a11y with QT_ACCESSIBILITY=0 / NO_AT_BRIDGE=1. Ship a font only if text is needed. Fall back to QS_DISABLE_FILE_WATCHER plus a manual reload if watching misbehaves.**
- Image size of about 1 GB (Qt + Mesa + LLVM) on a 7.4 GiB RAM machine. RSS is not yet measured. → **`pacman -Scc` and trim docs/locales. Measure RSS in the spike. If footprint matters most later, consider the Rust/SCTK engine (about 5–15 MB).**
- GTK plan-B gotchas: LD_PRELOAD/CDLL ordering, set_monitor does a full unrealize/realize, a 'ghost frame' is left when a widget stops drawing, and there is no hot reload. → **Copy the proven solutions in xtrimsystems/claude-pet: re-exec with LD_PRELOAD, per-monitor windows, paint transparent instead of hiding, and set_input_region on every move.**

## Perguntas em aberto

- Should the pet stay visible over fullscreen apps (games, video), or hide? This decides Overlay+gate vs the Top layer.
- When focus changes monitor, should the pet teleport instantly (Hyprland's fade-out/fade-in on remap, about 150 ms rebuild) or play an in-app 'jump/poof' animation? Is the parking pattern needed for instant switching?
- Is it acceptable to mount Hyprland's request socket, which can execute host commands, or should the least-privilege variant be used (event socket only + compositor-picked initial output)?
- Event ingress: UNIX socket written by socat/python3 from hooks (socat is not a guaranteed dependency), or HTTP on 127.0.0.1 via curl, which needs a small bridge process/container?
- Art pipeline: original pixel art authored for this project (e.g., Aseprite sheets + JSON metadata), or compatibility with Shimeji/Shijima packs like xtrimsystems/claude-pet? What base sprite size (32×32 → 128 logical px)?
- Is sound wanted for attention-grabbing? That would require mounting the pipewire/pulse socket.
- Will speech bubbles or text be needed? That decides whether to bake a bitmap font into sprites or ship a TTF and accept text crispness issues at scale 1.5.
- Is a ~1 GB image acceptable, and is RSS around 100 MB acceptable for the pet? The spike will measure the actual numbers.
- Not verified (no state-changing commands allowed): actual Quickshell RSS/CPU in a container, QT_QUICK_BACKEND=software behaviour with Quickshell, whether grim accepts a fractional -s, and the empty-workspace drag edge case.
