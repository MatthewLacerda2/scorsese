//! A dashed line `kit.draw` has drawn halfway still shows its gaps (#1067):
//! the author's dashes are laid down as it draws, never overwritten.

use super::drawn;

#[test]
fn a_dashed_line_drawn_halfway_keeps_its_gaps() {
    let page = r##"<!doctype html>
<html><head>
<script src="https://lib.scorsese/kit.js"></script>
<style>html, body { margin: 0; background: #000; } svg { display: block; width: 100vw; height: 100vh; }</style>
</head><body>
<svg viewBox="0 0 200 40" preserveAspectRatio="none">
  <path id="line" d="M 0 20 H 200" stroke="#fff" stroke-width="10" stroke-dasharray="10 10" />
</svg>
<script>
  const line = document.getElementById("line");
  kit.frame((t) => kit.draw(line, t, 0, { within: 1 }));
</script>
</body></html>"##;
    let frame = drawn("kit-dashes", page, (200, 40), 15);
    let lit: Vec<bool> = (0..200)
        .map(|x| frame.bytes()[(20 * 200 + x) * 4] > 128)
        .collect();
    let (drawn_half, rest) = lit.split_at(100);
    let dashes = drawn_half[..90].windows(2).filter(|w| w[0] != w[1]).count();
    assert!(dashes >= 8, "gaps in the drawn half: {drawn_half:?}");
    assert!(
        rest[10..].iter().all(|on| !on),
        "nothing past the drawn end: {rest:?}"
    );
}
