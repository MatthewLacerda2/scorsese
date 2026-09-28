//! What a recording keeps: the vendor's bytes, minus what identifies us.

use scorsese_providers::api::tap::Exchange;
use scorsese_providers::live::record::{SCRUBBED, scrub, write};

use crate::common::scratch;

fn scrubbed(body: &str, key: &str) -> String {
    String::from_utf8(scrub(body.as_bytes(), key)).expect("text stays text")
}

#[test]
fn the_key_is_gone_wherever_it_appears() {
    let body = r#"{"echo":"sk-live-123","nested":{"why":"key sk-live-123 was sent"}}"#;
    let out = scrubbed(body, "sk-live-123");
    assert!(!out.contains("sk-live-123"), "{out}");
}

/// Everything that is not a credential stays exactly as the vendor wrote it
/// — key order, spacing, numbers — or the fixture is a shape we imagined.
#[test]
fn account_ids_are_replaced_and_nothing_else_is_touched() {
    let body = "{\"voices\": [{\"voice_id\": \"abc\", \"public_owner_id\": \"f00d\",\n  \"z\": 1.50, \"a\": null}]}";
    let out = scrubbed(body, "unused");
    let expected = body.replace("\"f00d\"", &format!("\"{SCRUBBED}\""));
    assert_eq!(out, expected);
}

#[test]
fn a_signed_url_loses_its_query_and_a_plain_one_keeps_it() {
    let signed = "https://store.example/a.mp3?X-Goog-Signature=deadbeef&X-Goog-Expires=60";
    let plain = "https://generativelanguage.googleapis.com/v1beta/files/x:download?alt=media";
    let body = format!(r#"{{"preview_url":"{signed}","uri":"{plain}"}}"#);
    let out = scrubbed(&body, "unused");
    assert!(!out.contains("deadbeef"), "{out}");
    assert!(
        out.contains("https://store.example/a.mp3?scrubbed"),
        "{out}"
    );
    assert!(out.contains(plain), "{out}");
}

#[test]
fn a_stream_is_scrubbed_event_by_event() {
    let body = "event: x\ndata: {\"type\":\"x\",\"user_id\":\"u-1\"}\n\nevent: y\ndata: {\"type\":\"y\"}\n\n";
    let out = scrubbed(body, "unused");
    assert!(!out.contains("u-1") && out.contains("event: y"), "{out}");
}

#[test]
fn media_is_left_alone() {
    let mp3 = [0xFF, 0xFB, 0x90, 0x00, 0xC3];
    assert_eq!(scrub(&mp3, "key"), mp3);
}

#[test]
fn a_recording_writes_text_bodies_and_only_mentions_media() {
    let dir = scratch("record");
    let exchange = |url: &str, status, body: &[u8]| Exchange {
        url: url.to_owned(),
        status,
        body: body.to_vec(),
    };
    let said = write(
        &dir,
        "elevenlabs",
        &[
            exchange(
                "https://api.elevenlabs.io/v1/voices?category=premade",
                200,
                b"{\"voices\":[]}",
            ),
            exchange(
                "https://api.elevenlabs.io/v1/text-to-speech/v1d?f=mp3",
                200,
                &[0xFF, 0xFB, 0x90],
            ),
            exchange(
                "https://api.anthropic.com/v1/messages",
                200,
                b"event: ping\ndata: {}\n\n",
            ),
        ],
    )
    .unwrap();
    assert!(
        dir.join("elevenlabs-1-voices-200.json").is_file(),
        "{said:?}"
    );
    assert!(said[1].contains("media, not written"), "{said:?}");
    assert!(
        dir.join("elevenlabs-3-messages-200.sse").is_file(),
        "{said:?}"
    );
    std::fs::remove_dir_all(&dir).ok();
}
