//! A speech provider that costs nothing.
//!
//! Simpler than the video mock, because the provider it stands in for is: one
//! call, one answer, nothing to poll. What it keeps is the part that proves the
//! money guarantee — **a record of every line handed over**, since a cache that
//! works is a cache that produces no second request.

use std::cell::RefCell;

use scorsese_providers::speech::{Brief, ProviderError, SpeechProvider};

/// What the mock does when asked to speak.
#[derive(Debug, Clone)]
pub(crate) enum Answer {
    /// Hands back these bytes.
    Speaks(Vec<u8>),
    /// Refuses, saying this.
    Refuses(String),
}

/// A provider whose every answer was decided in advance.
#[derive(Debug, Default)]
pub(crate) struct Mock {
    answers: RefCell<Vec<Answer>>,
    /// Every line handed over, in order — the record the cache is checked
    /// against.
    pub(crate) spoken: RefCell<Vec<String>>,
}

impl Mock {
    /// A provider that gives `answers` in order, and repeats the last one.
    pub(crate) fn giving(answers: Vec<Answer>) -> Self {
        Self {
            answers: RefCell::new(answers.into_iter().rev().collect()),
            spoken: RefCell::new(Vec::new()),
        }
    }

    /// A provider that speaks anything, always.
    pub(crate) fn willing() -> Self {
        Self::giving(vec![Answer::Speaks(b"MP3".to_vec())])
    }

    /// How many lines have been handed over.
    pub(crate) fn requests(&self) -> usize {
        self.spoken.borrow().len()
    }
}

impl SpeechProvider for Mock {
    fn speak(&self, brief: &Brief) -> Result<Vec<u8>, ProviderError> {
        self.spoken.borrow_mut().push(brief.text.clone());
        let mut answers = self.answers.borrow_mut();
        // The last answer repeats rather than running out, so a test that only
        // cares about the first request does not have to script every one.
        let answer = if answers.len() > 1 {
            answers.pop()
        } else {
            answers.last().cloned()
        };
        match answer {
            Some(Answer::Speaks(bytes)) => Ok(bytes),
            Some(Answer::Refuses(message)) => Err(ProviderError::new("mock", message)),
            None => Ok(b"MP3".to_vec()),
        }
    }

    fn name(&self) -> &'static str {
        "mock"
    }
}
