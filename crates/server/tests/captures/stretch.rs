//! A capture asked for only the frames a render shows (#890): the worker draws
//! that stretch and no more, kept as a piece of the slot, and a render reusing
//! it marks the piece used.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use scorsese_render::Cancel;
use scorsese_render::page::Wanted;
use scorsese_server::captures::dispatch::Pages;
use scorsese_server::renders::evict::pages::touch;

use super::{request, spool, worker};

/// Every file under `dir` whose name starts with `prefix` and is not a partial.
fn files_in(dir: &Path, prefix: &str) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for path in entries.flatten().map(|entry| entry.path()) {
        if path.is_dir() {
            found.extend(files_in(&path, prefix));
        } else if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                name.starts_with(prefix) && name.ends_with(".mkv") && !name.contains("partial")
            })
        {
            found.push(path);
        }
    }
    found
}

#[test]
fn a_late_stretch_is_captured_as_a_piece_and_a_render_reusing_it_marks_it_used() {
    let (spool, root) = spool("stretch", "<body style='background:#00f'></body>");
    let pages = Pages {
        spool: spool.clone(),
        job: 1,
        cache: root.join("pages/7/9"),
    };
    let worker = worker(&spool, scorsese_server::captures::deadline);
    let answering = std::thread::spawn(move || {
        let job = loop {
            match worker.next() {
                Some(job) => break job,
                None => std::thread::sleep(Duration::from_millis(50)),
            }
        };
        worker.serve(&job);
    });
    // Three seconds at ten frames a second, of which a render shows the last few.
    let late = Wanted::new(request(3.0), 25..28);

    let captured = pages
        .capture(std::slice::from_ref(&late), &Cancel::new())
        .expect("not cancelled");
    answering.join().expect("the worker answered");

    assert!(captured.failed.is_empty(), "{:?}", captured.failed);
    assert!(
        files_in(&pages.cache, "frames").is_empty(),
        "not the whole page"
    );
    let pieces = files_in(&pages.cache, "part-");
    let [piece] = &pieces[..] else {
        panic!("one piece: {pieces:?}");
    };
    let name = piece
        .file_name()
        .and_then(|name| name.to_str())
        .expect("a name");
    assert!(
        name.ends_with("-28.mkv"),
        "drawn to the last frame shown: {name}"
    );
    assert!(
        !name.starts_with("part-0-"),
        "and not from the first: {name}"
    );

    let long_ago = SystemTime::now() - Duration::from_secs(49 * 3600);
    let file = std::fs::File::options().append(true).open(piece);
    file.expect("the piece")
        .set_modified(long_ago)
        .expect("aged");
    let chrome = captured.chrome.expect("the server knows the build");
    touch(&pages.folder(), &[late], chrome.version());
    let age = std::fs::metadata(piece).and_then(|meta| meta.modified());
    assert!(age.expect("a time") > long_ago + Duration::from_secs(3600));
    let _ = std::fs::remove_dir_all(root);
}
