//! The Whisper server behind the trait, reached on loopback: what it is sent,
//! how its words are folded, and what each failure says.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::thread;

use scorsese_providers::transcribe::{Language, TranscribeError, Transcriber, Whisper};

/// A server on loopback that answers one request with `status` and `body`,
/// handing back what it was sent.
fn serve(status: &'static str, body: &'static str) -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("as set up");
    let address = format!("http://{}", listener.local_addr().expect("as set up"));
    let handle = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("as set up");
        let mut reader = BufReader::new(stream);
        let mut head = String::new();
        let mut length = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).expect("as set up");
            if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                length = value.trim().parse().expect("as set up");
            }
            head.push_str(&line);
            if line == "\r\n" {
                break;
            }
        }
        let mut sent = vec![0; length];
        reader.read_exact(&mut sent).expect("as set up");
        let reply = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
             Connection: close\r\n\r\n{body}",
            body.len()
        );
        reader
            .get_mut()
            .write_all(reply.as_bytes())
            .expect("as set up");
        head + &String::from_utf8_lossy(&sent)
    });
    (address, handle)
}

#[test]
fn the_words_come_back_trimmed_with_the_language_heard() {
    let (address, server) = serve(
        "200 OK",
        include_str!("../../fixtures/whisper/transcription.json"),
    );
    let heard = Whisper::new(&address)
        .transcribe(
            b"RIFF....".to_vec(),
            Some(&"PT".parse().expect("as set up")),
        )
        .expect("as set up");
    let sent = server.join().expect("as set up");
    assert!(sent.starts_with("POST /v1/audio/transcriptions "), "{sent}");
    assert!(sent.contains("name=\"language\"\r\n\r\npt\r\n"), "{sent}");
    assert!(sent.contains("deepdml/faster-whisper-large-v3-turbo-ct2"));
    assert_eq!(heard.language, "en");
    assert_eq!(heard.duration, 4.055);
    let said: Vec<&str> = heard.words.words.iter().map(|w| w.text.as_str()).collect();
    assert_eq!(
        said,
        [
            "Hello", "there,", "the", "gradient", "flows", "back", "to", "the", "start,", "again",
            "and", "again."
        ]
    );
    assert_eq!(
        (heard.words.words[2].start, heard.words.words[2].end),
        (0.92, 1.02)
    );
}

#[test]
fn a_server_not_running_is_told_how_to_start() {
    let address = {
        let listener = TcpListener::bind("127.0.0.1:0").expect("as set up");
        format!("http://{}", listener.local_addr().expect("as set up"))
    };
    let error = Whisper::new(&address)
        .transcribe(Vec::new(), None)
        .expect_err("refused");
    assert!(
        matches!(&error, TranscribeError::NotRunning { address: at, .. } if *at == address),
        "{error:?}"
    );
    let said = error.to_string();
    for needed in [
        "docker compose --file deploy/whisper/compose.yaml up --detach",
        "SCORSESE_WHISPER_URL",
        "whisper_url",
    ] {
        assert!(said.contains(needed), "{needed} not in: {said}");
    }
}

#[test]
fn a_refusal_carries_what_the_server_said() {
    let (address, server) = serve(
        "404 Not Found",
        r#"{"detail":"Model 'x' is not installed locally."}"#,
    );
    let error = Whisper::new(&address)
        .transcribe(Vec::new(), None)
        .expect_err("refused");
    server.join().expect("as set up");
    let said = error.to_string();
    assert!(said.contains("404"), "{said}");
    assert!(said.contains("is not installed locally"), "{said}");
}

#[test]
fn a_language_is_a_code_whisper_knows_the_shape_of() {
    let code = |text: &str| text.parse::<Language>().map(|l| l.as_str().to_owned());
    assert_eq!(code(" PT ").expect("as set up"), "pt");
    assert_eq!(code("haw").expect("as set up"), "haw");
    for wrong in ["Portuguese", "p", "pt-BR", ""] {
        let error = code(wrong).expect_err("refused");
        assert!(
            error.to_string().contains("like `pt` or `en`"),
            "{wrong}: {error}"
        );
    }
}
