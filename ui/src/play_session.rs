//! Playback order is a snapshot, independent of browsing and checkbox selection.
use crate::model::{Entry, Settings};
#[derive(Default)]
enum Mode {
    #[default]
    Idle,
    Queue {
        entries: Vec<Entry>,
        index: usize,
    },
    Side {
        starts: Vec<f64>,
    },
}
#[derive(Default)]
pub struct Session {
    mode: Mode,
    pub settings: Settings,
    pub pending_seek: Option<f64>,
    deadline: Option<f64>,
    ended: bool,
}
pub enum Target {
    Entry(Entry),
    Seek(f64),
}
pub enum Advance {
    None,
    Pause,
    Play(Entry),
}
impl Session {
    pub fn queue(entries: Vec<Entry>, index: usize, settings: Settings) -> Self {
        let mode = if entries.is_empty() {
            Mode::Idle
        } else {
            Mode::Queue {
                index: index.min(entries.len() - 1),
                entries,
            }
        };
        Self {
            mode,
            settings,
            ..Self::default()
        }
    }
    pub fn side(entries: &[Entry], settings: Settings) -> Self {
        let mut at = settings.lead_in;
        let starts = entries
            .iter()
            .map(|e| {
                let start = at;
                at += e.duration + settings.gap;
                start
            })
            .collect();
        Self {
            mode: Mode::Side { starts },
            settings,
            ..Self::default()
        }
    }
    pub fn active(&self) -> bool {
        !matches!(self.mode, Mode::Idle)
    }
    pub fn entry(&self) -> Option<Entry> {
        match &self.mode {
            Mode::Queue { entries, index } => entries.get(*index).cloned(),
            _ => None,
        }
    }
    pub fn navigate(&mut self, delta: isize, position: f64) -> Option<Target> {
        self.deadline = None;
        self.ended = false;
        match &mut self.mode {
            Mode::Queue { entries, index } => {
                *index = index.saturating_add_signed(delta).min(entries.len() - 1);
                Some(Target::Entry(entries[*index].clone()))
            }
            Mode::Side { starts } if !starts.is_empty() => {
                let current = starts
                    .iter()
                    .rposition(|s| *s <= position + 0.001)
                    .unwrap_or(0);
                Some(Target::Seek(
                    starts[current.saturating_add_signed(delta).min(starts.len() - 1)],
                ))
            }
            _ => None,
        }
    }
    pub fn tick(&mut self, now: f64, finished: bool) -> Advance {
        if !finished || self.ended {
            return Advance::None;
        }
        let Mode::Queue { entries, index } = &mut self.mode else {
            return Advance::None;
        };
        if *index + 1 >= entries.len() {
            self.ended = true;
            return Advance::Pause;
        }
        let first = self.deadline.is_none();
        let deadline = *self.deadline.get_or_insert(now + self.settings.gap);
        if now >= deadline {
            self.deadline = None;
            *index += 1;
            Advance::Play(entries[*index].clone())
        } else if first {
            Advance::Pause
        } else {
            Advance::None
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn entries() -> Vec<Entry> {
        (0..4)
            .map(|i| Entry {
                track_id: i.to_string(),
                duration: 8.,
                offset: 0.,
                fade_in: 0.,
                fade_out: 0.,
                continuation: false,
            })
            .collect()
    }
    #[test]
    fn selected_and_direct_playback_keep_the_full_list_and_position() {
        let mut s = Session::queue(entries(), 2, Settings::default());
        assert_eq!(s.entry().unwrap().track_id, "2");
        let Some(Target::Entry(e)) = s.navigate(1, 5.) else {
            panic!()
        };
        assert_eq!(e.track_id, "3");
        let Some(Target::Entry(e)) = s.navigate(-1, 5.) else {
            panic!()
        };
        assert_eq!(e.track_id, "2");
        let Some(Target::Entry(e)) = s.navigate(-10, 5.) else {
            panic!()
        };
        assert_eq!(e.track_id, "0");
    }
    #[test]
    fn gaps_end_once_and_navigation_cancels_pending_advance() {
        let settings = Settings {
            gap: 2.,
            ..Default::default()
        };
        let mut s = Session::queue(entries(), 1, settings);
        assert!(matches!(s.tick(10., true), Advance::Pause));
        assert!(matches!(s.tick(11., true), Advance::None));
        assert!(matches!(s.navigate(1, 8.), Some(Target::Entry(_))));
        assert!(matches!(s.tick(11.5, false), Advance::None));
        assert!(matches!(s.tick(20., true), Advance::Pause));
        let Advance::Play(e) = s.tick(22., true) else {
            panic!()
        };
        assert_eq!(e.track_id, "3");
        assert!(matches!(s.tick(30., true), Advance::Pause));
        assert!(matches!(s.tick(31., true), Advance::None));
        assert!(s.navigate(-1, 8.).is_some());
    }
    #[test]
    fn side_seeks_include_lead_in_gaps_and_always_move_to_previous_track() {
        let settings = Settings {
            lead_in: 2.,
            gap: 1.,
            ..Default::default()
        };
        let mut s = Session::side(&entries(), settings);
        assert!(matches!(s.navigate(1, 0.), Some(Target::Seek(11.))));
        assert!(matches!(s.navigate(-1, 16.), Some(Target::Seek(2.))));
        assert!(matches!(s.navigate(1, 10.), Some(Target::Seek(11.))));
        assert!(matches!(s.tick(99., true), Advance::None));
        assert!(Session::default().navigate(1, 0.).is_none());
        assert!(
            Session::queue(vec![], 0, Settings::default())
                .navigate(1, 0.)
                .is_none()
        );
    }
}
