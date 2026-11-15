//! Telemetry sources. A source fills a [`Telemetry`] each tick and hands back
//! the raw session-info YAML whenever it changes (other games' sources write theirs in
//! iRacing's format, so the engine reads every game the same way).

pub mod games;
pub mod iracing;
pub mod lmu;
pub mod mock;

use crate::model::Telemetry;

pub enum Poll {
    /// Simulator not running / not in a session. The source already waited a bit.
    Disconnected,
    /// No new data this time around.
    Idle,
    /// `tel` holds a new row. `session_yaml` is set when the session info changed.
    Data { session_yaml: Option<String> },
}

pub trait Source: Send {
    fn name(&self) -> &'static str;
    /// The game being read right now ("iracing", "lmu", "mock"; "" while none is running).
    fn active(&self) -> &'static str {
        self.name()
    }
    /// Which game to read: "auto", "iracing" or "lmu" (sources reading one game ignore it).
    fn set_game(&mut self, _game: &str) {}
    /// A track outline the source built from the game's car positions, once complete
    /// (Le Mans Ultimate). Handed over once.
    fn take_track_map(&mut self) -> Option<crate::engine::trackmap::TrackMap> {
        None
    }
    /// How far that outline is (0..1), while it is being built.
    fn track_map_progress(&self) -> Option<f32> {
        None
    }
    /// Start the outline again (the player reset the track map).
    fn reset_track_map(&mut self) {}
    /// Wait for and read the next telemetry row (blocks at most ~1 frame).
    fn poll(&mut self, tel: &mut Telemetry) -> Poll;
    /// A known track outline, for sources that have one (the mock does).
    fn track_map(&self, _track_id: i64) -> Option<crate::engine::trackmap::TrackMap> {
        None
    }
    /// Corner list for sources that know their track (the mock does).
    fn corners(&self) -> Option<Vec<crate::config::Corner>> {
        None
    }
    /// Pit entry and exit as lap fractions, for sources that know their track.
    fn pit_lane(&self) -> Option<(f32, f32)> {
        None
    }
    /// Jump the replay to a moment and watch a car (`car_number_raw`, with the camera in use).
    fn replay(&mut self, _session_num: i32, _session_time: f64, _car_number_raw: Option<i32>, _camera: (i32, i32)) {}
}
