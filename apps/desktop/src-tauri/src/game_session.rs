use crate::lobby_focus;
use game_launcher::display::{game_window_exists, game_windows};
use std::time::{Duration, Instant};

pub const POLL_INTERVAL: Duration = Duration::from_millis(200);
const WINDOW_GRACE: Duration = Duration::from_millis(450);
const PROCESS_GRACE: Duration = Duration::from_millis(600);
const PLAY_WINDOW_MINIMUM: Duration = Duration::from_secs(1);

#[derive(Default)]
pub struct GameSession {
    windows: Vec<(usize, u32)>,
    foreground_since: Option<Instant>,
    played_in_foreground: bool,
    window_lost_at: Option<Instant>,
    process_lost_at: Option<Instant>,
}

impl GameSession {
    pub fn needs_process_scan(&self, last_scan: Instant) -> bool {
        last_scan.elapsed() >= Duration::from_millis(800)
            || !self.windows.iter().any(|&(window, pid)| game_window_exists(window, pid))
    }

    pub fn poll(&mut self, processes: &[u32], process_alive: bool) -> bool {
        self.windows.retain(|&(window, pid)| game_window_exists(window, pid));
        for window in game_windows(processes) {
            if !self.windows.contains(&window) {
                self.windows.push(window);
            }
        }
        let foreground = lobby_focus::foreground_pid();
        let active = self.windows.iter().any(|&(_, pid)| pid == foreground);
        self.update(Instant::now(), active, process_alive || !self.windows.is_empty())
    }

    fn update(&mut self, now: Instant, active: bool, process_alive: bool) -> bool {
        if active {
            let since = *self.foreground_since.get_or_insert(now);
            self.played_in_foreground |= now.duration_since(since) >= PLAY_WINDOW_MINIMUM;
        }
        if process_alive {
            self.process_lost_at = None;
        } else if now.duration_since(*self.process_lost_at.get_or_insert(now)) >= PROCESS_GRACE {
            return true;
        }
        if self.played_in_foreground && self.windows.is_empty() {
            now.duration_since(*self.window_lost_at.get_or_insert(now)) >= WINDOW_GRACE
        } else {
            self.window_lost_at = None;
            false
        }
    }

    pub fn took_foreground(&self) -> bool {
        self.foreground_since.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closes_after_the_game_window_disappears_even_with_a_helper_process() {
        let start = Instant::now();
        let mut session = GameSession { windows: vec![(1, 2)], ..Default::default() };
        assert!(!session.update(start, true, true));
        assert!(!session.update(start + PLAY_WINDOW_MINIMUM, true, true));
        assert!(session.took_foreground());
        session.windows.clear();
        let lost = start + Duration::from_secs(2);
        assert!(!session.update(lost, false, true));
        assert!(session.update(lost + WINDOW_GRACE, false, true));
    }

    #[test]
    fn window_replacement_and_process_handoff_cancel_pending_exit() {
        let start = Instant::now();
        let mut session = GameSession { played_in_foreground: true, ..Default::default() };
        assert!(!session.update(start, false, false));
        session.windows.push((2, 3));
        assert!(!session.update(start + Duration::from_millis(200), true, true));
        assert!(!session.update(start + Duration::from_secs(2), false, true));
    }
}
