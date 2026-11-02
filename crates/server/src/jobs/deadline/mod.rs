//! How long a local job may run (#1634): the timer the worker keeps beside
//! it, from outside the work ([`super::Allowance`] has the rule).
//!
//! The machine is shared, so one account's render must not hold a slot for
//! hours with nobody else's moving. At the deadline the job is stopped the
//! way its owner's stop stops it — its [`Cancel`] tripped, which kills its
//! ffmpeg and withdraws any capture it waits on — and once it has stopped it
//! ends [`Outcome::Failed`], which is never tried again.

use std::future::Future;
use std::time::{Duration, Instant};

use scorsese_render::Cancel;

use super::registry::Work;
use super::{Allowance, Outcome};

/// `running` to its end, unless it runs past `allowance` for the work it
/// measured in `work`: then `cancel` is tripped, and once it has stopped it
/// fails with [`overran`]'s words for `what` it was. One that finishes as
/// the deadline comes keeps what it made.
pub(super) async fn kept<E>(
    running: impl Future<Output = Result<Outcome, E>>,
    allowance: Allowance,
    work: &Work,
    cancel: &Cancel,
    what: &str,
) -> Result<Outcome, E> {
    let started = Instant::now();
    tokio::pin!(running);
    loop {
        // Asked again each time it wakes: the job may have measured its
        // work, or been granted time for its captures, since.
        let left = limit(allowance, work).saturating_sub(started.elapsed());
        if left.is_zero() {
            break;
        }
        tokio::select! {
            outcome = &mut running => return outcome,
            () = tokio::time::sleep(left) => {}
        }
    }
    let limit = limit(allowance, work);
    cancel.cancel();
    Ok(match running.await? {
        Outcome::Done(done) => Outcome::Done(done),
        _ => Outcome::Failed(overran(what, limit)),
    })
}

/// The job's deadline as it stands: its allowance for the work measured,
/// and whatever it was granted on top.
fn limit(allowance: Allowance, work: &Work) -> Duration {
    allowance.of(work.seconds()).saturating_add(work.extra())
}

/// Why a job that ran past `limit` was stopped, in its owner's words.
pub(super) fn overran(what: &str, limit: Duration) -> String {
    let minutes = limit.as_secs().div_ceil(60).max(1);
    let unit = if minutes == 1 { "minute" } else { "minutes" };
    format!("this {what} took longer than scorsese allows ({minutes} {unit}) and was stopped")
}

#[cfg(test)]
mod tests;
