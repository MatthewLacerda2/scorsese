//! An attached video, as the model is shown it (#1563): a glance at it, and
//! its transcript when the library has heard it.

use scorsese_providers::chat::ResultPart;

use super::MAX_INLINE_CHARS;
use crate::assistant::pictures;
use crate::db::UserId;
use crate::http::AppState;
use crate::library::Item;

/// The longest one attached video's glance may hold the message: the turn
/// waits on it, and eight long videos must not outlast the proxy in front.
const LONGEST_GLANCE: std::time::Duration = std::time::Duration::from_secs(20);

/// A video's line in the note, and a glance at it: its sheet, and its
/// transcript when the library has heard it — or a line saying it could
/// not be shown.
pub(super) async fn video(
    state: &AppState,
    user: UserId,
    item: &Item,
) -> (String, Vec<ResultPart>) {
    let file = state
        .library
        .storage()
        .library_file(user, &item.sha256, &item.extension);
    let words = match state.library.transcript(user, &item.sha256).await {
        Ok(held) => held.words,
        Err(error) => {
            tracing::warn!("assistant: reading video {}'s transcript: {error}", item.id);
            None
        }
    };
    let ffmpeg = state.tools.ffmpeg().clone();
    let glancing =
        tokio::task::spawn_blocking(move || pictures::glanced(&ffmpeg, &file, words.as_ref()));
    let glanced = match tokio::time::timeout(LONGEST_GLANCE, glancing).await {
        Ok(joined) => joined.unwrap_or_else(|error| Err(error.to_string())),
        Err(_) => Err(format!(
            "not ready in {}s, so sent named only",
            LONGEST_GLANCE.as_secs()
        )),
    };
    let (sheet, glance) = match glanced {
        Ok(glanced) => glanced,
        Err(why) => {
            tracing::warn!("assistant: showing video {}: {why}", item.id);
            let line = format!(
                "{} “{}” — a video that could not be prepared to show you",
                item.id, item.name
            );
            return (line, Vec::new());
        }
    };
    let frames = scorsese_render::contact::counted(glance.sheet.at_seconds.len());
    let mut line = format!(
        "{} “{}” — a video: shown in the message as one sheet of {frames} chosen across it, \
         each labelled with its time",
        item.id, item.name
    );
    let mut shown = vec![sheet];
    match glance.transcript {
        Some(lines) => {
            line.push_str(", with its transcript");
            shown.insert(
                0,
                ResultPart::Text {
                    text: transcript(item.id, &item.name, &lines),
                },
            );
        }
        None => line.push_str("; it has no transcript"),
    }
    (line, shown)
}

/// A video's transcript as the message carries it, headed with what it is,
/// and cut at a line where its lines would pass [`MAX_INLINE_CHARS`] — the
/// heading and the note of the cut included.
fn transcript(id: i64, name: &str, lines: &str) -> String {
    let heading = format!(
        "Transcript of attached video {id} “{name}”, each line led by when it is said, in the same \
         times the sheet's frames are labelled with:\n\n"
    );
    let room = MAX_INLINE_CHARS.saturating_sub(heading.chars().count() + CUT.chars().count());
    let mut text = heading;
    let mut used = 0;
    let mut cut = false;
    for (index, line) in lines.lines().enumerate() {
        let needs = line.chars().count() + usize::from(index > 0);
        if used + needs > room {
            cut = true;
            break;
        }
        if index > 0 {
            text.push('\n');
        }
        text.push_str(line);
        used += needs;
    }
    if cut {
        text.push_str(CUT);
    }
    text
}

/// What ends a transcript cut short.
const CUT: &str = "\n(cut here: the rest is too long to show in a message)";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_long_transcript_is_cut_at_a_line_within_the_limit() {
        let lines = vec!["0:01 Yes."; 9_000].join("\n");
        let text = transcript(4, "talk.mp4", &lines);
        assert!(
            text.chars().count() <= MAX_INLINE_CHARS,
            "{}",
            text.chars().count()
        );
        assert!(text.ends_with(CUT));
        assert!(text.contains("\n0:01 Yes.\n(cut here"), "cut at a line");
        let short = transcript(4, "talk.mp4", "0:01 Yes.\n0:02 No.");
        assert!(short.ends_with("0:01 Yes.\n0:02 No."), "{short}");
    }
}
