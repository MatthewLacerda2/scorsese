---
name: post-edit-feedback
description: The review of scorsese itself after a video is made with it — bugs, missing tools, documentation errors and lessons learned only while editing, each weighed for how much it would matter. Use when the user says the video is done, or once the video or audio has been rendered and delivered. While the video is still being made, findings are only noted down, never filed or discussed; a bug that stops the video is the one thing handled on the spot.
---

# Post-edit feedback

Making a real video is the hardest test scorsese gets, and most of what it
lacks is found that way: bugs, a tool that would have saved an hour, a doc that
said something untrue, a fact about where the video is posted (Instagram has a
recommended loudness). This is the user's ritual for collecting it. It happens
**after** the video, because during it the session is about the video.

## While editing: note, don't file

- Each finding goes into a scratch file named for the project the moment it is
  noticed, one line each, enough to recognise it later. A long session gets
  compacted; a finding kept only in the transcript is lost.
- No issue, no branch, no stopping to discuss it. Keep editing.
- **The one exception is a bug that stops the video being made.** Work around
  it if you can; if you cannot, fix it then (an issueless pull request is
  fine), because otherwise there is no video. It still goes on the list.

## When it runs

When the user says the video is done, or when you have rendered and delivered
the video or audio and consider it finished. Don't wait to be asked — offer
the review in the same message as the delivery.

## When the video was made on the web

The web app's built-in assistant edits there. It has no skills and files
nothing, so this review happens afterwards, in a Claude Code session, when the
maintainer asks for it. It works the same way with two differences:

- **The material is the record.** The material is the conversation the
  server kept, not notes taken while editing:
  - `chat_sessions` and `chat_turns` hold each prompt, answer, quote and
    question, with tokens and charges;
  - `tool_calls` holds every call with its arguments and reply;
  - the generation tables hold what was paid for.

  Read them from the web app's Postgres (docs/web.md), with the
  maintainer's own notes beside them. Their words in a prompt are as much a
  finding as a failed call.
- **A fifth kind: the assistant's behaviour.** For example: it chose an
  approach the person didn't want, asked for a yes that cost nothing, or
  named job numbers. The fix lands in its system prompt
  (`crates/server/src/assistant/prompt.rs`), a tool description or the
  web's chat panel, never in this skill. It is weighed and filed like a
  missing tool, unless it is plainly a bug.

## What to look for

Start from the scratch list, then go back over the session for what it missed:
retries, workarounds, anything done by hand or with raw ffmpeg or a script
because no tool did it, a tool that refused something reasonable, a value you
had to guess.

1. **Bugs** — something scorsese did wrong.
2. **Missing tools** — a feature that would have made the work faster or
   easier.
3. **Documentation errors** — a doc, tool description or skill that said
   something untrue, out of date, or left out something it should have said.
4. **Lessons learned only by doing it** — something that, had it been known
   beforehand (a line in a doc, a tip in a skill, a tool made for this kind of
   video), would have helped.

## Weigh each one

Say plainly whether each would be a **significant** improvement — it would help
other videos, save real time, or would have prevented a wrong result — or a
**small** one. Small and project-specific things are still listed, marked as
such: the user wants to see them, and a list made mostly of small things is a
sign the app is in good shape, which is worth saying too.

## Report, then file

Write the report in the user's language and in plain words, grouped by the four
kinds above, most significant first. Each item: what happened, what would have
helped, how much it matters.

Then:

- **Bugs are filed right away**, with no approval needed — whether they are
  real is not a judgement call. Search for an existing issue first (`gh issue
  list --search …`). Each carries the `agent` label (CLAUDE.md, *Issues, labels
  & priority*, has the rest).
- **Documentation errors are fixed directly**, with no approval round: a
  branch, a pull request, merged (CLAUDE.md, *Take the initiative* — making a
  description true is never the exception). File an issue instead only when
  the fix needs work beyond the words.
- Then tell the user what was filed and what was fixed, with links, so they
  know and can catch a mistake.
- **Everything else stops and waits.** Missing tools and lessons learned are
  the user's to choose from; ask which should become issues, and file only
  those.
