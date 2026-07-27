//! What a caller who names only a file gets.
//!
//! The defaults are the whole compatibility story: every invocation written
//! before the format was a setting has to still mean what it meant, and every
//! container has to have an answer of its own rather than mp4's.

use std::path::Path;

use scorsese_render::{AudioCodec, Container, OutputFormat, VideoCodec};

#[test]
fn the_default_format_is_what_delivery_has_always_meant() {
    let format = OutputFormat::default();
    assert_eq!(format.container(), Container::Mp4);
    assert_eq!(format.video(), VideoCodec::H264);
    assert_eq!(format.audio(), AudioCodec::Aac);
}

/// Per container, not globally. An avi defaulting to H.264 would be the bug
/// this whole setting exists to remove, so it is asserted container by
/// container rather than through a loop that would pass on one answer.
#[test]
fn each_container_defaults_to_its_own_codecs() {
    let pairs = [
        (Container::Mp4, VideoCodec::H264, AudioCodec::Aac),
        (Container::Mkv, VideoCodec::H264, AudioCodec::Aac),
        (Container::Avi, VideoCodec::Mpeg4, AudioCodec::PcmS16Le),
        (Container::Wmv, VideoCodec::Wmv2, AudioCodec::Wmav2),
    ];
    for (container, video, audio) in pairs {
        let format = OutputFormat::defaults_for(container);
        assert_eq!(format.video(), video, "{container} defaults to {video}");
        assert_eq!(format.audio(), audio, "{container} defaults to {audio}");
    }
}

/// Naming nothing but the container is the same thing as taking its defaults,
/// so the two ways in cannot drift apart.
#[test]
fn naming_no_codec_takes_the_containers_own() {
    for container in Container::ALL {
        let asked = OutputFormat::new(container, None, None).expect("defaults are accepted");
        assert_eq!(asked, OutputFormat::defaults_for(container));
    }
}

/// The default is the first codec listed, which is also what
/// `docs/output-formats.md` promises a reader.
#[test]
fn the_default_is_the_first_codec_listed() {
    for container in Container::ALL {
        let format = OutputFormat::defaults_for(container);
        assert_eq!(Some(&format.video()), container.video_codecs().first());
        assert_eq!(Some(&format.audio()), container.audio_codecs().first());
    }
}

#[test]
fn an_output_paths_extension_names_its_container() {
    for container in Container::ALL {
        let path = format!("cut.{container}");
        let found = Container::from_path(Path::new(&path)).expect("a container we write");
        assert_eq!(found, container);
    }
}

/// Nobody types extensions consistently, and `CUT.MP4` is an mp4.
#[test]
fn an_extension_is_read_whatever_its_case() {
    let found = Container::from_path(Path::new("CUT.MP4")).expect("still an mp4");
    assert_eq!(found, Container::Mp4);
}

#[test]
fn a_container_is_parsed_from_its_own_name() {
    for container in Container::ALL {
        assert_eq!(container.to_string().parse(), Ok(container));
    }
}

#[test]
fn codecs_are_parsed_from_their_own_names() {
    for codec in VideoCodec::ALL {
        assert_eq!(codec.to_string().parse(), Ok(codec));
    }
    for codec in AudioCodec::ALL {
        assert_eq!(codec.to_string().parse(), Ok(codec));
    }
}
