# Docker + Wayland

> Pesquisa automática de 2026-10-02 (somente leitura), em inglês. Pode envelhecer: confira antes de confiar.

## Relatório

# Running the pet (a Wayland GUI client) inside Docker on this host: socket access, lifecycle and hook ingress

Scope: brief items 1–8. Everything here was checked read-only on this machine or against upstream source and docs. Anything marked **(unverified)** or **(inference)** was not confirmed. Nothing was built, run, pulled or modified.

---

## 0. Recommended design

1. **Mount `/run/user` (the parent), not `/run/user/1000` and not individual sockets.** Use target `/host/run/user` with `read_only: true`, `bind.propagation: rslave` and `bind.create_host_path: false`.
   - `/run/user` exists from early boot (tmpfiles `d /run/user 0755 root root -`).
   - `/run` is a shared mount (`shared:12`), and dockerd and containerd run in the host mount namespace (no MountFlags or PrivateMounts).
   - So the per-user tmpfs (`/run/user/1000`, `shared:645`) propagates in and out of the container whenever systemd mounts or unmounts it.
2. **Give the container its own private `XDG_RUNTIME_DIR=/tmp/xdg`** (tmpfs, mode 0700). It holds only symlinks: `wayland-N`, `hypr` and `pulse`, each pointing into `/host/run/user/<uid>/`. There is no `bus` symlink.
3. **Never bake `WAYLAND_DISPLAY` or `HYPRLAND_INSTANCE_SIGNATURE` into compose.** A supervisor discovers them at runtime:
   - It reads `hypr/<HIS>/hyprland.lock`, whose format is `<pid>\n<wayland-socket>\n` (verified here: `1501\nwayland-1\n`).
   - It checks the instance is alive by connecting to `.socket2.sock` and to the Wayland socket.
4. **Process tree:** `init: true` (docker-init, which is tini 0.19.0) → `entrypoint.sh` → `exec` a Python supervisor → a renderer child (GTK4 or Quickshell).
   - The supervisor owns HTTP ingress, discovery and the renderer's lifecycle with backoff.
   - The renderer simply exits when the compositor disappears ("crash-only" design).
5. **Lifecycle:** `restart: unless-stopped` plus that supervisor is enough on this host, because `docker.service` is enabled and `Linger=yes`. A systemd user unit or Omarchy autostart is optional. Either is only needed on stock Omarchy, which enables only `docker.socket`.
6. **Hardening:**
   - Run as `user: ${APP_UID:-1000}:${APP_GID:-1000}`.
   - `cap_drop: [ALL]`, `no-new-privileges`, `read_only: true`, tmpfs `/tmp`.
   - `mem_limit: 256m`, `pids_limit: 128`, `stop_grace_period: 5s`.
   - json-file logging, 10m x 5.
7. **Software rendering by default** (no Mesa, no device):
   - GTK: `GSK_RENDERER=cairo` and `GDK_DISABLE=gl,vulkan`.
   - Qt/Quickshell: `QT_QUICK_BACKEND=software` (unverified for Quickshell).
   - GPU is an opt-in override: `/dev/dri/renderD128` plus Mesa in the image.
8. **Audio:** set `PULSE_SERVER=unix:/tmp/xdg/pulse/native` and use `paplay`. pipewire-pulse checks only the cookie's length, so no cookie mount is needed.
9. **Ingress from Claude Code:**
   - Compose: `ports: ["127.0.0.1:${PET_PORT:-27380}:27380"]`; inside the container the server listens on `0.0.0.0:27380`.
   - Claude Code side: hooks of `"type":"command"` with `"async":true` running `curl … || true`. These are silent and can never block Stop.
   - Ship the hooks as a Claude Code plugin in the same repo. The plugin name must **not** start with `claude-`.
10. **Base image per engine:**
    - **GTK4 + PyGObject:** `alpine:3.24`. Its gtk4.0 4.22.4, gtk4-layer-shell 1.3.0, py3-gobject3 3.56.3 and python3 3.14.8 match the host's versions.
    - **Quickshell:** `archlinux:base-20260927.0.600689` (quickshell 0.3.1, about 1.0 GiB) or `debian:trixie-backports` (0.3.0).
    - **Rust:** a static musl binary in `FROM scratch`.

---

## 1. Host facts verified in addition to the planner's list

### Docker and boot
- **Units:** `docker.service` is enabled, `docker.socket` is enabled, and `containerd.service` is disabled (dockerd pulls it in via `Wants=`).
- **Drop-in:** `/etc/systemd/system/docker.service.d/no-block-boot.conf` sets `DefaultDependencies=no`.
- **Stock Omarchy differs:** `/usr/share/omarchy/install/config/enable-services.sh` enables only `docker.socket`, and `install/config/docker.sh` deliberately does not add the user to the docker group. This user opted in to both.
- **`docker info`:**
  - Engine 29.7.2 (API 1.55), containerd 2.3.5, runc 1.5.1, docker-init 0.19.0.
  - Storage is overlayfs via the containerd snapshotter; cgroup v2 with the systemd driver.
  - Security options are only `seccomp (builtin)` and `cgroupns`. There is no userns-remap, so container uid 1000 is host uid 1000.
  - Live-restore is off, the firewall backend is iptables, and the userland proxy is on.
- **`/etc/docker/daemon.json`** already sets json-file with max-size 10m and max-file 5, `dns` 172.17.0.1 and `bip` 172.17.0.1/16.
- **Firewall:** ufw plus ufw-docker are configured by Omarchy (`install/config/firewall.sh`). Loopback-published ports already work here: your agenda dashboard listens on `127.0.0.1:8050` and is healthy.
- **Mount namespace:** both `docker.service` and `containerd.service` have empty `MountFlags`, `PrivateMounts=no` and `PrivateTmp=no`. They run in the host mount namespace, so mount propagation reaches containers.
- **Compose and buildx:** Compose 5.5.1 (v5 removed the internal builder; builds go through Bake) and buildx 0.37.0.

### Session
- **Linger:** `loginctl show-user 1000` reports `Linger=yes` (`/var/lib/systemd/linger/youruser`). `/run/user/1000` and `user@1000` live from boot to shutdown, so logging out does not destroy the runtime dir on this host.
- **Autologin:** SDDM's helper runs `uwsm start -g -1 -e -D Hyprland hyprland.desktop --autologin`.
- **Greeter:** the SDDM greeter runs its own Hyprland (`/etc/sddm.conf.d/10-wayland.conf`) under the sddm uid. Its runtime dir is another 0700 dir under `/run/user`, unreadable by uid 1000.

### Mounts (`/proc/self/mountinfo`)
- `/run` is tmpfs, `shared:12`.
- `/run/user/1000` is tmpfs (mode 700, uid 1000), `shared:645`.
- `/run/user/1000/doc` is `fuse.portal` (xdg-document-portal), `shared:1074`.

### What `/run/user/1000` exposes
- **Wayland:** `wayland-1` is `srwxr-xr-x`, so only the owner can connect.
- **Hyprland:** `hypr/<HIS>/` is 0700 and contains `.socket.sock` and `.socket2.sock` (both `srwxr-xr-x`), `hyprland.lock` and `hyprland.log`.
- **Audio:** `pulse/` is 0700 with `native` `srw-rw-rw-`; `pipewire-0` and `pipewire-0-manager` are `srw-rw-rw-`.
- **D-Bus and systemd:** `bus` is `srw-rw-rw-`; `systemd/private` is 0700; `systemd/io.systemd.Manager` is 0666.
- **Secrets and session state:** `gnupg/`, `keyring/`, `at-spi/`, `dconf/`.
- **Omarchy shell:** `quickshell/` holds the shell's `qs` IPC registry.
- **uwsm:** the `uwsm-app-daemon-in` and `uwsm-app-daemon-out` FIFOs.
- **Claude Code:** `cc-socks/1713473.sock` belongs to the running `claude --dangerously-skip-permissions` process (PID 1713473).

### Hyprland instance details
- **Signature format:** `<40-hex>_<epoch>_<rand>`. Here it is `efb50993…_1790020208_1687561921`, and the epoch decodes to 2026-09-21 19:50:08 UTC, which is the login time (15:50 EDT). That field is a natural "newest instance" sort key.
- **Path lengths (AF_UNIX limit is 108):** `/host/run/user/1000/hypr/<HIS>/.socket2.sock` is 101 bytes; `/tmp/xdg/hypr/<HIS>/.socket2.sock` is 90.
- **Xwayland** listens on `/tmp/.X11-unix/X0` (and `X0_`).

### Devices and ports
- **DRI:** `/dev/dri/renderD128` is `crw-rw-rw-` root:render (gid 987); `card1` is root:video 0660 with an ACL. The user's groups are 1000, docker (967) and wheel.
- **Listening TCP:** 53, 80, 631, 8050, 41039, 50684, 62872.
- **Ephemeral range:** 32768–60999.
- **27380** is free, absent from `/etc/services`, and below the ephemeral range.

### Time and locale
- Host timezone is **America/New_York** (`/etc/localtime` → `zoneinfo/America/New_York`).
- `LANG=en_US.UTF-8`, and `/usr/lib/locale/C.utf8` exists.
- Your compose convention sets TZ=America/Sao_Paulo, which conflicts with the host (see open questions).

### Reference measurements on this host
- Omarchy's own Quickshell (`quickshell -n -p /usr/share/omarchy/shell`: bar, background and menus) has **RSS ≈ 321 MiB** and 0.3 % average CPU.
- Hyprland's RSS is about 50 MiB.
- A Python + GLib daemon (udiskie) has an RSS of about 18 MiB.

### Claude Code
- `~/.claude/settings.json` has no hooks.
- The only marketplace is `claude-plugins-official`.
- There are no managed settings (`/etc/claude-code` is absent).

### Side observation
Your `netdata` and `central` containers mount `/var/run/docker.sock` read-only. As §2.2 shows, `:ro` does not restrict the Docker API.

---

## 2. Socket access

### 2.1 Mount strategies compared

**A. Short syntax `/run/user/1000:/run/user/1000`**
- *Boot before login:* dockerd can win the race against `user-runtime-dir@1000` (`DefaultDependencies=no`; linger starts it in parallel). Docker then **creates** `/run/user/1000` as root, and systemd tolerates `EEXIST` and mounts its tmpfs on top. The container keeps the empty underlying directory for good (default propagation is rprivate), so the pet never connects.
- *Re-login without linger:* the container holds the old, emptied tmpfs.
- *Compositor restart:* OK (same tmpfs).
- *Exposure:* the whole runtime dir.

**B. The same mount in long syntax with `create_host_path: false`**
- *Boot before login:* the container start **fails**. dockerd does not retry boot-time start failures (moby `daemon.go` restore loop: `logger.WithError(err).Error("failed to start container")`, no retry).
- *Re-login without linger:* same as A.
- *Compositor restart:* OK.
- *Exposure:* the whole runtime dir.

**C. File binds of `wayland-1`, the hypr sockets and `pulse/native`**
- *Boot before login:* the same create-or-fail problem as A and B.
- *Re-login without linger:* stale.
- *Compositor restart:* **stale.** A bind pins the inode, so a re-created `wayland-1` is a new inode and the container's copy refuses connections.
- *Exposure:* minimal.

**D. Recommended: `/run/user` → `/host/run/user`, `ro`, `rslave`**
- *Boot before login:* fine. `/run/user` always exists, and the per-user tmpfs propagates in when it is mounted.
- *Re-login without linger:* fine. systemd's `rm_rf` + `umount2(MNT_DETACH)` and the next mount both propagate.
- *Compositor restart:* fine. Symlinks are resolved by path at `connect()` time.
- *Exposure:* the whole runtime dir. Other uids' dirs are 0700 and unreadable.

Supporting details:
- **Compose spec:** the long-syntax `bind.create_host_path` "Defaults to `true`", so set `false` explicitly. Short syntax always creates the source directory.
- **Shared-or-slave requirement:** for rslave, Docker requires the source to sit on a shared or slave mount (moby `ensureSharedOrSlave`: "path %s is mounted on %s but it is not a shared or slave mount"). `/run` is `shared:12` here.
- **Keep recursion on:** leave `bind-recursive` at its default, `enabled`. The user tmpfs is a submount; with `disabled` you would see an empty directory.
- **`:ro` is defense-in-depth only:**
  - Docker 29 turns `:ro` binds into `rro` (recursive read-only via `mount_setattr`, kernel ≥ 5.12; moby `oci_linux.go` appends `"rro"` when supported).
  - **(inference)** Mounts that propagate in *after* the container starts, such as a re-created `/run/user/1000` on a host without linger, keep the master's read-write flag.
  - Kubernetes forbids recursive read-only together with non-private propagation (KEP-3857).
  - The private `XDG_RUNTIME_DIR` is the real guard against writes.
- **Path length:** keep the mount target short (`/host/run/user`) because of the 108-byte AF_UNIX limit.

### 2.2 Does `connect()` work through a read-only bind? Yes.

Linux `unix_find_bsd()` (`net/unix/af_unix.c`) does `kern_path(sun_path, LOOKUP_FOLLOW, &path)` and then `path_permission(&path, MAY_WRITE)`. That reaches `sb_permission()` (`fs/namei.c`) through `inode_permission()`:

```c
if (sb_rdonly(sb) && (S_ISREG(mode) || S_ISDIR(mode) || S_ISLNK(mode))) return -EROFS;
```

- Sockets are exempt from that read-only check.
- The per-mount read-only flag (a bind's `ro`) is never consulted on this path.
- Symlinks are followed (`LOOKUP_FOLLOW`).
- What matters is the socket's mode bits and the caller's uid, which is why the container must run as 1000.

### 2.3 Why a private `XDG_RUNTIME_DIR` (verified reasons)

- **D-Bus.** GLib auto-connects to `$XDG_RUNTIME_DIR/bus` when `DBUS_SESSION_BUS_ADDRESS` is unset and the path is a socket owned by the euid (`gio/gdbusaddress.c`, `get_session_address_xdg`). If the host dir were `XDG_RUNTIME_DIR`, every GTK process in the container would silently join the host session bus. From there, systemd --user can run arbitrary commands.
- **Quickshell registry.** Quickshell writes its instance registry to `$XDG_RUNTIME_DIR/quickshell/{by-id,by-pid,…}`. The host directory already holds Omarchy's shell registry.
- **Quickshell's Hyprland IPC.** The binary contains `HYPRLAND_INSTANCE_SIGNATURE`, `XDG_RUNTIME_DIR`, `/.socket.sock`, `/.socket2.sock` and the `/tmp/hypr/` fallback. So a `hypr` symlink is all it needs.
- **The symlink set is an allow-list.** libwayland, libpulse, PipeWire, Quickshell and hyprctl all derive their socket paths from `XDG_RUNTIME_DIR`.
- **Survives re-creation.** Symlinks follow new sockets that reuse the same path, unlike file binds.
- **Permissions.** Qt expects `XDG_RUNTIME_DIR` to be 0700 and owned by the user, so the entrypoint creates it with `chmod 0700`.

### 2.4 Discovery rules (implemented in the supervisor, §11)

- Scan only `/host/run/user/$(id -u)/hypr/*/` and sort by the epoch field of the signature, newest first.
- Read `hyprland.lock` to get the Wayland socket name.
- **Probe `.socket2.sock`, never hold `.socket.sock` open.** The Hyprland wiki IPC page says: "Hyprland evaluates connections to this socket completely synchronously, which means that any unclosed connections *will cause Hyprland to freeze* until the five-second timeout is reached."
- For commands such as `j/monitors`: connect, write, `shutdown(SHUT_WR)`, read, close.
- The PID in the lock file is useless inside the container's PID namespace, so liveness comes from `connect()`. Stale signature dirs left by a crash refuse the connection and are skipped.
- Allow `PET_HYPR_SIG` and `PET_WAYLAND_DISPLAY` overrides for debugging.
- Useful socket2 events (wiki):
  - `focusedmon` / `focusedmonv2>>MONNAME,WORKSPACEID` (active monitor changed)
  - `monitoradded` / `monitoraddedv2`
  - `monitorremoved` / `monitorremovedv2`
  - `fullscreen>>0/1`

---

## 3. Identity, GPU or software rendering, CPU, fonts, locale and timezone

### Identity
- Use the `APP_UID`/`APP_GID` build args (`useradd`/`adduser -u`) plus `user: "${APP_UID:-1000}:${APP_GID:-1000}"`.
- `$UID` is a bash shell variable, not an environment variable, so compose cannot see it. Other users set `APP_UID` in `.env`; here `id -u` is 1000.
- With `cap_drop: ALL`, root inside the container could not traverse `/run/user/1000` either. Running as uid 1000 is the only correct setup.

### Rendering
- **Pin `GSK_RENDERER`.** Since GTK 4.16 the default renderer on Wayland is Vulkan. Without a Vulkan driver GTK falls back from Vulkan to GL to cairo, with warnings.
- **Software path:** `GSK_RENDERER=cairo`, `GDK_DISABLE=gl,vulkan` (documented values) and `GDK_BACKEND=wayland`. Buffers go through `wl_shm`/memfd, which the default seccomp profile allows.
- **Qt/Quickshell:** `QT_QPA_PLATFORM=wayland`, `QT_QUICK_BACKEND=software` **(unverified with Quickshell; test it)**.
- **Image size:** Alpine and Debian then need no Mesa or LLVM. On Arch, gtk4 and quickshell pull in `mesa` and `llvm-libs` (163.7 MiB) regardless.
- **GPU override:** `devices: ["/dev/dri/renderD128:/dev/dri/renderD128"]`. The node is 0666, so no `group_add` is needed.
  - Add Mesa to the image: Alpine `mesa-dri-gallium mesa-egl`, Debian `libgl1-mesa-dri libegl1`.
  - Set `MESA_SHADER_CACHE_DIR=/tmp/cache/mesa`.
  - **Keep it opt-in.** A missing device aborts the container start (moby `devices_linux.go`: "error gathering device information while adding custom device %q"), and boot-time start errors are never retried.
  - A more robust variant, **unverified** here: bind `/dev/dri` and add `device_cgroup_rules: ["c 226:* rmw"]`.
- **Pixel-art crispness at scale 1.5** depends on the renderer and on fractional-scale handling. GTK's NEWS mentions cairo fractional-scale work and device-pixel snapping, but sharpness is **unverified**. Test both cairo and ngl.

### CPU and battery (estimates, measure with `docker stats`)
- A 96 px sprite at scale 1.5 is about 144 x 144 x 4 bytes ≈ 83 KB per frame, so blitting is negligible.
- Python + GTK at 8–12 fps is roughly 0.3–1.5 % of one core (unverified). Quickshell is similar; Omarchy's bar averages 0.3 %. Rust is below 0.5 %.
- **The real cost is idle power.** Every commit makes Hyprland recomposite and keeps the eDP panel out of panel self-refresh and the package out of deep C-states (inference).
- Recommendations:
  - Idle animation at 4 fps or less, with long pauses; stop drawing entirely after N minutes of inactivity.
  - Attention bursts at 12–24 fps for no more than about 5 s.
  - Only redraw on frame callbacks when something actually changed.

### Fonts and locale
- GTK needs at least one font: Alpine `font-dejavu` (main). Run `fc-cache -f` at build time because the rootfs is read-only at runtime; point `XDG_CACHE_HOME` at `/tmp/cache`.
- For a real pixel look, render text from a bitmap font atlas shipped in the repo, covering ã õ ç á à â é ê í ó ô ú.
- Set `LANG=C.UTF-8` (musl is UTF-8 by default; glibc ≥ 2.35 has C.UTF-8 built in) and `PYTHONUTF8=1`.

### Timezone
- Bind `/etc/localtime:/etc/localtime:ro`; your netdata container already does this here. musl and glibc both read the TZif file, so `tzdata` is not needed and the pet follows the host clock.
- **Never write `TZ: ${TZ:-}`.** An empty `TZ` means UTC.

---

## 4. Audio

- **No cookie needed.** pipewire-pulse (`module-protocol-pulse/pulse-server.c`, `do_command_auth`) only rejects `len != NATIVE_COOKIE_LENGTH` and then sets `client->authenticated = true`.
  - libpulse (`src/pulse/context.c`) logs "No cookie loaded. Attempting to connect without." and still sends a 256-byte cookie buffer.
  - So no cookie file or `PULSE_COOKIE` is required.
- **Socket access:** `pulse/native` is 0666 but sits inside 0700 dirs, so uid 1000 is required (already the case).
- **Environment:** `PULSE_SERVER=unix:/tmp/xdg/pulse/native` (through the symlink). An explicit server also avoids libpulse's autospawn attempts (inference).
- **Packages providing `paplay`:**
  - Alpine 3.24: `pulseaudio-utils` (community, `/usr/bin/paplay`).
  - Debian: `pulseaudio-utils` 17.0.
  - Arch: `libpulse` (verified: ships `paplay`, `pacat`, `pactl`).
- **Native PipeWire alternative:** `pw-play` (Arch `pipewire-audio`; Debian `pipewire-bin` 1.4.2 in trixie, 1.6.9 in forky) with `PIPEWIRE_REMOTE=/tmp/xdg/pipewire-0`. Compatibility with the host's 1.6.8 server is inferred, not tested.
- Use short `.ogg`/`.wav` clips (libsndfile formats). Spawn players asynchronously; tini reaps them.
- **Exposure:** the same socket permits microphone capture (`parec`).

---

## 5. Base image per engine (versions verified)

**GTK4 + gtk4-layer-shell + PyGObject**

| Base | Package versions | Size (uncompressed) |
|---|---|---|
| **alpine:3.24** (3.24.2 base is 3.85 MB compressed) | gtk4.0 4.22.4 (community; ships `Gtk-4.0.typelib`), gtk4-layer-shell 1.3.0 (community; `libgtk4-layer-shell.so.0`, `liblayer-shell-preload.so`, `Gtk4LayerShell-1.0.typelib`; **no unversioned `.so`**), py3-gobject3 3.56.3, python3 3.14.8 | about 150–250 MB (estimate) |
| debian:trixie-20260918-slim (29.8 MB compressed) | libgtk-4-1 4.18.6, gtk4-layer-shell 1.0.4, python3-gi 3.50.0; forky has 4.22.4 / 1.3.0 / 3.58.0 | about 250–400 MB (estimate) |
| archlinux:base-20260927.0.600689 (134 MB compressed) | gtk4 4.22.4, gtk4-layer-shell 1.3.0, python-gobject 3.56.3 (= host) | **≈ 1.16 GiB, computed** (see below) |

- **Arch size calculation:** the `base` closure is 137 packages, about 504 MiB. The GTK closure adds 117 packages, about 658 MiB. It includes `mesa`, `llvm-libs`, `gtk3` (via `xdg-desktop-portal-gtk`), gstreamer and `gst-plugins-bad-libs`.
- **Alpine caveat:** the community repo of a stable branch only gets updates until the next stable release (about 6 months), so bump the base periodically.
- **Loading gtk4-layer-shell from Python** (upstream `linking.md`): it must be loaded before libwayland. Either put `from ctypes import CDLL; CDLL("libgtk4-layer-shell.so.0")` before `import gi`, or set `LD_PRELOAD` for the renderer process only. Use the `.so.0` soname; the unversioned symlink lives in `-dev` packages.

**Quickshell 0.3.x**

| Base | Package versions | Size (uncompressed) |
|---|---|---|
| **archlinux:base-20260927.0.600689** | quickshell 0.3.1-1 (extra), Qt 6.11.2 (= host; Omarchy's shell uses the same version) | **≈ 1.0 GiB, computed**: base plus 69 packages, about 515 MiB, including mesa, llvm-libs, qt6-base and qt6-declarative |
| debian:trixie + trixie-backports | quickshell 0.3.0-1~bpo13+1; forky has 0.3.1 | about 500–700 MB (estimate) |
| alpine | **not packaged** (no edge result) → would need a source build | n/a |

- **RAM estimate:** a minimal pet is probably 80–150 MiB (unverified); the host's full Omarchy shell is 321 MiB.

**Rust (smithay-client-toolkit + pure-Rust wayland-client + wl_shm)**
- Build in `rust:*-alpine` with the musl target, then run `FROM scratch` or `gcr.io/distroless/static`.
- Estimated image 5–15 MB and RSS 5–20 MB.
- The healthcheck must be a subcommand of the binary itself, because `scratch` has no Python.
- Audio would need a pure-Rust Pulse client or a richer base (unverified).

**Pinning**
- Pin `FROM <tag>@sha256:<digest>`. Dated tags exist: `archlinux:base-YYYYMMDD.0.N`, `debian:trixie-YYYYMMDD-slim`, `alpine:3.24.2`.
- Arch is fully reproducible via an Arch Linux Archive mirror, e.g. `Server = https://archive.archlinux.org/repos/2026/09/28/$repo/os/$arch`.
- Debian can use snapshot.debian.org.
- Alpine cannot pin apk versions long-term; old versions are dropped. Pin the branch and the digest only.

**Memory limit:** start with `mem_limit: 256m` for GTK and 512m for Quickshell, then adjust from `docker stats`.

---

## 6. Lifecycle

### (a) Recommended: `restart: unless-stopped` plus a supervisor that waits for the compositor and reconnects

| Event | What happens |
|---|---|
| Boot | dockerd starts the container; the supervisor waits, polling every 2 s (about 15 MiB RSS). With linger, `/run/user/1000` appears at boot; `hypr/<HIS>` appears only after login. |
| Login | A new signature dir and `wayland-1` appear; the pet is up within about 2 s. |
| Logout or Hyprland crash | The renderer gets EOF and exits. The supervisor goes back to waiting and skips the stale signature dir. Without linger, the tmpfs unmount and remount propagate through rslave. |
| Suspend/resume | Connections survive. Monitors may be re-added, which the renderer handles via `wl_output` and `monitoradded`/`removed`; if it crashes, the supervisor restarts it. The lock screen (ext-session-lock) covers overlay layers. |
| Monitor hotplug | Handled inside the renderer: recreate the layer surface on the focused output. The container is unaffected. |
| `docker compose stop` | The container stays stopped across reboots. This is the "turn the pet off" switch. |
| `docker compose down` | The container is removed; the named volume is kept. |
| Daemon restart | live-restore is off, so the container stops and is restarted by policy. |

Docker restart semantics (moby `restartmanager.go`):
- Backoff starts at 100 ms, doubles, and is capped at 1 minute.
- It resets if the container ran for at least 10 s.
- `unless-stopped` with a manual stop is never restarted.
- Docker never restarts unhealthy containers; healthchecks are informational only.
- So the supervisor must self-heal and should never exit in normal operation.

### (b) systemd user unit bound to the session (optional)
Omarchy's own units (`omarchy-fcitx5.service`, `omarchy-crash-watch.service`) use this pattern. It is useful on stock Omarchy, where only `docker.socket` is enabled; the docker CLI socket-activates the daemon.

```ini
# ~/.config/systemd/user/claude-pet.service
[Unit]
Description=Bichinho do Claude Code (docker compose)
After=graphical-session.target
PartOf=graphical-session.target
ConditionEnvironment=WAYLAND_DISPLAY
[Service]
Type=oneshot
RemainAfterExit=yes
WorkingDirectory=%h/Documents/claude-pet
ExecStart=/usr/bin/docker compose up -d --wait --wait-timeout 180
ExecStop=/usr/bin/docker compose stop
TimeoutStartSec=600
[Install]
WantedBy=graphical-session.target
```

- Pros: precise session lifecycle.
- Cons: an extra install step, it is tied to the clone path, and the container is "manually stopped" at every logout.

### (c) Omarchy autostart (optional)
In `~/.config/hypr/autostart.lua` (currently empty apart from comments):

```lua
o.launch_on_start("docker compose --project-directory ~/Documents/claude-pet up -d")
```

`o.launch_on_start` (in `/usr/share/omarchy/default/hypr/helpers.lua`) runs `uwsm-app -- <cmd>` on `hyprland.start`. It is a simple "kick" for hosts without `docker.service`, and it does nothing at logout.

**Recommendation:** use (a) only on this host. Mention (b) and (c) in the README for docker.socket-only machines, or for anyone who wants no container outside the session.

---

## 7. Hook ingress (Claude Code on the host → container)

### Transport options
- **Recommended: publish on loopback only**, `ports: "127.0.0.1:${PET_PORT:-27380}:27380"`.
  - Inside the container the server must listen on `0.0.0.0:27380`, because docker-proxy/DNAT arrives on the bridge interface.
  - Proven on this host by the agenda dashboard (`127.0.0.1:8050`).
  - Docker 28+ blocks direct routing to container ports from other hosts, and ufw-docker filters only forwarded traffic.
  - Latency is sub-millisecond.
- **`network_mode: host` is not recommended.** No proxy is needed, but the container joins the host network namespace, including **abstract** Unix sockets (e.g., X11 or D-Bus abstract names), and port clashes with host services become direct.
- **A Unix socket in a bind-mounted host dir** gives the best isolation (it allows `network_mode: none`).
  - It needs a read-write host dir that exists before the container starts and is owned by 1000; Docker would otherwise create it root-owned.
  - Hooks would have to use `curl --unix-socket <absolute repo path>`, and Claude Code's `http` hooks cannot use Unix sockets.
  - Possible later; not for v1.

### Port
**27380** ("2" + P-E-T on a phone keypad + "0"):
- free on this host and absent from `/etc/services`;
- below the ephemeral range (32768–60999), so it never collides with outgoing connections;
- overridable with `PET_PORT`.

### Endpoint contract (supervisor)
- `POST /v1/event`:
  - Requires `Content-Type: application/json` and `Host` ∈ {`127.0.0.1:27380`, `localhost:27380`}, which defeats DNS rebinding.
  - Optionally require an `X-Pet` header; browsers cannot send it cross-origin without a preflight.
  - Cap the body at 256 KB to 1 MB and truncate `last_assistant_message`.
  - Reply **204 with no body**. For HTTP hooks, Claude Code treats "2xx with empty body" as success; a JSON body would be parsed for decisions.
- `GET /healthz` returns 200 with `{"ok":true,"display":"connected|waiting","instance":"<HIS>","restarts":N}`.
  - Keep it 200 while waiting for a login, because health is informational only.

### What Claude Code sends (verified in the hooks docs)
- **Stop** stdin JSON has `session_id`, `transcript_path`, `cwd`, `hook_event_name`, `last_assistant_message` and `stop_hook_active`.
- **Notification** adds `notification_type` and `message`. Matchers include `permission_prompt`, `idle_prompt`, `agent_needs_input` and `agent_completed`.
- Other relevant events: `StopFailure`, `UserPromptSubmit` (useful to measure turn length so short replies are not celebrated like long builds), `SubagentStop`, `SessionStart`.
- **Stop fires at the end of every turn.**

### Recommended hook: command + async, silent, never blocks
Exit code 2 or `{"decision":"block"}` from a Stop hook prevents Claude from stopping, so this command never prints and always exits 0. HTTP hooks treat "connection failure" as a non-blocking error, and the transcript then shows a "<hook> hook error" notice each time the pet is down. Only command hooks support `async`.

```json
{
  "hooks": {
    "Stop": [
      { "hooks": [ { "type": "command", "async": true,
        "command": "curl -fsS -m 2 -o /dev/null -H 'Content-Type: application/json' -H 'X-Pet: 1' --data-binary @- http://127.0.0.1:27380/v1/event >/dev/null 2>&1 || true" } ] }
    ],
    "Notification": [
      { "matcher": "permission_prompt|idle_prompt",
        "hooks": [ { "type": "command", "async": true,
        "command": "curl -fsS -m 2 -o /dev/null -H 'Content-Type: application/json' -H 'X-Pet: 1' --data-binary @- http://127.0.0.1:27380/v1/event >/dev/null 2>&1 || true" } ] }
    ]
  }
}
```

Use `127.0.0.1`, not `localhost`. `localhost` may resolve to `::1` first, which is not published.

### HTTP hook alternative (noisier when the pet is down; cannot be async)

```json
{ "type": "http", "url": "http://127.0.0.1:27380/v1/event", "timeout": 2, "headers": { "X-Pet": "1" } }
```

If `allowedHttpHookUrls` is ever defined at any settings level, the URL must be listed in it.

### Packaging as a plugin in the same repo (no manual `settings.json` edits)
**Layout:**
- `.claude-plugin/marketplace.json`, with plugins `[{ "name": "bichinho", "source": "./plugin", "description": … }]`.
- `plugin/.claude-plugin/plugin.json`, with `name` plus `userConfig.port`: `{"type":"number","title":"Porta","description":"Porta do bichinho em 127.0.0.1","default":27380,"min":1024,"max":65535}`.
- `plugin/hooks/hooks.json`, in the format with a top-level `"hooks"` wrapper (same shape as the official plugins on disk).
- `plugin/scripts/notify.sh`, which reads `$CLAUDE_PLUGIN_OPTION_PORT`.

**Install:**

```
claude plugin marketplace add butkeraites/<repo>
claude plugin install bichinho@<marketplace-name>
```

`/plugin …` inside a session does the same.

**Naming rule** (`claude plugin validate`):
- A plugin name starting with `claude-`, `anthropic-` or `cc-plugin-` is an **error** ("reserved").
- `claude` as a whole word elsewhere in the name is a warning.
- So the repo can be called `claude-pet`, but the plugin should be called something like `bichinho` or `pixel-pet`.

**Hook commands** in `hooks.json`: use `"command": "\"${CLAUDE_PLUGIN_ROOT}\"/scripts/notify.sh", "async": true`.

```bash
#!/usr/bin/env bash
# Repassa o JSON do hook (stdin) ao bichinho. Nunca imprime nada e sempre sai 0:
# Stop hook com exit 2 ou {"decision":"block"} impediria o Claude de parar.
porta="${CLAUDE_PLUGIN_OPTION_PORT:-${PET_PORT:-27380}}"
curl -fsS -m 2 -o /dev/null -H 'Content-Type: application/json' -H 'X-Pet: 1' \
     --data-binary @- "http://127.0.0.1:${porta}/v1/event" >/dev/null 2>&1 || true
exit 0
```

### Latency
- Bash plus curl is about 10–30 ms, and the hook is async, so Claude Code is never blocked.
- Proxy to supervisor to renderer is a few milliseconds; the reaction appears on the next animation frame, under 100 ms.

---

## 8. Developer loop

- **Do not name the dev file `compose.override.yml` or `docker-compose.override.yml`.** Compose merges those automatically, so `git clone && docker compose up -d` would pick up dev settings.
  - Use `docker-compose.dev.yml` and run `docker compose -f docker-compose.yml -f docker-compose.dev.yml up --build`.
  - Or put `COMPOSE_FILE=docker-compose.yml:docker-compose.dev.yml` in an untracked `.env`.
- **The dev override sets `restart: "no"`.** Otherwise the dev container, with its bind mount of the working tree, would come back at boot.
- **Hot reload:**
  - Bind the source read-only; inotify works through binds.
  - Python: the supervisor's `PET_RELOAD=1` mode polls mtimes and restarts the renderer in under 1 s.
  - Quickshell reloads QML by itself (`QS_DISABLE_FILE_WATCHER` would turn that off).
  - Python + GTK needs no pip dependencies at all (stdlib plus distro packages), which avoids PEP 668 and supply-chain issues.
- **`docker compose watch`:**
  - Spec actions are `rebuild`, `restart` (Compose 2.32+), `sync`, `sync+restart` and `sync+exec` (2.32+); plus `include` (2.34) and `initial_sync` (2.39.4).
  - `sync*` needs a writable target, `stat`/`mkdir`/`rmdir` in the image, and a target path the container user can write. A `read_only` rootfs blocks it.
  - So use the bind mount for code and `watch` with `action: rebuild` only for `Dockerfile` and `docker/`.
- **Logs:** `docker compose logs -f --tail=100 pet`.
- **Protocol tracing:** `WAYLAND_DEBUG=1` in the dev override.
- **Zero-edit start:** every interpolation has a default (`${APP_UID:-1000}`, `${PET_PORT:-27380}`, `${PET_SOUND:-1}`), and `image: <name>:local` plus `build:` means the first `up` builds the image.
  - The spec says Compose tries to pull first when `pull_policy` is unset, then builds; that is harmless for `library/<name>:local`, and your agenda project relies on the same pattern.
  - After `git pull`, run `docker compose up -d --build`.

---

## 9. Pitfalls checklist

- **Signals:** `init: true` gives tini (docker-init 0.19.0) as PID 1, which forwards SIGTERM and reaps zombies such as `paplay`.
  - The entrypoint must `exec` the supervisor.
  - The supervisor traps SIGTERM, sends SIGTERM to the renderer, waits up to 2 s, then exits 0.
  - In GTK, handle SIGTERM with `GLib.unix_signal_add`; Python signal handlers do not run while the C main loop blocks.
  - `stop_grace_period: 5s`.
- **Logging:** json-file with 10m x 5 (the daemon default already matches; keep it in compose for portability). Log state transitions only, never per-frame output.
- **Seccomp:** the default profile is fine (memfd, DRM ioctls). Never use `unconfined` or `privileged`.
- **Healthcheck:** use Python urllib, as in your agenda project. It is informational only, so keep it 200 while waiting for a login. Interval 60s, with `start_interval: 2s` (Engine ≥ 25).
- **Timezone:** see §3.
- **`$UID` is not exported**, so use `APP_UID`.
- **Never `docker.sock`**, not even `:ro`.
- **Never `network_mode: host`.**
- **Wayland and audio:** `PULSE_SERVER` set explicitly (no autospawn). `GDK_DEBUG=no-portals`, `GTK_A11Y=none` and `GSETTINGS_BACKEND=memory` remove every reason for GTK to touch D-Bus, dconf or at-spi.
- **AF_UNIX path limit** (108 bytes): keep paths short.
- **Never leave a `.socket.sock` connection open**; it freezes Hyprland.
- **Ports:** keep 27380 below the ephemeral range.
- **Optional Hyprland layer rule:** Hyprland's layer animation fires whenever the pet's surface is recreated on another monitor. To disable it, add `hl.layer_rule({ match = { namespace = "<pet-namespace>" }, no_anim = true })` to `looknfeel.lua`. This is the same syntax Omarchy uses for `omarchy-bar`.

---

## 10. Security posture (honest assessment)

**The container is packaging, not a sandbox.** Anything that can use `wayland-1` plus `hypr/<HIS>/.socket.sock` already has code execution as the user:
- `hyprctl dispatch exec` runs commands on the host.
- Hyprland's permission system is **off by default** (`enforce_permissions = false`), and even when enabled the `keyboard` permission (new keyboards, including virtual ones) defaults to ALLOW.
- The Hyprland wiki says IPC socket protection "Hyprland itself cannot enforce".

Mounting `/run/user` additionally exposes:
- the D-Bus session bus, which reaches systemd --user, keyring secrets and portals;
- `systemd/private`;
- gpg-agent;
- PipeWire `pipewire-0-manager` and pulse (microphone and screencast streams);
- the uwsm app FIFOs;
- Omarchy's Quickshell IPC registry;
- `at-spi`;
- the Claude Code session socket `cc-socks/<pid>.sock` (its protocol is unverified);
- Hyprland's log, which contains window titles.

**What still matters:**
- non-root, `cap_drop ALL`, `no-new-privileges`, read-only rootfs;
- a private `XDG_RUNTIME_DIR` so nothing auto-connects to `bus`;
- a minimal image from distro packages only (no pip or npm), pinned by digest;
- no docker.sock and no host network;
- ingress on loopback only, with Host and Content-Type checks.

**Narrower variant (mode B), not recommended for v1:**
- Per-file binds of `wayland-1`, `hypr/<HIS>` and `pulse` combined with lifecycle (b), recreating the container at every login, and `restart: "no"`.
- It removes the D-Bus, gpg, keyring, systemd and Claude Code socket exposure.
- It still leaves Wayland and Hyprland code-execution power, so the gain is marginal unless the pet drops `.socket.sock` (events only) and the user enables Hyprland permissions.
- Wayland security-context-v1 is not an option: compositors typically deny layer-shell to sandboxed clients (inference).

---

## 11. Reference files (drop-in drafts; comments in Portuguese per your convention)

### `docker-compose.yml`

```yaml
name: claude-pet

# Uma imagem, um build (convenção image: <nome>:local).
x-imagem: &imagem
  image: claude-pet:local
  build:
    context: .
    args:
      APP_UID: ${APP_UID:-1000}
      APP_GID: ${APP_GID:-1000}

# Sem isto o json-file cresce sem limite.
x-log: &log
  logging:
    driver: json-file
    options: {max-size: "10m", max-file: "5"}

services:
  pet:
    <<: [*imagem, *log]
    restart: unless-stopped      # o supervisor espera o Hyprland: funciona desde o boot, antes do login
    init: true                   # docker-init (tini) como PID 1: repassa SIGTERM e colhe zumbis (paplay)
    # TEM que ser o dono de /run/user/<uid> (0700) e do wayland-1 (0755): só ele consegue connect().
    user: "${APP_UID:-1000}:${APP_GID:-1000}"
    read_only: true
    cap_drop: [ALL]
    security_opt: ["no-new-privileges:true"]
    pids_limit: 128
    mem_limit: 256m              # Quickshell: 512m
    cpus: 1.0
    stop_grace_period: 5s
    ports:
      # Só loopback. Dentro do container o servidor escuta 0.0.0.0 (o docker-proxy chega pela bridge).
      - "127.0.0.1:${PET_PORT:-27380}:27380"
    environment:
      PET_HTTP_LISTEN: "0.0.0.0:27380"
      PET_HOST_RUNTIME: /host/run/user
      PET_SOUND: ${PET_SOUND:-1}
    tmpfs:
      - /tmp:size=64m,mode=1777  # XDG_RUNTIME_DIR privado (/tmp/xdg) e caches
    volumes:
      # /run/user e NÃO /run/user/1000: existe desde o boot (tmpfiles), então o Docker nunca cria
      # nada no host; rslave faz o tmpfs do usuário aparecer/sumir aqui quando o systemd monta/desmonta.
      # connect() em socket funciona mesmo com bind read-only (checagem é nos bits do socket).
      - type: bind
        source: /run/user
        target: /host/run/user
        read_only: true
        bind:
          propagation: rslave
          create_host_path: false
      # Relógio do host (hoje America/New_York); dispensa tzdata na imagem.
      - type: bind
        source: /etc/localtime
        target: /etc/localtime
        read_only: true
        bind:
          create_host_path: false
      - pet_estado:/var/lib/pet
    healthcheck:
      # Saúde = ingress HTTP vivo. "Esperando login" também é saudável (Docker não reinicia unhealthy).
      test: ["CMD", "python3", "-c",
             "import sys,urllib.request; sys.exit(0 if urllib.request.urlopen('http://127.0.0.1:27380/healthz', timeout=2).status == 200 else 1)"]
      interval: 60s
      timeout: 5s
      retries: 3
      start_period: 20s
      start_interval: 2s

volumes:
  pet_estado:
```

### `docker-compose.gpu.yml` (opt-in)

```yaml
services:
  pet:
    build:
      args: {COM_GPU: "1"}           # Dockerfile instala mesa-dri-gallium mesa-egl
    devices:
      - /dev/dri/renderD128:/dev/dri/renderD128   # 0666 aqui: sem group_add
    environment:
      GSK_RENDERER: ngl
      GDK_DISABLE: vulkan
      MESA_SHADER_CACHE_DIR: /tmp/cache/mesa
```

### `docker-compose.dev.yml`

```yaml
# docker compose -f docker-compose.yml -f docker-compose.dev.yml up --build
services:
  pet:
    restart: "no"                    # senão o container de dev volta no boot
    environment:
      PET_RELOAD: "1"                # supervisor reinicia o renderer quando o código muda
      PET_LOG_LEVEL: debug
      # WAYLAND_DEBUG: "1"
    volumes:
      - type: bind
        source: ./pet
        target: /opt/pet/pet
        read_only: true
    develop:
      watch:
        - path: ./Dockerfile
          action: rebuild
        - path: ./docker
          action: rebuild
```

### `Dockerfile` (GTK4 / Python on Alpine 3.24)

```dockerfile
# syntax=docker/dockerfile:1
FROM alpine:3.24            # fixar @sha256:<digest> no primeiro build
ARG APP_UID=1000
ARG APP_GID=1000
ARG COM_GPU=0
RUN set -eux; \
    apk add --no-cache python3 py3-gobject3 gtk4.0 gtk4-layer-shell \
        fontconfig font-dejavu pulseaudio-utils; \
    if [ "$COM_GPU" = 1 ]; then apk add --no-cache mesa-dri-gallium mesa-egl; fi; \
    addgroup -g "$APP_GID" pet; \
    adduser -D -H -u "$APP_UID" -G pet -h /var/lib/pet -s /sbin/nologin pet; \
    install -d -o "$APP_UID" -g "$APP_GID" -m 0750 /var/lib/pet
ENV LANG=C.UTF-8 PYTHONUNBUFFERED=1 PYTHONDONTWRITEBYTECODE=1 PYTHONUTF8=1 PYTHONPATH=/opt/pet \
    XDG_RUNTIME_DIR=/tmp/xdg XDG_CACHE_HOME=/tmp/cache XDG_CONFIG_HOME=/tmp/config \
    GDK_BACKEND=wayland GSK_RENDERER=cairo GDK_DISABLE=gl,vulkan \
    GTK_A11Y=none GDK_DEBUG=no-portals GSETTINGS_BACKEND=memory
COPY pet/ /opt/pet/pet/
COPY assets/ /opt/pet/assets/
COPY docker/entrypoint.sh /opt/pet/bin/entrypoint.sh
RUN chmod 0755 /opt/pet/bin/entrypoint.sh && fc-cache -f
USER ${APP_UID}:${APP_GID}
WORKDIR /opt/pet
ENTRYPOINT ["/opt/pet/bin/entrypoint.sh"]
# Renderer: primeira linha útil = from ctypes import CDLL; CDLL("libgtk4-layer-shell.so.0")  (antes de import gi)
```

**Quickshell variant on Arch** (only the differences from the above):
- Base `FROM archlinux:base-20260927.0.600689`, with optional Arch Linux Archive date pinning in the mirrorlist.
- `RUN pacman -Syu --noconfirm --needed quickshell python libpulse ttf-dejavu && pacman -Scc --noconfirm`.
- Users via `groupadd`/`useradd`.
- `ENV QT_QPA_PLATFORM=wayland QT_QUICK_BACKEND=software`.
- `RENDERER=["qs","-p","/opt/pet/shell"]`.
- Event delivery: Quickshell `IpcHandler` + `qs ipc call …`, or a Quickshell.Io socket under `/tmp/xdg` (unverified details).

### `docker/entrypoint.sh`

```sh
#!/bin/sh
# PID 1 é o docker-init (init: true). Só preparamos o runtime dir privado e damos exec:
# sem exec o SIGTERM não chega ao supervisor e o stop leva 10 s.
set -eu
: "${XDG_RUNTIME_DIR:=/tmp/xdg}"
export XDG_RUNTIME_DIR
mkdir -p "$XDG_RUNTIME_DIR" "${XDG_CACHE_HOME:-/tmp/cache}"
chmod 0700 "$XDG_RUNTIME_DIR"          # Qt exige 0700 e dono = usuário
exec python3 -m pet.supervisor
```

### `pet/supervisor.py` (sketch, stdlib only)

```python
import json, os, queue, signal, socket, subprocess, threading, time
from pathlib import Path
UID = os.getuid()
HOST_RT = Path(os.environ.get("PET_HOST_RUNTIME", "/host/run/user")) / str(UID)
RT = Path(os.environ["XDG_RUNTIME_DIR"])
RENDERER = ["python3", "-m", "pet.renderer"]      # Quickshell: ["qs", "-p", "/opt/pet/shell"]
estado = {"display": "waiting", "instance": None, "restarts": 0}
parar = threading.Event()

def conecta(p: Path) -> bool:
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM); s.settimeout(0.5)
    try: s.connect(str(p)); return True
    except OSError: return False
    finally: s.close()

def instancia_viva():
    try: dirs = [d for d in (HOST_RT / "hypr").iterdir() if d.is_dir()]
    except OSError: return None                 # sem login ainda (ou uid errado)
    def epoch(d):                               # HIS = <commit>_<epoch>_<rand>
        try: return int(d.name.split("_")[1])
        except (IndexError, ValueError): return 0
    for d in sorted(dirs, key=epoch, reverse=True):
        try: _pid, wl = (d / "hyprland.lock").read_text().split()[:2]
        except (OSError, ValueError): continue
        # socket2 (eventos), nunca .socket.sock: conexão pendurada nele congela o Hyprland até 5 s
        if conecta(d / ".socket2.sock") and conecta(HOST_RT / wl):
            return d.name, wl
    return None

def religa(nome, alvo):
    tmp = RT / f".{nome}.novo"; tmp.unlink(missing_ok=True)
    tmp.symlink_to(alvo); os.replace(tmp, RT / nome)    # troca atômica

def ciclo():
    espera = 1
    while not parar.is_set():
        inst = instancia_viva()
        if not inst:
            estado.update(display="waiting", instance=None); parar.wait(2); continue
        sig, wl = inst
        for nome in (wl, "hypr", "pulse"):
            religa(nome, HOST_RT / nome)
        env = dict(os.environ, WAYLAND_DISPLAY=wl, HYPRLAND_INSTANCE_SIGNATURE=sig,
                   PULSE_SERVER=f"unix:{RT}/pulse/native")
        t0 = time.monotonic()
        filho = subprocess.Popen(RENDERER, env=env, stdin=subprocess.PIPE, text=True)
        estado.update(display="connected", instance=sig)
        # thread: fila de eventos HTTP -> filho.stdin (1 linha JSON por evento)
        filho.wait()                                  # renderer sai quando o compositor some
        estado["restarts"] += 1
        espera = 1 if time.monotonic() - t0 > 60 else min(espera * 2, 30)
        parar.wait(espera)
# + ThreadingHTTPServer: POST /v1/event -> valida Host/Content-Type, enfileira, 204;
#   GET /healthz -> 200 json(estado); SIGTERM -> parar.set(), filho.terminate(), wait(2), exit 0.
```

Query helper for Hyprland commands: `s.connect(f"{RT}/hypr/{sig}/.socket.sock"); s.sendall(b"j/monitors"); s.shutdown(socket.SHUT_WR); read all; close`.

---

## 12. Verification commands for the implementation phase (NOT executed now)

- `docker compose config` validates the YAML and the merges.
- Check propagation and the read-only flag. Expect an `ro` flag and `master:12` on the `/host/run/user` line, and the socket listed:

  ```
  docker run --rm --user 1000:1000 --mount type=bind,src=/run/user,dst=/host/run/user,ro,bind-propagation=rslave alpine:3.24 sh -c 'grep " /host/run/user" /proc/self/mountinfo; ls -l /host/run/user/1000/wayland-1'
  ```

- Check that `connect()` works through the read-only bind:

  ```
  docker run --rm --user 1000:1000 --mount type=bind,src=/run/user,dst=/host/run/user,ro,bind-propagation=rslave python:3.13-alpine python3 -c "import socket;s=socket.socket(socket.AF_UNIX);s.connect('/host/run/user/1000/wayland-1');print('ok')"
  ```

- Hook round-trip:

  ```
  echo '{"hook_event_name":"Stop","cwd":"/tmp/x","session_id":"t"}' | curl -fsS -m 2 -H 'Content-Type: application/json' --data-binary @- http://127.0.0.1:27380/v1/event
  ```

- Health and resource use: `docker compose ps`, then `docker stats --no-stream`.

---

## Sources
- Claude Code hooks: https://code.claude.com/docs/en/hooks
- Claude Code plugin manifest: https://code.claude.com/docs/en/plugins-reference
- Claude Code marketplaces: https://code.claude.com/docs/en/plugin-marketplaces
- Docker bind mounts: https://docs.docker.com/engine/storage/bind-mounts/
- Compose services reference: https://docs.docker.com/reference/compose-file/services/
- compose-spec `05-services.md`, `build.md`, `develop.md`: https://github.com/compose-spec/compose-spec
- Docker restart policies: https://docs.docker.com/engine/containers/start-containers-automatically/
- moby source: https://github.com/moby/moby (`daemon/internal/restartmanager/restartmanager.go`, `daemon/daemon.go`, `daemon/oci_linux.go`, `daemon/pkg/oci/devices_linux.go`)
- Compose v5.0.0 release notes: https://github.com/docker/compose/releases/tag/v5.0.0
- Linux kernel: https://github.com/torvalds/linux (`net/unix/af_unix.c`, `fs/namei.c`, `include/linux/fs.h`)
- systemd `src/login/user-runtime-dir.c`: https://github.com/systemd/systemd
- GLib `gio/gdbusaddress.c`: https://github.com/GNOME/glib
- PipeWire `pulse-server.c`: https://github.com/PipeWire/pipewire
- PulseAudio `src/pulse/context.c`: https://github.com/pulseaudio/pulseaudio
- Hyprland wiki IPC and permissions pages: https://github.com/hyprwm/hyprland-wiki
- gtk4-layer-shell `linking.md`: https://github.com/wmww/gtk4-layer-shell
- GTK runtime environment variables: https://docs.gtk.org/gtk4/running.html
- GTK 4.16 Vulkan default: https://www.phoronix.com/news/GTK-4.16-Released
- Debian package versions (madison): https://qa.debian.org/madison.php
- Alpine packages: https://pkgs.alpinelinux.org/packages
- Docker Hub tag APIs (archlinux, debian, alpine): https://hub.docker.com/v2/repositories/library/
- Kubernetes KEP-3857: https://www.kubernetes.dev/resources/keps/3857/
- Local files: `/usr/share/doc/uwsm/README.md`, `/usr/share/omarchy/default/hypr/helpers.lua`, `/usr/share/omarchy/install/config/*.sh`, `~/Documents/agenda-presidencial/{docker-compose.yml,Dockerfile}`.

## Fatos-chave

- **[high]** /run is a shared tmpfs (shared:12); /run/user is a plain directory inside it, created at boot by tmpfiles (d /run/user 0755 root root -); /run/user/1000 is a separate tmpfs (shared:645) with a FUSE submount /run/user/1000/doc (fuse.portal).  
  _Evidência:_ /proc/self/mountinfo; findmnt; /usr/lib/tmpfiles.d/systemd.conf line 10
- **[high]** dockerd and containerd run in the host mount namespace (MountFlags empty, PrivateMounts=no, PrivateTmp=no), so rslave propagation from /run reaches containers.  
  _Evidência:_ systemctl show docker.service containerd.service -p MountFlags -p PrivateMounts -p PrivateTmp
- **[high]** Linger=yes for uid 1000, so /run/user/1000 and user@1000 exist from boot to shutdown; logout does not remove the runtime dir on this host.  
  _Evidência:_ loginctl show-user 1000 (Linger=yes); /var/lib/systemd/linger/youruser
- **[high]** docker.service and docker.socket are both enabled (containerd.service disabled; dockerd pulls it in via Wants=), and a drop-in sets DefaultDependencies=no. Stock Omarchy enables only docker.socket.  
  _Evidência:_ systemctl is-enabled docker.service docker.socket containerd.service; /etc/systemd/system/docker.service.d/no-block-boot.conf; /usr/share/omarchy/install/config/enable-services.sh
- **[high]** Docker 29.7.2 (API 1.55), containerd 2.3.5, runc 1.5.1, docker-init 0.19.0 (tini); seccomp builtin profile; cgroupns; no userns-remap; live-restore off; iptables backend; userland proxy on; Compose 5.5.1 (builds delegated to Bake), buildx 0.37.0.  
  _Evidência:_ docker version; docker info; docker compose version; compose v5.0.0 release notes
- **[high]** connect() on a Unix socket works through a read-only bind mount: unix_find_bsd checks path_permission(MAY_WRITE), which reaches sb_permission; that returns EROFS only for regular files, directories and symlinks on a read-only superblock, and never consults the per-mount ro flag. Symlinks are followed (LOOKUP_FOLLOW).  
  _Evidência:_ torvalds/linux net/unix/af_unix.c unix_find_bsd; fs/namei.c sb_permission; include/linux/fs.h path_permission
- **[high]** Short-syntax binds auto-create missing sources as directories (owned by root, since the daemon runs as root). In the current compose spec, long-syntax bind.create_host_path also defaults to true, so it must be set to false explicitly.  
  _Evidência:_ docs.docker.com/engine/storage/bind-mounts; compose-spec 05-services.md; docs.docker.com/reference/compose-file/services
- **[high]** systemd-user-runtime-dir tolerates a pre-existing /run/user/<uid> (EEXIST) and mounts its tmpfs over it. On logout it does rm_rf, then umount2(MNT_DETACH), then removes the directory.  
  _Evidência:_ systemd src/login/user-runtime-dir.c user_mkdir_runtime_path / user_remove_runtime_path
- **[high]** If a container fails to start at daemon boot (missing bind source or device), dockerd logs 'failed to start container' and never retries. Restart backoff starts at 100ms, doubles, is capped at 1 minute, and resets after 10s of uptime.  
  _Evidência:_ moby daemon/daemon.go restore loop; daemon/internal/restartmanager/restartmanager.go
- **[high]** rslave requires the bind source to sit on a shared or slave mount (otherwise Docker errors: 'path %s is mounted on %s but it is not a shared or slave mount'). Read-only binds become rro (recursive read-only) when the kernel supports it.  
  _Evidência:_ moby daemon/oci_linux.go ensureSharedOrSlave and rro logic
- **[medium]** Mounts propagated into an rslave + rro bind after the container starts will likely be writable; Kubernetes forbids recursive read-only together with non-private propagation.  
  _Evidência:_ Inference from mount_setattr semantics; KEP-3857 requirement text
- **[high]** A missing passthrough device makes container creation fail with 'error gathering device information while adding custom device'.  
  _Evidência:_ moby daemon/pkg/oci/devices_linux.go
- **[high]** hyprland.lock contains '<pid>\n<wayland socket>\n' (here '1501\nwayland-1\n'). The instance signature has the form <commit>_<epoch>_<rand>, and the epoch 1790020208 is the login time (2026-09-21 19:50:08 UTC).  
  _Evidência:_ od -c /run/user/1000/hypr/*/hyprland.lock; date -u -d @1790020208; loginctl Timestamp
- **[high]** Hyprland processes .socket.sock connections synchronously, and an unclosed connection freezes Hyprland for up to 5 s. .socket2.sock is the event stream (focusedmonv2, monitoraddedv2/removedv2, fullscreen, ...).  
  _Evidência:_ hyprwm/hyprland-wiki content/ipc/_index.md
- **[high]** Hyprland's permission system is off by default (enforce_permissions = false), the keyboard permission defaults to ALLOW, and the wiki says protecting the IPC sockets is something Hyprland itself cannot enforce.  
  _Evidência:_ hyprwm/hyprland-wiki content/configuring/core/advanced-configuration/permissions.md
- **[high]** GLib automatically connects to $XDG_RUNTIME_DIR/bus when DBUS_SESSION_BUS_ADDRESS is unset and the path is a socket owned by the euid. A private XDG_RUNTIME_DIR prevents silent access to the host session bus.  
  _Evidência:_ GNOME/glib gio/gdbusaddress.c get_session_address_xdg
- **[high]** Quickshell builds its Hyprland socket paths as $XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket.sock and .socket2.sock (falling back to /tmp/hypr), and keeps an instance registry under $XDG_RUNTIME_DIR/quickshell.  
  _Evidência:_ strings /usr/bin/quickshell; ls /run/user/1000/quickshell
- **[high]** /run/user/1000 exposes, among others: wayland-1 (0755), hypr sockets (0755), the D-Bus bus (0666), pipewire-0 and pipewire-0-manager (0666), pulse/native (0666 inside a 0700 dir), systemd/private, gnupg, keyring, at-spi, Omarchy's quickshell registry, the uwsm-app FIFOs, and cc-socks/1713473.sock, which belongs to the running 'claude --dangerously-skip-permissions' process.  
  _Evidência:_ ls -la /run/user/1000 and subdirs; ps -p 1713473
- **[high]** pipewire-pulse accepts any 256-byte cookie (it only checks the length), and libpulse connects without a cookie file. No cookie mount is needed.  
  _Evidência:_ PipeWire module-protocol-pulse/pulse-server.c do_command_auth; pulseaudio src/pulse/context.c
- **[high]** On Arch, paplay ships in libpulse and pw-play ships in pipewire-audio. On Alpine 3.24, paplay is in pulseaudio-utils (community).  
  _Evidência:_ pacman -Ql libpulse; pacman -Qo /usr/bin/pw-play; pkgs.alpinelinux.org contents search
- **[high]** Alpine 3.24 has gtk4.0 4.22.4 (with Gtk-4.0.typelib), gtk4-layer-shell 1.3.0 (only .so.0 and .so.1.3.0, no unversioned .so), py3-gobject3 3.56.3, python3 3.14.8 and font-dejavu. Quickshell is not packaged in Alpine.  
  _Evidência:_ pkgs.alpinelinux.org package and contents pages
- **[high]** Debian trixie has GTK 4.18.6, gtk4-layer-shell 1.0.4, python3-gi 3.50.0, quickshell 0.3.0 in trixie-backports, and pipewire-bin 1.4.2. Forky has GTK 4.22.4, gtk4-layer-shell 1.3.0, python3-gi 3.58.0 and quickshell 0.3.1.  
  _Evidência:_ qa.debian.org madison
- **[high]** Arch package closures (installed size): base is 137 packages (~504 MiB); gtk4 + gtk4-layer-shell + python-gobject adds 117 packages (~658 MiB, including mesa, llvm-libs at 163.7 MiB, gtk3 and gstreamer); quickshell adds 69 packages (~515 MiB, including mesa, llvm-libs and qt6).  
  _Evidência:_ pactree -s -u + expac -S '%m' on host
- **[high]** Pinnable base tags exist: archlinux base-20260927.0.600689 (134 MB compressed), debian trixie-20260918-slim (29.8 MB compressed), alpine 3.24.2 (3.85 MB compressed).  
  _Evidência:_ hub.docker.com v2 tags API
- **[high]** Omarchy's own Quickshell (bar + background) has an RSS of about 321 MiB and averages 0.3% CPU; Hyprland's RSS is about 50 MiB.  
  _Evidência:_ ps -eo rss,pcpu,args on host
- **[high]** /dev/dri/renderD128 is mode 0666 (root:render, gid 987), so no group_add is needed. The GPU is an Intel iGPU.  
  _Evidência:_ ls -la /dev/dri; getent group render
- **[high]** Port 27380 is free, not listed in /etc/services, and below the ephemeral range (32768-60999). 127.0.0.1-published ports already work on this host (agenda dashboard on 127.0.0.1:8050).  
  _Evidência:_ ss -ltn; /proc/sys/net/ipv4/ip_local_port_range; grep /etc/services
- **[high]** Claude Code HTTP hooks POST the event JSON. A 2xx with empty body is success; non-2xx, connection failure and timeout are non-blocking errors that show a '<hook> hook error' notice in the transcript. Only command hooks support async. A Stop hook that exits 2 or returns decision:block prevents Claude from stopping.  
  _Evidência:_ code.claude.com/docs/en/hooks
- **[high]** Stop hook input includes last_assistant_message and stop_hook_active. Notification matchers include permission_prompt, idle_prompt, agent_needs_input and agent_completed. StopFailure and UserPromptSubmit events exist.  
  _Evidência:_ code.claude.com/docs/en/hooks
- **[high]** Plugins put hooks in hooks/hooks.json (with a top-level 'hooks' wrapper), can declare userConfig options exported as CLAUDE_PLUGIN_OPTION_<KEY>, and plugin names starting with claude-, anthropic- or cc-plugin- fail validation as reserved.  
  _Evidência:_ code.claude.com/docs/en/plugins-reference; official plugin hooks.json under ~/.claude/plugins/marketplaces
- **[high]** Compose watch actions are rebuild, restart (2.32+), sync, sync+restart and sync+exec (2.32+); include arrived in 2.34 and initial_sync in 2.39.4. compose.override.yml is merged automatically.  
  _Evidência:_ compose-spec develop.md
- **[high]** Since GTK 4.16 the default renderer on Wayland is Vulkan; GSK_RENDERER (cairo/gl/vulkan), GDK_DISABLE (gl, vulkan, ...), GDK_DEBUG=no-portals and GTK_A11Y=none are documented.  
  _Evidência:_ docs.gtk.org/gtk4/running.html; Phoronix GTK 4.16 release article
- **[high]** gtk4-layer-shell must be loaded before libwayland-client. From Python, either CDLL it before importing gi or use LD_PRELOAD.  
  _Evidência:_ wmww/gtk4-layer-shell linking.md
- **[high]** Host timezone is America/New_York, which conflicts with the user's compose convention of TZ=America/Sao_Paulo; the C.utf8 locale exists on the host.  
  _Evidência:_ ls -la /etc/localtime; timedatectl; ls /usr/lib/locale
- **[high]** Omarchy's o.launch_on_start(cmd) runs 'uwsm-app -- cmd' on hyprland.start. The user's autostart.lua contains only comments, and their user units follow After= + PartOf=graphical-session.target + ConditionEnvironment=WAYLAND_DISPLAY.  
  _Evidência:_ /usr/share/omarchy/default/hypr/helpers.lua; ~/.config/hypr/autostart.lua; systemctl --user cat omarchy-fcitx5.service
- **[high]** The existing netdata and central containers mount /var/run/docker.sock read-only, which does not restrict Docker API access.  
  _Evidência:_ docker inspect netdata/central mounts; kernel connect() analysis

## Recomendações

- Bind /run/user to /host/run/user with read_only: true, bind.propagation: rslave and bind.create_host_path: false. Never bind /run/user/1000 or individual sockets: they break at boot or after a socket is re-created.
- Use a private XDG_RUNTIME_DIR=/tmp/xdg (tmpfs, mode 0700) containing only symlinks wayland-N, hypr and pulse pointing into /host/run/user/<uid>. Do not create a bus symlink. Keep paths short because of the 108-byte AF_UNIX limit.
- Discover the Hyprland signature and WAYLAND_DISPLAY at runtime in a supervisor. Pick the newest signature dir by its epoch field, read hyprland.lock, and probe liveness by connecting to .socket2.sock and the Wayland socket. Never keep .socket.sock open.
- Process model: init: true, then entrypoint.sh, then exec a Python supervisor (HTTP ingress + discovery + renderer restarts with 1s to 30s backoff), then the renderer child, which exits when the compositor dies.
- Lifecycle: restart: unless-stopped only. On this host that covers boot, login, logout, crash and resume. Document an optional systemd user unit (PartOf/After graphical-session.target) or an Omarchy autostart line for machines with only docker.socket enabled.
- Run as ${APP_UID:-1000}:${APP_GID:-1000} with cap_drop ALL, no-new-privileges, read_only rootfs, tmpfs /tmp, mem_limit 256m (512m for Quickshell), pids_limit 128, stop_grace_period 5s and json-file 10m x 5. Never mount docker.sock and never use host networking.
- Default to software rendering (GTK: GSK_RENDERER=cairo, GDK_DISABLE=gl,vulkan, GDK_DEBUG=no-portals, GTK_A11Y=none, GSETTINGS_BACKEND=memory). Put /dev/dri/renderD128 and Mesa in an opt-in docker-compose.gpu.yml, because a missing device aborts the container start and boot-time failures are not retried.
- Audio: set PULSE_SERVER=unix:/tmp/xdg/pulse/native and play sounds with paplay (pulseaudio-utils on Alpine, libpulse on Arch). No cookie is needed.
- Ingress: publish 127.0.0.1:${PET_PORT:-27380}:27380 and have the server listen on 0.0.0.0:27380 in the container. POST /v1/event returns 204; GET /healthz always returns 200 with the display state. Validate the Host and Content-Type headers.
- Claude Code side: command hooks with async: true running a curl ... || true that is silent and always exits 0, on Stop, StopFailure and Notification (permission_prompt|idle_prompt), plus UserPromptSubmit if turn length is used. Prefer this over type http, which shows hook errors whenever the pet is down.
- Ship the hooks as a Claude Code plugin with a marketplace.json in the same repo, using a userConfig port option. The plugin name must not start with 'claude-' (e.g. 'bichinho'); the GitHub repo can still be called claude-pet.
- Base image for GTK4+PyGObject: alpine:3.24 pinned by digest, which matches the host's GTK 4.22.4, gtk4-layer-shell 1.3.0 and PyGObject 3.56.3. Call CDLL('libgtk4-layer-shell.so.0') before importing gi. For Quickshell use archlinux:base-YYYYMMDD pinned with the Arch Linux Archive (about 1.0 GiB). Avoid Arch for GTK (about 1.16 GiB).
- Bind /etc/localtime read-only so the pet follows the host clock (America/New_York), and never set TZ to an empty value (empty means UTC).
- Dev loop: a docker-compose.dev.yml (not compose.override.yml) with restart: 'no', the source bind-mounted read-only, renderer auto-restart via PET_RELOAD=1 (Quickshell reloads QML on its own), and develop.watch with action: rebuild only for Dockerfile and docker/.
- Keep the idle animation at 4 fps or less with long pauses, stop drawing entirely after inactivity, and limit attention bursts to about 5 s at 12-24 fps, to protect battery (PSR and C-states) on the laptop.
- Record the security trade-off in DECISIONS.md: the container is packaging, not a sandbox, because Wayland plus Hyprland IPC already allow code execution as the user.

## Riscos

- The container is not a security boundary. Access to wayland-1 plus hypr/.socket.sock (dispatch exec, virtual keyboard allowed by default) already equals code execution as the user, and mounting /run/user also exposes the D-Bus session bus, systemd/private, gpg-agent, keyring, PipeWire and Pulse capture, the uwsm FIFOs and Claude Code's cc-socks session socket. → **Run non-root with cap_drop ALL, no-new-privileges and a read-only rootfs. Use a private XDG_RUNTIME_DIR without a bus link, a minimal pinned image from distro packages only (no pip or npm), loopback-only ingress, no docker.sock and no host network. Document the trade-off. A later hardening option is per-file binds plus a session-bound systemd unit and an events-only design.**
- Boot race: dockerd (DefaultDependencies=no) can start the container before user-runtime-dir@1000. With a /run/user/1000 bind, the container either gets an empty root-owned dir forever or fails to start and is never retried. → **Bind /run/user (which always exists) with rslave and create_host_path: false, and never bind paths under /run/user/1000.**
- Hyprland freezes for up to 5 s if a client leaves a .socket.sock connection open (for example a liveness probe that never closes). → **Probe liveness on .socket2.sock. For command queries, write, shutdown(SHUT_WR), read and close immediately, with timeouts.**
- A Stop hook that exits 2 or prints decision:block makes Claude keep going. An http hook shows a hook-error notice every turn while the pet is down. → **Use an async command hook running curl -m 2 ... >/dev/null 2>&1 || true; exit 0. The server replies 204 with no body.**
- The GPU passthrough device can be missing at start (other machines, or a boot race), which makes container creation fail with no retry at boot. → **Default to software rendering and make the GPU an opt-in override. A possible alternative is binding /dev/dri plus device_cgroup_rules 'c 226:* rmw' (unverified).**
- rro with rslave: mounts that propagate in after the container starts are probably writable, so :ro alone does not stop writes into the host runtime dir. → **Run as uid 1000 with a private XDG_RUNTIME_DIR so software never writes there by default. Treat :ro as defense-in-depth only.**
- On stock Omarchy only docker.socket is enabled, so the daemon (and the pet) does not start at boot until some docker CLI call. → **README: either sudo systemctl enable docker.service or install the optional user unit or autostart line that runs docker compose up -d (which socket-activates the daemon). Not needed on this host.**
- An always-animating overlay keeps Hyprland recompositing and prevents panel self-refresh and deep C-states, draining the laptop battery. → **Idle animation at 4 fps or less with pauses and sleep after inactivity, short attention bursts only, redraw only on change via frame callbacks. Measure with docker stats and powertop.**
- Large Arch-based images (about 1.0 to 1.16 GiB) slow builds and use disk. Alpine's community repo stops receiving updates after about 6 months. → **Use Alpine 3.24 for GTK and bump the base each release; use Arch only if Quickshell parity is required, pinned via the Arch Linux Archive.**
- A dev override named compose.override.yml would be merged automatically into production runs, and a dev container with restart unless-stopped would come back at boot with a bind mount of the working tree. → **Name it docker-compose.dev.yml and set restart: 'no' in it.**
- Pixel art may blur at fractional scale 1.5 depending on the renderer (cairo vs GL) and its fractional-scale handling. → **Prototype both GSK_RENDERER=cairo and ngl (or Qt software vs GL) and render sprites at integer physical-pixel multiples. Verify crispness on eDP-1 and HDMI-A-1.**
- Browser-originated requests (CSRF or DNS rebinding) could POST to 127.0.0.1:27380 and spoof pet events. → **Require Content-Type application/json and an X-Pet header, accept only Host 127.0.0.1:PORT or localhost:PORT, and cap the body size. Impact is low anyway.**
- Restart storms if the renderer crashes repeatedly, for example on monitor hotplug. → **Supervisor backoff from 1 s to 30 s, reset after 60 s of uptime; the HTTP ingress stays up so the healthcheck stays meaningful.**

## Perguntas em aberto

- Which engine will the planner pick: GTK4/PyGObject (alpine:3.24), Quickshell (Arch, about 1 GiB), or Rust (scratch)? This drives the base image, RAM limit and how events are delivered to the renderer.
- Timezone: follow the host via /etc/localtime (currently America/New_York), or force the user's usual TZ=America/Sao_Paulo convention?
- GPU: software rendering by default with an opt-in GPU override, or always ship Mesa and pass /dev/dri/renderD128 on this laptop?
- Does the pet need Hyprland's command socket (.socket.sock), for example to focus the terminal where Claude finished or to query monitors, or can it work from the event socket (.socket2.sock) plus compositor placement? This matters for the security write-up.
- Hook packaging: install hooks via a plugin and marketplace in the same repo (plugin name must not start with 'claude-'), or document a manual ~/.claude/settings.json snippet? Is an async command hook acceptable instead of type http?
- Should the Stop celebration scale with turn length (time since UserPromptSubmit, or tool-use count) so short replies don't trigger the big 'finished developing' animation?
- Sound on or off by default (PET_SOUND), and should the pet hide while the focused window is fullscreen?
- Untested here: whether Quickshell works with QT_QUICK_BACKEND=software in a container; whether GTK 4.22's cairo renderer keeps pixel art crisp at scale 1.5; whether pw-play 1.4.2 (Debian) works against the host's PipeWire 1.6.8. Validate early in a prototype.
