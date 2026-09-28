//! The login's brake (`src/accounts/throttle`, #557): the rules as arithmetic,
//! the table they are kept in, and `POST /api/login` refusing with them.

#[path = "../common/mod.rs"]
mod common;

mod counter;
mod login;
mod operator;
mod store;

use std::net::SocketAddr;

use common::{Response, request};
use scorsese_server::db;
use scorsese_server::http::{self, client::Clients};
use serde_json::json;
use sqlx::postgres::PgPool;

/// A migrated server on `pool` believing `clients` for where a request came
/// from.
async fn serve(pool: PgPool, clients: Clients) -> SocketAddr {
    let (listener, address) = common::listener().await;
    db::migrate(&pool).await.expect("the migrations apply");
    let members = db::member_pool(&pool)
        .await
        .expect("the member pool connects");
    let state = http::AppState::new(members, common::files("throttle")).with_clients(clients);
    tokio::spawn(http::serve(
        listener,
        http::router(state),
        std::future::pending(),
    ));
    address
}

/// A login attempt, with extra header lines.
async fn login(address: SocketAddr, email: &str, password: &str, headers: &[&str]) -> Response {
    let body = json!({ "email": email, "password": password });
    request(address, "POST", "/api/login", headers, Some(&body)).await
}
