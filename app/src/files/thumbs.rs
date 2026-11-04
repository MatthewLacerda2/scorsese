//! The pictures on the files panel's tiles, made off the UI thread.
//!
//! The web gets its thumbnails from the server; the window has none, so it
//! makes its own: one frame (or one still) through [`scorsese_render`]'s
//! `thumbnail` — never an ad-hoc ffmpeg call — written under the project's
//! rebuildable `cache/thumbs/`, then read back as a texture.
//!
//! One worker thread, one picture at a time: opening a project of a hundred
//! clips must not stall the panel, and a decode is as heavy as any other
//! ffmpeg work on a machine that may be serving someone's render. The file's
//! name carries the asset's hash, so a replaced file gets a new picture
//! instead of a stale one, and a picture already on disk is not made again.
//!
//! A `.scor` received from someone else may carry `cache/` (or anything below
//! it) as a link (#1737): the folder is made from the project root through
//! [`scorsese_core::write::in_cache`], which refuses one, and a link at a
//! picture's own name is never read as the picture — it is drawn again, and
//! the drawing replaces the link. A refusal is a tile without a picture, said
//! on stderr.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};

use egui::{ColorImage, Context, TextureHandle, TextureOptions};
use scorsese_core::{Asset, AssetKind};
use scorsese_render::Tools;
use scorsese_render::frames::{Thumbnail, thumbnail};

/// The folder below `cache/` the pictures are kept in.
const FOLDER: &str = "thumbs";

/// One picture to make: where from, how, the project it is kept in, and where
/// in it.
struct Job {
    source: PathBuf,
    how: Thumbnail,
    root: PathBuf,
    out: PathBuf,
}

/// A finished job: the picture, or `None` when it could not be made (a file
/// ffmpeg cannot read), which the tile answers with a glyph.
struct Done {
    out: PathBuf,
    image: Option<ColorImage>,
}

/// What has become of one request.
enum Slot {
    Pending,
    Ready(TextureHandle),
    Failed,
}

/// The worker and the textures it has delivered.
#[derive(Default)]
pub(crate) struct Thumbs {
    worker: Option<(Sender<Job>, Receiver<Done>)>,
    slots: HashMap<PathBuf, Slot>,
}

impl Thumbs {
    /// Forgets every picture, for when a different project is opened.
    pub(crate) fn reset(&mut self) {
        // Dropping the sender ends the thread once its queue is empty.
        self.worker = None;
        self.slots.clear();
    }

    /// Takes in whatever the worker has finished since the last frame.
    pub(crate) fn poll(&mut self, ctx: &Context) {
        let Some((_, done)) = &self.worker else {
            return;
        };
        while let Ok(Done { out, image }) = done.try_recv() {
            let slot = match image {
                Some(image) => Slot::Ready(ctx.load_texture(
                    out.to_string_lossy(),
                    image,
                    TextureOptions::LINEAR,
                )),
                None => Slot::Failed,
            };
            self.slots.insert(out, slot);
        }
    }

    /// The picture at `out` ([`cached`] under the project at `root`), asking
    /// for it to be made if nobody has yet.
    pub(crate) fn get(
        &mut self,
        ctx: &Context,
        source: &Path,
        how: Thumbnail,
        root: &Path,
        out: &Path,
    ) -> Option<&TextureHandle> {
        if !self.slots.contains_key(out) {
            let (jobs, _) = self.worker.get_or_insert_with(|| spawn(ctx.clone()));
            let job = Job {
                source: source.to_path_buf(),
                how,
                root: root.to_path_buf(),
                out: out.to_path_buf(),
            };
            let queued = jobs.send(job).is_ok();
            self.slots.insert(
                out.to_path_buf(),
                if queued { Slot::Pending } else { Slot::Failed },
            );
        }
        match self.slots.get(out) {
            Some(Slot::Ready(texture)) => Some(texture),
            _ => None,
        }
    }
}

/// What a picture of this asset is made of, or `None` when it has no file to
/// take one from (text, a colour, sound — those are drawn, not taken).
pub(crate) fn how(kind: AssetKind) -> Option<Thumbnail> {
    match kind {
        AssetKind::Video | AssetKind::GeneratedVideo => Some(Thumbnail::Frame { at_seconds: 0.0 }),
        AssetKind::Image | AssetKind::GeneratedImage => Some(Thumbnail::Picture),
        _ => None,
    }
}

/// Where the picture of `asset` is kept: named for the asset and the hash of
/// the file, so a replaced file is a new picture.
pub(crate) fn cached(root: &Path, asset: &Asset) -> PathBuf {
    kept(root, asset, FOLDER, "png")
}

/// Where something derived from `asset`'s file is kept under `cache/<folder>/`,
/// named for the asset and the file's hash so a replaced file is a new one.
pub(crate) fn kept(root: &Path, asset: &Asset, folder: &str, extension: &str) -> PathBuf {
    let hash = asset.sha256.as_deref().unwrap_or("unhashed");
    let name: String = format!("{}-{}", asset.id, &hash[..hash.len().min(16)])
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    root.join("cache")
        .join(folder)
        .join(format!("{name}.{extension}"))
}

fn spawn(ctx: Context) -> (Sender<Job>, Receiver<Done>) {
    let (jobs, queue) = channel::<Job>();
    let (done, results) = channel::<Done>();
    std::thread::spawn(move || {
        let tools = Tools::discover().ok();
        for job in queue {
            let image = made(tools.as_ref(), &job).then(|| read(&job.out)).flatten();
            let out = job.out;
            if done.send(Done { out, image }).is_err() {
                return;
            }
            ctx.request_repaint();
        }
    });
    (jobs, results)
}

/// Whether `job`'s picture is on disk once this returns: kept from before, or
/// drawn now — never through a link (the module's doc).
fn made(tools: Option<&Tools>, job: &Job) -> bool {
    if let Err(error) = scorsese_core::write::in_cache(&job.root, Path::new(FOLDER)) {
        eprintln!("no thumbnails are kept for this project: {error}");
        return false;
    }
    if is_kept(&job.out) {
        return true;
    }
    tools.is_some_and(|tools| thumbnail(tools, &job.source, job.how, &job.out).is_ok())
}

/// Whether a file of our own is kept at `path`: a link there, whatever it
/// leads to, is not one, and is made again over.
pub(crate) fn is_kept(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|meta| meta.is_file())
}

fn read(path: &Path) -> Option<ColorImage> {
    let rgba = image::open(path).ok()?.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    Some(ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use scorsese_core::{Asset, AssetId};

    fn asset(id: &str, sha256: Option<&str>) -> Asset {
        let path = scorsese_core::ProjectPath::new("assets/x.png");
        let mut asset = Asset::imported(AssetId::new(id), AssetKind::Image, path);
        asset.sha256 = sha256.map(str::to_owned);
        asset
    }

    #[test]
    fn a_replaced_file_gets_a_picture_of_its_own() {
        let root = Path::new("/p");
        let old = cached(root, &asset("plate", Some("aaaaaaaaaaaaaaaaaaaa")));
        let new = cached(root, &asset("plate", Some("bbbbbbbbbbbbbbbbbbbb")));
        assert_ne!(old, new);
        assert!(old.starts_with("/p/cache/thumbs"));
    }

    #[test]
    fn an_id_cannot_leave_the_cache_folder() {
        let path = cached(Path::new("/p"), &asset("../x/y", None));
        assert_eq!(path.parent(), Some(Path::new("/p/cache/thumbs")));
    }

    /// A project folder of its own, and a folder outside it to plant links to.
    pub(crate) fn planted(name: &str) -> (PathBuf, PathBuf) {
        let base = std::env::temp_dir().join(format!("scorsese-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let (root, outside) = (base.join("project"), base.join("outside"));
        std::fs::create_dir_all(&root).expect("a project folder");
        std::fs::create_dir_all(&outside).expect("a folder outside");
        (root, outside)
    }

    /// A picture to take a thumbnail of, drawn by ffmpeg.
    fn picture(tools: &Tools, root: &Path) -> PathBuf {
        let source = root.join("still.png");
        let made = tools
            .ffmpeg()
            .args(["-nostdin", "-v", "error", "-y", "-f", "lavfi"])
            .args(["-i", "testsrc=s=64x36", "-frames:v", "1"])
            .arg(&source)
            .status()
            .expect("ffmpeg runs");
        assert!(made.success(), "the fixture picture");
        source
    }

    fn job(root: &Path, source: PathBuf) -> Job {
        Job {
            source,
            how: Thumbnail::Picture,
            root: root.to_path_buf(),
            out: cached(root, &asset("plate", Some("aaaaaaaaaaaaaaaaaaaa"))),
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_linked_cache_folder_is_refused_and_nothing_lands_where_it_leads() {
        let tools = Tools::discover().expect("ffmpeg");
        for linked in ["cache", "cache/thumbs"] {
            let (root, outside) = planted("thumbs-linked-folder");
            let source = picture(&tools, &root);
            let link = root.join(linked);
            std::fs::create_dir_all(link.parent().expect("a parent")).expect("cache/");
            std::os::unix::fs::symlink(&outside, &link).expect("a link");

            assert!(!made(Some(&tools), &job(&root, source)), "{linked}");
            let landed = std::fs::read_dir(&outside).expect("outside").count();
            assert_eq!(landed, 0, "{linked}: written where the link leads");
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_link_at_a_pictures_name_is_drawn_over_never_shown_or_written_through() {
        let tools = Tools::discover().expect("ffmpeg");
        let (root, outside) = planted("thumbs-linked-file");
        let source = picture(&tools, &root);
        let victim = outside.join("victim.png");
        std::fs::copy(&source, &victim).expect("a victim");
        let before = std::fs::read(&victim).expect("the victim");
        let job = job(&root, source);
        std::fs::create_dir_all(job.out.parent().expect("a parent")).expect("cache/thumbs");
        std::os::unix::fs::symlink(&victim, &job.out).expect("a link");

        assert!(made(Some(&tools), &job));
        assert_eq!(std::fs::read(&victim).expect("the victim"), before);
        let landed = std::fs::symlink_metadata(&job.out).expect("the file");
        assert!(!landed.is_symlink(), "the link was not replaced");
    }

    #[test]
    fn only_footage_and_stills_are_taken_from_a_file() {
        assert!(how(AssetKind::Video).is_some() && how(AssetKind::GeneratedImage).is_some());
        assert!(how(AssetKind::Audio).is_none() && how(AssetKind::Text).is_none());
    }
}
