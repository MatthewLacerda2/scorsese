//! `scorsese transcribe`: footage's speech heard by the Whisper server named
//! in `SCORSESE_WHISPER_URL`, here one served on loopback with the reply the
//! real image gave (#1549), and a misheard word corrected with `--fix`.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::thread;

use scorsese_core::words::Words;
use scorsese_core::{AssetId, ProjectPath};
use scorsese_render::testing::tools;

use crate::common::media::sounding_video;
use crate::common::{documents, reload, run_in_env};

/// A server that answers one transcription with the captured reply, handing
/// back the request's head and how many bytes of form came with it.
fn whisper() -> (String, thread::JoinHandle<(String, usize)>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
    let address = format!("http://{}", listener.local_addr().expect("its address"));
    let served = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("one request");
        let mut reader = BufReader::new(stream);
        let (mut head, mut length) = (String::new(), 0);
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).expect("a header");
            if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                length = value.trim().parse().expect("a length");
            }
            head.push_str(&line);
            if line == "\r\n" {
                break;
            }
        }
        let mut form = vec![0; length];
        reader.read_exact(&mut form).expect("the form");
        let body = include_str!("../../../../providers/fixtures/whisper/transcription.json");
        let reply = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
             Connection: close\r\n\r\n{body}",
            body.len()
        );
        reader
            .get_mut()
            .write_all(reply.as_bytes())
            .expect("the reply");
        (head, length)
    });
    (address, served)
}

/// A project with two seconds of footage that has sound.
fn footage(label: &str) -> std::path::PathBuf {
    let dir = documents::written(
        label,
        &[documents::assets::raw("take", "video", "mp4")],
        &[],
    );
    sounding_video(&tools(), &dir, "take", "black", 2);
    dir
}

#[test]
fn footage_is_heard_and_its_words_kept_beside_it_then_corrected() {
    let dir = footage("transcribe-heard");
    let (address, served) = whisper();
    let run = run_in_env(
        &dir,
        &["transcribe", "take"],
        &[("SCORSESE_WHISPER_URL", Path::new(&address))],
    );
    assert!(!run.failed, "{}", run.output);
    let (head, sent) = served.join().expect("the server answered");
    assert!(head.starts_with("POST /v1/audio/transcriptions"), "{head}");
    // Two seconds of mono sixteen-bit speech at 16 kHz is 64 kB: the film
    // itself was not sent.
    assert!((64_000..70_000).contains(&sent), "{sent} bytes sent");
    assert!(
        run.output
            .contains("`take` transcribed (en, 4.1 s): 12 words"),
        "{}",
        run.output
    );
    assert!(
        run.output.contains("   2  0.44–0.72  there,"),
        "{}",
        run.output
    );
    let take = reload(&dir);
    let heard = Words::of(take.asset(&AssetId::new("take")).expect("take"), &dir);
    assert_eq!(heard.map(|w| w.words.len()), Some(12));

    let run = run_in_env(
        &dir,
        &["transcribe", "take", "--fix", "2=there!", "--fix", "3="],
        &[],
    );
    assert!(!run.failed, "{}", run.output);
    assert!(
        run.output.contains("2 fixes, 1 word dropped"),
        "{}",
        run.output
    );
    let words = Words::of(
        reload(&dir).asset(&AssetId::new("take")).expect("take"),
        &dir,
    )
    .expect("kept");
    assert_eq!(words.words[1].text, "there!");
    assert_eq!(words.words.len(), 11);
    assert!(
        Words::transcript(&ProjectPath::new("assets/take.mp4"))
            .resolve(&dir)
            .is_file()
    );
}

#[test]
fn a_server_not_running_is_told_how_to_start() {
    let dir = footage("transcribe-down");
    let closed = {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        format!("http://{}", listener.local_addr().expect("its address"))
    };
    let run = run_in_env(
        &dir,
        &["transcribe", "take"],
        &[("SCORSESE_WHISPER_URL", Path::new(&closed))],
    );
    assert!(run.failed);
    assert!(
        run.output
            .contains("docker compose --file deploy/whisper/compose.yaml up --detach"),
        "{}",
        run.output
    );
    let run = run_in_env(&dir, &["transcribe", "take", "--fix", "x"], &[]);
    assert!(run.failed);
    assert!(run.output.contains("NUMBER=TEXT"), "{}", run.output);
}
