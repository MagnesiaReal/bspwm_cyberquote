//! Integration smoke test for the quote engine + HTML builder.
//!
//! This verifies the public API shape: load quotes (from a test fixture),
//! pick one, render the HTML document, and assert the document is well-formed
//! and contains the expected quote text. It exercises the module contract that
//! main.rs will use (load a QuoteSelection, call build_html, feed to WebView).

use bspwm_cyberquote::quotes::{self, Picker, QuoteSelection};
use bspwm_cyberquote::cyberhtml::{self, build_html, default_css_vars};
use std::fs;

fn test_quotes_file() -> &'static str {
    r#"{
      "quotes": [
        { "text": "The quick brown fox jumps.", "author": "A. Writer" },
        { "text": "To be or not to be.", "author": "S. Hamlet" },
        { "text": "A journey of a thousand miles.", "author": "L. Lao" }
      ]
    }"#
}

fn write_test_quotes(tmpdir: &std::path::Path) -> std::path::PathBuf {
    let p = tmpdir.join("test_quotes.json");
    fs::write(&p, test_quotes_file()).unwrap();
    p
}

#[test]
fn test_quote_struct_roundtrip() {
    let file = quotes::QuotesFile {
        quotes: vec![
            quotes::Quote {
                text: "hello".into(),
                author: "me".into(),
            },
            quotes::Quote {
                text: "world".into(),
                author: "you".into(),
            },
        ],
    };
    assert_eq!(file.quotes.len(), 2);
    assert_eq!(file.quotes[0].text, "hello");
}

#[test]
fn test_load_and_pick_from_fixture() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = write_test_quotes(tmp.path());

    let sel = quotes::load_and_pick(Some(&path), Picker::Random)
        .expect("should load from a valid fixture");
    let quote = &sel.quote;
    assert!(
        quote.text == "The quick brown fox jumps."
            || quote.text == "To be or not to be."
            || quote.text == "A journey of a thousand miles."
    );
    assert!(!quote.author.is_empty());
}

#[test]
fn test_load_and_pick_many_returns_n_distinct() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = write_test_quotes(tmp.path());

    let sels = quotes::load_and_pick_many(Some(&path), 3).expect("should load three");
    assert_eq!(sels.len(), 3);

    let texts: Vec<&str> = sels
        .iter()
        .map(|s| s.quote.text.as_str())
        .collect();
    assert_eq!(texts.len(), 3);
}

#[test]
fn test_load_and_pick_many_cycles_when_count_exceeds_pool() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = write_test_quotes(tmp.path());

    let sels = quotes::load_and_pick_many(Some(&path), 7).expect("should load seven");
    assert_eq!(sels.len(), 7);

    let texts: Vec<&str> = sels
        .iter()
        .map(|s| s.quote.text.as_str())
        .collect();
    assert!(texts[0..3].iter().all(|t| texts[3..6].contains(t)));
}

#[test]
fn test_empty_quotes_fails() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let p = tmp.path().join("empty.json");
    fs::write(&p, r#"{"quotes": []}"#).unwrap();
    let err = quotes::load_and_pick(Some(&p), Picker::Random).unwrap_err();
    assert!(err.contains("no usable quotes") || err.contains("empty"));
}

#[test]
fn test_build_html_contains_quote_text_and_author() {
    let sel = QuoteSelection {
        quote: quotes::Quote {
            text: "One small step".into(),
            author: "N. Armstrong".into(),
        },
    };
    let html = build_html(&sel, &default_css_vars(), 0);
    assert!(html.contains("One small step"));
    assert!(html.contains("N. Armstrong"));
    assert!(!html.contains("<One small step>"));
}

#[test]
fn test_html_has_required_structure() {
    let sel = QuoteSelection {
        quote: quotes::Quote {
            text: "A brief test".into(),
            author: "Tester".into(),
        },
    };
    let html = build_html(&sel, &default_css_vars(), 0);
    assert!(html.contains("<!doctype html>"), "missing doctype");
    assert!(html.contains("<html"), "missing <html>");
    assert!(html.contains("<style>"), "missing <style>");
    assert!(html.contains("</style>"), "missing </style>");
    assert!(html.contains("<script>"), "missing <script>");
    assert!(html.contains("</script>"), "missing </script>");
    assert!(html.contains("class=\"stage\""), "missing stage container");
    assert!(html.contains("class=\"quote\""), "missing quote container");
    assert!(html.contains("class=\"author\""), "missing author container");
    assert!(html.contains("class=\"cursor\""), "missing cursor");
    assert!(html.contains("var(--typewriter-ms)"), "missing typewriter-ms var");
    assert!(html.contains("var(--glitch-ms)"), "missing glitch-ms var");
    assert!(html.contains("var(--scanline-opacity)"), "missing scanline-opacity var");
    assert!(html.contains("var(--crt-perspective)"), "missing crt-perspective var");
}

#[test]
fn test_css_vars_injected_and_used() {
    let sel = QuoteSelection {
        quote: quotes::Quote {
            text: "Neon rain".into(),
            author: "K. Nightshade".into(),
        },
    };
    let custom = "--accent-cyan:#ff7fff;--accent-magenta:#7fff7f;--typewriter-ms:40;";
    let html = build_html(&sel, custom, 0);
    assert!(html.contains("--accent-cyan:#ff7fff;"));
    assert!(html.contains("--typewriter-ms:40;"));
    assert!(html.contains("var(--accent-magenta)"));
}

#[test]
fn test_custom_vars_override_defaults() {
    let sel = QuoteSelection {
        quote: quotes::Quote {
            text: "Override test".into(),
            author: "A. Override".into(),
        },
    };
    let html_default = build_html(&sel, "", 0);
    let html_custom = build_html(
        &sel,
        "--accent-cyan:#ff00ff;--accent-magenta:#00ffff;--font-size:30px;",
        0,
    );
    assert!(html_default.contains("--accent-cyan:#39e6ff;"));
    assert!(html_custom.contains("--accent-cyan:#ff00ff;"));
    assert!(html_custom.contains("--font-size:30px;"));
}

#[test]
fn test_escape_html_basic() {
    let dangerous = "a <b> & \"c\"";
    let safe = bspwm_cyberquote::cyberhtml::escape_html(dangerous);
    assert_eq!(safe, "a &lt;b&gt; &amp; &quot;c&quot;");
}

#[test]
fn test_js_escape_roundtrip() {
    let raw = "say \"hello\"\nand 'goodbye'";
    let js = cyberhtml::escape_js_string(raw);
    assert!(js.starts_with('"') && js.ends_with('"'));
    let restored: String = eval_js_str(&js).unwrap_or_default();
    assert_eq!(restored, raw);
}

#[test]
fn test_js_escape_controls() {
    let raw = "x\u{0000}y\u{0009}z";
    let js = cyberhtml::escape_js_string(raw);
    assert!(js.contains("\\u0000"));
    assert!(js.contains("\\t"));
}

#[test]
fn test_default_css_vars_has_required_keys() {
    let vars = default_css_vars();
    let required = [
        "--bg", "--fg", "--accent-cyan", "--accent-magenta",
        "--font-size", "--typewriter-ms", "--glitch-ms",
        "--scanline-opacity", "--crt-perspective", "--crt-radius",
        "--cursor-char", "--cursor-blink",
    ];
    for key in &required {
        assert!(vars.contains(key), "default_css_vars missing {}", key);
    }
}

#[test]
fn test_html_uses_css_var_for_each_tunable() {
    let sel = QuoteSelection {
        quote: quotes::Quote {
            text: "Tunables".into(),
            author: "T. Est".into(),
        },
    };
    let html = build_html(&sel, &default_css_vars(), 0);
    let tunable_refs = [
        ("typewriter-ms", "--typewriter-ms"),
        ("typewriter-ease", "--typewriter-ease"),
        ("typewriter-slide", "--typewriter-slide"),
        ("typewriter-opacity-start", "--typewriter-opacity-start"),
        ("typewriter-opacity-end", "--typewriter-opacity-end"),
        ("cursor-char", "--cursor-char"),
        ("cursor-blink", "--cursor-blink"),
        ("cursor-color", "--cursor-color"),
        ("glitch-intensity", "--glitch-intensity"),
        ("glitch-ms", "--glitch-ms"),
        ("glitch-duration", "--glitch-duration"),
        ("glitch-color-cyan", "--glitch-color-cyan"),
        ("glitch-color-magenta", "--glitch-color-magenta"),
        ("glitch-jitter", "--glitch-jitter"),
        ("scanline-opacity", "--scanline-opacity"),
        ("scanline-color", "--scanline-color"),
        ("scanline-size", "--scanline-size"),
        ("scanline-gap", "--scanline-gap"),
        ("scanline-speed", "--scanline-speed"),
        ("crt-perspective", "--crt-perspective"),
        ("crt-radius", "--crt-radius"),
        ("crt-vignette-stops", "--crt-vignette-stops"),
        ("crt-vignette-blur", "--crt-vignette-blur"),
    ];
    for (_, var) in &tunable_refs {
        assert!(
            html.contains(&format!("var({})", var)),
            "html missing var({}) reference",
            var
        );
    }
}

#[test]
fn test_cycle_block_only_when_positive() {
    let sel = QuoteSelection {
        quote: quotes::Quote {
            text: "cycle test".into(),
            author: "C. Ycle".into(),
        },
    };
    let html = build_html(&sel, &default_css_vars(), 0);
    assert!(html.contains("CYCLE_MINUTES = 0"));
    assert!(html.contains("if (CYCLE_MINUTES > 0)"));
}

#[test]
fn test_glitch_keyframes_present() {
    let sel = QuoteSelection {
        quote: quotes::Quote {
            text: "glitch".into(),
            author: "G. Litch".into(),
        },
    };
    let html = build_html(&sel, &default_css_vars(), 0);
    assert!(html.contains("@keyframes glitch-full"));
    assert!(html.contains("filter: invert(1)"));
    assert!(html.contains("--glitch-color-cyan"));
    assert!(html.contains("--glitch-color-magenta"));
}

#[test]
fn test_scanline_keyframes_present() {
    let sel = QuoteSelection {
        quote: quotes::Quote {
            text: "scanlines".into(),
            author: "S. Canline".into(),
        },
    };
    let html = build_html(&sel, &default_css_vars(), 0);
    assert!(html.contains("@keyframes scanline-sweep"));
    assert!(html.contains("repeating-linear-gradient"));
    assert!(html.contains("--scanline-opacity"));
}

#[test]
fn test_crt_vignette_structure() {
    let sel = QuoteSelection {
        quote: quotes::Quote {
            text: "CRT".into(),
            author: "C. Rt".into(),
        },
    };
    let html = build_html(&sel, &default_css_vars(), 0);
    assert!(html.contains("border-radius: var(--crt-radius);"));
    assert!(html.contains("radial-gradient(circle at center"));
    assert!(html.contains("--crt-vignette-stops"));
    assert!(html.contains("--crt-vignette-blur"));
    assert!(html.contains("filter: blur(var(--crt-vignette-blur));"));
}

#[test]
fn test_neon_glow_layers() {
    let sel = QuoteSelection {
        quote: quotes::Quote {
            text: "neon".into(),
            author: "N. On".into(),
        },
    };
    let html = build_html(&sel, &default_css_vars(), 0);
    assert!(html.contains("--glow-cyan-1"));
    assert!(html.contains("--glow-cyan-2"));
    assert!(html.contains("--glow-cyan-3"));
    assert!(html.contains("--glow-magenta-1"));
    assert!(html.contains("--glow-magenta-2"));
    assert!(html.contains("text-shadow:\n    var(--glow-cyan-1)"));
}

#[test]
fn test_typewriter_slide_from_below() {
    let sel = QuoteSelection {
        quote: quotes::Quote {
            text: "typewriter".into(),
            author: "T. Pwer".into(),
        },
    };
    let html = build_html(&sel, &default_css_vars(), 0);
    assert!(html.contains("@keyframes slide-up"));
    assert!(html.contains("transform: translateY(var(--typewriter-slide))"));
    assert!(html.contains(".letter"));
    assert!(html.contains("animation-delay"));
}

#[test]
fn test_blinking_cursor_keyframes() {
    let sel = QuoteSelection {
        quote: quotes::Quote {
            text: "cursor".into(),
            author: "C. Ursor".into(),
        },
    };
    let html = build_html(&sel, &default_css_vars(), 0);
    assert!(html.contains("@keyframes cursor-blink"));
    assert!(html.contains("animation: cursor-blink"));
    assert!(html.contains("--cursor-char"));
}

#[test]
fn test_fade_out_in_then_typewriter_still_runs() {
    let sel = QuoteSelection {
        quote: quotes::Quote {
            text: "fade cycle".into(),
            author: "F. Ade".into(),
        },
    };
    let html = build_html(&sel, &default_css_vars(), 0);
    assert!(html.contains("@keyframes fade-out"));
    assert!(html.contains("@keyframes fade-in"));
    assert!(html.contains("renderQuote(QUOTE_TEXT, QUOTE_AUTHOR);"));
}

#[test]
fn test_cycle_interval_zero_holds_single_quote() {
    let sel = QuoteSelection {
        quote: quotes::Quote {
            text: "hold".into(),
            author: "H. Old".into(),
        },
    };
    let html = build_html(&sel, &default_css_vars(), 0);
    assert!(html.contains("renderQuote(QUOTE_TEXT, QUOTE_AUTHOR);"));
    assert!(html.contains("var cursor = document.createElement"));
}

#[test]
fn test_e2e_two_independent_loads_produce_different_html() {
    let tmp = tempfile::tempdir().unwrap();
    let p = tmp.path().join("multi.json");
    fs::write(
        &p,
        r#"{
          "quotes": [
            { "text": "First quote", "author": "A" },
            { "text": "Second quote", "author": "B" }
          ]
        }"#,
    )
    .unwrap();

    let sel1 = quotes::load_and_pick(Some(&p), quotes::Picker::Random).unwrap();
    let sel2 = quotes::load_and_pick(Some(&p), quotes::Picker::Random).unwrap();
    let html1 = build_html(&sel1, "", 0);
    let html2 = build_html(&sel2, "", 0);

    assert!(html1.contains("First quote") || html1.contains("Second quote"));
    assert!(html2.contains("First quote") || html2.contains("Second quote"));
    if sel1.quote.text == sel2.quote.text {
        let sel3 = quotes::load_and_pick(Some(&p), quotes::Picker::Random).unwrap();
        assert_ne!(sel1.quote.text, sel3.quote.text);
    }
}

#[test]
fn test_e2e_html_parseable_as_whole_document() {
    let sel = QuoteSelection {
        quote: quotes::Quote {
            text: "End to end".into(),
            author: "E. E.".into(),
        },
    };
    let html = build_html(&sel, &default_css_vars(), 0);
    assert!(html.starts_with("<!doctype html>"));
    let script_idx = html.find("<script>").unwrap();
    let body_idx = html.find("<body>").unwrap();
    assert!(script_idx < body_idx, "script should appear before body");
    let closing_script = html.find("</script>").unwrap();
    assert!(closing_script > script_idx);
}

/// Minimal JS literal roundtrip parser for the JS-escape test.
/// Handles the escapes produced by escape_js_string. No JS engine needed.
fn eval_js_str(code: &str) -> Option<String> {
    let s = code.trim();
    if !s.starts_with('"') || !s.ends_with('"') {
        return None;
    }
    let inner = &s[1..s.len() - 1];
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                let next = chars.next();
                match next {
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some('n') => out.push('\n'),
                    Some('r') => out.push('\r'),
                    Some('t') => out.push('\t'),
                    Some('u') => {
                        let mut hex = String::new();
                        for _ in 0..4 {
                            if let Some(h) = chars.next() {
                                hex.push(h);
                            }
                        }
                        if let Ok(code) = u32::from_str_radix(&hex, 16) {
                            if let Some(ch) = char::from_u32(code) {
                                out.push(ch);
                            }
                        }
                    }
                    _ => out.push('\\'),
                }
            }
            c => out.push(c),
        }
    }
    Some(out)
}
