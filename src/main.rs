//! bspwm-cyberquote — GTK4 + WebKitGTK 6.0 host
//!
//! Spawns one undecorated ApplicationWindow per Gdk monitor, each containing a
//! WebKitGTK 6.0 `WebView` (the `webkit6` crate) that loads a *distinct*
//! cyberpunk quote page produced by the `cyberhtml` module.  Every monitor
//! gets its own quote, sized/positioned to that monitor's geometry.
//!
//! GTK4 removed GTK3's `WindowTypeHint::Desktop` and `gtk_window_move()`, so
//! the toolkit can no longer make a window a desktop "wallpaper".  On X11
//! (bspwm) we restore the original behavior exactly like conky/glava do:
//! [`apply_desktop_placement`] marks the window's XID with the EWMH
//! `_NET_WM_WINDOW_TYPE_DESKTOP` atom and `XMoveResizeWindow`s it onto its
//! monitor, so bspwm skips tiling it and keeps it at the bottom of the stack.
//! On Wayland no wallpaper window type exists at all — we fall back to
//! `fullscreen_on_monitor`.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use gtk::gdk::{Backend, Display, Monitor};
use gtk::glib as glib;
use gtk::glib::prelude::*;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow};

use webkit6::prelude::WebViewExt;
use webkit6::WebView;

use bspwm_cyberquote::config::{ConfigMonitorInfo, Orientation};

// ---------------------------------------------------------------------------
// Config path — per-user file.  Agent 2's config::load_config reads this.
// ---------------------------------------------------------------------------
const CONFIG_PATH: &str = concat!(env!("HOME"), "/.config/bspwm-cyberquote/config.toml");

/// Precedence: per-user CONFIG_PATH → repo `./config.toml` → defaults.
///
/// git-pull then `cargo run --release` from the repo on a fresh machine has
/// no per-user config yet; without this fallback it silently used
/// `Config::defaults()` (font_size 18) instead of the committed 37.
fn load_cfg() -> Result<bspwm_cyberquote::config::Config, (PathBuf, String)> {
    let candidates: [PathBuf; 2] = [PathBuf::from(CONFIG_PATH), PathBuf::from("config.toml")];
    let mut last: Option<(PathBuf, String)> = None;
    for path in candidates {
        match bspwm_cyberquote::config::load_config(&path) {
            Ok(c) => {
                eprintln!("bspwm-cyberquote: config loaded from {}", path.display());
                return Ok(c);
            }
            Err(e) => last = Some((path, e.to_string())),
        }
    }
    match last {
        Some((p, s)) => Err((p, s)),
        None => {
            let e = "no config candidate could be loaded".to_string();
            Err((PathBuf::from(CONFIG_PATH), e))
        }
    }
}

// ---------------------------------------------------------------------------
// Global shutdown flag — set by SIGTERM/SIGINT handlers, polled on the main
// loop to quit the GtkApplication.  The handlers only write to an AtomicBool
// (async-signal-safe), so no glib signal machinery is needed.
// ---------------------------------------------------------------------------
static SHUTDOWN: AtomicBool = AtomicBool::new(false);

fn main() {
    // Install raw POSIX signal handlers (async-signal-safe: atomic write only).
    unsafe {
        extern "C" fn sig_handler(_sig: libc::c_int) {
            SHUTDOWN.store(true, Ordering::Relaxed);
        }
        libc::signal(
            libc::SIGTERM,
            sig_handler as *const () as libc::sighandler_t,
        );
        libc::signal(
            libc::SIGINT,
            sig_handler as *const () as libc::sighandler_t,
        );
    }

    // Build the GTK application.
    let app = Application::builder()
        .application_id("com.bspwm.cyberquote")
        .build();

    // Poll the shutdown flag on the application's main loop; GTK4 has no
    // gtk::main_quit(), so quit the Application itself.  `timeout_add_local`
    // runs on the main thread only, so capturing the (non-Send) Application
    // reference is sound.
    let app2 = app.clone();
    glib::timeout_add_local(
        std::time::Duration::from_millis(100),
        move || {
            if SHUTDOWN.load(Ordering::Relaxed) {
                app2.quit();
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        },
    );

    app.connect_activate(build_windows);

    let _ = app.run();
}

/// Called once when the GtkApplication is activated.
///
/// Enumerates every Gdk monitor, creates one undecorated ApplicationWindow per
/// monitor sized to that monitor, embeds a WebKitGTK 6.0 WebView loading a
/// unique cyberpunk HTML document (a different quote per monitor), and
/// presents the window — on X11 as a desktop-typed "wallpaper".
fn build_windows(app: &Application) {
    // ---- config + stylesheet (Agent 2) ----
    let cfg = match load_cfg() {
        Ok(c) => c,
        Err((_, e)) => {
            eprintln!("bspwm-cyberquote: config load failed ({}), using defaults", e);
            bspwm_cyberquote::config::Config::defaults()
        }
    };

    let stylesheet = bspwm_cyberquote::config::build_stylesheet(&cfg);
    eprintln!(
        "bspwm-cyberquote: config loaded, stylesheet {} bytes",
        stylesheet.len()
    );

    // ---- enumerate monitors and apply the monitor policy ----
    let display = Display::default().expect("no default Gdk display");
    let is_x11 = display.backend() == Backend::X11;

    // GDK4 exposes monitors as a GListModel.  `is_primary` is gone, so for
    // primary_only we use the geometry-origin heuristic: on X11 the primary
    // output owns the virtual screen origin (0,0).
    let mut monitors: Vec<Monitor> = display
        .monitors()
        .iter::<Monitor>()
        .filter_map(Result::ok)
        .collect();
    if cfg.monitors.primary_only {
        monitors.retain(is_primary_monitor);
    }

    let monitor_infos: Vec<ConfigMonitorInfo> = monitors
        .iter()
        .map(|m| {
            let g = m.geometry();
            ConfigMonitorInfo {
                width: g.width(),
                height: g.height(),
                x: g.x(),
                y: g.y(),
                rotation: Orientation::Normal,
            }
        })
        .collect();

    let geometries = bspwm_cyberquote::config::monitor_layout(&monitor_infos);
    eprintln!(
        "bspwm-cyberquote: {} monitor(s) -> {} window(s)",
        monitors.len(),
        geometries.len()
    );

    // ---- one distinct quote per monitor (Agent 3) ----
    let quote_path = std::path::Path::new(&cfg.quotes.source);
    let selections =
        match bspwm_cyberquote::quotes::load_and_pick_many(Some(quote_path), geometries.len()) {
            Ok(sel) => sel,
            Err(e) => {
                eprintln!(
                    "bspwm-cyberquote: quote load failed ({}), using fallback",
                    e
                );
                let fallback = bspwm_cyberquote::quotes::QuoteSelection {
                    quote: bspwm_cyberquote::quotes::Quote {
                        text: "System online — awaiting quote feed.".into(),
                        author: "bspwm-cyberquote".into(),
                    },
                };
                vec![fallback; geometries.len()]
            }
        };

    let cycle_interval_minutes: u64 = cfg.quotes.cycle_interval_minutes as u64;

    // ---- build a window per monitor with its own quote ----
    for (idx, geometry) in geometries.iter().enumerate() {
        let html = bspwm_cyberquote::cyberhtml::build_html(
            &selections[idx],
            &stylesheet,
            cycle_interval_minutes,
        );

        let window = ApplicationWindow::builder()
            .application(app)
            .decorated(false)
            .default_width(geometry.width)
            .default_height(geometry.height)
            .resizable(false)
            .build();

        // Embed the WebKitGTK 6.0 WebView — GTK4's set_child replaces GTK3's add.
        let webview = WebView::new();
        window.set_child(Some(&webview));

        // Load the cyberpunk HTML document (unique quote for this monitor).
        webview.load_html(html.as_str(), Some("file:///cyberquote.html"));

        // Placement.  The X11 surface exists once the window realizes; bspwm
        // reads `_NET_WM_WINDOW_TYPE_DESKTOP` at the map request, so the type
        // must be set in `connect_realize` (before mapping), not in the map
        // event which only arrives after the WM has already managed the window.
        // We re-assert geometry+type at map too, because the WM/GDK may have
        // resized/repositioned the surface in between.
        let (gx, gy, gw, gh) = (geometry.x, geometry.y, geometry.width, geometry.height);
        if is_x11 {
            window.connect_realize(move |w| apply_desktop_placement(w, gx, gy, gw, gh));
            window.connect_map(move |w| apply_desktop_placement(w, gx, gy, gw, gh));
        } else if let Some(m) = monitors.get(idx) {
            let m = m.clone();
            window.connect_map(move |w| w.fullscreen_on_monitor(&m));
        } else {
            window.connect_map(|w| w.fullscreen());
        }

        window.present();
    }
}

/// GDK4 removed `gdk_monitor_is_primary`; on X11 the primary output is the one
/// that owns the virtual screen origin (0,0).  Use that geometry heuristic.
fn is_primary_monitor(m: &Monitor) -> bool {
    let g = m.geometry();
    g.x() <= 0 && 0 < g.x() + g.width() && g.y() <= 0 && 0 < g.y() + g.height()
}

/// Make the window a desktop "wallpaper" on X11 — the same mechanism conky and
/// glava use: mark the X window `_NET_WM_WINDOW_TYPE_DESKTOP` and force its
/// geometry.  bspwm then skips tiling the window and keeps it at the bottom.
///
/// Must run at *realize* time (before the window is mapped): the window
/// manager reads the EWMH window type while processing the map request, and a
/// desktop-typed window maps onto the desktop layer underneath everything else.
/// `connect_map` alone is too late — the map event fires after the WM has
/// already managed the window as ordinary.
fn apply_desktop_placement(window: &ApplicationWindow, x: i32, y: i32, w: i32, h: i32) {
    use gdk4_x11::x11::xlib;
    use gdk4_x11::X11Surface;

    let widget = window.upcast_ref::<gtk::Widget>();
    let Some(native) = widget.native() else {
        eprintln!("bspwm-cyberquote: window has no native surface; skipping placement");
        return;
    };
    let Some(surface) = native.surface() else {
        eprintln!("bspwm-cyberquote: window surface is not X11; skipping placement");
        return;
    };
    let Some(x11surf) = surface.downcast_ref::<X11Surface>() else {
        eprintln!("bspwm-cyberquote: native surface is not X11; skipping placement");
        return;
    };

    // EWMH hints GTK4 dropped but the X11 backend still honors (deprecated,
    // functional) — mirrors the old GTK3 skip_taskbar_hint/skip_pager_hint.
    x11surf.set_skip_taskbar_hint(true);
    x11surf.set_skip_pager_hint(true);

    // `gdk4_x11::x11` is x11-dl: the raw Xlib functions live on the lazily
    // dlopened `Xlib` struct, not at module scope.
    let xl = xlib::Xlib::open().expect("gdk4-x11 xlib: could not dlopen libX11");
    let xid = x11surf.xid() as xlib::Window;

    unsafe {
        let dpy = (xl.XOpenDisplay)(std::ptr::null());
        if dpy.is_null() {
            eprintln!("bspwm-cyberquote: XOpenDisplay failed; skipping desktop placement");
            return;
        }

        let wm_type = (xl.XInternAtom)(dpy, c"_NET_WM_WINDOW_TYPE".as_ptr(), xlib::False);
        let desktop =
            (xl.XInternAtom)(dpy, c"_NET_WM_WINDOW_TYPE_DESKTOP".as_ptr(), xlib::False);
        let atom: xlib::Atom = desktop;

        // Window type: desktop → sits behind everything like a wallpaper.
        (xl.XChangeProperty)(
            dpy,
            xid,
            wm_type,
            xlib::XA_ATOM,
            32,
            xlib::PropModeReplace,
            &atom as *const xlib::Atom as *const u8,
            1,
        );

        // Pin to this monitor's geometry (GTK4 has no gtk_window_move).
        // x11-dl's width/height are c_uint.
        (xl.XMoveResizeWindow)(dpy, xid, x as libc::c_int, y as libc::c_int, w as u32, h as u32);

        (xl.XSync)(dpy, xlib::False);
        (xl.XCloseDisplay)(dpy);
    }

    eprintln!(
        "bspwm-cyberquote: desktop placement applied to xid {:#x} at {},{}+{}x{}",
        x11surf.xid(),
        x,
        y,
        w,
        h
    );
}