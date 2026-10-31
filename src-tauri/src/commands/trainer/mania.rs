//! osu!mania basics shared by the converters and the analysis: keys, columns
//! and notes.

use rosu_map::{
    Beatmap,
    section::hit_objects::{HitObject, HitObjectKind},
};

/// osu! plays up to 18K (9K+9K).
pub const MAX_KEYS: usize = 18;

/// osu! stable plays 1K to 10K: the key converter works within these.
pub const STABLE_MAX_KEYS: usize = 10;

/// Key count of a mania map, which is its circle size.
pub fn key_count(beatmap: &Beatmap) -> usize {
    beatmap.circle_size.round().clamp(1.0, MAX_KEYS as f32) as usize
}

/// Column of a note at `x` (0 to 512) on `keys` keys.
pub fn column(x: f32, keys: usize) -> usize {
    ((x * keys as f32 / 512.0).floor().max(0.0) as usize).min(keys - 1)
}

/// The `x` written for a note in `column`: the middle of the column.
pub fn column_x(column: usize, keys: usize) -> f32 {
    ((column as f32 + 0.5) * 512.0 / keys as f32).floor()
}

/// `x` of a note and when it ends: the release of a long note, the start of a
/// normal one. `None` for anything that is not a note.
pub fn note(obj: &HitObject) -> Option<(f32, f64)> {
    match &obj.kind {
        HitObjectKind::Circle(circle) => Some((circle.pos.x, obj.start_time)),
        HitObjectKind::Hold(hold) => Some((hold.pos_x, obj.start_time + hold.duration)),
        _ => None,
    }
}
