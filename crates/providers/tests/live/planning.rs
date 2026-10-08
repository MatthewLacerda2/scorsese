//! What a run will do before it does anything: keys, prices, the ceiling.
//!
//! Every plan here is built from supplied values, never the machine's own
//! environment — a test that found the maintainer's `.env` would be a test
//! holding real keys.

use scorsese_providers::credentials::{Budget, Environment, Provider, Settings};
use scorsese_providers::live::{Options, Report, Step, Verdict, permit, plan, run, total};

fn keys(pairs: &[(&str, &str)]) -> Environment {
    Environment::of(pairs.iter().copied())
}

#[test]
fn a_vendor_with_no_key_is_skipped_and_costs_nothing() {
    let planned = plan(
        &Options::default(),
        &Environment::default(),
        &Settings::default(),
    );
    assert_eq!(planned.len(), 4);
    for vendor in &planned {
        let why = vendor.skipped.as_deref().unwrap();
        assert!(why.starts_with("no key"), "{why}");
        assert!(
            why.contains(vendor.provider.variable()),
            "says where it looked: {why}"
        );
    }
    assert_eq!(total(&planned), 0);
}

/// The maintainer's case today: Gemini and ElevenLabs keys, no Anthropic one.
#[test]
fn only_the_vendors_with_keys_are_planned_and_priced() {
    let env = keys(&[("GEMINI_API_KEY", "g"), ("ELEVENLABS_API_KEY", "e")]);
    let planned = plan(&Options::default(), &env, &Settings::default());
    let cents: Vec<(Provider, u64, bool)> = planned
        .iter()
        .map(|p| (p.provider, p.cents, p.skipped.is_some()))
        .collect();
    assert_eq!(
        cents,
        [
            (Provider::Gemini, 5, false),
            (Provider::ElevenLabs, 2, false),
            (Provider::Anthropic, 0, true),
            (Provider::Pixabay, 0, true),
        ]
    );
    assert!(planned[0].calls.iter().any(|c| c.contains("free")));
}

#[test]
fn the_shot_is_priced_into_the_plan_only_when_asked_for() {
    let env = keys(&[("GEMINI_API_KEY", "g")]);
    let asked = Options {
        include_veo: true,
        ..Options::default()
    };
    assert_eq!(total(&plan(&asked, &env, &Settings::default())), 25);
}

/// The ceiling refuses the whole plan before anything is sent, `--yes` or
/// not — it is asked by `run` itself.
#[test]
fn a_plan_over_the_ceiling_is_refused_whole() {
    let env = keys(&[("ELEVENLABS_API_KEY", "e"), ("ANTHROPIC_API_KEY", "a")]);
    let planned = plan(&Options::default(), &env, &Settings::default());
    let over = permit(&planned, Budget::new(5, 0)).unwrap_err();
    assert_eq!(over.estimate, 17);
    assert!(permit(&planned, Budget::new(17, 0)).is_ok());
    assert!(permit(&planned, Budget::unlimited(0)).is_ok());
}

/// With no keys there is nothing to call, so this runs the real `run` with
/// no network at all: every vendor reports skipped, nothing is spent.
#[test]
fn a_run_with_no_keys_reports_every_vendor_skipped() {
    let planned = plan(
        &Options::default(),
        &Environment::default(),
        &Settings::default(),
    );
    let reports = run(
        &planned,
        &Options::default(),
        Budget::new(0, 0),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(reports.len(), 4);
    for report in &reports {
        assert!(
            matches!(report.verdict(), Verdict::Skipped { .. }),
            "{report:?}"
        );
        assert_eq!(report.cents, 0);
    }
}

fn report(verdicts: Vec<Verdict>) -> Report {
    Report {
        provider: Provider::Gemini,
        steps: verdicts
            .into_iter()
            .map(|v| Step::new("a call", v))
            .collect(),
        cents: 0,
        exchanges: Vec::new(),
    }
}

/// A vendor's verdict is its worst news: a failure over an unfinished call,
/// an unfinished call over a pass, and a pass over a skip.
#[test]
fn a_vendors_verdict_is_its_worst_step() {
    let skip = || Verdict::Skipped { why: String::new() };
    let refused = || Verdict::Refused {
        said: String::new(),
    };
    let unfinished = || Verdict::Unfinished {
        said: String::new(),
    };
    assert_eq!(report(vec![Verdict::Ok, skip()]).verdict(), &Verdict::Ok);
    assert_eq!(
        report(vec![Verdict::Ok, unfinished()]).verdict(),
        &unfinished()
    );
    assert_eq!(report(vec![unfinished(), refused()]).verdict(), &refused());
    assert_eq!(report(vec![skip()]).verdict(), &skip());
    assert!(!unfinished().is_failure() && !skip().is_failure() && refused().is_failure());
}
