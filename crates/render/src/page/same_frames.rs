//! The proof #809 rests on: a capture run ahead to its first frame, and one
//! made in pieces and joined, hold exactly the frames of one capture drawn
//! from the first frame to the last — on a page written to tell them apart.
//!
//! Needs the pinned browser, as the page pipeline tests do.

use std::ops::Range;
use std::path::{Path, PathBuf};

use scorsese_compositor::Resolution;
use scorsese_core::Fps;

use super::browser::Chrome;
use super::capture::Served;
use super::request::Request;
use super::{cache, capture, pieces};
use crate::tools::Tools;

/// Everything a jump of the clock would get wrong: a loop that counts frames
/// rather than time, an animation a timer starts part-way, an interval, a
/// Web Animation, a canvas drawn from all of it — and an animation started by
/// another's `animationend`, an event the browser sends only from a frame of
/// its own, so a run ahead that skipped those frames would start it late.
const PAGE: &str = r##"<!doctype html>
<style>
  body { margin: 0 }
  #bar { position: absolute; top: 0; left: 0; height: 200px; background: #0f0 }
  #box { position: absolute; top: 300px; width: 200px; height: 200px; background: #00f }
  #box.go { animation: slide 2s linear infinite alternate }
  @keyframes slide { from { left: 0 } to { left: 800px } }
  #dot { position: absolute; top: 600px; width: 100px; height: 100px; background: #f0f }
  canvas { position: absolute; top: 800px; left: 0 }
  #chain { position: absolute; top: 0; right: 0; width: 150px; height: 150px; background: #f80;
           animation: grow 1s linear }
  #chain.next { animation: spin 0.8s linear infinite }
  @keyframes grow { from { width: 10px } to { width: 150px } }
  @keyframes spin { to { transform: rotate(360deg) } }
</style>
<div id="bar"></div><div id="box"></div><div id="dot"></div><div id="chain"></div>
<canvas id="c" width="1080" height="280"></canvas>
<script>
  let frames = 0, x = 0, last = null, flip = false;
  const bar = document.getElementById("bar");
  const dot = document.getElementById("dot");
  const pen = document.getElementById("c").getContext("2d");
  setTimeout(() => document.getElementById("box").classList.add("go"), 1530);
  setInterval(() => { flip = !flip; dot.style.background = flip ? "#ff0" : "#f0f"; }, 370);
  dot.animate([{ left: "0px" }, { left: "900px" }], { duration: 3100, iterations: Infinity });
  const tick = (now) => {
    frames += 1;
    x = (x + 0.37 * (last === null ? 0 : now - last)) % 1080;
    last = now;
    bar.style.width = (frames * 7) % 1080 + "px";
    pen.fillStyle = `rgb(${frames % 256}, ${Math.floor(x) % 256}, 90)`;
    pen.fillRect(x, (frames * 13) % 280, 40, 40);
    requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
  const chain = document.getElementById("chain");
  chain.addEventListener("animationend", () => chain.classList.add("next"));
</script>"##;

/// One hash per frame of a capture's file, in order.
fn hashes(tools: &Tools, file: &Path) -> Vec<String> {
    let out = tools
        .ffmpeg()
        .args(["-v", "error", "-i"])
        .arg(file)
        .args(["-f", "framemd5", "-"])
        .output()
        .expect("ffmpeg runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| line.to_owned())
        .collect()
}

/// The hash field alone, for frames compared across files that start at
/// different places.
fn pictures(lines: &[String]) -> Vec<&str> {
    lines
        .iter()
        .map(|line| line.rsplit(',').next().expect("a hash").trim())
        .collect()
}

/// Holds two captures to the same frames, saying where they first part.
fn same<T: PartialEq + std::fmt::Debug>(got: &[T], want: &[T], what: &str) {
    assert_eq!(got.len(), want.len(), "{what}: how many frames");
    if let Some(at) = got.iter().zip(want).position(|(g, w)| g != w) {
        panic!(
            "{what}: frame {at} differs: {:?} against {:?}",
            got[at], want[at]
        );
    }
}

struct Bench {
    chrome: Chrome,
    tools: Tools,
    root: PathBuf,
    fonts: PathBuf,
    request: Request,
}

impl Bench {
    fn new(name: &str) -> Self {
        // Relative, as a project named on the command line is: a join that
        // only works from an absolute path once passed here and failed there.
        // Under `target/`, which is ignored wherever it is.
        let root = PathBuf::from(format!("target/same-frames-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("pages")).expect("pages/");
        std::fs::write(root.join("pages/proof.html"), PAGE).expect("the page");
        let fonts = cache::fonts(&root).expect("fonts");
        Self {
            chrome: Chrome::discover().expect("the pinned browser (SCORSESE_CHROME)"),
            tools: Tools::discover().expect("ffmpeg"),
            root,
            fonts,
            request: Request {
                page: "pages/proof.html".into(),
                resolution: Resolution::new(64, 64).expect("a raster"),
                fps: Fps::THIRTY,
                duration: 9.0,
            },
        }
    }

    fn served(&self) -> Served<'_> {
        Served {
            project_root: &self.root,
            follow: &[],
            fonts: &self.fonts,
        }
    }

    fn sequential(&self, frames: Range<u64>, name: &str) -> Vec<String> {
        let out = self.root.join(name);
        capture::run(
            &self.chrome,
            &self.tools,
            self.served(),
            &self.request,
            frames,
            &out,
        )
        .expect("captured");
        hashes(&self.tools, &out)
    }
}

#[test]
fn a_capture_run_ahead_draws_the_frames_one_from_the_start_draws() {
    let bench = Bench::new("run-ahead");
    let whole = bench.sequential(0..bench.request.frames(), "whole.mkv");
    let late = bench.sequential(150..200, "late.mkv");
    same(
        &pictures(&late),
        &pictures(&whole[150..200]),
        "run ahead to 150",
    );
    let _ = std::fs::remove_dir_all(&bench.root);
}

#[test]
fn a_capture_made_in_pieces_is_the_file_one_capture_makes() {
    let bench = Bench::new("pieces");
    let frames = 0..bench.request.frames();
    let whole = bench.sequential(frames.clone(), "whole.mkv");
    let joined = bench.root.join("joined.mkv");
    let pieced = pieces::split(&bench.request, frames.clone(), 3);
    assert_eq!(pieced.len(), 2, "{pieced:?}");
    pieces::capture_in(
        &bench.chrome,
        &bench.tools,
        bench.served(),
        &bench.request,
        pieced,
        &joined,
    )
    .expect("captured in pieces");
    // Timestamps and all: a joined file stamps each frame where one capture does.
    same(&hashes(&bench.tools, &joined), &whole, "joined");
    let _ = std::fs::remove_dir_all(&bench.root);
}
