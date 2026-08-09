//! Project directories for the tests to draw and to rewrite.
//!
//! Shared by more than one test binary, and each uses a different slice of it,
//! so items unused from where you are reading are expected rather than dead.
#![allow(dead_code)]
//!
//! Written out as text rather than built through `scorsese-core`, for the same
//! reason `crates/cli`'s fixtures are: the document is the contract, and a
//! fixture that goes through our own writer cannot express a document our own
//! writer would never produce — including the broken one.

use std::path::{Path, PathBuf};

use scorsese_providers::voices::{
    Availability, Catalogue, Filters, Page, Unusable, Voice, VoiceError,
};

/// A project directory that removes itself when the test ends.
pub(crate) struct Fixture(PathBuf);

impl Fixture {
    /// Where it is.
    pub(crate) fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A whole edit: a title over a colour, music under narration, a duck already
/// written on the music, and two assets nobody has generated.
///
/// Everything a panel can draw, in one project — a snapshot of a project that
/// exercises one case at a time would need six projects and would still miss
/// how they look beside each other.
pub(crate) fn project(label: &str) -> Fixture {
    write(label, DOCUMENT)
}

/// The same project with a voice chosen for its narration line — the one state
/// in which narration is priced above zero.
///
/// **No id is written down anywhere.** A [`Fake`] catalogue lists one voice
/// into the project's own `cache/voices/`, through the same call that fills a
/// real window's cache, and the document then names the voice **the cache came
/// back with** — the direction an id travels when a person picks one out of the
/// picker. So the only place an id ever exists is a rebuildable cache this
/// fixture creates and [`Drop`] deletes with the rest of the project.
///
/// That is not fastidiousness. Every ElevenLabs default voice expires on
/// 2026-12-31, so an id committed to this repository is an outage with a date
/// on it — and the rule against one is exactly what had left the narration
/// figure in every reference image at $0.00.
pub(crate) fn voiced(label: &str) -> Fixture {
    let fixture = write(label, DOCUMENT);
    let chosen = listed(fixture.path());
    let document = DOCUMENT.replace(
        LINE,
        &format!("{LINE}\n      \"speech\": {{ \"voice_id\": \"{chosen}\" }},"),
    );
    std::fs::write(fixture.path().join("project.json"), document)
        .expect("write the project back with a voice chosen");
    fixture
}

/// Lists one voice into `root`'s cache and hands back its id.
///
/// Through [`voices::builtin`](scorsese_providers::voices::builtin) rather than
/// by writing the cache file directly: that call is what a window's cache is
/// filled by, and a second way of writing one would be a second file format to
/// keep in step.
fn listed(root: &Path) -> String {
    let fake = Fake { id: minted() };
    scorsese_providers::voices::builtin(root, &fake, false).expect("cache the fake listing");
    scorsese_providers::voices::cached(root)
        .first()
        .map(|voice| voice.id.clone())
        .expect("read back the listing just cached")
}

/// A voice id invented for this run and no other.
///
/// Never a vendor's, and never the same twice, so nothing in this repository
/// can come to depend on one particular id — which is the whole of what is
/// forbidden. It is also never drawn: the picker resolves an id it knows to the
/// voice's *name*, and the generate dialog prices a line without naming its
/// voice at all, so a reference image stays reproducible.
fn minted() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.subsec_nanos());
    format!("fixture-voice-{nanos:09}")
}

/// A catalogue with one voice in it and no network behind it.
///
/// The seam `scorsese-providers` leaves for exactly this — see that crate's
/// `voices` module doc. Nothing here reaches ElevenLabs, so a snapshot needs no
/// key and spends nothing.
struct Fake {
    /// The id this run minted.
    id: String,
}

impl Fake {
    /// The one voice it lists, filled in the way a listing fills one in.
    fn voice(&self) -> Voice {
        Voice {
            id: self.id.clone(),
            name: String::from("Narrator"),
            traits: vec![String::from("en"), String::from("male")],
            description: None,
            preview: None,
        }
    }
}

impl Catalogue for Fake {
    fn name(&self) -> &'static str {
        "fake"
    }

    fn builtin(&self) -> Result<Vec<Voice>, VoiceError> {
        Ok(vec![self.voice()])
    }

    fn library(&self, _filters: &Filters) -> Result<Page, VoiceError> {
        Ok(Page::whole(vec![self.voice()]))
    }

    /// Answers about its own voice and refuses every other id, rather than
    /// saying yes to whatever it is handed: a fake that approves an id it has
    /// never heard of would let a caller's mistake pass as a working voice.
    fn one(&self, voice_id: &str) -> Result<Availability, VoiceError> {
        Ok(if voice_id == self.id {
            Availability::Available(self.voice())
        } else {
            Availability::Unusable(Unusable::Gone)
        })
    }
}

/// The same project with a clip pointing at an asset that is not there.
///
/// Parses; does not validate. That is the case the window **opens** —
/// read-only, with the problem on screen — because the document is right there
/// and which clip is wrong is what somebody needs to see.
pub(crate) fn broken(label: &str) -> Fixture {
    write(
        label,
        &DOCUMENT.replace(r#""asset": "bed""#, r#""asset": "ghost""#),
    )
}

/// A `project.json` that is not JSON at all.
///
/// The other half of the pair, and kept beside [`broken`] because the two used
/// to be one state: nothing parses, so there is no document to show and no clip
/// to point at, and refusing is the only honest answer.
pub(crate) fn unparseable(label: &str) -> Fixture {
    write(label, "{ this was never a document")
}

/// Named for the label alone — no process id, no counter.
///
/// The window puts the project directory's name in its menu bar, so a name
/// that changed between runs would change the picture between runs, and a
/// snapshot that cannot reproduce itself is not a reference. Each test uses a
/// different label, so nothing collides inside one run of this binary.
fn write(label: &str, document: &str) -> Fixture {
    let dir = std::env::temp_dir().join(format!("scorsese-panels-{label}.scor"));
    let _ = std::fs::remove_dir_all(&dir);
    for inside in ["assets", "generated", "recipes", "cache"] {
        std::fs::create_dir_all(dir.join(inside)).expect("create the project directory");
    }
    std::fs::write(dir.join("project.json"), document).expect("write project.json");
    // A real file behind the image asset. An asset whose file is missing is a
    // warning the pool draws, and a snapshot of a panel should not also be a
    // snapshot of an unrelated complaint.
    std::fs::create_dir_all(dir.join("assets")).expect("create assets/");
    std::fs::write(dir.join("assets/plate.png"), PIXEL).expect("write the image");
    Fixture(dir)
}

/// The smallest thing that is really a PNG: one opaque black pixel.
///
/// Bytes rather than a checked-in file, because what the pool needs is a path
/// that resolves — nothing in these tests decodes it. Its asset carries a
/// `media` block for the same reason a real one does: `scorsese import` probes
/// what it brings in, so an asset without one is an asset the window's
/// background probe would go and fill in, which is a second writer in the
/// middle of a test about something else.
const PIXEL: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
    0x89, 0x00, 0x00, 0x00, 0x0a, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0d, 0x0a, 0x2d, 0xb4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae,
    0x42, 0x60, 0x82,
];

/// The opening of the narration asset in [`DOCUMENT`], which is where
/// [`voiced`] writes the chosen voice in.
///
/// An anchor rather than a placeholder, so the document stays a document that
/// parses as it stands: the one every other test draws is the one written here,
/// not a template with a hole in it.
const LINE: &str = r#"{ "id": "vo", "kind": "generated_audio", "state": "sketch","#;

const DOCUMENT: &str = r##"{
  "schema_version": 17,
  "name": "Narrated teaser",
  "timeline_fps": { "num": 30, "den": 1 },
  "assets": [
    { "id": "title", "kind": "text", "text": "CHAPTER ONE",
      "style": { "font": "serif", "size": 0.14, "color": "#f5f0e6" } },
    { "id": "shot-city", "kind": "generated_video", "state": "sketch",
      "prompt": "wide aerial of a city at dawn, slow push in",
      "video": { "model": "fast", "resolution": "1080p", "seconds": 8 } },
    { "id": "plate", "kind": "image", "path": "assets/plate.png",
      "media": { "width": 1, "height": 1 } },
    { "id": "vo", "kind": "generated_audio", "state": "sketch",
      "prompt": "In a city that never sleeps, one editor never blinks." },
    { "id": "bed", "kind": "synth_audio", "state": "sketch",
      "recipe": "recipes/bed.json" }
  ],
  "tracks": [
    { "id": "v1", "kind": "video", "name": "Main", "clips": [
      { "id": "c-shot", "asset": "shot-city", "start": 0, "duration": 300 },
      { "id": "c-title", "asset": "title", "start": 300, "duration": 120 } ] },
    { "id": "a1", "kind": "audio", "name": "Music", "clips": [
      { "id": "c-bed", "asset": "bed", "start": 0, "duration": 420,
        "keyframes": [
          { "property": "volume", "by": "duck", "keyframes": [
            { "t": 51, "value": 1.0, "easing": "ease_out" },
            { "t": 60, "value": 0.25 },
            { "t": 230, "value": 0.25, "easing": "ease_in" },
            { "t": 248, "value": 1.0 } ] } ] } ] },
    { "id": "a2", "kind": "audio", "name": "Narration", "clips": [
      { "id": "c-vo", "asset": "vo", "start": 60, "duration": 170 } ] }
  ]
}
"##;
