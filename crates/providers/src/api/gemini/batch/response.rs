//! What a batch job answers, read permissively.
//!
//! [`veo::response`](crate::api::veo::response)'s rule: only the fields acted
//! on are named, and an added field never stops a paid-for picture from being
//! kept. Two spellings are tolerated where the page shows two: the state is
//! `BATCH_STATE_…` in its REST examples and `JOB_STATE_…` in its SDK ones, and
//! the inline answers may or may not be wrapped in an object of the same name.

use serde::Deserialize;
use serde_json::Value;

/// A job, as creating it or asking after it answers.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Operation {
    /// `batches/…` — the ticket.
    #[serde(default)]
    pub name: String,
    /// Where the state is.
    #[serde(default)]
    pub metadata: Option<Metadata>,
    /// Whether it has stopped, one way or another.
    #[serde(default)]
    pub done: bool,
    /// Why it failed, when the job as a whole did.
    #[serde(default)]
    pub error: Option<Status>,
    /// The answers, once it has succeeded.
    #[serde(default)]
    pub response: Option<Output>,
}

/// The job's progress.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Metadata {
    /// `BATCH_STATE_RUNNING`, `JOB_STATE_SUCCEEDED`, ...
    #[serde(default)]
    pub state: Option<String>,
}

/// A refusal, the job's or one request's.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Status {
    /// The vendor's sentence.
    #[serde(default)]
    pub message: String,
}

/// A finished job's output.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Output {
    /// The inline answers, wrapped or bare.
    #[serde(default)]
    pub inlined_responses: Value,
}

/// One request's answer.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Answer {
    /// The `generateContent` reply, when it drew.
    #[serde(default)]
    pub response: Option<Reply>,
    /// Why not, when it did not.
    #[serde(default)]
    pub error: Option<Status>,
    /// The key it was sent with.
    #[serde(default)]
    pub metadata: Option<AnswerKey>,
}

/// The key a request was sent with.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AnswerKey {
    /// As sent.
    #[serde(default)]
    pub key: String,
}

/// A `generateContent` reply.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Reply {
    /// The model's answers; one, for a drawing.
    #[serde(default)]
    pub candidates: Vec<Candidate>,
}

/// One answer.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Candidate {
    /// What it is made of.
    #[serde(default)]
    pub content: Option<Content>,
}

/// Words and pictures.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Content {
    /// In order.
    #[serde(default)]
    pub parts: Vec<Part>,
}

/// One piece.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Part {
    /// Words.
    #[serde(default)]
    pub text: Option<String>,
    /// A picture.
    #[serde(default)]
    pub inline_data: Option<Data>,
    /// Whether this is the model thinking rather than answering.
    #[serde(default)]
    pub thought: bool,
}

/// A picture, base64.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Data {
    /// Standard base64.
    #[serde(default)]
    pub data: String,
}

/// Where a job is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    /// Waiting or drawing.
    Running,
    /// Every answer is in.
    Succeeded,
    /// Stopped without answers — failed, cancelled or expired, in the words
    /// the vendor or the state gives.
    Stopped(String),
}

impl Operation {
    /// Where the job is.
    pub fn state(&self) -> State {
        let state = self
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.state.as_deref())
            .unwrap_or_default();
        let ended = |word: &str| state.ends_with(word);
        if ended("_SUCCEEDED") || (self.done && self.error.is_none() && self.response.is_some()) {
            return State::Succeeded;
        }
        if let Some(error) = &self.error {
            return State::Stopped(error.message.clone());
        }
        let stopped = ["_FAILED", "_CANCELLED", "_EXPIRED"];
        if stopped.iter().any(|word| ended(word)) || self.done {
            return State::Stopped(format!("the batch ended {state}"));
        }
        State::Running
    }

    /// Every answer, wrapped or bare; none before the job has succeeded.
    pub fn answers(&self) -> Vec<Answer> {
        let Some(output) = &self.response else {
            return Vec::new();
        };
        let list = match &output.inlined_responses {
            Value::Object(wrapped) => wrapped.get("inlinedResponses").cloned(),
            Value::Array(_) => Some(output.inlined_responses.clone()),
            _ => None,
        };
        list.and_then(|list| serde_json::from_value(list).ok())
            .unwrap_or_default()
    }
}

impl Answer {
    /// The picture it drew, base64 — the last one outside the thinking — or
    /// why there is none.
    pub fn picture(&self) -> Result<&str, String> {
        if let Some(error) = &self.error {
            return Err(error.message.clone());
        }
        let parts: Vec<&Part> = self
            .response
            .iter()
            .flat_map(|reply| &reply.candidates)
            .filter_map(|candidate| candidate.content.as_ref())
            .flat_map(|content| &content.parts)
            .filter(|part| !part.thought)
            .collect();
        if let Some(data) = parts
            .iter()
            .filter_map(|part| part.inline_data.as_ref())
            .map(|data| data.data.as_str())
            .next_back()
        {
            return Ok(data);
        }
        let said: Vec<&str> = parts
            .iter()
            .filter_map(|part| part.text.as_deref())
            .collect();
        Err(if said.is_empty() {
            String::from("the model answered with no picture")
        } else {
            said.join(" ")
        })
    }

    /// The key it was sent with.
    pub fn key(&self) -> &str {
        self.metadata.as_ref().map_or("", |metadata| &metadata.key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(json: &str) -> Operation {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn a_job_still_drawing_is_running() {
        let job = read(r#"{"name": "batches/1", "metadata": {"state": "BATCH_STATE_RUNNING"}}"#);
        assert_eq!(job.state(), State::Running);
        assert!(job.answers().is_empty());
    }

    #[test]
    fn a_finished_job_hands_back_each_picture_by_its_key() {
        let job = read(
            r#"{"name": "batches/1", "done": true,
                "metadata": {"state": "JOB_STATE_SUCCEEDED"},
                "response": {"inlinedResponses": {"inlinedResponses": [
                  {"metadata": {"key": "a"}, "response": {"candidates": [{"content": {"parts": [
                    {"text": "draft", "thought": true},
                    {"inlineData": {"mimeType": "image/png", "data": "UE5H"}}]}}]}},
                  {"metadata": {"key": "b"}, "error": {"code": 400, "message": "refused"}}
                ]}}}"#,
        );
        assert_eq!(job.state(), State::Succeeded);
        let answers = job.answers();
        assert_eq!(answers[0].key(), "a");
        assert_eq!(answers[0].picture(), Ok("UE5H"));
        assert_eq!(answers[1].picture(), Err(String::from("refused")));
    }

    #[test]
    fn a_bare_list_of_answers_is_read_too() {
        let job = read(
            r#"{"metadata": {"state": "BATCH_STATE_SUCCEEDED"},
                "response": {"inlinedResponses": [{"metadata": {"key": "a"},
                  "response": {"candidates": [{"content": {"parts": [{"text": "no"}]}}]}}]}}"#,
        );
        assert_eq!(job.answers()[0].picture(), Err(String::from("no")));
    }

    #[test]
    fn an_expired_job_stopped_and_says_so() {
        let job = read(r#"{"metadata": {"state": "BATCH_STATE_EXPIRED"}}"#);
        assert_eq!(
            job.state(),
            State::Stopped(String::from("the batch ended BATCH_STATE_EXPIRED"))
        );
    }
}
