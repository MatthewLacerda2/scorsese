//! `scorsese-server capture-launcher`: one container per capture (#852).
//!
//! The `capture-launcher` container holds the Docker socket and nothing else
//! of the service's: no network, no database, no secret. It reads the spool
//! like the worker it replaces ([`super::worker`]) and runs each capture as a
//! `docker run` of its own, whose every flag is written below. **Nothing in
//! that command comes from a job but the job's folder**: the image, the walls,
//! the caps and the command inside are constants, and the mounts are worked
//! out here from what the folder links to, each one checked against the two
//! places a project may link to. So a server that has been taken over can ask
//! for captures of what it likes, and still cannot start any other container.
//!
//! **What a capture can see** is its own job's folder, read-only; the media
//! its project links to, one file at a time, read-only; and its project's page
//! cache, the one thing it may write. Each is mounted at the path the server
//! laid it out with, so the project's links resolve inside the container just
//! as they do outside. Another user's library is not refused by a filter: it
//! is not there. And every file mounted must belong to the user whose page
//! cache the job writes to, so even a server bug laying out the wrong file
//! cannot hand one user's file to another's page.
//!
//! **The deadline** is the same as the worker's ([`super::deadline`]), kept from
//! outside: at it the container is killed by name, and with it everything the
//! browser started.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use scorsese_core::CACHE_DIR;

use super::{ASK, PAGES, PROJECT, Spool};

/// The image every capture runs: the server's own, which bakes the browser.
pub const IMAGE: &str = "scorsese-server";

/// Where the launcher's container keeps the capture's seccomp profile
/// (`deploy/capture/seccomp.json`, mounted there by compose). The Docker
/// client reads it and sends it to the daemon, so it lives beside the client.
pub const SECCOMP: &str = "/etc/scorsese/capture-seccomp.json";

/// The label every capture container carries, so a launcher that restarts can
/// remove what its predecessor left.
pub const LABEL: &str = "org.scorsese.capture";

/// The walls and caps of every capture (#773, #778): offline, no capability,
/// no new privilege, a read-only root and a user that is not root.
///
/// - A PID 1 that reaps: the browser's processes outlive their parents, and
///   without a reaper they pile up against the process cap.
/// - `/tmp` is the browser's profile and HOME: small, and memory, so nothing
///   big lands there. The frames go to the page cache, on disk (#773: frames
///   on a tmpfs count against the memory cap and blow it).
/// - #773's caps. Two cores (~245 ms a 1080p frame, so a minute of page takes
///   ~7.5 min) leave the rest of the machine to renders. 768 MiB is the 1080p
///   floor; 4K is unmeasured.
const WALLS: &[&str] = &[
    "--init",
    "--network=none",
    "--cap-drop=ALL",
    "--security-opt=no-new-privileges",
    "--read-only",
    "--tmpfs=/tmp:rw,size=64m",
    "--env=HOME=/tmp",
    "--cpus=2",
    "--memory=1g",
    "--memory-swap=1g",
    "--pids-limit=256",
];

/// `scorsese-server capture-launcher`
#[derive(Debug, clap::Args)]
pub struct Args {
    /// The spool the server writes jobs into, at the path the server sees it.
    #[arg(long)]
    pub spool: PathBuf,
    /// The library the server links media into projects from, at the path the
    /// server sees it.
    #[arg(long)]
    pub library: PathBuf,
    /// The folder this container sees the service's data at — `/data`, as the
    /// server does — so the path of each mount can be told to the host.
    #[arg(long)]
    pub data: PathBuf,
    /// Where that folder is on the host: `SCORSESE_DATA`.
    #[arg(long)]
    pub host_data: PathBuf,
    /// Who the captures run as, `uid:gid`: the host user, never root.
    #[arg(long, value_parser = user)]
    pub user: (u32, u32),
}

fn user(text: &str) -> Result<(u32, u32), String> {
    let parse = |part: &str| part.parse::<u32>().map_err(|_| "a uid:gid".to_owned());
    let (uid, gid) = text.split_once(':').ok_or("a uid:gid")?;
    let (uid, gid) = (parse(uid)?, parse(gid)?);
    if uid == 0 {
        return Err("a capture never runs as root".to_owned());
    }
    Ok((uid, gid))
}

/// How the launcher starts a capture: everything a capture's `docker run`
/// needs that is the service's, never a job's.
#[derive(Debug, Clone)]
pub struct Launch {
    spool: PathBuf,
    library: PathBuf,
    data: PathBuf,
    host_data: PathBuf,
    user: (u32, u32),
}

/// One bind mount: where it is in the container, and whether it may be
/// written to.
type Mounts = BTreeMap<PathBuf, bool>;

impl Launch {
    /// From the command line.
    pub fn from_args(args: &Args) -> Self {
        Self {
            spool: args.spool.clone(),
            library: args.library.clone(),
            data: args.data.clone(),
            host_data: args.host_data.clone(),
            user: args.user,
        }
    }

    /// The spool it reads.
    pub fn spool(&self) -> Spool {
        Spool::new(&self.spool)
    }

    /// The name of the container capturing request `index` of the job at `job`.
    pub fn name(job: &Path, index: usize) -> String {
        let job = job
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("job");
        format!("scorsese-capture-{job}-{index}")
    }

    /// The `docker run` capturing request `index` of the job at `job`, or why
    /// the job's folder is not one a capture may be given.
    pub fn command(&self, job: &Path, index: usize) -> Result<Command, String> {
        let mut command = Command::new("docker");
        command.args(self.arguments(job, index)?);
        Ok(command)
    }

    fn arguments(&self, job: &Path, index: usize) -> Result<Vec<OsString>, String> {
        let (mut mounts, owner) = self.mounts(job)?;
        mounts.insert(job.to_path_buf(), false);
        let (uid, gid) = self.user;
        let mut arguments: Vec<OsString> = vec![
            "run".into(),
            "--rm".into(),
            format!("--name={}", Self::name(job, index)).into(),
            format!("--label={LABEL}=1").into(),
            format!("--user={uid}:{gid}").into(),
            format!("--security-opt=seccomp={SECCOMP}").into(),
        ];
        arguments.extend(WALLS.iter().map(OsString::from));
        for (path, writable) in &mounts {
            arguments.push(format!("--mount={}", self.mount(path, *writable)?).into());
        }
        let project = job.join(PROJECT);
        let ask = job.join(ASK);
        arguments.extend([
            IMAGE.into(),
            "scorsese-server".into(),
            "capture-one".into(),
            "--project".into(),
            project.into_os_string(),
            "--ask".into(),
            ask.into_os_string(),
            "--index".into(),
            index.to_string().into(),
            "--follow".into(),
            library_of(&self.library, &owner).into_os_string(),
        ]);
        Ok(arguments)
    }

    /// What the job's project links to, each one checked: its page cache,
    /// writable, and the media files, read-only — all of one user's, whom it
    /// names.
    fn mounts(&self, job: &Path) -> Result<(Mounts, String), String> {
        let project = job.join(PROJECT);
        let cache = project.join(CACHE_DIR);
        let owner = owner(&self.spool, &project)
            .ok_or("the job's project has no page cache of its own to capture into")?;
        let mut mounts = Mounts::from([(cache_target(&cache)?, true)]);
        let mut links = Vec::new();
        links_in(&project, &mut links).map_err(|error| format!("reading the job: {error}"))?;
        for link in links.into_iter().filter(|link| link != &cache) {
            let file = self.media(&link, &owner).ok_or_else(|| {
                format!(
                    "the project links {} to something that is not its owner's file",
                    link.strip_prefix(&project).unwrap_or(&link).display()
                )
            })?;
            mounts.insert(file, false);
        }
        Ok((mounts, owner))
    }

    /// The library file `link` points at, if it is one of `owner`'s:
    /// `<library>/users/<owner>/…`, a file.
    fn media(&self, link: &Path, owner: &str) -> Option<PathBuf> {
        let target = plain_link(link)?;
        (target.starts_with(library_of(&self.library, owner)) && target.is_file()).then_some(target)
    }

    /// `--mount`'s value for `path`, from the host's side.
    fn mount(&self, path: &Path, writable: bool) -> Result<String, String> {
        let host = self.host_data.join(
            path.strip_prefix(&self.data)
                .map_err(|_| format!("{} is not under {}", path.display(), self.data.display()))?,
        );
        let (Some(source), Some(target)) = (plain(&host), plain(path)) else {
            return Err(format!(
                "{} has characters a mount cannot carry",
                path.display()
            ));
        };
        let access = if writable { "" } else { ",readonly" };
        Ok(format!("type=bind,source={source},target={target}{access}"))
    }
}

/// The user whose page cache the project at `project` captures into, if its
/// `cache/` is a link to one: exactly `<spool>/pages/<user>/<project>`, a
/// folder. Whose job it is, then, and so whose files it may be shown.
pub(super) fn owner(spool: &Path, project: &Path) -> Option<String> {
    let target = plain_link(&project.join(CACHE_DIR))?;
    let rest = target.strip_prefix(spool.join(PAGES)).ok()?;
    let parts: Vec<&str> = rest
        .iter()
        .map(|part| part.to_str())
        .collect::<Option<_>>()?;
    let numbers = parts.len() == 2 && parts.iter().all(|part| is_number(part));
    (numbers && target.is_dir()).then(|| parts[0].to_owned())
}

/// `owner`'s part of the library at `library`: the one folder outside its
/// project a capture's links may lead into (#857), and every file it is
/// mounted is under it.
pub(super) fn library_of(library: &Path, owner: &str) -> PathBuf {
    library.join("users").join(owner)
}

/// Where the page cache is mounted: at its link's target, so the link in the
/// read-only project resolves to it.
fn cache_target(cache: &Path) -> Result<PathBuf, String> {
    plain_link(cache).ok_or_else(|| "the job's page cache moved".to_owned())
}

/// The absolute target of the link at `link`, if it is a link whose target is
/// already where it leads: no `..`, and no further link on the way.
fn plain_link(link: &Path) -> Option<PathBuf> {
    let target = std::fs::read_link(link).ok()?;
    let plain = target.is_absolute()
        && target
            .components()
            .all(|part| matches!(part, Component::RootDir | Component::Normal(_)));
    (plain && std::fs::canonicalize(&target).ok()? == target).then_some(target)
}

/// Every link under `folder`, not following any.
fn links_in(folder: &Path, links: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(folder)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            links.push(entry.path());
        } else if kind.is_dir() {
            links_in(&entry.path(), links)?;
        }
    }
    Ok(())
}

fn is_number(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

/// `path` as text `--mount` reads one way only: no comma, quote, `=` or space.
fn plain(path: &Path) -> Option<&str> {
    let text = path.to_str()?;
    let safe = |c: char| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '-' | '_');
    text.chars().all(safe).then_some(text)
}

/// Stops the capture container `name`: the launcher's deadline.
pub fn kill(name: &str) {
    let _ = Command::new("docker")
        .args(["kill", name])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

/// Removes every capture container a previous launcher left, and says whether
/// the image and the seccomp profile a capture needs are there.
pub fn ready() -> Result<(), String> {
    if !Path::new(SECCOMP).is_file() {
        return Err(format!("no seccomp profile at {SECCOMP}"));
    }
    let image = Command::new("docker")
        .args(["image", "inspect", "--format={{.Id}}", IMAGE])
        .output()
        .map_err(|error| format!("docker would not start: {error}"))?;
    if !image.status.success() {
        let said = String::from_utf8_lossy(&image.stderr);
        return Err(format!("no {IMAGE} image to capture with: {}", said.trim()));
    }
    let left = Command::new("docker")
        .args(["ps", "--all", "--quiet", &format!("--filter=label={LABEL}")])
        .output()
        .map_err(|error| format!("docker would not start: {error}"))?;
    let left = String::from_utf8_lossy(&left.stdout);
    let left: Vec<&str> = left.split_whitespace().collect();
    if !left.is_empty() {
        let _ = Command::new("docker")
            .args(["rm", "--force"])
            .args(&left)
            .stdout(std::process::Stdio::null())
            .status();
    }
    Ok(())
}

#[cfg(test)]
mod tests;
