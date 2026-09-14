# AGENTS.md

Cyberpunk CRT "quote ticker" running as an always-on desktop app under bspwm.
Two sibling projects share most code:

- `/home/magneciareal/bspwm-cyberquote`       — original host (GTK4 + WebKitGTK 6.0, `webkit6` crate)
- `/home/magneciareal/bspwm-cyberquote-native` — native fork (GTK3 + Cairo/Pango, no WebView).

The native fork was created to drop CPU/memory by replacing the WebView with
direct Cairo rendering. It is NOT a git repo; the original is (see Git below).

## GTK4 host facts (original repo, `webkit6` crate)

The original repo migrated from GTK3 + webkit2gtk-4.1 to GTK4 + WebKitGTK 6.0
(crates: `gtk` = `gtk4` 0.11, `webkit6` 0.6, `gdk4-x11` 0.11 `xlib` feature,
`glib` 0.22, gated behind feature `link-gtk`). Things learned the hard way:

- GTK4 removed `gtk_window_move`, `gtk_window_set_position`, `set_type_hint`,
  `GdkWindowTypeHint::Desktop`, `skip_taskbar_hint`/`skip_pager_hint`, and
  `gdk_monitor_is_primary`. The desktop "wallpaper" behavior is restored in
  `main.rs::apply_desktop_placement` via raw Xlib: the window's XID is marked
  `_NET_WM_WINDOW_TYPE_DESKTOP` (EWMH) and `XMoveResizeWindow`'d onto its
  monitor — the same mechanism conky/glava use. On Wayland the fallback is
  `fullscreen_on_monitor`. Xlib symbols come from `gdk4_x11::x11::xlib`.
- GDK4 monitor list is a `GListModel`: `display.monitors()` then
  `.iter::<Monitor>()`. No per-index getter, no `n_monitors`. Primary
  detection = geometry-origin heuristic (monitor containing (0,0)).
- Widget embedding: `window.set_child(Some(&webview))`, loading via
  `webview.load_html(html, Some("file:///cyberquote.html"))`. Desktop placement
  must run at **realize** time (the X11 surface exists then, and the WM reads
  `_NET_WM_WINDOW_TYPE` at the map request) — `connect_map` alone is too late.
  The shim is wired to BOTH `connect_realize` and `connect_map` (re-assert).
- `glib` is re-exported by gtk4 as `gtk::glib`; main.rs uses
  `use gtk::glib as glib;`. `timeout_add` + `ControlFlow` for the shutdown
  poll; SIGTERM/SIGINT handled by raw POSIX handlers writing an AtomicBool.
- GTK4 re-exports gdk as `gtk::gdk`; `display.backend() == Backend::X11` is
  the X11/Wayland switch. `set_skip_taskbar_hint`/`set_skip_pager_hint` still
  exist (deprecated) on `gdk4_x11::X11Surface`.

## Commands

Work in the project dir you are editing (both have their own `Cargo.toml`):

```sh
cargo build --release     # link + release build (slow the first time)
cargo test                # unit + integration + doctests
cargo test --lib          # fast unit-tests only
bash -n scripts/*.sh        # syntax-check shell scripts
```

No linter/formatter config exists. Do not run `cargo fmt` — the template
string blocks in `cyberhtml.rs` are whitespace-sensitive (run only if asked).

## THE SYNCHRONIZATION INVARIANT (most likely gotcha)

Any visual/stylesheet change MUST be applied to **all three** rendering layers,
plus optionally `config.toml`:

1. `cyberquote.html`           — static standalone copy
2. `src/cyberhtml.rs`          — Rust template (`{{`/`}}` escape CSS `{`/`}`)
3. `src/config.rs`             — `build_stylesheet()` emits `:root` CSS vars
                               (also `default_css_vars()` in cyberhtml.rs)

Rule of thumb: a CSS variable declared in `build_stylesheet()` must exist as a
matching default in `cyberquote.html` and `default_css_vars()`. If you edit one,
grep them all:

```sh
grep -rn "the-variable" cyberquote.html src/cyberhtml.rs src/config.rs
```

## Config resolution (spooky bug: "font looks small on other machine")

The app reads `~/.config/bspwm-cyberquote/config.toml` (`CONFIG_PATH` in
`src/main.rs`), NOT the repo `config.toml`. `git pull` does not touch it.
Resolution chain in `main.rs::load_cfg()`:

1. `~/.config/bspwm-cyberquote/config.toml`
2. `./config.toml` (repo dir, cwd)  — added so `git pull && cargo run --release` works fresh
3. `Config::defaults()` — `default_font_size()` is **18px**, which is why a
   missing config looks "smaller" than the committed `font_size = 37`.

On a fresh machine: prefix the newest values in the repo `config.toml`; users
can still override per-machine in `~/.config/bspwm-cyberquote/config.toml`.

## Scanline system (CPU model, tunables, seam rule)

The scanline overlay is a single `repeating-linear-gradient` on a
`height:200vh` layer animated `translateY(0 → 100vh)` over
`--scanline-speed:90s` with `steps(var(--scanline-steps)) infinite`.

- The gradient is rasterized to ONE cached texture. Stripe count is ~free;
  per-frame cost is set by **how often it composites** = `steps / duration`
  (default `2160` over 90s ≈ 24 updates/s). This is the whole CPU story.
- Tunables feeding the CSS from `config.toml` `[accent]`:
  `scanline_opacity`, `scanline_steps`, `scanline_lines` (density),
  `scanline_size_rem` (line thickness). `scanline_lines`/`scanline_size_rem`
  are clamped in `build_stylesheet()`; gap is auto-derived as
  `calc((100vh / lines) - size_rem)`.
- **Seam rule:** line+gapped period must divide `100vh` evenly or the
  translating sweep shows a wrap seam. Keep gap derived from `100vh / N`, never
  a plain px/rem constant.
- **Moiré:** ultra-thin lines stepped fractionally shimmer. The gradient uses a
  soft edge (`color 0→0.6·size`, then fade to transparent) to avoid this. Don't
  reintroduce a hard edge.
- Fine-tune instructions (make it SMALLER `scanline_lines`, adjust
  `scanline_size_rem`) belong in the doc-comment box in `src/config.rs` (~line 30).

## Native fork notes (`~/bspwm-cyberquote-native`)

- Reuses `src/config.rs`, `src/quotes.rs`, and `config.toml` copied verbatim;
  `quotes.json` is a symlink back to the original repo, so both apps share the
  quote pool and the same `CONFIG_PATH`.
- `src/render.rs` is the Cairo paint pipeline (scanlines, vignette, RGB-split
  glitch, cursor).
- gtk 0.18 API facts learned the hard way:
  - `connect_draw` returns `glib::Propagation::Proceed`, not `glib::Inhibit`.
  - `pangocairo` is NOT re-exported by `gtk`; depends on it directly.
  - `gtk::cairo::Context::new(&surface)` returns `Result` → `.expect()`.
  - `layout.index_to_pos(i32) -> gtk::pango::Rectangle`.
  - `add_color_stop_rgba` takes 5 args.
  - many `cr.paint()/save()/restore()` return `Result`; render.rs has
    `#![allow(unused_must_use)]` at the top.
- The quote/text layer is event-driven only (drawn on quote change, typewriter
  reveal, glitch burst, or resize). Each phrase opens with a ONE-SHOT typewriter
  reveal: `main.rs::start_typewriter` advances `typewriter_chars` (~30 ms/char,
  generation-token so a later phrase retires a stale timer), and
  `render::draw_text_block` clips the centered/wrapped quote line-by-line to the
  caret of the revealed chars (`push_typewriter_clip` + `pango::Layout::
  index_to_line_x`), drawing the caret (`draw_terminal_cursor`) inside the
  revealed region. The author line appears only once the phrase is fully
  revealed. The quote body is a single crisp `render::draw_text` pass in the
  foreground color. The single continuous animation is the migrating scanlines:
  a transparent RGBA
  `gtk::Overlay` layer paints a PRE-RENDERED A8
  tile (`render::scanline_tile` — exactly one `period` tall, holding one band)
  via `render::draw_scanline_tile` as a single `Extend::Repeat` pattern paint
  (one composited blit per frame — no clear, no per-band rects). The timer
  rolls the phase 20 fps/90s, wrapping on the SAME integer period the tile is
  built with (`render::scanline_tile_height`) so no seam can accumulate. When
  that overlay exists it also paints the blinking terminal caret
  (`render::cursor_rect` gated by `DrawState.cursor_on`; toggled by
  `arm_cursor_blink` at ~1.9 Hz) — the heavy main layer is only redrawn on
  quote change, glitch burst, typewriter progress, or resize.  Needs a
  compositor (`screen.rgba_visual()`); without one the main layer draws
  static scanlines AND the caret itself (`Theme.animated_scanlines`). Tunables
  `scanline_opacity`/`scanline_lines`/`scanline_size_rem` are read from
  `config.toml` but the fork derives band thickness/spacing in Cairo, not CSS.
  Measuring it: `scripts/cpu_measure.sh` (see below).
- If you edit the original `src/config.rs`, mirror it into the fork
  (`cp ../bspwm-cyberquote/src/config.rs src/config.rs`) — kept md5-identical.
  The fork-only `accent.author_color` drives author ink (HTML host ignores it,
  keeping its cyan author via CSS); `accent.cursor_color` drives the terminal
  caret + blink overlay, defaulting to the same violet as the author.

## CPU measurement

`~/bspwm-cyberquote-native/scripts/cpu_measure.sh <pattern> [interval] [dur]`
samples per-process CPU from `/proc/<pid>/stat` (utime+stime deltas, all
threads), logs a CSV, prints mean/min/max/stddev. A bare name matches by
`/proc/<pid>/exe` basename (immune to the 15-char comm truncation and to this
script's own subshells); a pattern with `/` is a raw cmdline match. See its
help header. Example: `./scripts/cpu_measure.sh bspwm-cyberquote-native 0.5 30`.

## Known test state (do NOT "fix" these silently)

Original (`~/bspwm-cyberquote`): 17 lib tests pass. Integration suite: 21 pass,
**4 pre-existing failures** that are unrelated to current work:
`test_e2e_html_parseable_as_whole_document`, `test_html_has_required_structure`,
`test_html_uses_css_var_for_each_tunable`, `test_typewriter_slide_from_below`,
plus 1 flaky random: `test_e2e_two_independent_loads_produce_different_html`.
Fork: 21 lib tests pass, no integration tests.

Gotcha: `src/config.rs`'s module doc used to embed a broken code doctest
(referencing `webkit_web_view_run_javascript`). It's fixed — keep the header
doc free of Rust code fences that would be compiled as doctests.

## Operational rules

- **Never launch a GUI app to "test"** — it opens windows on the user's live
  desktop. Verify by building + unit tests; ask the user to run it.
- `pkill -f bspwm-cyberquote` matches the calling shell's own cmdline; use the
  bracketed pattern `pgrep -f "bspwm-cyberquot[e]"` when a target is needed.
- The app prints noisy logs prefixed `bspwm-cyberquote: ...` and
  `bspwm-cyberquote-native: ...`; those are expected even when healthy.
- Keyboard secrets/API keys: none; never add any.

## Git

Original is on `origin/main` (`https://github.com/anomalyco/opencode` — the
user's fork). Latest pushed commit `1e80388 "feat(app):optimization work"`.
Uncommitted at last check (not yet pushed): scanline smoothness (2160 steps),
config fallback `load_cfg()`, scanline mesh knobs (`scanline_lines`,
`scanline_size_rem`), and the doctest fix. Only commit/push when explicitly
asked. The native fork is untracked anywhere (suggest: separate repo).