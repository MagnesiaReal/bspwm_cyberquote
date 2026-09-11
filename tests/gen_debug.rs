use bspwm_cyberquote::config::{build_stylesheet, Config, load_config};
use bspwm_cyberquote::cyberhtml::build_html;
use bspwm_cyberquote::quotes::{load_and_pick, Picker};

#[test]
fn dump_test_html() {
    let cfg = match load_config("/nonexistent.toml") {
        Ok(c) => c,
        Err(_) => Config::defaults(),
    };
    let css = build_stylesheet(&cfg);
    let sel = load_and_pick(
        Some(std::path::Path::new("/home/magneciareal/bspwm-cyberquote/quotes.json")),
        Picker::Random,
    )
    .unwrap();
    let html = build_html(&sel, &css, 5);
    std::fs::write("/tmp/opencode/generated.html", &html).unwrap();
    eprintln!("HTML length: {}", html.len());
}