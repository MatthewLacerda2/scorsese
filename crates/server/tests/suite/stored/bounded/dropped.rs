//! A call whose request is dropped while its tool runs.

use std::sync::Arc;
use std::time::{Duration, Instant};

use scorsese_render::Cancel;
use scorsese_server::tools::Client;
use serde_json::json;
use sqlx::postgres::PgPool;

use super::{Gate, Opens, arrived, held, laid_out};

#[sqlx::test]
async fn a_call_whose_request_is_dropped_is_stopped_and_its_folder_outlives_the_work(pool: PgPool) {
    let gate = Arc::new(Gate::default());
    let (state, ana, id, scratch) = held(&pool, "bounded-dropped", &gate).await;
    let cancel = Cancel::new();
    let request = {
        let (state, cancel) = (state.clone(), cancel.clone());
        let search = json!({ "project": id, "query": "sea" });
        tokio::spawn(async move {
            let client = Client::External { token: None };
            let tools = &state.tools;
            tools
                .call_cancellable(ana, client, "stock_search", &search, &cancel)
                .await
        })
    };

    arrived(&gate, 1).await;
    // Opened however the test ends, or its search's thread outlives it.
    let _opens = Opens(gate.clone());
    request.abort();
    assert!(request.await.is_err_and(|ended| ended.is_cancelled()));
    assert!(cancel.is_cancelled(), "the tool is told to stop");
    assert_eq!(laid_out(&scratch), 1, "its folder is kept while it runs");

    gate.open();
    let asked = Instant::now();
    while laid_out(&scratch) > 0 {
        assert!(
            asked.elapsed() < Duration::from_secs(20),
            "the folder goes once the work ends"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}
