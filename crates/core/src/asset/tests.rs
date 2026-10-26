use super::*;

fn lasting(kind: AssetKind, seconds: f64) -> Asset {
    Asset {
        media: Some(MediaMetadata {
            duration_seconds: Some(seconds),
            ..MediaMetadata::default()
        }),
        ..Asset::imported(AssetId::new("a"), kind, ProjectPath::new("assets/a"))
    }
}

#[test]
fn an_image_sequence_is_unbounded_whatever_was_measured() {
    let fps = Fps::THIRTY;
    assert_eq!(lasting(AssetKind::ImageSequence, 2.0).length(fps), None);
    assert_eq!(
        lasting(AssetKind::Video, 2.0).length(fps),
        Some(fps.frames(2.0))
    );
}

#[test]
fn renderable_media_is_a_generated_file_or_an_inline_kind() {
    let file = |state, path: Option<&str>| Asset {
        state,
        path: path.map(ProjectPath::new),
        ..Asset::bare(AssetId::new("a"), AssetKind::Video)
    };
    assert!(file(None, Some("assets/a.mp4")).has_renderable_media());
    assert!(!file(None, None).has_renderable_media());
    let generated = Some(GenerationState::Generated);
    assert!(file(generated, Some("generated/a.mp4")).has_renderable_media());
    assert!(!file(generated, None).has_renderable_media());
    let sketch = Some(GenerationState::Sketch);
    assert!(!file(sketch, Some("generated/a.mp4")).has_renderable_media());
    assert!(Asset::bare(AssetId::new("t"), AssetKind::Text).has_renderable_media());
}

#[test]
fn a_measurement_reads_as_one_line() {
    let measured = MediaMetadata {
        width: Some(1920),
        height: Some(1080),
        frame_rate: Some(Fps::THIRTY),
        duration_seconds: Some(2.5),
        audio_channels: Some(2),
        ..MediaMetadata::default()
    };
    assert_eq!(measured.to_string(), "1920x1080, 30 fps, 2.50s, 2 audio ch");
    assert_eq!(MediaMetadata::default().to_string(), "no metadata reported");
}

#[test]
fn an_image_resolution_is_written_as_its_price_table_names_it() {
    let written: Vec<_> = ImageResolution::ALL.iter().map(|r| r.as_str()).collect();
    assert_eq!(written, ["1K", "2K", "4K"]);
}
