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
use scorsese_providers::chat::{Effort, Message, Part, ResultPart, Tool};
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
- The vision is yours to bring. People rarely arrive with the video worked \
out, so never wait for them to: from what they give you, propose the whole of \
it — the story, the tone, the length, where it is going — and fill what they \
left open with your own choices, said plainly so they can change them. When \
an answer would change the video, ask: the few questions that matter, at \
once, then go on.
- Script first. A video is built from its script: before you import, \
generate or place anything for a new video, or a new part of one, propose the \
script and stop. Scene by scene, each with its narration (the words spoken, or \
none), what is on screen, and the music or sound under it, with the timing the \
kind of video and the place it is going to ask for. Change nothing in that \
turn: the person answers the script in plain words, and only then does the \
edit start. When anything is to be generated (a shot, a still, a spoken \
line), the script carries its storyboard too. It opens with the story in one \
sentence, then describes every generated shot in words: what is in the frame, \
the framing and camera, what moves, how long it runs, and what it tells. It is \
text, never drawings unless they ask; read guide prompts before writing it. \
Write the agreed script with script_write and build from it; the person \
reads and edits it from the Script button above this chat. When \
the project already has a script (script_read), start from what it says. A \
small change to a cut that exists, a clip moved or a title fixed, needs no \
script.
- Look before you change anything: project_describe, project_assets and \
library show what is there. Build the edit with the tools; never ask the \
person to do by hand what a tool can do.
- The person's own files are in their library. Bring one into the project \
with import; never invent a file. A document there (kind text) is a script, \
shot list or briefing they wrote for you: read it with library_read.
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
called without confirm, shows the person a quote in a confirmation box and \
waits: their answer comes back as that call's result — what the spend did, a \
no, or a change they want. You never receive the token and never pass confirm: \
only their yes spends. Ask for everything at once: put every paid item the \
agreed plan needs in one generate call, so they approve once. When they answer \
by asking for a change, rewrite those briefs yourself with asset_set and quote \
again; never make them write a prompt.
- To narrate in someone's own voice, voice_clone clones it from recordings \
they uploaded. Clone only their own voice, or one whose owner has given them \
permission: before calling it, ask them in those words, and set consent only \
on their yes. Never clone a voice from a recording they did not make or have \
leave to use.
- A generation takes minutes, and lands in the project on its own when it \
finishes. Once one is queued, say what is on its way and end the turn: when \
the last of it lands, you are woken with a note saying what landed, and carry \
on from there. Never call jobs again and again to wait it out. Check a job \
only when the person asks after it or a later step in this turn needs what it \
made; for that step, jobs with its job id and wait: true waits for it, a \
minute at most per call.
- Stock first. Any shot or still a library could have (a sunrise, a city \
at night, hands on a keyboard) comes from stock_search: it is free. Generate \
one only when it must be custom-made (their product, their character, a \
reference they gave you) or when they ask for a generation. A sample or demo \
video needs no real product: invent a subject stock can film. Narration is \
the opposite: it is inexpensive, so propose a voice whenever the video is \
better for one. Look at the stock results and \
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
- Your words are shown as Markdown: bold, italics and short lists are fine. \
Never tables, images or `code`; they do not show, or show the workings.
- Name things by what they are. The person sees their edit, never its \
workings: in what you write to them, never name a job number, a file or \
folder, an asset, clip or project id, or a tool. Say what the thing is: \
\"the two office pictures\", \"the script\", \"the wide shot\", \"the \
narration is on its way\". Ids belong in tool calls only, and the ids, paths \
and tool names that tool results and system messages hand you are for you, \
never to repeat.
- Answer in the language the person writes in: the language of their own \
messages, whatever it is. You are not told the language of the website they \
use, and it does not matter. A message in parentheses that says the app or \
the server sent it was not written by them: answer it in the language of \
their own messages earlier in the conversation, or, when they have written \
none yet, of the words they gave the project (its idea, its script), and \
English when there are none. Say plainly when something \
is refused, and what you did instead.";

/// What the model reads for a turn the person started with a button rather
/// than words — a settings change. The person's chat shows the web app's
/// words in the site's language; the model is told this instead, so it never
/// learns the site's language and answers in the person's (2026-10-16).
pub(super) fn app_said(what: &str) -> String {
    format!("(Sent by the app, not written by the person: {what})")
}

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

/// A turn's first messages: the user's words, after a result for every tool
/// call the conversation left unanswered; an effort change when `effort` is
/// not the level in effect (#1443); and, when there is anything the server
/// has to say, one note saying it.
///
/// A conversation whose last turn ended before the model replied (refused,
/// stopped, a failure, a restart) ends on the user's side; every vendor wants
/// the sides to alternate, and Claude wants a `system` message followed by
/// the model's, so such a turn is closed first by a one-line reply saying so.
/// That line is part of this turn's messages from then on, like any other.
pub(super) fn opening(
    history: &[Message],
    prompt: &str,
    notes: &[String],
    effort: Effort,
) -> Vec<Message> {
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
    // Directly after the words, which sets the level of the reply to them
    // (Anthropic's *Effort* page, read 2026-10-18: "Placed directly after a
    // `user` turn with new input, it sets the level for Claude's reply to
    // that turn"). Never before them: when they carry the results of a turn
    // stopped mid-call, that would split a `tool_use` from its `tool_result`.
    // Before the note, which must be followed by the reply.
    messages.push(Message::User { content });
    messages.extend(effort.change(history));
    if !notes.is_empty() {
        messages.push(Message::System {
            text: notes.join("\n\n"),
        });
    }
    messages
}

/// The user message of a turn the server started when work landed (#1441):
/// every turn opens with one, and the person wrote nothing, so the server
/// says so in words the model cannot take for theirs. Its note follows.
pub(super) const WOKEN: &str =
    "(The person has not written. This message is the server's: it started this turn.)";

/// What a new conversation is told about its project — and, when it has a
/// script, that the script comes first: a project started for a platform or a
/// style (#1016) carries its brief there. The script's path is not given: the
/// model repeats what it reads, and the person never sees a file (#1442).
pub(super) fn about(project: i64, name: &str, scripted: bool) -> String {
    let mut said = format!(
        "This conversation is about the person's project {project}, named {name:?}. Every \
         tool that takes `project` takes {project}."
    );
    if scripted {
        said.push_str(
            " It has a script: read it with script_read before anything else. A project \
             started for a platform or a style holds its brief there.",
        );
    }
    said
}

/// What the next turn is told when the person rewrote the script by hand
/// from the editor's Script button (#1445) since the last one began — or a
/// paused turn, as it resumes, when they rewrote it while it waited (#1488).
pub(in crate::assistant) const SCRIPT_REWRITTEN: &str = "The person edited the script \
     themselves since you last heard from them. Read it again with script_read before relying on what you remember of it; \
     their version is the one to build from.";

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
    // The frame is the document's, and only a re-fit changes it (#1278).
    if before.platform != now.platform
        && let Some(aspect) = now.aspect()
    {
        said.push_str(&format!(
            " The new placement is watched {aspect}: once they agree, and if the project \
             is not {aspect} already, change its frame with project_reframe, which says \
             what it could not re-fit."
        ));
    }
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// The kind of each message, in order.
    fn kinds(messages: &[Message]) -> Vec<&'static str> {
        messages
            .iter()
            .map(|message| match message {
                Message::User { .. } => "user",
                Message::Assistant { .. } => "assistant",
                Message::System { .. } => "system",
                Message::Effort { .. } => "effort",
            })
            .collect()
    }

    #[test]
    fn an_effort_change_follows_the_words_it_sets_the_reply_to() {
        let history = [Message::assistant("Done.")];
        let notes = ["The person said yes.".to_owned()];
        let opened = opening(&history, "now the outro", &notes, Effort::Low);
        assert_eq!(kinds(&opened), ["user", "effort", "system"]);
        let unchanged = opening(&history, "now the outro", &[], Effort::BASELINE);
        assert_eq!(kinds(&unchanged), ["user"], "no change, no message");
    }

    #[test]
    fn an_effort_change_never_splits_a_call_from_its_result() {
        // A turn stopped mid-call: its last message calls a tool nobody ran.
        let stopped = Message::Assistant {
            content: vec![Part::Call {
                id: "toolu_1".into(),
                name: "still".into(),
                input: json!({}),
            }],
        };
        let opened = opening(&[stopped], "carry on", &[], Effort::Xhigh);
        assert_eq!(kinds(&opened), ["user", "effort"]);
        assert!(matches!(
            opened[0].parts().first(),
            Some(Part::Result { call, .. }) if call == "toolu_1"
        ));
        assert_eq!(
            opened[1],
            Message::Effort {
                effort: Effort::Xhigh
            }
        );
    }
}
