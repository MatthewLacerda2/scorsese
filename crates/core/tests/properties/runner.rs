//! One way to run a property, with the seed nailed down.
//!
//! `proptest!` draws its seed from entropy, which makes a failure a thing
//! that happened rather than a thing that reproduces. Everything here goes
//! through [`check`] instead: a fixed case count and
//! [`TestRng::deterministic_rng`], so a run is a function of the source and
//! the tool version and nothing else. Re-running a red build with no change
//! is not a strategy that works against this target, which is the point.

use proptest::strategy::Strategy;
use proptest::test_runner::{
    Config, FileFailurePersistence, RngAlgorithm, TestCaseError, TestRng, TestRunner,
};

/// How many inputs each property is examined at.
///
/// Fixed rather than left to `PROPTEST_CASES`, so the environment cannot
/// change what a green run means. A thousand cases of integer arithmetic over
/// a crate whose whole suite answers in under a second costs nothing anyone
/// will notice; the number is worth raising only alongside a measurement.
const CASES: u32 = 1024;

/// Examines `property` at [`CASES`] inputs drawn from `strategy`, and at every
/// input a previous run wrote down.
///
/// `source_file` is the caller's own `file!()`. It is how the regression file
/// finds its way beside the test that produced it rather than into whichever
/// directory the suite was run from.
pub(crate) fn check<S: Strategy>(
    source_file: &'static str,
    strategy: S,
    property: impl Fn(S::Value) -> Result<(), TestCaseError>,
) {
    let config = Config {
        cases: CASES,
        source_file: Some(source_file),
        // Committed, not gitignored: the input that broke a claim once is the
        // best example test there is, and it is only written down here.
        failure_persistence: Some(Box::new(FileFailurePersistence::SourceParallel(
            "proptest-regressions",
        ))),
        ..Config::default()
    };
    let mut runner =
        TestRunner::new_with_rng(config, TestRng::deterministic_rng(RngAlgorithm::ChaCha));
    if let Err(failure) = runner.run(&strategy, property) {
        // The shrunken input and the assertion that objected to it, which is
        // the whole of what a reader needs to reproduce this by hand.
        panic!("{failure}");
    }
}
