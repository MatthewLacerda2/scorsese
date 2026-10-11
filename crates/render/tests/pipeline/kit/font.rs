//! `kit.font` names what is missing (#1142): a family it does not ship, and a
//! family it ships without the cut asked for, are two different errors.

use super::passes;

#[test]
fn an_italic_a_family_lacks_is_named_as_that() {
    passes(
        "kit-font",
        r#"const said = (promise) => promise.then(() => "found", (e) => e.message);
  window.answers ??= Promise.all([said(kit.font("Anton", { italic: true })), said(kit.font("Nope"))])
    .then((got) => { window.got = got; });
  if (!window.got) return;
  const [anton, nope] = window.got;
  if (anton !== "kit.font: Anton ships no italic cut") throw new Error(`Anton italic: ${anton}`);
  if (!nope.startsWith("kit.font: no shipped face is called Nope (there are ")) throw new Error(`Nope: ${nope}`);"#,
    );
}
