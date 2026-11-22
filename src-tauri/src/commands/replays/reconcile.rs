//! Puts the game's own judgements (from the `.osg`) on the notes the judge
//! found: each judgement there carries the time osu! gave it, which is the
//! time of the press or release that caused it, so it can be told which note
//! it belongs to. What the judge can't model (long note quirks, presses on
//! the edge of a window) then comes out as osu! counted it.
//!
//! When osu! gives each judgement, measured on local replays: a hit, when
//! the key is pressed (99.8% to the ms); a long note, when it is let go (94%),
//! or when its 50 window closes if it is held past it (90%); a note nobody
//! pressed, when its 100 window closes (95%); a long note let go too early,
//! when the 100 window of its end closes (70%).

use crate::commands::replays::judge::{Judged, Judgement, Windows};

/// How far apart two times can be and still be the same instant (ms).
const SAME_TIME: f64 = 2.0;
/// How far a judgement can be from where the judge expects it (ms).
const NEAR: f64 = 300.0;

/// How many of the game's judgements were put on a note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reconciled {
    pub matched: usize,
    /// The game's judgements in all.
    pub total: usize,
}

/// One judgement the judge gave: the note and whether it is its tail
/// (ScoreV2 judges the end of a long note on its own).
struct Ours {
    index: usize,
    tail: bool,
    time: f64,
    judgement: Judgement,
}

/// Gives `judged` the judgements of `game` (time in map ms, judgement).
pub fn reconcile(judged: &mut [Judged], game: &[(i32, Judgement)], windows: &Windows) -> Reconciled {
    let mut ours = expected(judged, windows);
    ours.sort_by(|a, b| a.time.total_cmp(&b.time));
    let mut game: Vec<(f64, Judgement)> = game.iter().map(|&(t, j)| (f64::from(t), j)).collect();
    game.sort_by(|a, b| a.0.total_cmp(&b.0));

    let mut taken = vec![false; game.len()];
    let mut placed = vec![false; ours.len()];
    // At the same instant, first the same judgement (chords: any of their
    // notes would do), then any; then the nearest left.
    for (limit, same) in [(SAME_TIME, true), (SAME_TIME, false), (NEAR, false)] {
        for (o, our) in ours.iter().enumerate() {
            if placed[o] {
                continue;
            }
            if let Some(g) = nearest(&game, &taken, our.time, limit, same.then_some(our.judgement)) {
                taken[g] = true;
                placed[o] = true;
                let note = &mut judged[our.index];
                if our.tail { note.tail = Some(game[g].1) } else { note.judgement = game[g].1 }
            }
        }
    }
    Reconciled { matched: placed.iter().filter(|&&p| p).count(), total: game.len() }
}

/// When osu! would give each judgement of `judged` (map ms).
fn expected(judged: &[Judged], windows: &Windows) -> Vec<Ours> {
    let (n100, n50) = (windows.n100.floor(), windows.n50.floor());
    let mut out = Vec::with_capacity(judged.len());
    for (index, j) in judged.iter().enumerate() {
        let start = j.note.start;
        let pressed_at = j.hit.map_or(start + n100, |hit| start + hit);
        match (j.note.end, j.tail) {
            (None, _) => out.push(Ours { index, tail: false, time: pressed_at, judgement: j.judgement }),
            // ScoreV2: the start when pressed, the end when let go.
            (Some(end), Some(tail)) => {
                out.push(Ours { index, tail: false, time: pressed_at, judgement: j.judgement });
                out.push(Ours { index, tail: true, time: released_at(end, j.release, n100, n50), judgement: tail });
            }
            (Some(end), None) => {
                out.push(Ours { index, tail: false, time: released_at(end, j.release, n100, n50), judgement: j.judgement })
            }
        }
    }
    out
}

fn released_at(end: f64, release: Option<f64>, n100: f64, n50: f64) -> f64 {
    match release {
        Some(r) if r > n50 => end + n50,
        Some(r) if r < -n50 => end + n100,
        Some(r) => end + r,
        None => end + n100,
    }
}

/// The game judgement left nearest `time`, within `limit`, of `judgement` if given.
fn nearest(game: &[(f64, Judgement)], taken: &[bool], time: f64, limit: f64, judgement: Option<Judgement>) -> Option<usize> {
    let from = game.partition_point(|&(t, _)| t < time - limit);
    (from..game.len())
        .take_while(|&g| game[g].0 <= time + limit)
        .filter(|&g| !taken[g] && judgement.is_none_or(|j| game[g].1 == j))
        .min_by(|&a, &b| (game[a].0 - time).abs().total_cmp(&(game[b].0 - time).abs()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::replays::judge::Note;

    fn note(key: usize, start: f64, end: Option<f64>, judgement: Judgement, hit: Option<f64>, release: Option<f64>) -> Judged {
        Judged { note: Note { key, start, end }, judgement, tail: None, hit, release }
    }

    #[test]
    fn the_game_decides_each_note() {
        let windows = Windows::stable(8.0, 0);
        let mut judged = vec![
            note(0, 1000.0, None, Judgement::Max, Some(5.0), None),
            // A chord: either note can take either judgement.
            note(0, 2000.0, None, Judgement::N300, Some(30.0), None),
            note(1, 2000.0, None, Judgement::Max, Some(30.0), None),
            // A long note the judge got wrong, judged when it was let go.
            note(2, 3000.0, Some(3500.0), Judgement::N50, Some(-150.0), Some(20.0)),
            // Nobody pressed it: a miss when its 100 window closed.
            note(3, 4000.0, None, Judgement::Miss, None, None),
        ];
        let game = [
            (1005, Judgement::Max),
            (2030, Judgement::N300),
            (2030, Judgement::N300),
            (3520, Judgement::Miss),
            (4103, Judgement::Miss),
        ];

        let done = reconcile(&mut judged, &game, &windows);

        assert_eq!(done, Reconciled { matched: 5, total: 5 });
        let got: Vec<Judgement> = judged.iter().map(|j| j.judgement).collect();
        assert_eq!(got, [Judgement::Max, Judgement::N300, Judgement::N300, Judgement::Miss, Judgement::Miss]);
    }

    #[test]
    fn what_the_game_lacks_stays_as_judged() {
        let windows = Windows::stable(8.0, 0);
        let mut judged = vec![note(0, 1000.0, None, Judgement::N100, Some(90.0), None)];

        let done = reconcile(&mut judged, &[], &windows);

        assert_eq!(done, Reconciled { matched: 0, total: 0 });
        assert_eq!(judged[0].judgement, Judgement::N100);
    }
}
