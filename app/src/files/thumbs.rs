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

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};

use egui::{ColorImage, Context, TextureHandle, TextureOptions};
use scorsese_core::{Asset, AssetKind};
use scorsese_render::Tools;
use scorsese_render::frames::{Thumbnail, thumbnail};

/// One picture to make: where from, how, and where to keep it.
struct Job {
    source: PathBuf,
    how: Thumbnail,
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
pub(super) struct Thumbs {
    worker: Option<(Sender<Job>, Receiver<Done>)>,
    slots: HashMap<PathBuf, Slot>,
}

impl Thumbs {
    /// Forgets every picture, for when a different project is opened.
    pub(super) fn reset(&mut self) {
        // Dropping the sender ends the thread once its queue is empty.
        self.worker = None;
        self.slots.clear();
    }

    /// Takes in whatever the worker has finished since the last frame.
    pub(super) fn poll(&mut self, ctx: &Context) {
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

    /// The picture at `out`, asking for it to be made if nobody has yet.
    pub(super) fn get(
        &mut self,
        ctx: &Context,
        source: &Path,
        how: Thumbnail,
        out: &Path,
    ) -> Option<&TextureHandle> {
        if !self.slots.contains_key(out) {
            let (jobs, _) = self.worker.get_or_insert_with(|| spawn(ctx.clone()));
            let job = Job {
                source: source.to_path_buf(),
                how,
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
pub(super) fn how(kind: AssetKind) -> Option<Thumbnail> {
    match kind {
        AssetKind::Video | AssetKind::GeneratedVideo => Some(Thumbnail::Frame { at_seconds: 0.0 }),
        AssetKind::Image | AssetKind::GeneratedImage => Some(Thumbnail::Picture),
        _ => None,
    }
}

/// Where the picture of `asset` is kept: named for the asset and the hash of
/// the file, so a replaced file is a new picture.
pub(super) fn cached(root: &Path, asset: &Asset) -> PathBuf {
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
    root.join("cache/thumbs").join(format!("{name}.png"))
}

fn spawn(ctx: Context) -> (Sender<Job>, Receiver<Done>) {
    let (jobs, queue) = channel::<Job>();
    let (done, results) = channel::<Done>();
    std::thread::spawn(move || {
        let tools = Tools::discover().ok();
        for Job { source, how, out } in queue {
            let made = out.is_file()
                || tools
                    .as_ref()
                    .is_some_and(|tools| thumbnail(tools, &source, how, &out).is_ok());
            let image = made.then(|| read(&out)).flatten();
            if done.send(Done { out, image }).is_err() {
                return;
            }
            ctx.request_repaint();
        }
    });
    (jobs, results)
}

fn read(path: &Path) -> Option<ColorImage> {
    let rgba = image::open(path).ok()?.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    Some(ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()))
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn only_footage_and_stills_are_taken_from_a_file() {
        assert!(how(AssetKind::Video).is_some() && how(AssetKind::GeneratedImage).is_some());
        assert!(how(AssetKind::Audio).is_none() && how(AssetKind::Text).is_none());
    }
}
