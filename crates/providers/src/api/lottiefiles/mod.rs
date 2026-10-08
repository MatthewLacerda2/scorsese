//! LottieFiles' public GraphQL API: searching its free animations (#903).
//!
//! One endpoint, two queries. `searchPublicAnimations` finds animations by
//! words; `publicAnimation` reads one back by id. Both are answered
//! **anonymously** — no account, no key — which is the API LottieFiles' own
//! MCP server exposes as its "GraphQL workbench", checked on 2026-10-08.
//!
//! What is particular to this vendor:
//!
//! - **A refusal arrives as `200`**, with `errors` beside (or instead of)
//!   `data` — GraphQL's way. So [`Reply::into_data`] is what turns one into
//!   an [`HttpError`], naming what the server said.
//! - **Paging is by cursor** (`first` / `after`). Nothing here holds a cursor:
//!   a later page is asked for as the first `n` results and the tail kept
//!   ([`crate::stock`]'s business), which the endpoint allows up to 200.
//! - **An animation's size, rate and length are in `metadata`**, as floats,
//!   and may be missing on an old upload.
//! - **The files are public** on LottieFiles' CDN: the Lottie JSON
//!   (`jsonUrl`), a still of one frame (`imageUrl`, a PNG on white), a GIF
//!   and an MP4 of it playing. Nothing is signed.

use serde::{Deserialize, Serialize};

use crate::api::http::{Caller, HttpError};

/// The endpoint, versioned by date as LottieFiles versions it.
pub const ENDPOINT: &str = "https://graphql.lottiefiles.com/2022-08";

/// The most results one search hands out at once.
pub const MAX_FIRST: u32 = 200;

/// The fields read off an animation — the same for a search and a lookup.
/// A macro so the two documents below are constants, written out whole.
macro_rules! fields {
    () => {
        "id name description jsonUrl gifUrl imageUrl videoUrl url lottieFileSize \
         createdBy { username firstName lastName } \
         metadata { width height frameRate duration }"
    };
}

/// A search by words, the first `first` results.
const SEARCH: &str = concat!(
    "query Search($query: String!, $first: Int!) { ",
    "searchPublicAnimations(query: $query, first: $first) { ",
    "totalCount edges { node { ",
    fields!(),
    " } } } }"
);

/// One animation by its id.
const ONE: &str = concat!(
    "query One($id: Int!) { publicAnimation(id: $id) { ",
    fields!(),
    " } }"
);

/// A GraphQL request: the document and its variables.
#[derive(Debug, Clone, Serialize)]
struct Request<V> {
    query: &'static str,
    variables: V,
}

/// What a search is asked with.
#[derive(Debug, Clone, Serialize)]
struct SearchVariables<'a> {
    query: &'a str,
    first: u32,
}

/// What a lookup is asked with.
#[derive(Debug, Clone, Serialize)]
struct OneVariables {
    id: u64,
}

/// A GraphQL reply: the data, or what went wrong, or both.
#[derive(Debug, Clone, Deserialize)]
pub struct Reply<D> {
    /// What was asked for, when anything of it could be answered.
    #[serde(default = "Option::default")]
    pub data: Option<D>,
    /// What could not be.
    #[serde(default)]
    pub errors: Vec<GraphqlError>,
}

/// One thing a GraphQL server could not do.
#[derive(Debug, Clone, Deserialize)]
pub struct GraphqlError {
    /// What it said.
    #[serde(default)]
    pub message: String,
}

impl<D> Reply<D> {
    /// The data, or the server's errors as a refusal of `ENDPOINT`.
    pub fn into_data(self) -> Result<D, HttpError> {
        match self.data {
            Some(data) if self.errors.is_empty() => Ok(data),
            _ => Err(HttpError::Refused {
                url: ENDPOINT.to_owned(),
                status: 200,
                body: if self.errors.is_empty() {
                    String::from("no data and no error")
                } else {
                    let said: Vec<&str> = self.errors.iter().map(|e| e.message.as_str()).collect();
                    said.join("; ")
                },
            }),
        }
    }
}

/// What a search answers with.
#[derive(Debug, Clone, Deserialize)]
pub struct Searched {
    /// The results.
    #[serde(rename = "searchPublicAnimations")]
    pub found: Connection,
}

/// What a lookup answers with: the animation, or `null` for an id there is
/// none of.
#[derive(Debug, Clone, Deserialize)]
pub struct Looked {
    /// The animation.
    #[serde(rename = "publicAnimation")]
    pub animation: Option<Animation>,
}

/// One page of results.
#[derive(Debug, Clone, Deserialize)]
pub struct Connection {
    /// How many animations match, in all.
    #[serde(default, rename = "totalCount")]
    pub total_count: u64,
    /// This page of them.
    #[serde(default)]
    pub edges: Vec<Edge>,
}

/// One result, in GraphQL's wrapping.
#[derive(Debug, Clone, Deserialize)]
pub struct Edge {
    /// The animation.
    pub node: Animation,
}

/// A public animation, as the API describes it.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Animation {
    /// Its id.
    pub id: u64,
    /// Its title.
    #[serde(default)]
    pub name: Option<String>,
    /// What its author wrote about it.
    #[serde(default)]
    pub description: Option<String>,
    /// The Lottie JSON.
    #[serde(default)]
    pub json_url: Option<String>,
    /// A GIF of it playing.
    #[serde(default)]
    pub gif_url: Option<String>,
    /// A PNG of one frame, on white.
    #[serde(default)]
    pub image_url: Option<String>,
    /// An MP4 of it playing.
    #[serde(default)]
    pub video_url: Option<String>,
    /// Its page on lottiefiles.com.
    #[serde(default)]
    pub url: Option<String>,
    /// The JSON's size, in bytes.
    #[serde(default)]
    pub lottie_file_size: Option<u64>,
    /// Who published it.
    #[serde(default)]
    pub created_by: Option<Author>,
    /// Its size, rate and length.
    #[serde(default)]
    pub metadata: Option<Metadata>,
}

/// Who published an animation.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Author {
    /// A handle, often an opaque `/abc123`.
    #[serde(default)]
    pub username: Option<String>,
    /// Their first name, when they gave one.
    #[serde(default)]
    pub first_name: Option<String>,
    /// Their last name, when they gave one.
    #[serde(default)]
    pub last_name: Option<String>,
}

/// An animation's measurements.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    /// Its width, in its own units.
    #[serde(default)]
    pub width: Option<f64>,
    /// Its height, in its own units.
    #[serde(default)]
    pub height: Option<f64>,
    /// Frames a second.
    #[serde(default)]
    pub frame_rate: Option<f64>,
    /// Seconds long.
    #[serde(default)]
    pub duration: Option<f64>,
}

/// LottieFiles, reachable.
#[derive(Debug, Clone)]
pub struct LottieFiles {
    caller: Caller,
}

impl Default for LottieFiles {
    fn default() -> Self {
        Self::new()
    }
}

impl LottieFiles {
    /// One that asks anonymously, as the API allows.
    pub fn new() -> Self {
        Self {
            caller: Caller::anonymous(),
        }
    }

    /// The same client, copying every exchange into `tap` — how the live
    /// check records what came back.
    pub fn tapped(mut self, tap: &crate::api::tap::Tap) -> Self {
        self.caller = self.caller.tapped(tap);
        self
    }

    /// The first `first` animations matching `words`, at most [`MAX_FIRST`].
    pub fn search(&self, words: &str, first: u32) -> Result<Connection, HttpError> {
        let request = Request {
            query: SEARCH,
            variables: SearchVariables {
                query: words,
                first: first.min(MAX_FIRST),
            },
        };
        let reply: Reply<Searched> = self.caller.post(ENDPOINT, &request)?;
        reply.into_data().map(|data| data.found)
    }

    /// Animation `id`, or `None` when there is none.
    pub fn one(&self, id: u64) -> Result<Option<Animation>, HttpError> {
        let request = Request {
            query: ONE,
            variables: OneVariables { id },
        };
        let reply: Reply<Looked> = self.caller.post(ENDPOINT, &request)?;
        reply.into_data().map(|data| data.animation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Three results for "cat waving" as LottieFiles sent them on 2026-10-08.
    const FOUND: &str = include_str!("../../../fixtures/lottiefiles/search.json");

    #[test]
    fn a_search_reads_every_field_scorsese_uses() {
        let reply: Reply<Searched> = serde_json::from_str(FOUND).unwrap();
        let found = reply.into_data().unwrap().found;
        assert_eq!(found.total_count, 83127);
        let kitty = &found.edges[1].node;
        assert_eq!(kitty.id, 121035);
        assert_eq!(kitty.name.as_deref(), Some("Waving kitty"));
        assert!(kitty.json_url.as_deref().unwrap().ends_with(".json"));
        assert!(kitty.image_url.as_deref().unwrap().ends_with(".png"));
        let metadata = kitty.metadata.as_ref().unwrap();
        assert_eq!(
            (metadata.width, metadata.height),
            (Some(1291.0), Some(1200.0))
        );
        assert_eq!(metadata.frame_rate, Some(48.0));
        let author = kitty.created_by.as_ref().unwrap();
        assert_eq!(author.first_name.as_deref(), Some("Kati"));
        assert_eq!(author.last_name, None);
    }

    #[test]
    fn an_error_answered_as_200_is_a_refusal_naming_what_was_said() {
        let said = r#"{"errors":[{"message":"String cannot represent a non string value: 5"}]}"#;
        let reply: Reply<Searched> = serde_json::from_str(said).unwrap();
        let Err(HttpError::Refused { status, body, .. }) = reply.into_data() else {
            panic!("an error is a refusal");
        };
        assert_eq!(status, 200);
        assert!(body.contains("non string value"), "{body}");
    }

    #[test]
    fn an_id_there_is_none_of_is_null() {
        let reply: Reply<Looked> =
            serde_json::from_str(r#"{"data":{"publicAnimation":null}}"#).unwrap();
        assert!(reply.into_data().unwrap().animation.is_none());
    }

    #[test]
    fn the_queries_name_their_variables() {
        assert!(SEARCH.contains("searchPublicAnimations(query: $query, first: $first)"));
        assert!(ONE.contains("publicAnimation(id: $id)"));
        let body = serde_json::to_value(Request {
            query: SEARCH,
            variables: SearchVariables {
                query: "rocket",
                first: 50,
            },
        })
        .unwrap();
        assert_eq!(body["variables"]["first"], 50);
    }
}
