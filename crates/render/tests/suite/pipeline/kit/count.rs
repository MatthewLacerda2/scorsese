//! `kit.count` catches up with the native counter (#1065): grouping that can
//! be turned off, an easing that shapes when the count arrives but never what
//! it says, and a figure padded so a centred line holds still as it counts.

use super::passes;

#[test]
fn a_count_groups_on_request_never_overshoots_and_holds_its_width() {
    passes(
        "kit-count",
        r#"const is = (got, want, what) => {
    if (got !== want) throw new Error(`${what}: ${JSON.stringify(got)}, not ${JSON.stringify(want)}`);
  };
  is(kit.count(1, 0, 1, 2026, { pad: false }), "2,026", "grouped by default");
  is(kit.count(1, 0, 1, 2026, { grouping: false }), "2026", "a year, ungrouped");
  is(kit.count(0, 0, 1, 1234), "\u2007\u2008\u2007\u20070", "padded to 1,234's columns");
  is(kit.count(0, 0, 1, 1234, { pad: false }), "0", "unpadded");
  is(kit.count(0.5, 0, 1, 1234, { from: 10000, decimals: 1 }), "\u20072,329.8", "padded to the wider end");
  is(kit.count(1, 0, 1, -0.2, { pad: false }), "0", "no minus on nothing");
  for (let i = 0; i <= 40; i++) {
    const shown = Number(kit.count(i / 40, 0, 1, 144, { by: kit.ease.spring, pad: false }));
    if (shown > 144) throw new Error(`a spring count showed ${shown} on its way to 144`);
  }
  is(kit.count(0.3, 0, 1, 144, { by: kit.ease.spring }), "144", "arrives early and holds");
  const line = document.createElement("span");
  line.style.font = "800 220px Montserrat";
  line.style.fontVariantNumeric = "tabular-nums";
  document.body.append(line);
  const width = (t) => {
    line.textContent = kit.count(t, 0, 1, 1250000);
    return line.getBoundingClientRect().width;
  };
  const whole = width(1);
  for (const t of [0, 0.01, 0.1, 0.4]) {
    if (Math.abs(width(t) - whole) > 0.5) throw new Error(`at ${t} the line is ${width(t)} px, not ${whole}`);
  }
  line.remove();"#,
    );
}
