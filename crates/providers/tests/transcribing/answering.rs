//! What the server's answers mean: a refusal by its status, and an answer
//! nobody waits for any more.

use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use scorsese_providers::transcribe::{Language, TranscribeError, Transcriber, Whisper};

use super::run::never;
use super::whisper::{serve, serve_telling};

/// Each status says what it means, and only a forced language's `500` is
/// put down to the language.
#[test]
fn a_refusal_says_what_its_status_means() {
    let refused = |status: &'static str, language: Option<Language>| {
        let (address, server) = serve(status, r#"{"detail":"no"}"#);
        let error = Whisper::new(&address)
            .transcribe(Vec::new(), language.as_ref(), &never)
            .expect_err("refused")
            .to_string();
        server.join().expect("as set up");
        error
    };
    let pt = || Some("pt".parse().expect("as set up"));
    assert!(refused("404 Not Found", None).contains("rebuild it"));
    assert!(refused("413 Payload Too Large", None).contains("more than the server takes"));
    assert!(refused("503 Service Unavailable", None).contains("try again in a minute"));
    assert!(refused("500 Internal Server Error", pt()).contains("a language it does not know"));
    let unforced = refused("500 Internal Server Error", None);
    assert!(!unforced.contains("language"), "{unforced}");
    assert!(!refused("404 Not Found", pt()).contains("language"));
}

/// Stopped while the server is still listening, it answers at once; and an
/// answer landing as the stop comes is not kept either.
#[test]
fn a_stop_ends_the_wait_and_an_answer_after_it_is_dropped() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("as set up");
    let address = format!("http://{}", listener.local_addr().expect("as set up"));
    // Takes the request and never answers.
    let _silent = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("as set up");
        thread::sleep(Duration::from_secs(30));
        drop(stream);
    });
    let asked = Instant::now();
    let error = Whisper::new(&address)
        .transcribe(b"RIFF".to_vec(), None, &|| {
            asked.elapsed() > Duration::from_millis(300)
        })
        .expect_err("stopped");
    assert!(matches!(error, TranscribeError::Stopped), "{error:?}");
    assert!(
        asked.elapsed() < Duration::from_secs(5),
        "{:?}",
        asked.elapsed()
    );

    let answered = Arc::new(AtomicBool::new(false));
    let (address, server) = serve_telling(
        "200 OK",
        include_str!("../../fixtures/whisper/transcription.json"),
        Arc::clone(&answered),
    );
    let error = Whisper::new(&address)
        .transcribe(b"RIFF".to_vec(), None, &|| answered.load(Ordering::SeqCst))
        .expect_err("stopped as it answered");
    server.join().expect("as set up");
    assert!(matches!(error, TranscribeError::Stopped), "{error:?}");
}
