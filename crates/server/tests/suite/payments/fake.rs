//! Stripe as the payments tests need it: the sessions it made, paid or
//! expired on a test's say-so. Nothing reaches a network.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use scorsese_providers::api::http::HttpError;
use scorsese_providers::payments::{Checkout, Order, Session};

/// Stripe as a test needs it: sessions it made, and their state.
#[derive(Default)]
pub(crate) struct Fake {
    pub(crate) created: Mutex<Vec<Order>>,
    sessions: Mutex<Vec<Session>>,
    /// Whether the next expire finds its session just paid, as when the
    /// person pays in another tab between our retrieve and our expire.
    pub(crate) paid_before_expire: AtomicBool,
}

impl Fake {
    /// Make session `id` read as paid, as Stripe would once the card cleared.
    pub(crate) fn pay(&self, id: &str) -> Session {
        let mut sessions = self.sessions.lock().expect("no test panics holding it");
        let session = sessions
            .iter_mut()
            .find(|s| s.id == id)
            .expect("only a session it made is paid");
        session.status = Some("complete".into());
        session.payment_status = "paid".into();
        session.clone()
    }

    /// Forget every session, as Stripe does once its key is another
    /// account's.
    pub(crate) fn forget(&self) {
        self.sessions
            .lock()
            .expect("no test panics holding it")
            .clear();
    }

    pub(crate) fn created(&self) -> usize {
        self.created
            .lock()
            .expect("no test panics holding it")
            .len()
    }
}

impl Checkout for Fake {
    fn create(&self, order: &Order) -> Result<Session, HttpError> {
        let mut created = self.created.lock().expect("no test panics holding it");
        created.push(order.clone());
        let session = Session {
            id: format!("cs_test_{}", created.len()),
            url: Some(format!(
                "https://checkout.stripe.com/c/pay/{}",
                created.len()
            )),
            status: Some("open".into()),
            payment_status: "unpaid".into(),
            amount_total: Some(order.centavos),
            currency: Some("brl".into()),
        };
        self.sessions
            .lock()
            .expect("no test panics holding it")
            .push(session.clone());
        Ok(session)
    }

    fn retrieve(&self, id: &str) -> Result<Session, HttpError> {
        let sessions = self.sessions.lock().expect("no test panics holding it");
        let session = sessions.iter().find(|s| s.id == id);
        session.cloned().ok_or_else(|| HttpError::Refused {
            url: format!("https://api.stripe.com/v1/checkout/sessions/{id}"),
            status: 404,
            body: "No such checkout.session".into(),
        })
    }

    fn expire(&self, id: &str) -> Result<Session, HttpError> {
        if self.paid_before_expire.swap(false, Ordering::SeqCst) {
            self.pay(id);
        }
        let mut sessions = self.sessions.lock().expect("no test panics holding it");
        let session = sessions.iter_mut().find(|s| s.id == id);
        let session = session.expect("only a session it made is expired");
        if session.status.as_deref() != Some("open") {
            // Stripe expires only an open session.
            return Err(HttpError::Refused {
                url: format!("https://api.stripe.com/v1/checkout/sessions/{id}/expire"),
                status: 400,
                body: "This Checkout Session is not in an expirable state.".into(),
            });
        }
        session.status = Some("expired".into());
        session.url = None;
        Ok(session.clone())
    }
}
