//! v51 → v52: the project carries its frame's shape (#1278).

use serde_json::Value;

use crate::frame::Ratio;

/// The first version whose documents carry their own `aspect`. A document
/// migrated from before it had its shape read off its footage, and a caller
/// that knows better — the web server, which remembers the platform a
/// project was started for — may correct it.
pub const FRAMED: u32 = 52;

/// v51 → v52: every document gains its `aspect`.
///
/// Before this version a project had no shape, and each surface guessed one.
/// The guess carried forward is the one MCP `still` made: the shape of the
/// first clip, in time, on the lowest video track that has one whose shape is
/// a fact — a probed size, or the aspect a generated shot or still was asked
/// for. That is the bottom layer, which is what fills the frame.
///
/// That shape is kept only when it is one a video is made in ([`PICKED`],
/// the nearest within [`NEAR`]); anything else — nothing sized, or a photo's
/// own 735x478 (#1430) — reads as 16:9, which is what every other surface
/// assumed. A picture's size is a fact about the picture, not a frame anyone
/// chose.
pub(super) fn frames_arrive(document: &mut Value) -> Result<(), String> {
    let shape = footage(document).and_then(picked).unwrap_or_default();
    let Some(fields) = document.as_object_mut() else {
        return Err("the document is not an object".to_owned());
    };
    fields.insert("aspect".to_owned(), Value::from(shape.to_string()));
    Ok(())
}

/// The frames a video is made in, `across:down`: the editors' four and the
/// three #1430 adds (4:3, 3:4 and 21:9). The photograph and
/// print shapes (3:2, 5:4 and their portraits) are left out on purpose: a
/// still of that shape says what camera took it, not what the edit is for.
const PICKED: [(u32, u32); 7] = [(16, 9), (9, 16), (1, 1), (4, 5), (4, 3), (3, 4), (21, 9)];

/// How far footage may be from a [`PICKED`] shape and still be read as it:
/// a twentieth of the ratio. 4:5 and 3:4 are only a fifteenth apart, so a
/// shape between them can be near both, and [`picked`] takes the nearer.
const NEAR: f64 = 0.05;

/// The [`PICKED`] shape nearest `shape`, if it is within [`NEAR`].
fn picked(shape: Ratio) -> Option<Ratio> {
    PICKED
        .into_iter()
        .filter_map(|(across, down)| {
            let frame = Ratio::new(across, down).ok()?;
            Some(((shape.value() / frame.value() - 1.0).abs(), frame))
        })
        .min_by(|(one, _), (other, _)| one.total_cmp(other))
        .filter(|(off, _)| *off < NEAR)
        .map(|(_, frame)| frame)
}

/// The shape of the first sized clip on the lowest video track that has one.
fn footage(document: &Value) -> Option<Ratio> {
    let assets = document.get("assets")?.as_array()?;
    document
        .get("tracks")?
        .as_array()?
        .iter()
        .filter(|track| track.get("kind").and_then(Value::as_str) == Some("video"))
        .find_map(|track| {
            let mut clips: Vec<&Value> = track.get("clips")?.as_array()?.iter().collect();
            clips.sort_by_key(|clip| clip.get("start").and_then(Value::as_u64));
            clips.into_iter().find_map(|clip| {
                let id = clip.get("asset")?.as_str()?;
                let asset = assets
                    .iter()
                    .find(|asset| asset.get("id").and_then(Value::as_str) == Some(id))?;
                shape(asset)
            })
        })
}

/// One asset's shape as a v51 document records it, when it is a fact.
fn shape(asset: &Value) -> Option<Ratio> {
    let measured = asset.get("media").and_then(|media| {
        let side = |name| media.get(name)?.as_u64()?.try_into().ok();
        Ratio::of_size(side("width")?, side("height")?)
    });
    if measured.is_some() {
        return measured;
    }
    let block = match asset.get("kind")?.as_str()? {
        "generated_video" => "video",
        "generated_image" => "image",
        _ => return None,
    };
    // An absent block or field is the default request, which is 16:9.
    let spelled = asset
        .get(block)
        .and_then(|block| block.get("aspect"))
        .and_then(Value::as_str)
        .unwrap_or("16:9");
    spelled.parse().ok()
}

#[cfg(test)]
mod tests;
