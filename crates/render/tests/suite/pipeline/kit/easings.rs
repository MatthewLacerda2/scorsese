//! The kit's overshooting easings are native keyframes' own curves (#1064):
//! each is evaluated in the page at the same progresses as `Easing::apply`,
//! and a curve drifting from its native twin, or a spring that no longer
//! overshoots, is named by the page that found it.

use scorsese_core::Easing;

use super::passes;

/// Progress from 0 to 1 in twentieths: fine enough that the spring's swing
/// over and back under the mark both fall on one.
fn progresses() -> Vec<f64> {
    (0..=20).map(|i| f64::from(i) / 20.0).collect()
}

/// The JS easing `name` against `native` at every progress, to a billionth.
fn agrees(name: &str, native: Easing) -> String {
    let expected: Vec<f64> = progresses().iter().map(|&p| native.apply(p)).collect();
    format!(
        "{{ const by = {name}; {expected:?}.forEach((want, i) => {{
            const got = by(i / 20);
            if (!(Math.abs(got - want) < 1e-9)) throw new Error(`{name} at ${{i / 20}}: ${{got}}, native says ${{want}}`);
        }}); }}\n"
    )
}

#[test]
fn the_kit_s_overshooting_easings_are_the_native_curves() {
    let spring: Vec<f64> = progresses()
        .iter()
        .map(|&p| Easing::Spring.apply(p))
        .collect();
    assert!(
        spring.iter().any(|&v| v > 1.1) && spring.iter().any(|&v| v < 1.0 && v > 0.9),
        "the native spring passes the mark and swings back under: {spring:?}"
    );
    let curves = [
        ("kit.ease.back", Easing::BackOut),
        ("kit.ease.backIn", Easing::BackIn),
        ("kit.ease.backInOut", Easing::BackInOut),
        ("kit.ease.spring", Easing::Spring),
        (
            "kit.ease.cubicBezier(0.34, 1.56, 0.64, 1)",
            Easing::CubicBezier([0.34, 1.56, 0.64, 1.0]),
        ),
        (
            "kit.ease.cubicBezier(0.25, 0.1, 0.25, 1)",
            Easing::CubicBezier([0.25, 0.1, 0.25, 1.0]),
        ),
        (
            "kit.ease.cubicBezier(0, 0.5, 0, 0.5)",
            Easing::CubicBezier([0.0, 0.5, 0.0, 0.5]),
        ),
    ];
    let mut checks: String = curves
        .iter()
        .map(|&(name, native)| agrees(name, native))
        .collect();
    checks.push_str(
        r#"let refused = "";
  try { kit.ease.cubicBezier(1.2, 0, 0.5, 1); } catch (e) { refused = e.message; }
  if (!refused.includes("x1 and x2 must lie in 0..1")) throw new Error(`an x past 1 was not refused: "${refused}"`);
  if (kit.enter(0.3, 0, 1, kit.ease.spring) <= 1) throw new Error("enter clamps what a spring answers");"#,
    );
    passes("kit-easings", &checks);
}
