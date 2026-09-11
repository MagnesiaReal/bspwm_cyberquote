//! bspwm-cyberquote — GTK3 + webkit2gtk-4.1 host
//!
//! Spawns one undecorated ApplicationWindow per Gdk monitor, each containing a
//! webkit2gtk WebView that loads a *distinct* cyberpunk quote page produced by
//! the `cyberhtml` module.  Every monitor gets its own quote, sized/positioned
//! to that monitor's geometry.
//!
//! GTK3 is used (not GTK4) because the `webkit2gtk` crate (2.0.2) exposes
//! `WebView` as a GTK3 widget (it depends on `gtk` 0.18, not `gtk4`).  Using
//! GTK3 avoids the glib version diamond (glib 0.18 from webkit2gtk vs glib 0.20
//! from gtk4) that would otherwise make `WebView` incompatible as a window child.

use std::sync::atomic::{AtomicBool, Ordering};

use glib::prelude::*;
use glib::ControlFlow;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow};
use webkit2gtk::WebView;
use webkit2gtk::WebViewExt;

use bspwm_cyberquote::config::{ConfigMonitorInfo, Orientation};

// ---------------------------------------------------------------------------
// Config path — per-user file.  Agent 2's config::load_config reads this.
// ---------------------------------------------------------------------------
const CONFIG_PATH: &str = concat!(env!("HOME"), "/.config/bspwm-cyberquote/config.toml");



// ---------------------------------------------------------------------------
// Global shutdown flag — set by SIGTERM/SIGINT handlers, polled on the main
// loop to call gtk::main_quit().
//
// glib 0.18 does not expose a cross-platform unix::Signal helper, so we
// install raw POSIX signal handlers ourselves.  The handlers only write to an
// AtomicBool (async-signal-safe).
// ---------------------------------------------------------------------------
static SHUTDOWN: AtomicBool = AtomicBool::new(false);

fn main() {
    // Poll the shutdown flag on the main loop; when set, quit.
    // glib 0.18: timeout_add takes (Duration, FnMut) returning ControlFlow.
    glib::timeout_add(
        std::time::Duration::from_millis(100),
        move || {
            if SHUTDOWN.load(Ordering::Relaxed) {
                gtk::main_quit();
                ControlFlow::Break
            } else {
                ControlFlow::Continue
            }
        },
    );

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

    app.connect_activate(build_windows);

    app.run();
}

/// Called once when the GtkApplication is activated.
///
/// Enumerates every Gdk monitor, creates one undecorated ApplicationWindow per
/// monitor sized/positioned to that monitor, embeds a webkit2gtk WebView
/// loading a unique cyberpunk HTML document (a different quote per monitor),
/// and shows the window.
fn build_windows(app: &Application) {
    use gtk::gdk::{Display, Monitor, Rectangle};

    // ---- config + stylesheet (Agent 2) ----
    let cfg = match bspwm_cyberquote::config::load_config(CONFIG_PATH) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "bspwm-cyberquote: config load failed ({}), using defaults",
                e
            );
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
    let n = display.n_monitors();

    let mut monitors: Vec<ConfigMonitorInfo> = Vec::new();
    for i in 0..n {
        let monitor: Monitor = display
            .monitor(i)
            .expect("monitor list item should exist")
            .downcast::<Monitor>()
            .expect("monitor list item should be a Monitor");
        if cfg.monitors.primary_only && !monitor.is_primary() {
            continue;
        }
        let geo: Rectangle = monitor.geometry();
        monitors.push(ConfigMonitorInfo {
            width: geo.width(),
            height: geo.height(),
            x: geo.x(),
            y: geo.y(),
            rotation: Orientation::Normal,
        });
    }

    let geometries = bspwm_cyberquote::config::monitor_layout(&monitors);
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
            .type_hint(gtk::gdk::WindowTypeHint::Desktop)
            .skip_taskbar_hint(true)
            .skip_pager_hint(true)
            .build();

        // Position and size to this monitor's geometry.
        window.move_(geometry.x, geometry.y);
        window.resize(geometry.width, geometry.height);

        // Embed a webkit2gtk WebView — GTK3 container::add works because
        // WebView implements gtk::Widget (GTK3), matching our gtk::Container.
        let webview = WebView::new();
        window.add(&webview);

        // Load the cyberpunk HTML document (unique quote for this monitor).
        webview.load_html(html.as_str(), Some("file:///cyberquote.html"));

        // Show the window.
        window.show_all();
    }
}
