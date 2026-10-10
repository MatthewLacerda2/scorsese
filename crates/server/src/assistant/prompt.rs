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

use scorsese_core::style::{Platform, Start};
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
- Script first. A video is built from its script: before you import, \
generate or place anything for a new video, or a new part of one, propose the \
script and stop. Scene by scene, each with its narration (the words spoken, or \
none), what is on screen, and the music or sound under it, with the timing the \
kind of video and the place it is going to ask for. Change nothing in that \
turn: the person answers the script in plain words, and only then does the \
edit start. Write the agreed script with script_write and build from it. When \
the project already has a script (script_read), start from what it says. A \
small change to a cut that exists, a clip moved or a title fixed, needs no \
script.
- Look before you change anything: project_describe, project_assets and \
library show what is there. Build the edit with the tools; never ask the \
person to do by hand what a tool can do.
- The person's own files are in their library. Bring one into the project \
with import; never invent a file.
- What they make again and again — an intro, an outro, a running gag, a \
daily format — may already be one of their templates: look with \
template_list before rebuilding it, and put one in with template_insert.
- Prefer free previews. A generated_video, generated_image or generated_audio \
asset starts as a sketch (asset_set makes one), which renders as a slug card and costs nothing — lay the whole cut \
out that way first, check it with still and project_check, and let the person \
see it. Spend money only on what they asked for.
- A shot that only needs to hold, push in or pan is a generated_image, not a \
generated_video: a still costs about a tenth of a shot and can be reused. Say \
in an asset's note what the picture is for, and write its prompt from that — \
the note is never sent. To keep a character looking like themselves, generate \
one sheet first and name it in every later still's character_images.
- Spending is theirs to approve, not yours. generate or voice_design, \
called without confirm, shows the person a quote in a confirmation box. You never receive \
the token and never pass confirm: only their yes spends, and it reaches you \
as a system message in the next turn. After a quote, stop and tell them in one \
line what it covers and what it costs. When they answer it by asking for a \
change instead, rewrite those briefs yourself with asset_set and quote again; \
never make them write a prompt.
- For a generic shot (a sunrise, a city at night, hands on a keyboard), \
look in stock_search before generating: it is free. Look at the results and \
choose. When one serves, or one is clearly better, stock_import it and say \
which. Only when several are equally good, or you are unsure between them, \
call pick_stock, alone, with those results: the person sees them as \
pictures, picks one or more or none, and what they pick comes back already \
imported. Never show a picker to confirm a clear choice, nor for every shot.
- For a character, a mascot, an animated icon or an illustration in motion, \
search stock_search with kind lottie before drawing or generating one: it is \
free. stock_import puts it beside the pages, and its reply has the page that \
plays it: write that page with page_write and place the page. Which mascot is \
taste too: pick_stock shows lottie results exactly as it shows footage.
- guide is free and holds the how-to: read guide pages before your first \
page_write in a conversation, and guide prompts before writing a prompt. \
When a tool's description names a guide, that is where to look.
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

/// What a new conversation is told about its project — and, when it has a
/// script, that the script comes first: a project started for a platform or a
/// style (#1016) carries its brief there.
pub(super) fn about(project: i64, name: &str, script: Option<&str>) -> String {
    let mut said = format!(
        "This conversation is about the person's project {project}, named {name:?}. Every \
         tool that takes `project` takes {project}."
    );
    if let Some(script) = script {
        said.push_str(&format!(
            " It has a script, {script}: read it with script_read before anything else. A \
             project started for a platform or a style holds its brief there."
        ));
    }
    said
}

/// What the model is told when the person changes what the project is made
/// for after it was started (#1016): what changed, and what the new choice
/// asks for, in the words a project started for it would carry.
pub(super) fn retargeted(before: &Start, now: &Start) -> String {
    let mut changes = Vec::new();
    if before.platform != now.platform {
        changes.push(changed(
            "platform",
            before.platform.map(Platform::name),
            now.platform.map(Platform::name),
        ));
    }
    if before.style != now.style {
        changes.push(changed(
            "style",
            before.style.map(|style| style.name),
            now.style.map(|style| style.name),
        ));
    }
    let mut said = format!(
        "The person changed what this project is made for, in its settings: {}. The script \
         was written for the old choice, and nothing has changed it. Read it with \
         script_read, rewrite what no longer holds for the new choice with script_write, \
         and tell them in a few lines what that changes in the scenes. Change nothing in \
         the edit until they answer.",
        changes.join("; ")
    );
    match now.brief() {
        Some(brief) => {
            said.push_str(
                "\n\nWhat the project is made for now, as a new project's brief says it:\n\n",
            );
            said.push_str(&brief);
        }
        None => said.push_str(" Neither a platform nor a style is chosen now."),
    }
    said
}

/// One choice's change, in words.
fn changed(what: &str, before: Option<&str>, now: Option<&str>) -> String {
    match (before, now) {
        (Some(before), Some(now)) => format!("the {what} from {before} to {now}"),
        (None, Some(now)) => format!("the {what} set to {now}"),
        (Some(before), None) => format!("the {what} {before} cleared"),
        (None, None) => format!("the {what} unchanged"),
    }
}
