use std::os::unix::fs::symlink;

use scorsese_core::CACHE_DIR;

use crate::captures::PROJECT;
use crate::captures::launch::tests::Laid;

#[test]
fn what_is_mounted_is_the_copy_made_whatever_the_server_does_after() {
    let laid = Laid::new("swapped");
    laid.stage().unwrap();
    let page = laid.job.join(PROJECT).join("pages/title.html");
    std::fs::remove_file(&page).unwrap();
    symlink("/etc/passwd", &page).unwrap();

    let staged = laid.staged().join("pages/title.html");

    assert!(!staged.is_symlink());
    assert_eq!(std::fs::read_to_string(staged).unwrap(), "<h1>mine</h1>");
    assert_eq!(
        std::fs::read_link(laid.staged().join("assets/a.png")).unwrap(),
        laid.file("7")
    );
    assert_eq!(
        std::fs::read_dir(laid.staged().join(CACHE_DIR))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn a_fifo_in_the_project_refuses_the_job() {
    let laid = Laid::new("fifo");
    let made = scorsese_render::tools::child("mkfifo")
        .arg(laid.job.join(PROJECT).join("pages/pipe"))
        .status()
        .unwrap();
    assert!(made.success());
    let why = laid.stage().unwrap_err();
    assert!(why.contains("pages/pipe"), "{why}");
}

#[test]
fn a_link_in_the_way_of_a_media_file_refuses_the_job() {
    let laid = Laid::new("in-the-way");
    let theirs = laid.launch.library.join("users/7");
    std::fs::rename(theirs.join("library"), theirs.join("real")).unwrap();
    symlink(theirs.join("real"), theirs.join("library")).unwrap();
    assert!(laid.stage().is_err());
}

#[test]
fn only_the_files_and_folders_a_capture_wrote_reach_the_page_cache() {
    let laid = Laid::new("publish");
    let staged = laid.stage().unwrap();
    laid.launch.command(&staged, 0).unwrap();
    let out = laid.launch.staging.join("out/job-1-0");
    std::fs::create_dir_all(out.join("pages/slot/told-none")).unwrap();
    std::fs::write(out.join("pages/slot/told-none/frames.mkv"), "frames").unwrap();
    std::fs::write(out.join("pages/slot/part-0-9.123.0.partial.mkv"), "cut").unwrap();
    symlink("/etc/passwd", out.join("pages/slot/capture.json")).unwrap();
    let made = scorsese_render::tools::child("mkfifo")
        .arg(out.join("pages/slot/pipe"))
        .status()
        .unwrap();
    assert!(made.success());

    laid.launch.publish(&staged, 0).unwrap();

    let slot = laid.pages().join("pages/slot");
    assert_eq!(
        std::fs::read_to_string(slot.join("told-none/frames.mkv")).unwrap(),
        "frames"
    );
    let left: Vec<_> = std::fs::read_dir(&slot)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(left, ["told-none"]);
    assert!(!out.exists(), "what the capture wrote is gone once kept");
}

#[test]
fn keeping_a_capture_never_writes_through_a_link_in_the_page_cache() {
    let laid = Laid::new("publish-link");
    let staged = laid.stage().unwrap();
    laid.launch.command(&staged, 0).unwrap();
    let out = laid.launch.staging.join("out/job-1-0");
    std::fs::create_dir_all(out.join("pages")).unwrap();
    std::fs::write(out.join("pages/frames.mkv"), "frames").unwrap();
    let elsewhere = laid.root.join("elsewhere");
    std::fs::create_dir_all(&elsewhere).unwrap();
    symlink(&elsewhere, laid.pages().join("pages")).unwrap();

    assert!(laid.launch.publish(&staged, 0).is_err());
    assert_eq!(std::fs::read_dir(&elsewhere).unwrap().count(), 0);
}

#[test]
fn a_job_once_served_leaves_nothing_staged() {
    let laid = Laid::new("unstage");
    let staged = laid.stage().unwrap();
    laid.launch.command(&staged, 0).unwrap();
    laid.launch.command(&staged, 1).unwrap();

    laid.launch.unstage(&staged);

    assert!(!laid.launch.staging.join("jobs/job-1").exists());
    assert_eq!(
        std::fs::read_dir(laid.launch.staging.join("out"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn a_link_on_the_owners_own_folder_refuses_the_job() {
    let laid = Laid::new("owner-linked");
    let users = laid.launch.library.join("users");
    std::fs::rename(users.join("7"), users.join("seven")).unwrap();
    symlink("seven", users.join("7")).unwrap();
    assert!(laid.stage().is_err());
}

#[test]
fn a_record_is_kept_only_after_the_frames_it_vouches_for() {
    let laid = Laid::new("record-last");
    let staged = laid.stage().unwrap();
    laid.launch.command(&staged, 0).unwrap();
    let out = laid.launch.staging.join("out/job-1-0/pages/told-none");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("frames.mkv"), "new frames").unwrap();
    std::fs::write(out.join("capture.json"), "new record").unwrap();
    let shelf = laid.pages().join("pages/told-none");
    std::fs::create_dir_all(shelf.join("frames.mkv")).unwrap();
    std::fs::write(shelf.join("capture.json"), "old record").unwrap();

    assert!(
        laid.launch.publish(&staged, 0).is_err(),
        "the frames could not land"
    );

    let record = std::fs::read_to_string(shelf.join("capture.json")).unwrap();
    assert_eq!(record, "old record");
}

#[test]
fn a_file_the_page_cache_already_holds_as_it_is_is_left_alone() {
    use std::os::unix::fs::MetadataExt;
    let laid = Laid::new("same");
    let fonts = laid.pages().join("pages/fonts");
    std::fs::create_dir_all(&fonts).unwrap();
    std::fs::write(fonts.join("a.ttf"), "face").unwrap();
    std::fs::write(fonts.join("b.ttf"), "old!").unwrap();
    let inode = |name: &str| std::fs::metadata(fonts.join(name)).unwrap().ino();
    let (a, b) = (inode("a.ttf"), inode("b.ttf"));
    let staged = laid.stage().unwrap();
    laid.launch.command(&staged, 0).unwrap();
    let out = laid.launch.staging.join("out/job-1-0/pages/fonts");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("a.ttf"), "face").unwrap();
    std::fs::write(out.join("b.ttf"), "new!").unwrap();

    laid.launch.publish(&staged, 0).unwrap();

    assert_eq!(inode("a.ttf"), a, "the same bytes are not copied again");
    assert_ne!(inode("b.ttf"), b);
    assert_eq!(
        std::fs::read_to_string(fonts.join("b.ttf")).unwrap(),
        "new!"
    );
}
