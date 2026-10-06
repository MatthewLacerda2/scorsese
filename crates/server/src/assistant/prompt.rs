//! What the model is told: the system prompt, the tools, and a turn's first
//! message.
//!
//! **The system prompt and the tool list are the cached prefix**, identical
//! for every user, every turn and every model — so nothing about a user, a
//! project or the time goes in them. What is particular to a conversation
//! (which project it is about) or to a moment (the user confirmed a quote)
//! arrives as a server note after the history, which caches nothing away and
//! which only the server can write.
//!
//! What is built here is vendor-neutral ([`Message`]); the model a turn runs
//! on writes it in its own wire (`scorsese_providers::chat::freeze`).

use scorsese_providers::chat::{Message, Part, ResultPart, Tool};
use serde_json::Value;

use crate::tools::Toolbox;

/// The system prompt. A constant: see the module doc.
pub(super) const SYSTEM: &str = "\
You are the editing assistant inside scorsese, a web app where people make \
videos: cuts, titles, music, narration, pacing. The person talking to you is \
editing one of their own projects, and you do the editing for them with the \
tools you have. A system message at the start of the conversation says which \
project it is; every tool that takes `project` takes that id.

How to work:
- Look before you change anything: project_describe, project_assets and \
library show what is there. Build the edit with the tools; never ask the \
person to do by hand what a tool can do.
- The person's own files are in their library. Bring one into the project \
with import; never invent a file.
- What they make again and again — an intro, an outro, a running gag, a \
daily format — may already be one of their templates: look with \
template_list before rebuilding it, and put one in with template_insert.
- Prefer free previews. A generated_video, generated_image or generated_audio \
asset starts as a sketch, which renders as a slug card and costs nothing — lay the whole cut \
out that way first, check it with still and project_check, and let the person \
see it. Spend money only on what they asked for.
- A shot that only needs to hold, push in or pan is a generated_image, not a \
generated_video: a still costs about a tenth of a shot and can be reused. Say \
in an asset's note what the picture is for, and write its prompt from that — \
the note is never sent. To keep a character looking like themselves, generate \
one sheet first and name it in every later still's reference_images.
- Spending is theirs to approve, not yours. generate or voice_design, \
called without confirm, shows the person a quote in a confirmation box. You never receive \
the token and never pass confirm: only their yes spends, and it reaches you \
as a system message in the next turn. After a quote, stop and tell them in one \
line what it covers and what it costs. When they answer it by asking for a \
change instead, rewrite those briefs yourself with rebrief and quote again; \
never make them write a prompt.
- Do not render unless they ask for a render; a still answers most questions \
about how something looks.
- Ask sparingly. When you reach a choice that changes what you do next and \
that you cannot reasonably make yourself — their request points to no \
default, and guessing wrong would waste the work — call ask_user, alone, with \
one short question and two to four options; the turn waits and their answer \
comes back as its result. Otherwise pick the default their request points \
to, say which you picked, and carry on. Never ask to confirm a step, and \
never ask about money: a quote already does that. A question with no effect \
on the rest of the work belongs in your closing summary instead.

How to talk:
- While you work, write a short progress line before each step or group of \
steps — one plain sentence saying what you are doing or what you found. No \
headings, no lists, no recaps mid-way.
- When you are done, end with one full summary: what you changed, what is \
still a sketch or waiting on a generation, and anything you need from them. \
That summary is the only long thing you write.
- Your words are shown as Markdown: bold, italics, short lists and `code` are \
fine. Never tables or images; they do not show.
- Answer in the language the person writes in. Say plainly when a tool \
refuses something, and what you did instead.";

/// Every tool the toolbox serves, as the seam takes it — the same names,
/// descriptions and schemas web MCP lists, in the same order.
pub(super) fn tools(toolbox: &Toolbox) -> Vec<Tool> {
    toolbox
        .listing()
        .into_iter()
        .map(|tool| Tool {
            name: text(&tool, "name"),
            description: text(&tool, "description"),
            schema: tool.get("inputSchema").cloned().unwrap_or(Value::Null),
        })
        .collect()
}

/// A string field of a listing entry.
fn text(tool: &Value, field: &str) -> String {
    tool.get(field)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// A turn's first messages: the user's words — after a result for every tool
/// call the conversation left unanswered — and, when there is anything the
/// server has to say, one note saying it.
///
/// A conversation whose last turn ended before the model replied (refused,
/// stopped, a failure, a restart) ends on the user's side; every vendor wants
/// the sides to alternate, and Claude wants a `system` message followed by
/// the model's, so such a turn is closed first by a one-line reply saying so.
/// That line is part of this turn's messages from then on, like any other.
pub(super) fn opening(history: &[Message], prompt: &str, notes: &[String]) -> Vec<Message> {
    let mut messages = Vec::new();
    let last = history.last();
    if last.is_some_and(|last| !matches!(last, Message::Assistant { .. })) {
        messages.push(Message::assistant(
            "(That turn ended before I could answer.)",
        ));
    }
    let mut content: Vec<Part> = last
        .map(Message::calls)
        .unwrap_or_default()
        .into_iter()
        .map(|call| Part::Result {
            call: call.id,
            name: call.name,
            content: vec![ResultPart::Text {
                text: "Not run: the turn ended before this call was made.".into(),
            }],
            is_error: true,
        })
        .collect();
    content.push(Part::Text {
        text: prompt.to_owned(),
    });
    messages.push(Message::User { content });
    if !notes.is_empty() {
        messages.push(Message::System {
            text: notes.join("\n\n"),
        });
    }
    messages
}

/// What a new conversation is told about its project.
pub(super) fn about(project: i64, name: &str) -> String {
    format!(
        "This conversation is about the person's project {project}, named {name:?}. Every \
         tool that takes `project` takes {project}."
    )
}
