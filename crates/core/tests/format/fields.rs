//! Nothing in [`super::document`] is left out of what it writes.
//!
//! A struct literal makes the document *set* every field, but a field equal to
//! its default is skipped when written, and a skipped field is one the
//! fingerprint cannot see. So each struct's own list of fields — which serde
//! names when it refuses an unknown one — is held to the keys written for it.

use std::collections::BTreeSet;

use scorsese_core::{
    Anchor, Asset, Attach, ChromaKey, Clip, Counter, Crop, Follow, Fps, Glow, Grade, Group, Icon,
    ImageRequest, ImageSequence, Keyframe, KeyframeTrack, Linear, Matte, MediaMetadata, Origin,
    Point, Project, Radial, Reveal, Shadow, Shape, SpeechRequest, TextStyle, Track, Vhs,
    VideoRequest,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

/// The fields `T` reads, from the message refusing a field it does not.
fn fields_of<T: DeserializeOwned>() -> BTreeSet<String> {
    let refused = serde_json::from_value::<T>(json!({ "?": 0 }))
        .err()
        .expect("refused");
    let message = refused.to_string();
    let (_, expected) = message
        .split_once("expected")
        .expect("serde lists the fields");
    expected
        .split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

fn keys(written: &Value, at: &str) -> BTreeSet<String> {
    let object = written.pointer(at).and_then(Value::as_object);
    object
        .unwrap_or_else(|| panic!("nothing written at {at}"))
        .keys()
        .cloned()
        .collect()
}

macro_rules! held {
    ($written:expr, $($ty:ty => $at:expr),+ $(,)?) => {$(
        assert_eq!(keys($written, $at), fields_of::<$ty>(), "{} at {}", stringify!($ty), $at);
    )+};
}

#[test]
fn every_field_of_every_struct_is_written() {
    let w = &super::written();
    let (asset, clip) = ("/project/assets/0", "/project/tracks/0/clips/0");
    let arrow = format!("{asset}/shape/geometry/arrow");
    held!(w, Project => "/project", Fps => "/project/timeline_fps", Track => "/project/tracks/0");
    held!(w, Asset => asset, MediaMetadata => &format!("{asset}/media"),
        TextStyle => &format!("{asset}/style"), Reveal => &format!("{asset}/style/reveal"),
        Counter => &format!("{asset}/style/number"), Shape => &format!("{asset}/shape"),
        Icon => &format!("{asset}/icon"), Group => &format!("{asset}/group"),
        ImageSequence => &format!("{asset}/sequence"), VideoRequest => &format!("{asset}/video"),
        SpeechRequest => &format!("{asset}/speech"), ImageRequest => &format!("{asset}/image"),
        Linear => &format!("{asset}/color/linear"), Point => &format!("{arrow}/from"),
        Attach => &format!("{arrow}/to/attach"), Radial => "/variants/Fill/2/radial");
    held!(w, Clip => clip, Crop => &format!("{clip}/crop"), Anchor => &format!("{clip}/anchor"),
        Origin => &format!("{clip}/origin"), Grade => &format!("{clip}/grade"),
        ChromaKey => &format!("{clip}/chroma_key"), Vhs => &format!("{clip}/vhs"),
        Shadow => &format!("{clip}/shadow"), Glow => &format!("{clip}/glow"),
        Matte => &format!("{clip}/matte"), Follow => &format!("{clip}/follow"),
        KeyframeTrack => &format!("{clip}/keyframes/0"),
        Keyframe => &format!("{clip}/keyframes/0/keyframes/0"));
}
