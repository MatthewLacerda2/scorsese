//! An encoder the ffmpeg on hand was built without, refused before anything is
//! spent (#505).
//!
//! mp3 is the one codec here that comes from an external library, and a
//! legitimate ffmpeg can be built without it. Nothing on a development machine
//! or in CI is such a build, so this stands one up: a script that answers
//! `ffmpeg -encoders` with a list lacking `libmp3lame`, and would fail at
//! anything else asked of it — which is how the test knows the question was
//! asked first.

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;

use scorsese_render::{Container, FrameRange, OutputFormat, RenderError, Renderer, Tools};

use crate::common::ffmpeg::fixture_dir;
use crate::common::project;
use crate::settings;

/// An ffmpeg that has AAC and nothing from a library.
const WITHOUT_LAME: &str = "#!/bin/sh\n\
    case \"$*\" in\n\
    *-encoders*) printf ' A....D aac                  AAC (Advanced Audio Coding)\\n' ;;\n\
    *) exit 1 ;;\n\
    esac\n";

#[test]
fn an_mp3_is_refused_by_an_ffmpeg_without_lame_and_named() {
    let dir = fixture_dir("no-lame");
    let fake = dir.join("ffmpeg");
    std::fs::write(&fake, WITHOUT_LAME).expect("write the stand-in ffmpeg");
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755))
        .expect("make it runnable");
    let tools = Tools::at(&fake, &fake);
    let out = dir.join("score.mp3");

    let refused = Renderer::new(&tools, settings(OutputFormat::defaults_for(Container::Mp3)))
        .render(&project(vec![], vec![]), &dir, FrameRange::ALL, &out);
    let written = out.exists();
    std::fs::remove_dir_all(&dir).ok();

    let error = refused.expect_err("an ffmpeg without libmp3lame cannot write an mp3");
    assert!(
        matches!(
            error,
            RenderError::MissingEncoder {
                codec: "mp3",
                library: "libmp3lame"
            }
        ),
        "{error:?}"
    );
    assert!(error.to_string().contains("libmp3lame"), "{error}");
    assert!(!written, "nothing was encoded");
}
