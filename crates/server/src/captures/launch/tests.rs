use std::os::unix::fs::symlink;

use super::*;
use crate::captures::Ask;

/// A service's data folder with two users' libraries, and job 1 laid out for
/// user 7's project 9: one picture of theirs linked in, a page, its cache
/// linked to its page cache — and the launcher's staging beside it.
pub(super) struct Laid {
    pub(super) root: PathBuf,
    pub(super) launch: Launch,
    pub(super) job: PathBuf,
}

impl Laid {
    pub(super) fn new(name: &str) -> Self {
        let root = crate::scratch::fresh(&format!("scorsese-launch-{name}"), "");
        let _ = std::fs::remove_dir_all(&root);
        let data = root.join("data");
        for user in ["7", "8"] {
            let library = data.join("library/users").join(user).join("library");
            std::fs::create_dir_all(&library).unwrap();
            std::fs::write(library.join("abc.png"), user).unwrap();
        }
        let spool = Spool::new(data.join("cache/captures"));
        let job = spool.job(1);
        let project = job.join(PROJECT);
        std::fs::create_dir_all(project.join("assets")).unwrap();
        std::fs::create_dir_all(project.join("pages")).unwrap();
        std::fs::write(project.join("pages/title.html"), "<h1>mine</h1>").unwrap();
        std::fs::create_dir_all(spool.root().join("pages/7/9")).unwrap();
        symlink(spool.root().join("pages/7/9"), project.join(CACHE_DIR)).unwrap();
        let laid = Self {
            launch: Launch {
                spool: spool.root().to_path_buf(),
                library: data.join("library"),
                data,
                data_volume: "scorsese-capture-data".into(),
                staging: root.join("staging"),
                staging_volume: "scorsese-capture-staging".into(),
                user: (1000, 1000),
            },
            job,
            root,
        };
        laid.link("assets/a.png", &laid.file("7"));
        laid
    }

    pub(super) fn file(&self, user: &str) -> PathBuf {
        self.launch
            .library
            .join("users")
            .join(user)
            .join("library/abc.png")
    }

    pub(super) fn link(&self, at: &str, to: &Path) {
        symlink(to, self.job.join(PROJECT).join(at)).unwrap();
    }

    pub(super) fn ask() -> Ask {
        Ask {
            requests: Vec::new(),
        }
    }

    pub(super) fn stage(&self) -> Result<Staged, String> {
        self.launch.stage(&self.job, &Self::ask())
    }

    pub(super) fn arguments(&self) -> Result<Vec<String>, String> {
        let staged = self.stage()?;
        let arguments = self.launch.arguments(&staged, 0)?;
        Ok(arguments
            .into_iter()
            .map(|argument| argument.into_string().unwrap())
            .collect())
    }

    /// Where the launcher staged the job's project.
    pub(super) fn staged(&self) -> PathBuf {
        self.launch.staging.join("jobs/job-1").join(PROJECT)
    }

    /// The page cache the server reads.
    pub(super) fn pages(&self) -> PathBuf {
        self.launch.spool.join("pages/7/9")
    }
}

impl Drop for Laid {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn a_capture_gets_its_job_its_page_cache_and_its_owners_files_and_nothing_else() {
    let laid = Laid::new("mounts");
    let arguments = laid.arguments().unwrap();
    let mounts: Vec<&str> = arguments
        .iter()
        .filter_map(|argument| argument.strip_prefix("--mount="))
        .collect();
    let data = laid.launch.data.display().to_string();
    let job = format!("{data}/cache/captures/jobs/job-1");
    let staged = "type=volume,source=scorsese-capture-staging";
    assert_eq!(
        mounts,
        [
            format!("{staged},volume-subpath=jobs/job-1,target={job},readonly"),
            format!("{staged},volume-subpath=out/job-1-0,target={job}/project.scor/cache"),
            format!(
                "type=volume,source=scorsese-capture-data,\
                 volume-subpath=library/users/7/library/abc.png,\
                 target={data}/library/users/7/library/abc.png,readonly"
            ),
        ]
    );
}

#[test]
fn the_command_is_the_services_own_whatever_the_job() {
    let laid = Laid::new("command");
    let arguments = laid.arguments().unwrap();
    for wall in WALLS {
        assert!(arguments.iter().any(|argument| argument == wall), "{wall}");
    }
    assert!(arguments.contains(&format!("--security-opt=seccomp={SECCOMP}")));
    assert!(arguments.contains(&"--user=1000:1000".to_owned()));
    let image = arguments
        .iter()
        .position(|argument| argument == IMAGE)
        .unwrap();
    let job = laid.job.display();
    let theirs = laid.launch.library.join("users/7");
    assert_eq!(
        arguments[image + 1..],
        [
            "scorsese-server".to_owned(),
            "capture-one".into(),
            "--project".into(),
            format!("{job}/project.scor"),
            "--ask".into(),
            format!("{job}/ask.json"),
            "--index".into(),
            "0".into(),
            // The owner's library, and only theirs: where its media links
            // lead (#857).
            "--follow".into(),
            theirs.display().to_string(),
        ]
    );
}

#[test]
fn a_link_to_another_users_file_refuses_the_job() {
    let laid = Laid::new("theirs");
    laid.link("assets/b.png", &laid.file("8"));
    let why = laid.arguments().unwrap_err();
    assert!(why.contains("assets/b.png"), "{why}");
}

#[test]
fn a_link_out_of_the_library_or_round_about_refuses_the_job() {
    for (name, to) in [
        ("outside", PathBuf::from("/etc/passwd")),
        (
            "dotdot",
            PathBuf::from("LIBRARY/users/7/../8/library/abc.png"),
        ),
        ("relative", PathBuf::from("../../x.png")),
    ] {
        let laid = Laid::new(name);
        let to = to
            .to_str()
            .unwrap()
            .replace("LIBRARY", laid.launch.library.to_str().unwrap());
        laid.link("assets/b.png", Path::new(&to));
        assert!(laid.arguments().is_err(), "{name}");
    }
}

#[test]
fn a_job_without_a_page_cache_of_its_own_is_refused() {
    for (name, to) in [
        ("library", "LIBRARY/users/7"),
        ("user", "SPOOL/pages/7"),
        ("named", "SPOOL/pages/7/nine"),
        ("file", "SPOOL/pages/7/8"),
    ] {
        let laid = Laid::new(name);
        let spool = laid.launch.spool.to_str().unwrap();
        std::fs::create_dir_all(Path::new(spool).join("pages/7/nine")).unwrap();
        std::fs::write(Path::new(spool).join("pages/7/8"), "").unwrap();
        let to = to
            .replace("LIBRARY", laid.launch.library.to_str().unwrap())
            .replace("SPOOL", spool);
        let cache = laid.job.join(PROJECT).join(CACHE_DIR);
        std::fs::remove_file(&cache).unwrap();
        symlink(to, &cache).unwrap();
        let why = laid.arguments().unwrap_err();
        assert!(why.contains("page cache"), "{name}: {why}");
    }
}

#[test]
fn a_link_to_a_folder_of_the_owners_is_not_a_file_of_theirs() {
    let laid = Laid::new("folder");
    laid.link("assets/all", &laid.launch.library.join("users/7/library"));
    assert!(laid.arguments().is_err());
}

#[test]
fn a_path_a_mount_would_misread_is_refused() {
    let laid = Laid::new("comma");
    let odd = laid
        .launch
        .library
        .join("users/7/library/a,readonly=false.png");
    std::fs::write(&odd, "x").unwrap();
    laid.link("assets/b.png", &odd);
    assert!(laid.arguments().unwrap_err().contains("characters"));
}

#[test]
fn a_capture_never_runs_as_root() {
    assert_eq!(user("1000:100"), Ok((1000, 100)));
    assert!(user("0:0").is_err());
    assert!(user("1000").is_err());
    assert!(user("me:me").is_err());
}

#[test]
fn each_capture_is_named_by_its_job_and_place() {
    assert_eq!(
        Launch::name(Path::new("/s/jobs/job-12"), 3),
        "scorsese-capture-job-12-3"
    );
}

#[test]
fn a_job_folder_that_is_a_link_is_refused() {
    let laid = Laid::new("linked-job");
    let elsewhere = laid.root.join("elsewhere");
    std::fs::rename(&laid.job, &elsewhere).unwrap();
    symlink(&elsewhere, &laid.job).unwrap();
    assert!(laid.arguments().is_err());
}

#[test]
fn no_mount_is_a_host_path_the_server_could_change() {
    // The daemon resolves a bind's source again, on the host, after the
    // launcher's checks: anything under SCORSESE_DATA could be a link by then
    // (#1636). Every mount is a volume's, resolved inside the volume.
    let laid = Laid::new("no-binds");
    let arguments = laid.arguments().unwrap();
    let mounts: Vec<&String> = arguments
        .iter()
        .filter(|argument| argument.starts_with("--mount="))
        .collect();
    assert!(!mounts.is_empty());
    for mount in mounts {
        assert!(!mount.contains("type=bind"), "{mount}");
    }
}

#[test]
fn a_capture_never_mounts_the_page_cache_the_server_reads() {
    // A capture still running could otherwise swap a link in after the
    // server's checks of what it wrote (#1677).
    let laid = Laid::new("no-pages");
    let arguments = laid.arguments().unwrap();
    for mount in arguments.iter().filter(|a| a.starts_with("--mount=")) {
        assert!(!mount.contains("captures/pages"), "{mount}");
    }
}

#[test]
fn the_daemon_must_mount_by_volume_subpath() {
    assert_eq!(api_version("1.56\n"), Some((1, 56)));
    assert!(api_version("1.44").unwrap() < SUBPATH_API);
    assert!(api_version("1.45").unwrap() >= SUBPATH_API);
    assert_eq!(api_version("garbage"), None);
}

#[test]
fn capture_worker_follows_no_link_to_a_page_cache() {
    let laid = Laid::new("owner-link");
    let pages = laid.launch.spool.join("pages");
    std::fs::create_dir_all(pages.join("8/9")).unwrap();
    std::fs::remove_dir_all(pages.join("7")).unwrap();
    symlink("8", pages.join("7")).unwrap();
    let project = laid.job.join(PROJECT);
    assert_eq!(owner(&laid.launch.spool, &project), None);
    std::fs::remove_file(pages.join("7")).unwrap();
    std::fs::create_dir_all(pages.join("7/9")).unwrap();
    assert_eq!(owner(&laid.launch.spool, &project), Some("7".into()));
}
