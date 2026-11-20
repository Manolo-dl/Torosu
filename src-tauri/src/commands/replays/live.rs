//! Notices the replays osu! saves while Torosu runs, so a play can be
//! analysed as soon as it ends.
//!
//! osu! writes the replay of a passed play to `Data/r` when the play ends (and
//! `scores.db` only later). The replays there are listed when a play starts;
//! once it is no longer on screen, the folder is checked for new ones for a
//! few seconds.

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use crate::commands::replays::scores::replay_files;

/// How long after a play the replay is waited for: failed or quit plays have none.
const WATCH_FOR: Duration = Duration::from_secs(15);
/// How often the folder is listed meanwhile.
const CHECK_EVERY: Duration = Duration::from_secs(1);

pub struct ReplayWatch {
    dir: PathBuf,
    /// The replays there were when the play started; `None` between plays.
    known: Option<HashSet<String>>,
    /// When to stop waiting for the replay of the play that ended.
    until: Option<Instant>,
    next_check: Instant,
}

impl ReplayWatch {
    /// Watches `dir`, osu!'s `Data/r`.
    pub fn new(dir: PathBuf) -> Self {
        Self { dir, known: None, until: None, next_check: Instant::now() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Call on every read of osu!'s screen, with whether it shows a play.
    /// Returns the replays saved since the last play started, once.
    pub fn poll(&mut self, playing: bool, now: Instant) -> Vec<String> {
        if playing {
            if self.known.is_none() {
                self.known = Some(replay_files(&self.dir));
            }
            self.until = None;
            return Vec::new();
        }

        let Some(known) = &mut self.known else { return Vec::new() };
        let until = *self.until.get_or_insert(now + WATCH_FOR);
        if now < self.next_check {
            return Vec::new();
        }
        self.next_check = now + CHECK_EVERY;

        let new: Vec<String> = replay_files(&self.dir).into_iter().filter(|name| !known.contains(name)).collect();
        known.extend(new.iter().cloned());
        if !new.is_empty() || now >= until {
            self.known = None;
            self.until = None;
        }
        new
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestDir;

    #[test]
    fn finds_the_replay_saved_after_a_play() {
        let dir = TestDir::new("replay-watch");
        dir.write("old.osr", "");
        let mut watch = ReplayWatch::new(dir.0.clone());
        let start = Instant::now();

        assert!(watch.poll(true, start).is_empty());
        assert!(watch.poll(false, start).is_empty());
        dir.write("new.osr", "");
        dir.write("new.osg", "");

        assert_eq!(watch.poll(false, start + CHECK_EVERY), ["new.osr"]);
        // Only once.
        assert!(watch.poll(false, start + CHECK_EVERY * 2).is_empty());
    }

    #[test]
    fn stops_waiting_after_a_while() {
        let dir = TestDir::new("replay-watch-timeout");
        let mut watch = ReplayWatch::new(dir.0.clone());
        let start = Instant::now();

        watch.poll(true, start);
        assert!(watch.poll(false, start).is_empty());
        assert!(watch.poll(false, start + WATCH_FOR).is_empty());
        // A replay saved later belongs to no play it saw.
        dir.write("late.osr", "");
        assert!(watch.poll(false, start + WATCH_FOR * 2).is_empty());
    }

    #[test]
    fn nothing_before_a_play() {
        let dir = TestDir::new("replay-watch-idle");
        let mut watch = ReplayWatch::new(dir.0.clone());
        dir.write("other.osr", "");

        assert!(watch.poll(false, Instant::now()).is_empty());
    }
}
