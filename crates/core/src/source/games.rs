//! Reads whichever game is running: iRacing or Le Mans Ultimate, picked automatically, or one
//! of them chosen by hand (Settings → General → Game).

use super::iracing::IRacing;
use super::lmu::Lmu;
use super::{Poll, Source};
use crate::model::Telemetry;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Game {
    IRacing,
    Lmu,
}

impl Game {
    fn name(self) -> &'static str {
        match self {
            Game::IRacing => "iracing",
            Game::Lmu => "lmu",
        }
    }
}

pub struct Games {
    iracing: IRacing,
    lmu: Lmu,
    /// A game chosen by hand; `None` = automatic.
    chosen: Option<Game>,
    /// The game being read (in automatic mode: the one found running).
    active: Option<Game>,
    /// Automatic mode looks for the games in turn, starting here.
    next: Game,
}

impl Default for Games {
    fn default() -> Self {
        Self::new()
    }
}

impl Games {
    pub fn new() -> Self {
        Self { iracing: IRacing::new(), lmu: Lmu::new(), chosen: None, active: None, next: Game::IRacing }
    }

    fn poll_game(&mut self, g: Game, tel: &mut Telemetry) -> Poll {
        match g {
            Game::IRacing => self.iracing.poll(tel),
            Game::Lmu => self.lmu.poll(tel),
        }
    }
}

impl Source for Games {
    fn name(&self) -> &'static str {
        "games"
    }

    fn active(&self) -> &'static str {
        self.active.map_or("", Game::name)
    }

    fn set_game(&mut self, game: &str) {
        let chosen = match game {
            "iracing" => Some(Game::IRacing),
            "lmu" => Some(Game::Lmu),
            _ => None,
        };
        if chosen != self.chosen {
            self.chosen = chosen;
            // Start again from the chosen game (or look for one).
            self.active = chosen;
        }
    }

    fn poll(&mut self, tel: &mut Telemetry) -> Poll {
        if let Some(g) = self.chosen {
            self.active = Some(g);
            return self.poll_game(g, tel);
        }
        // Automatic: keep reading the game found running until it stops.
        if let Some(g) = self.active {
            let r = self.poll_game(g, tel);
            if matches!(r, Poll::Disconnected) {
                self.active = None;
                self.next = g;
            }
            return r;
        }
        // Look for a running game, one per call (each waits a little when its game is not running).
        let g = self.next;
        self.next = match g {
            Game::IRacing => Game::Lmu,
            Game::Lmu => Game::IRacing,
        };
        match self.poll_game(g, tel) {
            r @ Poll::Data { .. } => {
                self.active = Some(g);
                r
            }
            // Present but between rows: not chosen yet, but no need to report a disconnect.
            Poll::Idle => Poll::Idle,
            Poll::Disconnected => Poll::Disconnected,
        }
    }

    fn replay(&mut self, session_num: i32, session_time: f64, car_number_raw: Option<i32>, camera: (i32, i32)) {
        // Replays can only be driven in iRacing.
        if self.active == Some(Game::IRacing) {
            self.iracing.replay(session_num, session_time, car_number_raw, camera);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chosen_game_or_automatic() {
        let mut g = Games::new();
        assert_eq!(g.active(), "", "automatic, nothing running yet");
        g.set_game("lmu");
        assert_eq!(g.active(), "lmu");
        g.set_game("iracing");
        assert_eq!(g.active(), "iracing");
        g.set_game("auto");
        assert_eq!(g.active(), "", "automatic again: looks for a running game");
        // Off Windows neither game can run: automatic mode finds nothing.
        let mut tel = Telemetry::default();
        for _ in 0..2 {
            assert!(matches!(g.poll(&mut tel), Poll::Disconnected));
        }
        assert_eq!(g.active(), "");
    }
}
