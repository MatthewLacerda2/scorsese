//! Keeping the rate fresh: a loop beside the server, like the fee sweep.

use tokio::sync::watch;

use super::Payments;
use scorsese_providers::fx::REFRESH_EVERY;

/// Read the rate now and every [`REFRESH_EVERY`] until `stop`, keeping the
/// last good one through a failed read and logging the failure.
pub async fn refresh(payments: Payments, mut stop: watch::Receiver<bool>) {
    loop {
        let rates = payments.rates.clone();
        match tokio::task::spawn_blocking(move || rates.refresh()).await {
            Ok(Ok(_)) => {}
            Ok(Err(error)) => {
                eprintln!("scorsese-server: the dollar's rate was not refreshed: {error}");
            }
            Err(error) => eprintln!("scorsese-server: the rate refresh was cut off: {error}"),
        }
        tokio::select! {
            _ = stop.wait_for(|stopping| *stopping) => return,
            () = tokio::time::sleep(REFRESH_EVERY) => {}
        }
    }
}
