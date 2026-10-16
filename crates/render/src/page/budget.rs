//! How many browsers may draw at once, shared by every capture a run makes
//! together (#1095).
//!
//! One capture alone splits a long page across the machine's browsers
//! ([`super::pieces`]), but a page under eight seconds is worth only one or
//! two of them. A render of a dozen short pages captured one after another
//! therefore left most of the machine idle. Captured together, they would each
//! start the browsers they are worth, and a dozen at once is far more process
//! trees than the machine has cores. So the captures of one run lease their
//! browsers from one [`Browsers`]: each takes what is free, up to what it is
//! worth, and waits only when none is.
//!
//! How many browsers drew a capture never changes its frames: the pieces join
//! into the file one browser would have drawn ([`super::pieces`]), so a lease
//! smaller than asked costs time, never a different picture.

use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};

/// A number of browsers, shared by the captures drawing at the same time.
#[derive(Debug)]
pub struct Browsers {
    most: usize,
    free: Mutex<usize>,
    freed: Condvar,
}

impl Browsers {
    /// A budget of `most` browsers at once — never fewer than one, or nothing
    /// could ever be drawn.
    pub fn new(most: usize) -> Self {
        let most = most.max(1);
        Self {
            most,
            free: Mutex::new(most),
            freed: Condvar::new(),
        }
    }

    /// The most browsers drawing at once.
    pub fn most(&self) -> usize {
        self.most
    }

    /// Up to `wanted` browsers, as many as are free — at least one, waiting
    /// for one to be given back when none is.
    pub(crate) fn take(&self, wanted: usize) -> Lease<'_> {
        let mut free = self.free();
        while *free == 0 {
            free = self
                .freed
                .wait(free)
                .unwrap_or_else(PoisonError::into_inner);
        }
        let count = wanted.clamp(1, *free);
        *free -= count;
        Lease {
            browsers: self,
            count,
        }
    }

    /// The count, locked. A capture that panicked holding the lock left a
    /// number that is still a number.
    fn free(&self) -> MutexGuard<'_, usize> {
        self.free.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Browsers taken from a [`Browsers`], given back when it is dropped.
#[derive(Debug)]
pub(crate) struct Lease<'b> {
    browsers: &'b Browsers,
    count: usize,
}

impl Lease<'_> {
    /// How many browsers this lease may draw with.
    pub(crate) fn count(&self) -> usize {
        self.count
    }
}

impl Drop for Lease<'_> {
    fn drop(&mut self) {
        *self.browsers.free() += self.count;
        self.browsers.freed.notify_all();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use super::*;

    #[test]
    fn a_lease_takes_what_is_free_up_to_what_it_wants_and_never_none() {
        let browsers = Browsers::new(4);
        let three = browsers.take(3);
        assert_eq!(three.count(), 3);
        assert_eq!(browsers.take(2).count(), 1, "only one was free");
        assert_eq!(browsers.take(0).count(), 1, "asking for none takes one");
        drop(three);
        assert_eq!(browsers.take(9).count(), 4, "all of them, given back");
    }

    #[test]
    fn a_budget_of_none_is_a_budget_of_one() {
        assert_eq!(Browsers::new(0).most(), 1);
    }

    #[test]
    fn captures_together_never_draw_with_more_than_the_budget() {
        let browsers = Browsers::new(3);
        let drawing = AtomicUsize::new(0);
        let most_seen = AtomicUsize::new(0);
        std::thread::scope(|scope| {
            for wanted in [1, 2, 3, 1, 2, 4, 1, 1] {
                let (browsers, drawing, most_seen) = (&browsers, &drawing, &most_seen);
                scope.spawn(move || {
                    let lease = browsers.take(wanted);
                    let now = drawing.fetch_add(lease.count(), Ordering::SeqCst) + lease.count();
                    most_seen.fetch_max(now, Ordering::SeqCst);
                    std::thread::sleep(Duration::from_millis(5));
                    drawing.fetch_sub(lease.count(), Ordering::SeqCst);
                });
            }
        });
        let most = most_seen.load(Ordering::SeqCst);
        assert!((1..=3).contains(&most), "at most three at once: {most}");
    }
}
