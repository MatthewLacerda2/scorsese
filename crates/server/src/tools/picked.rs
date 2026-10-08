//! Stock media the user picked, brought into their project (#901).
//!
//! A model brings a stock result in with `stock_import`. A pick from the
//! assistant's picker is the **user's** choice, made in the web app, so the
//! server makes the import itself — recorded as the user's call, the way their
//! yes to a quote is.
//!
//! Footage and photos come in with the same `scorsese_providers::stock::import`
//! the tool runs, on the project laid out the way every tool call lays it out
//! (`stored`), the files it brings down admitted to the library before the
//! document naming them is saved (`fetched`). A Lottie (#908) changes no
//! document — it is a file under `pages/` — so it is not imported here at
//! all: the registry's `stock_import` itself is run for it, as the user's
//! call, which writes the file, saves it with the project's files, and words
//! the reply with the page that plays it. Either way the library is the
//! toolbox's, the same one `stock_import` answers from — which is how a test
//! imports with no network.

use std::sync::Arc;

use scorsese_mcp::{Reply, Stock};
use scorsese_providers::credentials::{Provider, resolve};
use scorsese_providers::stock::{
    self, Choice, Fetched, Medium, PixabayLibrary, StockError, cache_dir,
};
use scorsese_render::Ffprobe;
use serde_json::json;

use super::{Client, Toolbox, database, fetched, folder, lay_out, log};
use crate::db::UserId;
use crate::projects::{self, ProjectError};

/// How many times an import is made again on a project that moved under it.
const ATTEMPTS: usize = 3;

/// What the call is recorded as: the tool a model would have called.
const TOOL: &str = "stock_import";

impl Toolbox {
    /// Import `choices` into `user`'s project `project` — footage and photos
    /// each as the smallest file that fills a `frame` of width×height, a
    /// Lottie as its JSON under `pages/` — and record it as `client`'s
    /// `stock_import`. What came in and what did not, a line each — footage
    /// and photos first, in the order asked, then the animations — or why
    /// nothing did.
    pub async fn import_stock(
        &self,
        user: UserId,
        client: Client,
        project: i64,
        choices: &[Choice],
        frame: (u32, u32),
    ) -> Result<String, String> {
        let (lotties, footage): (Vec<Choice>, Vec<Choice>) =
            choices.iter().partition(|one| one.medium == Medium::Lottie);
        let mut answers = Vec::new();
        if !footage.is_empty() {
            let answer = self
                .import_footage(user, client, project, &footage, frame)
                .await;
            answers.push(answer);
        }
        if !lotties.is_empty() {
            answers.push(self.keep_lotties(user, client, project, &lotties).await);
        }
        joined(answers)
    }

    /// The animations in a pick, brought in by the registry's own
    /// `stock_import`, recorded as `client`'s: its reply, in words.
    async fn keep_lotties(
        &self,
        user: UserId,
        client: Client,
        project: i64,
        lotties: &[Choice],
    ) -> Result<String, String> {
        let ids: Vec<u64> = lotties.iter().map(|one| one.id).collect();
        let arguments = json!({ "project": project, "id": ids, "kind": Medium::Lottie.word() });
        let reply = self.call(user, client, TOOL, &arguments).await?;
        let said: Vec<&str> = reply.parts.iter().map(|part| part.text.as_str()).collect();
        Ok(said.join("\n"))
    }

    /// The footage and photos in a pick, imported and recorded.
    async fn import_footage(
        &self,
        user: UserId,
        client: Client,
        project: i64,
        choices: &[Choice],
        frame: (u32, u32),
    ) -> Result<String, String> {
        let named: Vec<String> = choices.iter().map(|one| named(*one)).collect();
        let arguments = json!({
            "project": project,
            "picked": named,
            "resolution": format!("{}x{}", frame.0, frame.1),
        });
        let id = log::begin(&self.pool, user, client, TOOL, &arguments)
            .await
            .map_err(database)?;
        let outcome = match self.footage() {
            Ok(library) => self.imported(user, project, choices, frame, library).await,
            Err(why) => Err(why),
        };
        let reply = outcome.as_ref().map(|lines| Reply::from(lines.clone()));
        log::end(
            &self.pool,
            user,
            id,
            reply.as_ref().map_err(|why| (*why).clone()),
        )
        .await;
        outcome
    }

    /// [`Toolbox::import_footage`]'s work, unrecorded.
    async fn imported(
        &self,
        user: UserId,
        project: i64,
        choices: &[Choice],
        frame: (u32, u32),
        library: Stock,
    ) -> Result<String, String> {
        for _ in 0..ATTEMPTS {
            let (stored, kept) = projects::open_with_files(&self.pool, user, project)
                .await
                .map_err(|error| match error {
                    ProjectError::NotFound => "there is no such project of yours".to_owned(),
                    other => database(other),
                })?;
            let folder = lay_out(&self.pool, &self.storage, user, &stored.document, &kept).await?;
            let root = folder.root().to_path_buf();
            folder::keep_stock_cache(&self.storage, user, &root)?;
            let (mut document, probe) = (stored.document.clone(), Ffprobe::new(self.tools.clone()));
            let (library, picked) = (library.clone(), choices.to_vec());
            let (document, answers) = tokio::task::spawn_blocking(move || {
                let cache = cache_dir(&root);
                let answers = stock::import(
                    &mut document,
                    &root,
                    &cache,
                    &*library,
                    &picked,
                    frame,
                    &probe,
                );
                (document, answers)
            })
            .await
            .map_err(|_| "the import crashed on the server; that is a bug".to_owned())?;
            let lines = said(choices, &answers)?;
            if !answers.iter().flatten().any(|one| !one.imported.reused) {
                return Ok(lines);
            }
            fetched::keep(
                &self.library,
                user,
                &stored.document,
                &document,
                folder.root(),
            )
            .await?;
            let revision = stored.summary.revision;
            match projects::save(&self.pool, user, project, revision, &document).await {
                Ok(_) => return Ok(lines),
                Err(ProjectError::Conflict { .. }) => {}
                Err(error) => return Err(database(error)),
            }
        }
        Err("the project kept changing while this ran, so nothing was imported".to_owned())
    }
}

impl Toolbox {
    /// The library a pick of footage or a photo is imported from: the
    /// toolbox's own when it was given one, and otherwise Pixabay, keyed from
    /// the one resolver. Why there is none, in words for the model, which
    /// tells the person.
    fn footage(&self) -> Result<Stock, String> {
        if let Some(library) = &self.stock {
            return Ok(library.clone());
        }
        match resolve(Provider::Pixabay) {
            Ok(key) => Ok(Arc::new(PixabayLibrary::new(&key.secret))),
            Err(error) => {
                eprintln!("scorsese-server: stock import: {error}");
                Err(format!(
                    "{} is not set on this server, so stock media cannot be brought in",
                    Provider::Pixabay.variable()
                ))
            }
        }
    }
}

/// What each part of a pick answered, as one answer: a refusal only when
/// every part was refused, and otherwise what came in, with what did not
/// beside it.
fn joined(answers: Vec<Result<String, String>>) -> Result<String, String> {
    if answers.iter().all(Result::is_err) {
        let why: Vec<String> = answers.into_iter().filter_map(Result::err).collect();
        return Err(why.join("; "));
    }
    let lines: Vec<String> = answers
        .into_iter()
        .map(|answer| answer.unwrap_or_else(|why| format!("Not imported: {why}")))
        .collect();
    Ok(lines.join("\n"))
}

/// A choice in words: `video 39009`.
fn named(choice: Choice) -> String {
    format!("{} {}", choice.medium.word(), choice.id)
}

/// A line for each answer — or, when none came in, why not.
fn said(choices: &[Choice], answers: &[Result<Fetched, StockError>]) -> Result<String, String> {
    let lines: Vec<String> = choices
        .iter()
        .zip(answers)
        .map(|(choice, answer)| match answer {
            Ok(one) if one.imported.reused => format!(
                "{} — asset `{}`, already in the project",
                named(*choice),
                one.imported.id
            ),
            Ok(one) => {
                let rendition = &one.rendition;
                let mut line = format!(
                    "{} — asset `{}` ({}x{}), by {}",
                    named(*choice),
                    one.imported.id,
                    rendition.width,
                    rendition.height,
                    one.candidate.author
                );
                if !one.fills {
                    line.push_str(", smaller than the frame: it will look soft full-frame");
                }
                line
            }
            Err(error) => format!("{} — failed: {error}", named(*choice)),
        })
        .collect();
    if answers.iter().all(Result::is_err) {
        return Err(lines.join("; "));
    }
    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::joined;

    #[test]
    fn a_pick_is_refused_only_when_every_part_was() {
        let (came, refused) = (Ok("pixabay-1".to_owned()), Err("no key".to_owned()));
        assert_eq!(
            joined(vec![came.clone(), refused.clone()]),
            Ok("pixabay-1\nNot imported: no key".to_owned())
        );
        assert_eq!(joined(vec![came.clone()]), came);
        assert_eq!(
            joined(vec![refused.clone(), Err("gone".to_owned())]),
            Err("no key; gone".to_owned())
        );
    }
}
