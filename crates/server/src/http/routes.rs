//! Every route the API answers, declared once (#994).
//!
//! axum cannot list the routes a [`Router`](axum::Router) holds, so the list
//! comes first and the router is built from it — the shape `registry()` gives
//! the MCP tools. What that buys is a test: `docs/web.md`'s route tables are
//! held to [`table`], so a route added without a row, or a row left behind by
//! a route that went away, fails the build instead of misleading whoever reads
//! the page next.
//!
//! **Adding a route is one line here**, in the block of the feature it
//! belongs to, and one row in `docs/web.md`. Two lines with the same path and
//! different methods are one route with two methods, as axum merges them.

use axum::handler::Handler;
use axum::http::Method;
use axum::routing::{MethodFilter, MethodRouter, on};

use super::{
    AppState, account, chat, credits, editor, events, health, jobs, library, mcp, projects,
    renders, templates, tokens, uploads,
};

/// One method on one path, and what answers it.
pub struct Route {
    /// The HTTP method.
    pub method: Method,
    /// The path under `/api`, with `{name}` for each captured segment.
    pub path: &'static str,
    /// The handler, answering `method` alone.
    pub handler: MethodRouter<AppState>,
}

impl Route {
    fn new<H, T>(method: Method, path: &'static str, handler: H) -> Self
    where
        H: Handler<T, AppState>,
        T: 'static,
    {
        let filter = MethodFilter::try_from(method.clone())
            .unwrap_or_else(|_| panic!("{method} cannot be routed"));
        Self {
            method,
            path,
            handler: on(filter, handler),
        }
    }
}

/// Every route, under `/api`, grouped by the feature that added it.
pub fn table() -> Vec<Route> {
    use Method as M;
    vec![
        Route::new(M::GET, "/health", health),
        // Accounts (#533) and their API tokens (#539).
        Route::new(M::POST, "/login", account::login),
        Route::new(M::POST, "/logout", account::logout),
        Route::new(M::GET, "/me", account::me),
        Route::new(M::POST, "/me/password", account::change_password),
        Route::new(M::GET, "/tokens", tokens::list),
        Route::new(M::POST, "/tokens", tokens::issue),
        Route::new(M::DELETE, "/tokens/{id}", tokens::revoke),
        // Jobs, and the live stream that reports them.
        Route::new(M::GET, "/jobs", jobs::list),
        Route::new(M::GET, "/jobs/{id}", jobs::get),
        Route::new(M::POST, "/jobs/{id}/cancel", jobs::cancel),
        Route::new(M::GET, "/events", events::stream),
        // Projects.
        Route::new(M::GET, "/projects", projects::list),
        Route::new(M::POST, "/projects", projects::create),
        Route::new(M::GET, "/projects/{id}", projects::open),
        Route::new(M::PUT, "/projects/{id}", projects::save),
        Route::new(M::PATCH, "/projects/{id}", projects::rename),
        Route::new(M::DELETE, "/projects/{id}", projects::delete),
        // Web MCP (#539): only `POST` is served; the others say so.
        Route::new(M::POST, "/mcp", mcp::post),
        Route::new(M::GET, "/mcp", mcp::refuse),
        Route::new(M::DELETE, "/mcp", mcp::refuse),
        // Credits (#537).
        Route::new(M::GET, "/credits", credits::balance),
        Route::new(M::GET, "/credits/history", credits::history),
        // The library (#535): its files, and the tus uploads that fill it.
        Route::new(M::GET, "/library", library::list),
        Route::new(M::GET, "/library/{id}", library::details),
        Route::new(M::PATCH, "/library/{id}", library::update),
        Route::new(M::DELETE, "/library/{id}", library::delete),
        Route::new(M::GET, "/library/{id}/file", library::file),
        Route::new(M::GET, "/library/{id}/thumbnail", library::thumbnail),
        Route::new(M::OPTIONS, "/uploads", uploads::options),
        Route::new(M::POST, "/uploads", uploads::announce),
        Route::new(M::HEAD, "/uploads/{id}", uploads::progress),
        Route::new(M::PATCH, "/uploads/{id}", uploads::append),
        Route::new(M::DELETE, "/uploads/{id}", uploads::cancel),
        // Renders (#541) and previews (#542), which are renders at a preview
        // quality.
        Route::new(M::GET, "/projects/{id}/renders", renders::list),
        Route::new(M::POST, "/projects/{id}/renders", renders::request),
        Route::new(M::POST, "/projects/{id}/previews", renders::preview),
        Route::new(M::GET, "/renders/{id}/file", renders::file),
        // The assistant (#540): a project's conversation and its model (#705),
        // a turn's log, stopping one, and answering it.
        Route::new(M::GET, "/projects/{id}/chat", chat::conversation),
        Route::new(M::POST, "/projects/{id}/chat", chat::send),
        Route::new(M::PUT, "/projects/{id}/chat/model", chat::choose_model),
        Route::new(M::GET, "/chat/turns/{id}", chat::turn),
        Route::new(M::POST, "/chat/turns/{id}/stop", chat::stop),
        Route::new(M::POST, "/chat/turns/{id}/quote", chat::quote),
        Route::new(M::POST, "/chat/turns/{id}/answer", chat::answer),
        // The web editor's tools, and templates.
        Route::new(M::POST, "/projects/{id}/tools/{name}", editor::call),
        Route::new(M::GET, "/templates", templates::list),
        Route::new(M::DELETE, "/templates/{id}", templates::delete),
    ]
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::table;

    /// The page the tables live in, as the build sees it.
    const WEB_MD: &str = include_str!("../../../../docs/web.md");

    /// Every `` | `METHOD /api/…` | `` row in `docs/web.md`, as
    /// `(METHOD, path under /api)`.
    fn documented() -> BTreeSet<(String, String)> {
        WEB_MD
            .lines()
            .filter_map(|line| line.strip_prefix("| `"))
            .filter_map(|rest| rest.split_once('`').map(|(cell, _)| cell))
            .filter_map(|cell| cell.split_once(" /api"))
            .filter(|(method, _)| method.chars().all(|c| c.is_ascii_uppercase()))
            .map(|(method, path)| (method.to_owned(), path.to_owned()))
            .collect()
    }

    fn routed() -> BTreeSet<(String, String)> {
        table()
            .into_iter()
            .map(|route| (route.method.to_string(), route.path.to_owned()))
            .collect()
    }

    #[test]
    fn every_route_has_a_row_in_web_md() {
        let missing: Vec<_> = routed().difference(&documented()).cloned().collect();
        assert!(
            missing.is_empty(),
            "routes with no row in docs/web.md: {missing:?}"
        );
    }

    #[test]
    fn every_row_in_web_md_is_a_route() {
        let stale: Vec<_> = documented().difference(&routed()).cloned().collect();
        assert!(
            stale.is_empty(),
            "docs/web.md rows for routes the server does not have: {stale:?}"
        );
    }

    #[test]
    fn the_rows_are_found_at_all() {
        // A change to how the page writes its rows must not pass both tests
        // above by finding nothing.
        assert!(documented().len() >= table().len());
    }

    #[test]
    fn no_route_is_declared_twice() {
        assert_eq!(routed().len(), table().len());
    }
}
