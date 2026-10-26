//! `kit.reveal` (#1066): text that arrives a character, word or line at a
//! time, cut once into pieces the way native text's reveal cuts it.

use super::passes;

/// What every check here leans on: `is`, and `el`, a fresh element with
/// `html` inside, `width` px wide, in the page.
const SETUP: &str = r#"const is = (got, want, what) => {
    if (got !== want) throw new Error(`${what}: ${JSON.stringify(got)}, not ${JSON.stringify(want)}`);
  };
  const el = (html, width = 1000) => {
    const div = document.createElement("div");
    div.style.cssText = `font: 400 20px Inter; width: ${width}px; margin: 200px`;
    div.innerHTML = html;
    document.body.append(div);
    return div;
  };
  const shown = (div) => [...div.querySelectorAll("span")].filter((s) => !s.children.length)
    .map((s) => [s.textContent, s.style.opacity]);"#;

#[test]
fn a_grapheme_cluster_is_one_character_and_the_text_reads_the_same() {
    passes(
        "kit-reveal-chars",
        &format!(
            r#"{SETUP}
  const text = "👋🏽 été 🇧🇷";
  const div = el(text);
  kit.reveal(div, 0, 0, {{ unit: "char" }});
  is(shown(div).map(([piece]) => piece).join("|"), "👋🏽|é|t|é|🇧🇷", "one piece a cluster");
  is(div.textContent, text, "the text reads as it did");
  let threw = "";
  try {{ kit.reveal(div, 0, 0, {{ unit: "word" }}); }} catch (e) {{ threw = e.message; }}
  if (!threw.includes("already cut by char")) throw new Error(`a second cut said ${{threw}}`);"#
        ),
    );
}

#[test]
fn words_keep_their_punctuation_and_markup_and_arrive_in_turn() {
    passes(
        "kit-reveal-words",
        &format!(
            r#"{SETUP}
  const div = el("Hello, <em>big</em> wide world");
  const lands = kit.reveal(div, 0.4, 0, {{ length: 0.4, by: kit.ease.linear }});
  is(lands, 0.8, "the last of three lands a stagger of half a length apart");
  is(JSON.stringify(shown(div)), JSON.stringify([["Hello,", "1"], ["big", "0.5"], ["wide world", "0"]]), "in turn");
  is(div.querySelector("em span").textContent, "big", "the em is kept");
  is(div.textContent, "Hello, big wide world", "the text reads as it did");
  const first = div.querySelector("span");
  is(first.style.transform, "translateY(0px)", "a landed word is in place");
  is(div.querySelectorAll("span")[2].style.transform, "translateY(4px)", "rising a fifth of 20 px");"#
        ),
    );
}

#[test]
fn lines_arrive_whole_one_after_another() {
    passes(
        "kit-reveal-lines",
        &format!(
            r#"{SETUP}
  const div = el("one two three four five six seven eight", 120);
  kit.reveal(div, 1.5, 0, {{ unit: "line", stagger: 1, length: 1, rise: 0, by: kit.ease.linear }});
  const rows = new Map();
  for (const [piece, opacity] of shown(div)) {{
    const top = [...div.querySelectorAll("span")].find((s) => s.textContent === piece).offsetTop;
    rows.set(top, [...(rows.get(top) ?? []), opacity]);
  }}
  const seen = [...rows.values()].map((row) => [...new Set(row)].join());
  if (rows.size < 3) throw new Error(`only ${{rows.size}} lines at 120 px`);
  is(seen.slice(0, 3).join("|"), "1|0.5|0", "each line at one opacity, in turn");"#
        ),
    );
}
