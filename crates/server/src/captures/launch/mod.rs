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
//! **What a capture can see** is a copy of its own job's folder, read-only;
//! the media its project links to, one file at a time, read-only; and an
//! empty folder of its own as the project's `cache/`, the one thing it may
//! write. Each is mounted at the path the server laid it out with, so the
//! project's links resolve inside the container just as they do outside.
//! Another user's library is not refused by a filter: it is not there. And
//! every file mounted must belong to the user whose page cache the job writes
//! to, so even a server bug laying out the wrong file cannot hand one user's
//! file to another's page.
//!
//! **Nothing mounted is a path the server can change** (#1636). The Docker
//! daemon looks a bind's source up again, on the host, when it makes the
//! container, and everything under `SCORSESE_DATA` is the server's to swap for
//! a link by then: a checked file could arrive as any file on the host, the
//! page cache as any folder, writable. So no mount is a bind:
//!
//! - **The job** is copied ([`Launch::stage`]) into `staging/jobs/<job>`, a
//!   volume only this container mounts, each file read from a handle opened
//!   without following links (`launch::handle`): what is checked is what is copied.
//!   A job folder, project or file that is a link, a FIFO or a device is
//!   refused; a link is kept only when it is one of the owner's media.
//! - **The media** are mounted as `volume-subpath`s of the data volume, which
//!   the daemon resolves inside that volume itself, refusing a link out of it
//!   (Docker 26, API 1.45; [`Launch::ready`] says so when the daemon is older).
//! - **The page cache** is never mounted (#1677). The capture writes into
//!   `staging/out/<job>-<index>`, and once its container is gone the launcher
//!   copies what it left into the project's page cache ([`Launch::publish`]):
//!   files and folders only, from handles, never through a link. So no capture
//!   still running can change a file after the server has checked it.
//!
//! **The deadline** is the same as the worker's ([`super::deadline`]), kept from
//! outside: at it the container is killed by name, and with it everything the
//! browser started.

use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use scorsese_core::CACHE_DIR;

use super::{ASK, JOBS, PAGES, PROJECT, Spool};

mod handle;
mod stage;

pub use stage::Staged;

/// The image every capture runs: the server's own, which bakes the browser.
pub const IMAGE: &str = "scorsese-server";

/// Where the launcher's container keeps the capture's seccomp profile
/// (`deploy/capture/seccomp.json`, mounted there by compose). The Docker
/// client reads it and sends it to the daemon, so it lives beside the client.
pub const SECCOMP: &str = "/etc/scorsese/capture-seccomp.json";

/// The label every capture container carries, so a launcher that restarts can
/// remove what its predecessor left.
pub const LABEL: &str = "org.scorsese.capture";

/// The oldest Docker API that mounts a `volume-subpath`: Docker 26.
const SUBPATH_API: (u32, u32) = (1, 45);

/// The folders inside staging: the jobs as copied, and what each capture
/// writes.
const STAGED_JOBS: &str = "jobs";
const OUT: &str = "out";

/// The walls and caps of every capture (#773, #778): offline, no capability,
/// no new privilege, a read-only root and a user that is not root.
///
/// - A PID 1 that reaps: the browser's processes outlive their parents, and
///   without a reaper they pile up against the process cap.
/// - `/tmp` is the browser's profile and HOME: small, and memory, so nothing
///   big lands there. The frames go to the capture's own folder, on disk
///   (#773: frames on a tmpfs count against the memory cap and blow it).
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
    /// The folder the server sees the service's data at — `/data` — which
    /// the library is under.
    #[arg(long)]
    pub data: PathBuf,
    /// The Docker volume that is that folder, which each capture's media is
    /// mounted from, one `volume-subpath` a file.
    #[arg(long)]
    pub data_volume: String,
    /// Where this container sees its staging volume: the jobs as copied, and
    /// what each capture writes. Mounted here and nowhere else.
    #[arg(long)]
    pub staging: PathBuf,
    /// The Docker volume that is that folder, which each capture's job and
    /// `cache/` are mounted from.
    #[arg(long)]
    pub staging_volume: String,
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
    data_volume: String,
    staging: PathBuf,
    staging_volume: String,
    user: (u32, u32),
}

impl Launch {
    /// From the command line.
    pub fn from_args(args: &Args) -> Self {
        Self {
            spool: args.spool.clone(),
            library: args.library.clone(),
            data: args.data.clone(),
            data_volume: args.data_volume.clone(),
            staging: args.staging.clone(),
            staging_volume: args.staging_volume.clone(),
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

    /// The `docker run` capturing request `index` of `staged`, its `cache/`
    /// an empty folder of its own.
    pub fn command(&self, staged: &Staged, index: usize) -> Result<Command, String> {
        let out = self.out(staged, index);
        let _ = std::fs::remove_dir_all(&out);
        std::fs::create_dir_all(&out).map_err(|error| format!("staging the capture: {error}"))?;
        let mut command = scorsese_render::tools::child("docker");
        command.args(self.arguments(staged, index)?);
        Ok(command)
    }

    fn arguments(&self, staged: &Staged, index: usize) -> Result<Vec<OsString>, String> {
        let job = self.spool.join(JOBS).join(&staged.name);
        let project = job.join(PROJECT);
        let (uid, gid) = self.user;
        let mut arguments: Vec<OsString> = vec![
            "run".into(),
            "--rm".into(),
            format!("--name={}", Self::name(&job, index)).into(),
            format!("--label={LABEL}=1").into(),
            format!("--user={uid}:{gid}").into(),
            format!("--security-opt=seccomp={SECCOMP}").into(),
        ];
        arguments.extend(WALLS.iter().map(OsString::from));
        let staged_job = Path::new(STAGED_JOBS).join(&staged.name);
        let out = Path::new(OUT).join(format!("{}-{index}", staged.name));
        let mut mounts = vec![
            mount(&self.staging_volume, &staged_job, &job, false)?,
            mount(&self.staging_volume, &out, &project.join(CACHE_DIR), true)?,
        ];
        for file in &staged.media {
            let inside = file
                .strip_prefix(&self.data)
                .map_err(|_| format!("{} is not under {}", file.display(), self.data.display()))?;
            mounts.push(mount(&self.data_volume, inside, file, false)?);
        }
        arguments.extend(
            mounts
                .into_iter()
                .map(|mount| format!("--mount={mount}").into()),
        );
        let (user, _) = &staged.cache;
        arguments.extend([
            IMAGE.into(),
            "scorsese-server".into(),
            "capture-one".into(),
            "--project".into(),
            project.into_os_string(),
            "--ask".into(),
            job.join(ASK).into_os_string(),
            "--index".into(),
            index.to_string().into(),
            "--follow".into(),
            library_of(&self.library, user).into_os_string(),
        ]);
        Ok(arguments)
    }

    /// Removes every capture container a previous launcher left and
    /// everything it staged, and says whether the daemon, the image and the
    /// seccomp profile a capture needs are there.
    pub fn ready(&self) -> Result<(), String> {
        if !Path::new(SECCOMP).is_file() {
            return Err(format!("no seccomp profile at {SECCOMP}"));
        }
        let api = scorsese_render::tools::child("docker")
            .args(["version", "--format={{.Server.APIVersion}}"])
            .output()
            .map_err(|error| format!("docker would not start: {error}"))?;
        let api = String::from_utf8_lossy(&api.stdout);
        if api_version(&api).is_none_or(|version| version < SUBPATH_API) {
            return Err(format!(
                "the Docker daemon speaks API {}; captures mount by volume-subpath, which takes \
                 1.45 (Docker 26) or later",
                api.trim()
            ));
        }
        let image = scorsese_render::tools::child("docker")
            .args(["image", "inspect", "--format={{.Id}}", IMAGE])
            .output()
            .map_err(|error| format!("docker would not start: {error}"))?;
        if !image.status.success() {
            let said = String::from_utf8_lossy(&image.stderr);
            return Err(format!("no {IMAGE} image to capture with: {}", said.trim()));
        }
        let left = scorsese_render::tools::child("docker")
            .args(["ps", "--all", "--quiet", &format!("--filter=label={LABEL}")])
            .output()
            .map_err(|error| format!("docker would not start: {error}"))?;
        let left = String::from_utf8_lossy(&left.stdout);
        let left: Vec<&str> = left.split_whitespace().collect();
        if !left.is_empty() {
            let _ = scorsese_render::tools::child("docker")
                .args(["rm", "--force"])
                .args(&left)
                .stdout(std::process::Stdio::null())
                .status();
        }
        for folder in [STAGED_JOBS, OUT] {
            let folder = self.staging.join(folder);
            let _ = std::fs::remove_dir_all(&folder);
            std::fs::create_dir_all(&folder).map_err(|error| {
                format!("cannot stage captures in {}: {error}", folder.display())
            })?;
        }
        Ok(())
    }
}

/// The user and project of `target`, if it is exactly
/// `<spool>/pages/<user>/<project>`, both numbers.
fn cache_parts(spool: &Path, target: &Path) -> Option<(String, String)> {
    let rest = target.strip_prefix(spool.join(PAGES)).ok()?;
    let parts: Vec<&str> = rest
        .iter()
        .map(|part| part.to_str())
        .collect::<Option<_>>()?;
    match parts[..] {
        [user, project] if is_number(user) && is_number(project) && is_plain(target) => {
            Some((user.to_owned(), project.to_owned()))
        }
        _ => None,
    }
}

/// The user whose page cache the project at `project` captures into, if its
/// `cache/` is a link to one: exactly `<spool>/pages/<user>/<project>`, a
/// folder. What `capture-worker` follows links for; the launcher asks the
/// same of handles ([`Launch::stage`]).
pub(super) fn owner(spool: &Path, project: &Path) -> Option<String> {
    let target = std::fs::read_link(project.join(CACHE_DIR)).ok()?;
    let (user, _) = cache_parts(spool, &target)?;
    // No link on the way: canonical, it is where it says.
    (std::fs::canonicalize(&target).ok()? == target && target.is_dir()).then_some(user)
}

/// `owner`'s part of the library at `library`: the one folder outside its
/// project a capture's links may lead into (#857), and every file it is
/// mounted is under it.
pub(super) fn library_of(library: &Path, owner: &str) -> PathBuf {
    library.join("users").join(owner)
}

/// Whether `path` is absolute and already where it leads: no `.` or `..`.
fn is_plain(path: &Path) -> bool {
    path.is_absolute()
        && path
            .components()
            .all(|part| matches!(part, Component::RootDir | Component::Normal(_)))
}

/// `--mount`'s value for `inside` of `volume`, at `target`.
fn mount(volume: &str, inside: &Path, target: &Path, writable: bool) -> Result<String, String> {
    let (Some(volume), Some(inside), Some(target)) = (
        plain(Path::new(volume)),
        plain(inside).filter(|_| !inside.is_absolute()),
        plain(target),
    ) else {
        return Err(format!(
            "{} has characters a mount cannot carry",
            target.display()
        ));
    };
    let access = if writable { "" } else { ",readonly" };
    Ok(format!(
        "type=volume,source={volume},volume-subpath={inside},target={target}{access}"
    ))
}

fn is_number(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

/// `path` as text `--mount` reads one way only: no comma, quote, `=` or space.
fn plain(path: &Path) -> Option<&str> {
    let text = path.to_str()?;
    let safe = |c: char| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '-' | '_');
    (!text.is_empty() && text.chars().all(safe)).then_some(text)
}

/// The `major.minor` of an API version as `docker version` prints it.
fn api_version(text: &str) -> Option<(u32, u32)> {
    let (major, minor) = text.trim().split_once('.')?;
    Some((major.parse().ok()?, minor.parse().ok()?))
}

/// Stops the capture container `name`: the launcher's deadline.
pub fn kill(name: &str) {
    let _ = scorsese_render::tools::child("docker")
        .args(["kill", name])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

#[cfg(test)]
mod tests;
