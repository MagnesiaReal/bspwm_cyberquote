//! bspwm-cyberquote library crate.
//!
//! Agent 3 owns the quote engine (`quotes`) and the animated HTML/CSS/JS
//! content layer (`cyberhtml`).  Agent 1 (main.rs) is the GTK4 host that
//! creates WebViews and feeds them the HTML produced by `cyberhtml::build_html`.
//!
//! Public API consumed by main.rs:
//!   - `quotes::load_and_pick(...)`  →  `QuoteSelection`
//!   - `cyberhtml::build_html(quote, css_vars)`  →  `String` (full HTML doc)
//!   - `cyberhtml::default_css_vars()`  →  default CSS custom-property block

pub mod quotes;
pub mod cyberhtml;
pub mod config;
