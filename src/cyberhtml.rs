use crate::quotes::QuoteSelection;

/// CSS custom properties consumed by the page's inline stylesheet.
///
/// Agent 2's theme engine can retune palette / font / size / speed by changing
/// the values below (fed in through `css_vars`) without touching the HTML/CSS
/// structure. Missing or invalid values fall back to cyberpunk defaults.
pub fn default_css_vars() -> String {
    let mut vars = String::new();
    vars.push_str("--bg:#05070d;");
    vars.push_str("--fg:#ffffff;");
    vars.push_str("--accent-cyan:#39e6ff;");
    vars.push_str("--accent-magenta:#ff3de6;");
    vars.push_str("--accent-orange:#ffe3b3;");
    vars.push_str("--glow-cyan-1:0 0 6px var(--accent-cyan);");
    vars.push_str("--glow-cyan-2:0 0 18px var(--accent-cyan);");
    vars.push_str("--glow-cyan-3:0 0 40px var(--accent-cyan);");
    vars.push_str("--glow-magenta-1:0 0 6px var(--accent-magenta);");
    vars.push_str("--glow-magenta-2:0 0 18px var(--accent-magenta);");
    vars.push_str("--glow-magenta-3:0 0 40px var(--accent-magenta);");
    vars.push_str("--font-family:'JetBrains Mono', 'Fira Code', 'Courier New', monospace;");
    vars.push_str("--font-size:37px;");
    vars.push_str("--letter-spacing:0.02em;");
    vars.push_str("--line-height:1.5;");
    vars.push_str("--max-width:760px;");
    vars.push_str("--typewriter-ms:80;");
    vars.push_str("--typewriter-ease:cubic-bezier(.22,.61,.36,1);");
    vars.push_str("--typewriter-slide:38px;");
    vars.push_str("--typewriter-opacity-start:0;");
    vars.push_str("--typewriter-opacity-end:1;");
    vars.push_str("--cursor-char:'▍';");
    vars.push_str("--cursor-blink:0.9s;");
    vars.push_str("--cursor-color:#ffffff;");
    vars.push_str("--glitch-intensity:6px;");
    vars.push_str("--glitch-ms:6500ms;");
    vars.push_str("--glitch-duration:500ms;");
    vars.push_str("--glitch-color-cyan:137px 0 0 var(--accent-cyan);");
    vars.push_str("--glitch-color-magenta:-137px 0 0 var(--accent-magenta);");
    vars.push_str("--glitch-jitter:1.2px;");
    vars.push_str("--scanline-opacity:0.30;");
    vars.push_str("--scanline-color:#000;");
    vars.push_str("--scanline-size:calc(100vh / 216);");
    vars.push_str("--scanline-gap:calc(100vh / 216);");
    vars.push_str("--scanline-speed:90s;");
    vars.push_str("--scanline-steps:2160;");
    vars.push_str("--crt-perspective:600px;");
    vars.push_str("--crt-radius:40px;");
    vars.push_str("--crt-vignette-stops:transparent 0%, transparent 70%, var(--vignette-edge) 100%;");
    vars.push_str("--crt-vignette-blur:12px;");
    vars.push_str("--vignette-edge:#02030a;");
    vars.push_str("--center-padding:36px;");
    vars.push_str("--max-quote-chars:900;");
    vars
}

/// Build the full HTML document that each WebView loads.
///
/// `quote` is the currently selected quote. `css_vars` is a semicolon-separated
/// string of CSS custom-property declarations (e.g. `--accent-cyan:#39e6ff;`).
/// Values that are not present default to the cyberpunk theme defined above.
///
/// `cycle_interval_minutes` controls quote rotation:
///   - 0  → hold the single revealed quote indefinitely
///   - >0 → after each interval, fade out the current quote, slide in the next
///          (re-running the typewriter reveal), then repeat
pub fn build_html(quote: &QuoteSelection, css_vars: &str, cycle_interval_minutes: u64) -> String {
    let text = &quote.quote.text;
    let author = &quote.quote.author;
    let safe_text = escape_js_string(text);
    let safe_author = escape_js_string(author);

    let css_vars_block = if css_vars.trim().is_empty() {
        default_css_vars()
    } else {
        css_vars.to_string()
    };

    let cycle_minutes_js = if cycle_interval_minutes > 0 {
        cycle_interval_minutes.to_string()
    } else {
        "0".into()
    };

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>cyberquote</title>
<style>
:root {{
  {css_vars_block}
}}

* {{ box-sizing: border-box; }}

html, body {{
  margin: 0; padding: 0;
  height: 100%; width: 100%;
  background: var(--bg);
  color: var(--fg);
  font-family: var(--font-family);
  overflow: hidden;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}}

/* ---- CRT vignette ---- */
body {{
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--crt-radius);
  overflow: hidden;
  position: relative;
}}

body::before {{
  content: "";
  position: absolute;
  inset: 0;
  border-radius: var(--crt-radius);
  background:
    radial-gradient(circle at center, var(--crt-vignette-stops)),
    radial-gradient(circle at center,
      transparent 0%, transparent 70%, rgba(2,3,10,0.16) 100%
    );
  filter: blur(var(--crt-vignette-blur));
  pointer-events: none;
  z-index: 5;
}}

/* ---- scanline overlay ---- */
body::after {{
  content: "";
  position: absolute;
  top: -100vh;
  left: 0;
  right: 0;
  height: 200vh;
  background: repeating-linear-gradient(
    to bottom,
    var(--scanline-color) 0px,
    var(--scanline-color) var(--scanline-size),
    transparent var(--scanline-size),
    transparent calc(var(--scanline-size) + var(--scanline-gap))
  );
  opacity: var(--scanline-opacity);
  pointer-events: none;
  z-index: 7;
  will-change: transform;
  animation: scanline-sweep var(--scanline-speed) steps(var(--scanline-steps)) infinite;
}}

@keyframes scanline-sweep {{
  0% {{ transform: translateY(0); }}
  100% {{ transform: translateY(100vh); }}
}}

/* ---- quote stage ---- */
.stage {{
  position: relative;
  z-index: 4;
  padding: var(--center-padding);
  max-width: var(--max-width);
  width: 92%;
  text-align: left;
}}

/* author line */
.author {{
  margin-top: 26px;
  opacity: 0;
  animation: author-fade 0.8s var(--typewriter-ease) 200ms forwards;
  font-size: calc(var(--font-size) * 0.55);
  letter-spacing: 0.18em;
  text-transform: uppercase;
  color: var(--accent-orange);
text-shadow:
    0 0 7px rgba(255, 160, 40, 0.9),
    0 0 18px rgba(255, 130, 10, 0.7),
    0 0 36px rgba(255, 100, 0, 0.55),
    0 0 68px rgba(255, 90, 0, 0.4),
    0 0 120px rgba(255, 80, 0, 0.3);
}}

@keyframes author-fade {{
  0% {{ opacity: 0; transform: translateY(8px); }}
  100% {{ opacity: 0.85; transform: translateY(0); }}
}}

/* quote text — each span is one letter */
.quote {{
  display: block;
  font-size: var(--font-size);
  letter-spacing: var(--letter-spacing);
  line-height: var(--line-height);
  color: var(--fg);
  text-shadow:
    var(--glow-cyan-1),
    var(--glow-cyan-2),
    var(--glow-cyan-3),
    var(--glow-magenta-1),
    var(--glow-magenta-2);
  will-change: transform, opacity;
  margin: 0;
}}

/* per-letter typewriter reveal: slide-from-below + fade */
.quote .word {{
  display: inline-block;
  white-space: nowrap;
}}

.quote .letter {{
  display: inline-block;
  white-space: pre;
  opacity: var(--typewriter-opacity-start);
  transform: translateY(var(--typewriter-slide));
  animation: slide-up var(--typewriter-ms) var(--typewriter-ease) both;
  /* delay is applied inline by JS for per-letter staggering */
text-shadow:
    0 0 6px rgba(255, 255, 255, 0.85),
    0 0 14px rgba(255, 255, 255, 0.65),
    0 0 28px rgba(255, 255, 255, 0.45),
    0 0 52px rgba(255, 255, 255, 0.3);
}}

.quote .letter.revealed {{
  opacity: var(--typewriter-opacity-end);
  transform: translateY(0);
text-shadow:
    0 0 3px rgba(255, 255, 255, 0.65),
    0 0 8px rgba(255, 255, 255, 0.4),
    0 0 14px rgba(255, 255, 255, 0.2);
}}

@keyframes slide-up {{
  0% {{
    opacity: var(--typewriter-opacity-start);
    transform: translateY(var(--typewriter-slide));
  }}
  100% {{
    opacity: var(--typewriter-opacity-end);
    transform: translateY(0);
  }}
}}

/* ---- full-text glitch: RGB split + heavy jitter + negative snap-back ---- */
.stage.glitch {{
  animation: glitch-full var(--glitch-duration) ease-in-out both;
}}

@keyframes glitch-full {{
  0%   {{ transform: translateX(0); filter: none; text-shadow: none; clip-path: none; }}
  10%  {{ transform: translateX(-36px); filter: none; text-shadow: var(--glitch-color-cyan); clip-path: inset(20% 0 55% 0); }}
  20%  {{ transform: translateX(42px); filter: none; text-shadow: none; clip-path: inset(55% 0 10% 0); }}
  30%  {{ transform: translateX(-40px); filter: none; text-shadow: var(--glitch-color-magenta); clip-path: inset(8% 0 68% 0); }}
  40%  {{ transform: translateX(36px); filter: none; text-shadow: var(--glitch-color-cyan), var(--glitch-color-magenta), 0 0 7px var(--accent-cyan), 0 0 7px var(--accent-magenta); clip-path: inset(35% 0 42% 0); }}
  50%  {{ transform: translateX(-48px); filter: none; text-shadow: none; clip-path: inset(0 0 0 0); }}
  60%  {{ transform: translateX(44px); filter: none; text-shadow: var(--glitch-color-cyan); clip-path: inset(62% 0 8% 0); }}
  70%  {{ transform: translateX(-32px); filter: invert(1); text-shadow: none; clip-path: inset(18% 0 58% 0); }}
  80%  {{ transform: translateX(30px); filter: invert(1); text-shadow: var(--glitch-color-cyan), var(--glitch-color-magenta); clip-path: inset(42% 0 30% 0); }}
  90%  {{ transform: translateX(-14px); filter: none; text-shadow: none; clip-path: inset(0 0 0 0); }}
  95%  {{ transform: translateX(6px); filter: none; text-shadow: none; clip-path: none; }}
  100% {{ transform: translateX(0); filter: none; text-shadow: none; clip-path: none; }}
}}

/* ---- blinking cursor ---- */
.cursor {{
  display: inline-block;
  font-family: var(--font-family);
  font-size: var(--font-size);
  color: var(--cursor-color);
  content: var(--cursor-char);
  /* character driven by --cursor-char, applied inline by JS */
  animation: cursor-blink var(--cursor-blink) steps(2) infinite;
  vertical-align: middle;
  margin-left: 2px;
  text-shadow: 0 0 6px var(--cursor-color);
}}

@keyframes cursor-blink {{
  0%, 49% {{ opacity: 1; }}
  50%, 100% {{ opacity: 0; }}
}}

/* ---- fade transitions for quote cycling ---- */
.stage.fade-out .quote,
.stage.fade-out .author {{
  animation: fade-out 500ms var(--typewriter-ease) forwards;
}}
.stage.fade-in .quote {{
  animation: fade-in 600ms var(--typewriter-ease) 80ms both;
}}
.stage.fade-in .author {{
  animation: fade-in-author 600ms var(--typewriter-ease) 260ms both;
}}

@keyframes fade-out {{
  0% {{ opacity: var(--typewriter-opacity-end); transform: translateY(0); filter: blur(0px); }}
  100% {{ opacity: 0; transform: translateY(-8px); filter: blur(2px); }}
}}
@keyframes fade-in {{
  0% {{ opacity: 0; transform: translateY(12px); filter: blur(3px); }}
  100% {{ opacity: var(--typewriter-opacity-end); transform: translateY(0); filter: blur(0px); }}
}}
@keyframes fade-in-author {{
  0% {{ opacity: 0; transform: translateY(8px); }}
  100% {{ opacity: 0.85; transform: translateY(0); }}
}}
</style>
</head>
<body>
  <div class="stage" id="stage">
    <div class="quote" id="quote"></div>
    <div class="author" id="author"></div>
  </div>

<script>
(function () {{
  "use strict";

  // Wait for DOM to be ready before accessing elements
  function init() {{

    var stage = document.getElementById("stage");
    var quoteEl = document.getElementById("quote");
    var authorEl = document.getElementById("author");

    // ---- quote data (injected by Rust) ----
    var QUOTE_TEXT = {safe_text};
    var QUOTE_AUTHOR = {safe_author};
    var CYCLE_MINUTES = {cycle_minutes_js};

    // ---- letter typewriter ----
    function renderQuote(text, author) {{
      // split into words; wraps happen only at spaces, never mid-word
      var letters = [];

      quoteEl.innerHTML = "";

      var words = text.split(" ");
      for (var w = 0; w < words.length; w++) {{
        var wordSpan = document.createElement("span");
        wordSpan.className = "word";
        for (var c = 0; c < words[w].length; c++) {{
          var letter = document.createElement("span");
          letter.className = "letter";
          letter.textContent = words[w].charAt(c);
          wordSpan.appendChild(letter);
          letters.push(letter);
        }}
        quoteEl.appendChild(wordSpan);
        if (w < words.length - 1) {{
          var space = document.createElement("span");
          space.className = "letter space";
          space.textContent = " ";
          quoteEl.appendChild(space);
          letters.push(space);
        }}
      }}

      // build cursor — strip surrounding CSS string quotes from --cursor-char
      var rawChar = getComputedStyle(document.documentElement)
        .getPropertyValue("--cursor-char").trim();
      if ((rawChar.charAt(0) === "'" && rawChar.charAt(rawChar.length - 1) === "'") ||
          (rawChar.charAt(0) === '"' && rawChar.charAt(rawChar.length - 1) === '"')) {{
        rawChar = rawChar.slice(1, -1);
      }}
      var cursor = document.createElement("span");
      cursor.className = "cursor";
      cursor.textContent = rawChar || "\u258D";

      quoteEl.insertBefore(cursor, quoteEl.firstChild);

      // move cursor forward after each letter finishes its reveal animation
      for (var m = 0; m < letters.length; m++) {{
        (function (letter) {{
          function onReveal() {{
            letter.removeEventListener("animationend", onReveal);
            letter.parentNode.insertBefore(cursor, letter.nextSibling);
          }}
          letter.addEventListener("animationend", onReveal);
        }}(letters[m]));
      }}

      // per-letter delay: stagger evenly so longer quotes still feel alive
      var baseMs = parseFloat(getComputedStyle(document.documentElement)
        .getPropertyValue("--typewriter-ms")) || 80;

      for (var k = 0; k < letters.length; k++) {{
        letters[k].style.animationDelay = (k * baseMs) + "ms";
      }}

      // author
      authorEl.textContent = "\u2014 " + author;
    }}

    // ---- single quote load ----
    renderQuote(QUOTE_TEXT, QUOTE_AUTHOR);

    // ---- cycling ----
    var cycleTimer = null;
    if (CYCLE_MINUTES > 0) {{
      cycleTimer = setInterval(cycle, CYCLE_MINUTES * 60 * 1000);
    }}

    function cycle() {{
      if (stage.classList.contains("fade-out")) return;
      stage.classList.add("fade-out");
      setTimeout(function () {{
        renderQuote(QUOTE_TEXT, QUOTE_AUTHOR);
        stage.classList.remove("fade-out");
        stage.classList.add("fade-in");
        setTimeout(function () {{
          stage.classList.remove("fade-in");
        }}, 1200);
      }}, 520);
    }}

    // ---- random full-text glitch ----
    function randomBetween(min, max) {{
      return min + Math.random() * (max - min);
    }}

    function scheduleGlitch() {{
      setTimeout(triggerGlitch, randomBetween(3000, 25000));
    }}

    function triggerGlitch() {{
      if (stage.classList.contains("fade-out")) return scheduleGlitch();
      stage.classList.add("glitch");
      var durMs = parseFloat(getComputedStyle(document.documentElement)
        .getPropertyValue("--glitch-duration")) + 40;
      setTimeout(function () {{
        stage.classList.remove("glitch");
        scheduleGlitch();
      }}, durMs);
    }}

    scheduleGlitch();
  }}

  if (document.readyState === "loading") {{
    document.addEventListener("DOMContentLoaded", init);
  }} else {{
    init();
  }}
}})();
</script>
</body>
</html>"#,
        css_vars_block = css_vars_block,
        safe_text = safe_text,
        safe_author = safe_author,
        cycle_minutes_js = cycle_minutes_js,
    )
}

/// Simple HTML escaping for text injected into the document body.
pub fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\n', "<br>")
}

/// Escape a Rust string for safe use inside an inline JS string literal.
pub fn escape_js_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_html_basic() {
        let dangerous = "a <b> & \"c\"";
        let safe = escape_html(dangerous);
        assert_eq!(safe, "a &lt;b&gt; &amp; &quot;c&quot;");
    }

    #[test]
    fn test_js_escape_roundtrip() {
        let raw = "say \"hello\"\nand 'goodbye'";
        let js = escape_js_string(raw);
        assert!(js.starts_with('"') && js.ends_with('"'));
        let restored: String = eval_js_str(&js).unwrap_or_default();
        assert_eq!(restored, raw);
    }

    #[test]
    fn test_js_escape_controls() {
        let raw = "x\u{0000}y\u{0009}z";
        let js = escape_js_string(raw);
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
        for key in required.iter() {
            assert!(vars.contains(key), "default_css_vars missing {}", key);
        }
    }
}

/// Minimal JS literal roundtrip parser for the JS-escape test.
pub fn eval_js_str(code: &str) -> Option<String> {
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
