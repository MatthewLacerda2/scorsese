use super::*;

#[test]
fn a_form_is_its_parts_between_one_boundary() {
    let form = Form::new()
        .text("name", "My voice")
        .file("files", "a.mp3", "audio/mpeg", b"ID3 one".to_vec())
        .file("files", "b.wav", "audio/wav", b"RIFF two".to_vec());
    let (content_type, body) = form.encode();
    let boundary = content_type
        .strip_prefix("multipart/form-data; boundary=")
        .expect("a multipart content type");
    let expected = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"name\"\r\n\r\nMy voice\r\n\
         --{boundary}\r\nContent-Disposition: form-data; name=\"files\"; \
         filename=\"a.mp3\"\r\nContent-Type: audio/mpeg\r\n\r\nID3 one\r\n\
         --{boundary}\r\nContent-Disposition: form-data; name=\"files\"; \
         filename=\"b.wav\"\r\nContent-Type: audio/wav\r\n\r\nRIFF two\r\n\
         --{boundary}--\r\n"
    );
    assert_eq!(String::from_utf8(body).unwrap(), expected);
    assert_eq!(form.body().1, expected.len() as u64, "the length said");
}

/// Read a few bytes at a time, as a socket takes them, the body is the
/// same bytes, and a file's are lent by the form rather than copied.
#[test]
fn a_body_read_in_small_pieces_is_the_same_bytes() {
    let speech = vec![7_u8; 1000];
    let form = Form::new()
        .file("file", "speech.wav", "audio/wav", speech)
        .text("model", "m");
    let (_, length, mut body) = form.body();
    assert!(
        body.pieces
            .iter()
            .any(|piece| matches!(piece, Cow::Borrowed(bytes) if bytes.len() == 1000))
    );
    let mut read = Vec::new();
    let mut small = [0; 7];
    loop {
        let got = body.read(&mut small).unwrap();
        if got == 0 {
            break;
        }
        read.extend_from_slice(&small[..got]);
    }
    assert_eq!(read.len() as u64, length);
    assert_eq!(read, form.encode().1);
}

#[test]
fn a_stopped_body_reads_no_further() {
    let form = Form::new().text("a", "b");
    let stopped = Arc::new(AtomicBool::new(false));
    let mut body = Stoppable {
        body: form.body().2,
        stopped: Arc::clone(&stopped),
    };
    assert!(body.read(&mut [0; 4]).is_ok());
    stopped.store(true, Ordering::SeqCst);
    assert!(body.read(&mut [0; 4]).is_err());
}

#[test]
fn the_same_form_is_the_same_bytes_and_another_is_not() {
    let form = || Form::new().text("name", "a");
    assert_eq!(form().encode(), form().encode());
    assert_ne!(form().encode().0, Form::new().text("name", "b").encode().0);
}

#[test]
fn no_part_contains_the_boundary() {
    let first = Form::new().text("x", "y").boundary();
    // A part carrying the boundary the form would otherwise have chosen.
    let form =
        Form::new()
            .text("x", "y")
            .file("files", "f", "audio/mpeg", first.clone().into_bytes());
    let boundary = form.boundary();
    assert!(
        !form
            .parts
            .iter()
            .any(|part| part.contains(boundary.as_bytes()))
    );
}

#[test]
fn a_needle_is_found_in_every_field_of_a_part_and_nowhere_else() {
    let text = |name: &str, value: &str| Part::Text {
        name: name.to_owned(),
        value: value.to_owned(),
    };
    let file = |name: &str, file_name: &str, bytes: &[u8]| Part::File {
        name: name.to_owned(),
        file_name: file_name.to_owned(),
        media_type: String::from("audio/mpeg"),
        bytes: bytes.to_vec(),
    };
    assert!(text("xNEEDLEx", "v").contains(b"NEEDLE"));
    assert!(text("n", "xNEEDLEx").contains(b"NEEDLE"));
    assert!(!text("n", "v").contains(b"NEEDLE"));
    assert!(file("xNEEDLE", "f", b"b").contains(b"NEEDLE"));
    assert!(file("n", "NEEDLE.mp3", b"b").contains(b"NEEDLE"));
    assert!(file("n", "f", b"..NEEDLE..").contains(b"NEEDLE"));
    assert!(!file("n", "f", b"b").contains(b"NEEDLE"));
}

#[test]
fn a_quote_or_a_line_break_cannot_end_a_header() {
    assert_eq!(quoted("a\"b\r\nc"), "a%22b%0D%0Ac");
}

/// Stopped after the whole body is sent, while the server is still working
/// it out, the request ends at once and the connection is closed — nothing
/// is left waiting on a reply, holding the form.
#[test]
fn a_stop_after_the_upload_closes_the_connection() {
    use std::io::{BufRead, BufReader};
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::time::Instant;

    let listener = TcpListener::bind("127.0.0.1:0").expect("as set up");
    let url = format!("http://{}/up", listener.local_addr().expect("as set up"));
    let (uploaded, heard_all) = mpsc::channel();
    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("as set up");
        let mut reader = BufReader::new(stream);
        let mut length = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).expect("as set up");
            if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                length = value.trim().parse().expect("as set up");
            }
            if line == "\r\n" {
                break;
            }
        }
        let mut sent = vec![0; length];
        reader.read_exact(&mut sent).expect("as set up");
        uploaded.send(()).expect("as set up");
        // Never answers: reads until the client hangs up.
        let mut rest = Vec::new();
        let _ = reader.read_to_end(&mut rest);
        Instant::now()
    });
    let stopped = Arc::new(AtomicBool::new(false));
    let (answer, answered) = mpsc::channel();
    let stopping = Arc::clone(&stopped);
    std::thread::spawn(move || {
        let form = Form::new().file("file", "speech.wav", "audio/wav", vec![1; 64 * 1024]);
        let reply: Result<serde_json::Value, HttpError> = Caller::anonymous().post_multipart_until(
            &url,
            &form,
            Duration::from_secs(60),
            &stopping,
        );
        let _ = answer.send(reply.is_err());
    });
    heard_all
        .recv_timeout(Duration::from_secs(10))
        .expect("the body arrives");
    let stop = Instant::now();
    stopped.store(true, Ordering::SeqCst);
    let failed = answered
        .recv_timeout(Duration::from_secs(5))
        .expect("the request ends soon after the stop, not at its timeout");
    assert!(failed, "a stopped request is no answer");
    let closed = server.join().expect("as set up");
    assert!(closed.duration_since(stop) < Duration::from_secs(5));
}
