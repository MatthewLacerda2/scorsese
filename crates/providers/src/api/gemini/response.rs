//! What Gemini sends back, read permissively.
//!
//! Only the fields scorsese acts on are named; anything else is ignored, for
//! [`veo::response`](crate::api::veo::response)'s reason — a reply is somebody
//! else's to change, and a field added to it should not stop a picture that
//! has been paid for from being kept.
//!
//! The picture is looked for in two places. The documented one is the last
//! image the model produced in its `steps`; Google's SDKs also expose an
//! `output_image` shortcut, and if the reply carries it, it is the same
//! picture. Thought steps are skipped: a thinking model may sketch on the way,
//! and a draft is not the answer.

use serde::Deserialize;

/// One finished interaction.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Interaction {
    /// `completed`, or the reason it is not.
    #[serde(default)]
    pub status: Option<String>,
    /// What the model did, in order.
    #[serde(default)]
    pub steps: Vec<Step>,
    /// The SDKs' shortcut to the last picture, if the reply carries it.
    #[serde(default)]
    pub output_image: Option<Content>,
}

/// One step of the model's work.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Step {
    /// `model_output`, `thought`, ...
    #[serde(rename = "type", default)]
    pub kind: String,
    /// What it produced.
    #[serde(default)]
    pub content: Vec<Content>,
}

/// One piece of output: words or a picture.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Content {
    /// `text` or `image`.
    #[serde(rename = "type", default)]
    pub kind: String,
    /// The words, on a text part.
    #[serde(default)]
    pub text: Option<String>,
    /// The picture, base64, on an image part.
    #[serde(default)]
    pub data: Option<String>,
}

impl Interaction {
    /// The picture the model answered with, base64 — the last one it made
    /// outside its thinking.
    pub fn image(&self) -> Option<&str> {
        self.steps
            .iter()
            .filter(|step| step.kind != "thought")
            .flat_map(|step| &step.content)
            .filter(|part| part.kind == "image")
            .filter_map(|part| part.data.as_deref())
            .next_back()
            .or_else(|| self.output_image.as_ref()?.data.as_deref())
    }

    /// Whatever the model said in words — which, when there is no picture, is
    /// usually why.
    pub fn said(&self) -> String {
        let words: Vec<&str> = self
            .steps
            .iter()
            .filter(|step| step.kind != "thought")
            .flat_map(|step| &step.content)
            .filter_map(|part| part.text.as_deref())
            .collect();
        words.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(json: &str) -> Interaction {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn the_last_picture_outside_the_thinking_is_the_answer() {
        let reply = read(
            r#"{"id": "i1", "status": "completed", "steps": [
                {"type": "thought", "summary": [], "content": [{"type": "image", "data": "RFJBRlQ="}]},
                {"type": "model_output", "content": [
                    {"type": "text", "text": "Here it is."},
                    {"type": "image", "mime_type": "image/png", "data": "UE5H"}
                ]}
            ]}"#,
        );
        assert_eq!(reply.image(), Some("UE5H"));
        assert_eq!(reply.said(), "Here it is.");
    }

    #[test]
    fn the_shortcut_is_read_when_the_steps_carry_nothing() {
        let reply = read(r#"{"output_image": {"type": "image", "data": "UE5H"}}"#);
        assert_eq!(reply.image(), Some("UE5H"));
    }

    #[test]
    fn a_refusal_in_words_has_no_picture_and_keeps_the_words() {
        let reply = read(
            r#"{"steps": [{"type": "model_output", "content": [
                {"type": "text", "text": "I can't draw that."}]}]}"#,
        );
        assert_eq!(reply.image(), None);
        assert_eq!(reply.said(), "I can't draw that.");
    }
}
