//! The few flat colours a picture is drawn in.
//!
//! A flat illustration is a handful of colours plus the noise around them: the
//! anti-aliased edge between two of them, and a JPEG's ringing. Tracing every
//! shade would trace the noise, so the picture is first reduced to at most the
//! number of colours asked for. Median cut picks a first guess that spans the
//! picture, a few rounds of k-means move each guess onto the colour it stands
//! for, and guesses that end up the same colour are merged — so a picture of
//! five colours asked for eight comes out in five, not in eight with three
//! shades of its background.

/// One colour, red, green and blue.
pub(super) type Rgb = [u8; 3];

/// How many pixels the palette is chosen from at most. A 1024² picture is
/// sampled one pixel in four; the palette of a flat picture does not need more.
const SAMPLE: usize = 262_144;

/// Rounds of k-means after the median cut.
const ROUNDS: usize = 6;

/// Two palette colours nearer than this (squared distance in RGB) are one
/// colour that the noise split in two.
const SAME: u32 = 24 * 24;

/// At most `wanted` colours that the opaque pixels of `rgba` are drawn in, most
/// used first. `rgba` is four bytes a pixel; a pixel under half opaque is not
/// part of the picture.
pub(super) fn choose(rgba: &[u8], wanted: usize) -> Vec<Rgb> {
    let opaque: Vec<Rgb> = rgba
        .chunks_exact(4)
        .filter(|pixel| pixel[3] >= 128)
        .map(|pixel| [pixel[0], pixel[1], pixel[2]])
        .collect();
    if opaque.is_empty() || wanted == 0 {
        return Vec::new();
    }
    let stride = opaque.len().div_ceil(SAMPLE);
    let sample: Vec<Rgb> = opaque.iter().step_by(stride).copied().collect();
    let mut palette = median_cut(sample.clone(), wanted);
    for _ in 0..ROUNDS {
        palette = refine(&sample, &palette);
    }
    merged(&sample, palette)
}

/// The index in `palette` of the colour nearest `colour`.
pub(super) fn nearest(palette: &[Rgb], colour: Rgb) -> usize {
    let mut best = (0, u32::MAX);
    for (index, entry) in palette.iter().enumerate() {
        let distance = distance(*entry, colour);
        if distance < best.1 {
            best = (index, distance);
        }
    }
    best.0
}

/// Squared distance between two colours.
pub(super) fn distance(a: Rgb, b: Rgb) -> u32 {
    a.iter()
        .zip(b)
        .map(|(x, y)| u32::from(x.abs_diff(y)).pow(2))
        .sum()
}

/// How light a colour looks, 0 for black to 1 for white (Rec. 709 weights on
/// the stored values — near enough to tell a pen line from a colour).
pub(super) fn lightness(colour: Rgb) -> f64 {
    let [r, g, b] = colour.map(|channel| f64::from(channel) / 255.0);
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// Splits the colours into `wanted` boxes, each time cutting the box with the
/// widest spread of one channel at its median, and answers each box's mean.
fn median_cut(colours: Vec<Rgb>, wanted: usize) -> Vec<Rgb> {
    let mut boxes = vec![colours];
    while boxes.len() < wanted {
        let Some((at, channel)) = widest(&boxes) else {
            break;
        };
        let mut cut = boxes.swap_remove(at);
        cut.sort_unstable_by_key(|colour| colour[channel]);
        let upper = cut.split_off(cut.len() / 2);
        boxes.push(cut);
        boxes.push(upper);
    }
    boxes.iter().map(|colours| mean(colours)).collect()
}

/// The box with the widest spread, and the channel it is widest in — or none
/// when no box has two colours left to tell apart.
fn widest(boxes: &[Vec<Rgb>]) -> Option<(usize, usize)> {
    let mut best: Option<(usize, usize, u8)> = None;
    for (at, colours) in boxes.iter().enumerate() {
        if colours.len() < 2 {
            continue;
        }
        for channel in 0..3 {
            let (low, high) = colours.iter().fold((u8::MAX, 0), |(low, high), colour| {
                (low.min(colour[channel]), high.max(colour[channel]))
            });
            let spread = high - low;
            if spread > 0 && best.is_none_or(|(_, _, widest)| spread > widest) {
                best = Some((at, channel, spread));
            }
        }
    }
    best.map(|(at, channel, _)| (at, channel))
}

/// One round of k-means: every colour joins its nearest entry, and each entry
/// moves to the mean of the colours that joined it. An entry nobody joined is
/// dropped.
fn refine(sample: &[Rgb], palette: &[Rgb]) -> Vec<Rgb> {
    let mut sums = vec![([0u64; 3], 0u64); palette.len()];
    for colour in sample {
        let (sum, count) = &mut sums[nearest(palette, *colour)];
        for (total, channel) in sum.iter_mut().zip(colour) {
            *total += u64::from(*channel);
        }
        *count += 1;
    }
    sums.iter()
        .filter(|(_, count)| *count > 0)
        .map(|(sum, count)| sum.map(|total| channel(total, *count)))
        .collect()
}

/// The palette with near-duplicates merged, most used colour first.
fn merged(sample: &[Rgb], palette: Vec<Rgb>) -> Vec<Rgb> {
    let mut counts = vec![0usize; palette.len()];
    for colour in sample {
        counts[nearest(&palette, *colour)] += 1;
    }
    let mut ranked: Vec<(Rgb, usize)> = palette.into_iter().zip(counts).collect();
    ranked.sort_by_key(|(colour, count)| (std::cmp::Reverse(*count), *colour));
    let mut kept: Vec<Rgb> = Vec::new();
    for (colour, count) in ranked {
        if count > 0 && kept.iter().all(|entry| distance(*entry, colour) >= SAME) {
            kept.push(colour);
        }
    }
    kept
}

/// The mean of some colours.
fn mean(colours: &[Rgb]) -> Rgb {
    let count = colours.len().max(1) as u64;
    let mut sum = [0u64; 3];
    for colour in colours {
        for (total, channel) in sum.iter_mut().zip(colour) {
            *total += u64::from(*channel);
        }
    }
    sum.map(|total| channel(total, count))
}

/// A channel's mean, rounded.
fn channel(total: u64, count: u64) -> u8 {
    u8::try_from((total + count / 2) / count).unwrap_or(u8::MAX)
}
