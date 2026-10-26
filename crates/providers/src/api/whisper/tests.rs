use super::*;

/// What the image answered for a synthesised English sentence, sent as an MP3
/// with no language forced.
const HEARD: &str = include_str!("../../../fixtures/whisper/transcription.json");

#[test]
fn a_reply_reads_as_its_language_and_words_with_their_spaces() {
    let reply: Verbose = serde_json::from_str(HEARD).expect("as set up");
    assert_eq!(reply.language, "en");
    assert_eq!(reply.duration, 4.055);
    assert_eq!(reply.words.len(), 12);
    assert_eq!(
        reply.words[1],
        Word {
            word: " there,".into(),
            start: 0.44,
            end: 0.72,
        }
    );
    let silent: Verbose =
        serde_json::from_str(include_str!("../../../fixtures/whisper/silence.json"))
            .expect("as set up");
    assert_eq!(silent.words, []);
}

#[test]
fn the_form_asks_for_word_timings_and_a_language_only_when_forced() {
    let detected = Request {
        wav: b"RIFF".to_vec(),
        model: "deepdml/faster-whisper-large-v3-turbo-ct2",
        language: None,
    };
    let (_, body) = detected.form().encode();
    let body = String::from_utf8_lossy(&body);
    for field in [
        "name=\"model\"\r\n\r\ndeepdml/faster-whisper-large-v3-turbo-ct2\r\n",
        "name=\"response_format\"\r\n\r\nverbose_json\r\n",
        "name=\"timestamp_granularities[]\"\r\n\r\nword\r\n",
        "name=\"file\"; filename=\"speech.wav\"\r\nContent-Type: audio/wav\r\n\r\nRIFF",
    ] {
        assert!(body.contains(field), "{field} not in {body}");
    }
    assert!(!body.contains("name=\"language\""));
    let forced = Request {
        language: Some("pt"),
        ..detected
    };
    let (_, body) = forced.form().encode();
    assert!(String::from_utf8_lossy(&body).contains("name=\"language\"\r\n\r\npt\r\n"));
}

#[test]
fn the_endpoint_hangs_off_the_address_with_or_without_a_slash() {
    for address in ["http://127.0.0.1:8970", "http://127.0.0.1:8970/"] {
        assert_eq!(
            Transcriptions::new(address).url(),
            "http://127.0.0.1:8970/v1/audio/transcriptions"
        );
    }
}
