//! The kit's path helpers (#1067): `kit.along`, an arrowhead that rides the
//! end `kit.draw` is drawing, and `kit.erase`, all on `kit.draw`'s measuring.

use super::passes;

#[test]
fn along_lands_on_the_path_by_distance_and_turns_with_it() {
    passes(
        "kit-along",
        r#"const is = (got, want, what) => {
    if (got !== want) throw new Error(`${what}: ${JSON.stringify(got)}, not ${JSON.stringify(want)}`);
  };
  const board = kit.svg("svg", { viewBox: "0 0 200 200" }, document.body);
  const route = kit.svg("path", { d: "M 0 0 H 100 V 100", fill: "none" }, board);
  const dot = kit.svg("circle", { r: 4 }, board);
  kit.along(dot, route, 0.5);
  is(dot.getAttribute("transform"), "translate(100 0)", "halfway is the corner");
  kit.along(dot, route, 0.75, { orient: true });
  is(dot.getAttribute("transform"), "translate(100 50) rotate(90)", "a quarter on, heading down");"#,
    );
}

#[test]
fn a_marker_end_rides_the_drawn_end_and_erase_takes_from_the_start() {
    passes(
        "kit-heads",
        r##"const is = (got, want, what) => {
    if (got !== want) throw new Error(`${what}: ${JSON.stringify(got)}, not ${JSON.stringify(want)}`);
  };
  document.body.innerHTML = `<svg viewBox="0 0 200 40">
    <defs><marker id="head" viewBox="0 0 10 10" refX="5" refY="5" markerWidth="4" markerHeight="4" orient="auto">
      <path d="M 0 0 L 10 5 L 0 10 z" fill="#fff" /></marker></defs>
    <path id="line" d="M 0 10 H 100" stroke="#fff" stroke-width="2" marker-end="url(#head)" />
    <path id="gone" d="M 0 30 H 100" stroke="#fff" stroke-width="2" />
  </svg>`;
  const line = document.getElementById("line");
  is(kit.draw(line, 0.5, 0, { within: 1 }), 1, "drawn within a second");
  const head = document.querySelector("[data-kit-head] g");
  is(head.getAttribute("transform"), "translate(50 10) rotate(0) scale(0.8) translate(-5 -5)", "the head at the drawn end");
  is(getComputedStyle(line).markerEnd, "none", "the marker itself is off");
  is(document.querySelectorAll("[data-kit-head] path").length, 1, "one head, not one a frame");
  kit.draw(line, 0.5, 0, { within: 1 });
  is(document.querySelectorAll("[data-kit-head]").length, 1, "dressed once");
  const gone = document.getElementById("gone");
  kit.erase(gone, 0.5, 0, { within: 1 });
  is(gone.style.strokeDashoffset, "-51", "half of the line and its width taken from the start");
  kit.erase(gone, 1, 0, { within: 1 });
  is(gone.style.visibility, "hidden", "all gone");"##,
    );
}
