# bspwm-cyberquote

Cyberpunk motivational quote overlay for BSPWM — a GTK4 application that
spawns one undecorated, desktop-hint WebView per monitor, each filling its
monitor, and drives a glitch/scanlines/CRT/typewriter quote pipeline through
webkit2gtk-4.1.

## Architecture

```
src/main.rs            ← this crate: GTK4 host, monitor enumeration, windows,
                          WebViews, signal handling, placeholder HTML
  ↕  MARKER:AGENT2_CSS  — Agent 2 injects the cyberpunk CSS into the WebView
  ↕  MARKER:AGENT2_JS   — Agent 2's JS payload for glitch/scanlines/CRT
  ↕  MARKER:AGENT3_QUOTE — Agent 3 injects quote text into #quote-content
  ↕  MARKER:AGENT3_CONFIG — Agent 3 reads CONFIG_PATH for config
config (future)        ← Agent 3 parses this; main.rs only declares the path
```

- **main.rs** owns the GTK application life-cycle.  It declares the config-path
  constant `CONFIG_PATH` (currently `/etc/bspwm-cyberquote/config.toml`) so
  Agent 3 can locate it; no parsing is done by this crate.
- The WebView loads `PLACEHOLDER_HTML`, which contains an empty
  `<div id="quote-content">` placeholder and `<script>` stub.  Agent 2 and
  Agent 3 plug in at the marked locations (see source for `MARKER:` comments).
- The windows are undecorated dialogs (`WINDOW_TYPE_HINT_DIALOG`) with
  `skip_taskbar`/`skip_pager` set, positioned to each monitor's geometry.

## System dependencies (Debian/Ubuntu 24.04+)

```bash
sudo apt-get install -y \
    libgtk-4-dev \
    libwebkit2gtk-4.1-dev \
    pkg-config \
    libglib2.0-dev
```

On Fedora:

```bash
sudo dnf install -y gtk4-devel webkit2gtk4.1-devel glib2-devel pkg-config
```

## Build

```bash
cd /home/magneciareal/bspwm-cyberquote
cargo build --release
```

A plain `cargo check` is enough to validate the skeleton before Agent 2/3
complete their work:

```bash
cargo check
```

## Run

```bash
./target/release/bspwm-cyberquote
```

The app spawns one WebView per monitor.  Use SIGTERM/SIGINT to quit
(`kill <pid>` or Ctrl-C if running in a terminal).

## Integration points

| Marker | Owner | What goes here |
|--------|-------|----------------|
| `MARKER:AGENT2_CSS` | Agent 2 | Cyberpunk CSS: glitch, scanlines, CRT curvature, monospace typewriter style |
| `MARKER:AGENT2_JS`  | Agent 2 | JS effects payload (animations, CSS manipulation, WebGL if desired) |
| `MARKER:AGENT3_QUOTE` | Agent 3 | Quote text injected into `#quote-content` via `webkit_web_view_run_javascript()` or localStorage |
| `MARKER:AGENT3_CONFIG` | Agent 3 | Read `CONFIG_PATH` (`/etc/bspwm-cyberquote/config.toml`) to drive quote selection, source URLs, refresh interval, etc. |

Agent 2 is responsible for the visual layer (CSS + JS).  Agent 3 is responsible
for the content layer (which quotes, when to refresh, how to fetch/rotate them).
Neither is implemented in this skeleton — the WebView is intentionally bare
except for a `#quote-content` DOM hook that both agents target.
