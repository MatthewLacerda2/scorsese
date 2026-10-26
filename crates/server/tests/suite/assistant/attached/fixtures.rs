//! What a test attaches: files of each kind, put in a member's library.

use std::path::PathBuf;

use scorsese_render::testing::generate;
use scorsese_server::db::UserId;
use scorsese_server::http::AppState;
use scorsese_server::library::{Arrival, Kind};

use super::super::common;

/// `name` in `user`'s library: a square of a colour of its own for a
/// picture, a second of test card and tone for a video, a tone for a sound,
/// a line of text otherwise.
pub(crate) async fn kept(state: &AppState, user: UserId, name: &str, kind: Kind) -> i64 {
    let scratch = common::scratch::dir("attached");
    let file: PathBuf = scratch.join(name);
    let colour = name.bytes().fold(0x5a_5a5a_u32, |hash, byte| {
        hash.wrapping_mul(31).wrapping_add(u32::from(byte)) & 0xff_ffff
    });
    let card = format!("color=c=0x{colour:06x}:s=16x16");
    match kind {
        Kind::Image => generate(
            &common::tools(),
            &file,
            &["-f", "lavfi", "-i", &card, "-frames:v", "1"],
        ),
        Kind::Video => generate(
            &common::tools(),
            &file,
            &[
                "-f",
                "lavfi",
                "-i",
                "testsrc=s=64x36:d=1:r=30",
                "-f",
                "lavfi",
                "-i",
                "sine=duration=1",
                "-shortest",
                "-pix_fmt",
                "yuv420p",
            ],
        ),
        Kind::Audio => generate(
            &common::tools(),
            &file,
            &["-f", "lavfi", "-i", "sine=duration=1"],
        ),
        _ => return written(state, user, name, "Scene one: a door opens.").await,
    }
    admitted(state, user, file, name, kind).await
}

/// A document called `name` holding `text` in `user`'s library.
pub(crate) async fn written(state: &AppState, user: UserId, name: &str, text: &str) -> i64 {
    let file = common::scratch::dir("attached").join(name);
    std::fs::write(&file, text).expect("the test setup holds");
    admitted(state, user, file, name, Kind::Text).await
}

/// `file` admitted to `user`'s library as `name`, of `kind`.
async fn admitted(state: &AppState, user: UserId, file: PathBuf, name: &str, kind: Kind) -> i64 {
    let extension = name.rsplit_once('.').map(|(_, e)| e).expect("an extension");
    let arrival = Arrival {
        file,
        name: name.to_owned(),
        kind,
        extension: extension.to_owned(),
        announced: None,
        brief_hash: None,
        words: None,
    };
    state
        .library
        .admit(user, arrival)
        .await
        .expect("the test setup holds")
        .id
}
