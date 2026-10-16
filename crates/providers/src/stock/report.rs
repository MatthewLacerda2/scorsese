//! What an import from a stock library says, once, for every surface.
//!
//! `scorsese stock import`, the `stock_import` tool and a pick in the web
//! assistant's picker each used to word their own answer, and the web's left
//! out where a file came from and under what licence (#1289). The line for
//! each result, the failures beside them and the licence are said here; a
//! surface adds only what is its own, such as how a page plays a Lottie.

use scorsese_core::bytes::format_bytes;

use super::{Choice, Fetched, Kept, LICENCE, LOTTIE_LICENCE, StockError};

/// What importing footage and photos came to: a line for each, in the order
/// asked.
#[derive(Debug)]
pub struct Imports {
    answers: Vec<(Choice, Result<Fetched, StockError>)>,
    frame: (u32, u32),
}

impl Imports {
    /// `answers` to `choices`, as `super::import` gives them for a `frame`
    /// of width×height.
    pub fn new(
        choices: &[Choice],
        answers: Vec<Result<Fetched, StockError>>,
        frame: (u32, u32),
    ) -> Self {
        Self {
            answers: choices.iter().copied().zip(answers).collect(),
            frame,
        }
    }

    /// Whether the project changed and so is to be saved: something came in
    /// that it did not already hold, or a file it held learned where it came
    /// from.
    pub fn needs_save(&self) -> bool {
        self.answers.iter().any(|(_, answer)| {
            answer
                .as_ref()
                .is_ok_and(|one| one.recorded || !one.imported.reused)
        })
    }

    /// What came in, from where and by whom, what did not, and the licence
    /// it came under — or, when nothing came in, why not.
    pub fn lines(&self) -> Result<Vec<String>, String> {
        let mut lines = Vec::new();
        let mut failures = Vec::new();
        for (choice, answer) in &self.answers {
            match answer {
                Ok(one) => lines.push(fetched(one, self.frame)),
                Err(error) => failures.push(format!("{} — failed: {error}", named(*choice))),
            }
        }
        if lines.is_empty() {
            return Err(format!("{} — nothing was imported", failures.join("; ")));
        }
        lines.extend(failures);
        lines.push(format!("From Pixabay. {LICENCE}"));
        Ok(lines)
    }
}

/// What keeping Lottie animations came to: a line for each, in the order
/// asked.
#[derive(Debug)]
pub struct Animations {
    answers: Vec<(u64, Result<Kept, StockError>)>,
}

impl Animations {
    /// `answers` to `ids`, as `super::keep` gives them.
    pub fn new(ids: &[u64], answers: Vec<Result<Kept, StockError>>) -> Self {
        Self {
            answers: ids.iter().copied().zip(answers).collect(),
        }
    }

    /// The first animation kept, which a surface shows a page playing.
    pub fn first(&self) -> Option<&Kept> {
        self.answers
            .iter()
            .find_map(|(_, answer)| answer.as_ref().ok())
    }

    /// What came in and what did not, and the licence — or, when nothing
    /// came in, why not.
    pub fn lines(&self) -> Result<Vec<String>, String> {
        let mut lines = Vec::new();
        let mut failures = Vec::new();
        for (id, answer) in &self.answers {
            match answer {
                Ok(one) => lines.push(animation(one)),
                Err(error) => failures.push(format!("lottie {id} — failed: {error}")),
            }
        }
        if lines.is_empty() {
            return Err(format!("{} — nothing was imported", failures.join("; ")));
        }
        lines.extend(failures);
        lines.push(format!("From LottieFiles. {LOTTIE_LICENCE}"));
        Ok(lines)
    }
}

/// A choice in words: `video 39009`.
fn named(choice: Choice) -> String {
    format!("{} {}", choice.medium.word(), choice.id)
}

/// One import's line: the asset it is, what it was and who made it.
fn fetched(one: &Fetched, (width, height): (u32, u32)) -> String {
    let (rendition, id, candidate) = (&one.rendition, &one.imported.id, &one.candidate);
    let what = format!("{} {}", candidate.medium.word(), candidate.id);
    let mut line = if one.imported.reused {
        format!("{id} — {what}, already in the project, nothing copied")
    } else {
        format!(
            "{id} — {what} ({}, {}x{})",
            rendition.name, rendition.width, rendition.height
        )
    };
    line.push_str(&format!(
        ", by {} — {}",
        candidate.author, candidate.page_url
    ));
    if let Some(wanted) = &one.imported.wanted {
        line.push_str(&format!(", renamed: `{wanted}` taken"));
    }
    if !one.fills {
        line.push_str(&format!(
            "\n   smaller than the {width}x{height} frame: it is the largest Pixabay offers, \
             and will look soft full-frame"
        ));
    }
    line
}

/// One animation's line.
fn animation(one: &Kept) -> String {
    let mut line = format!(
        "{} — \"{}\" by {}, {}x{}, {} frames at {} fps ({:.2}s), {} — {}",
        one.path,
        one.candidate.title,
        one.candidate.author,
        one.size.0,
        one.size.1,
        one.frames,
        one.fps,
        one.seconds(),
        format_bytes(one.bytes),
        one.candidate.page_url,
    );
    if one.reused {
        line.push_str("\n   already there, nothing written");
    }
    line.push_str(&format!(
        "\n   from a page in pages/ it is \"{}\"",
        one.file
    ));
    if !one.outside.is_empty() {
        line.push_str(&format!(
            "\n   it names pictures it does not carry ({}), which a page will not find",
            one.outside.join(", ")
        ));
    }
    line
}
