//! Cars around you for the radar overlay, in metres: ahead (+) or behind (-) and to the right
//! (+) or left (-) of your car.
//!
//! Le Mans Ultimate says where every car is in the world: each car's offset from yours is turned
//! into your car's frame by your heading, so cars show exactly where they are. iRacing only says
//! how far round the lap each car is and whether there is a car on your left or right (or both),
//! so there the cars are placed by their distance along the lap, and the nearest ones are put
//! beside you on the side iRacing reports.

use super::Engine;
use crate::model::surface;
use serde::Serialize;

/// How far the radar looks, ahead/behind and to the sides (metres).
pub const RANGE: f32 = 40.0;
/// A car's length, roughly (for placing cars side by side).
pub const CAR_LENGTH: f32 = 4.8;
/// Lateral distance of a car alongside (iRacing: unknown, so a typical one).
const ALONGSIDE: f32 = 3.2;

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RadarCar {
    pub idx: usize,
    /// Metres to the right (+) or left (-), and ahead (+) or behind (-).
    pub x: f32,
    pub y: f32,
    /// Class colour and car number.
    pub color: String,
    pub number: String,
}

#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct RadarOut {
    /// Positions are exact (Le Mans Ultimate) or placed from lap distance (iRacing).
    pub exact: bool,
    /// You are driving (the radar has a car to centre on).
    pub active: bool,
    pub range: f32,
    pub car_length: f32,
    pub cars: Vec<RadarCar>,
}

fn round1(v: f32) -> f32 {
    (v * 10.0).round() / 10.0
}

pub fn build(e: &Engine) -> RadarOut {
    let t = &e.tel;
    let mut out = RadarOut { range: RANGE, car_length: CAR_LENGTH, ..Default::default() };
    let Some(me) = usize::try_from(t.player_idx).ok().filter(|&i| i < t.cars.len()) else { return out };
    let mine = &t.cars[me];
    if !mine.in_world() || mine.surface == surface::IN_PIT_STALL {
        return out;
    }
    out.active = true;
    let len = e.info.track_length_km * 1000.0;
    let car = |idx: usize, x: f32, y: f32| {
        let s = e.static_of(idx);
        RadarCar {
            idx,
            x: round1(x),
            y: round1(y),
            color: s.map(|s| crate::topics::hex(s.class.color)).unwrap_or_default(),
            number: s.map(|s| s.number.clone()).unwrap_or_default(),
        }
    };
    // Cars that count: on track, not in their pit box, and in the same part of the world as you
    // (a car in the pit lane while you are on track still shows when it is close).
    let others = (0..t.cars.len())
        .filter(|&i| i != me && t.cars[i].in_world() && t.cars[i].surface != surface::IN_PIT_STALL && e.static_of(i).is_some_and(|s| !s.hidden));
    if let Some([px, pz]) = mine.pos {
        // Exact: your heading (compass, radians from north towards east).
        out.exact = true;
        let (sin, cos) = t.yaw_north.sin_cos();
        for i in others {
            let Some([x, z]) = t.cars[i].pos else { continue };
            let (dx, dz) = (x - px, z - pz);
            let ahead = dx * sin + dz * cos;
            let right = dx * cos - dz * sin;
            if ahead.abs() <= RANGE && right.abs() <= RANGE {
                out.cars.push(car(i, right, ahead));
            }
        }
    } else if len > 0.0 {
        // From lap distance: ahead (+) or behind (-), the shorter way round.
        let mut near: Vec<(usize, f32)> = others
            .filter_map(|i| {
                let mut d = (t.cars[i].pct - mine.pct) * len;
                if d > len / 2.0 {
                    d -= len;
                } else if d < -len / 2.0 {
                    d += len;
                }
                (d.abs() <= RANGE).then_some((i, d))
            })
            .collect();
        near.sort_by(|a, b| a.1.abs().total_cmp(&b.1.abs()));
        // iRacing's CarLeftRight: 2 left, 3 right, 4 both, 5 two left, 6 two right.
        let mut sides: Vec<f32> = match t.car_left_right {
            2 => vec![-ALONGSIDE],
            3 => vec![ALONGSIDE],
            4 => vec![-ALONGSIDE, ALONGSIDE],
            5 => vec![-ALONGSIDE, -2.0 * ALONGSIDE],
            6 => vec![ALONGSIDE, 2.0 * ALONGSIDE],
            _ => vec![],
        };
        for (i, d) in near {
            // The nearest cars overlapping you are the ones alongside; the rest are in line.
            let x = if d.abs() < CAR_LENGTH * 1.2 && !sides.is_empty() { sides.remove(0) } else { 0.0 };
            out.cars.push(car(i, x, d));
        }
    }
    out.cars.sort_by(|a, b| (a.x.abs() + a.y.abs()).total_cmp(&(b.x.abs() + b.y.abs())));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::tests::{engine, place};

    #[test]
    fn exact_positions_turn_with_your_heading() {
        let mut e = engine(4, true);
        e.info.track_length_km = 4.0;
        for i in 0..4 {
            place(&mut e, i, 3, 0.5);
        }
        // You face east (90°): a car 10 m east is 10 m ahead, one 3 m north is 3 m to your left.
        e.tel.yaw_north = std::f32::consts::FRAC_PI_2;
        e.tel.cars[0].pos = Some([100.0, 200.0]);
        e.tel.cars[1].pos = Some([110.0, 200.0]);
        e.tel.cars[2].pos = Some([100.0, 203.0]);
        e.tel.cars[3].pos = Some([400.0, 200.0]);
        let r = build(&e);
        assert!(r.exact && r.active);
        let at = |i: usize| r.cars.iter().find(|c| c.idx == i).map(|c| (c.x, c.y));
        assert_eq!(at(1), Some((0.0, 10.0)), "ahead");
        assert_eq!(at(2), Some((-3.0, 0.0)), "to the left");
        assert_eq!(at(3), None, "out of range");
        assert_eq!(r.cars[0].idx, 2, "nearest first");
    }

    #[test]
    fn from_lap_distance_with_the_side_iracing_reports() {
        let mut e = engine(4, true);
        e.info.track_length_km = 4.0;
        place(&mut e, 0, 3, 0.5);
        place(&mut e, 1, 3, 0.5005); // 2 m ahead: alongside
        place(&mut e, 2, 3, 0.495); // 20 m behind
        place(&mut e, 3, 3, 0.99); // far away
        e.tel.car_left_right = 3; // a car on the right
        let r = build(&e);
        assert!(!r.exact && r.active);
        let at = |i: usize| r.cars.iter().find(|c| c.idx == i).map(|c| (c.x, c.y));
        assert_eq!(at(1), Some((3.2, 2.0)), "alongside, on the right");
        assert_eq!(at(2), Some((0.0, -20.0)), "behind, in line");
        assert_eq!(at(3), None);
        // Across the line: 0.999 is just behind 0.001.
        place(&mut e, 0, 3, 0.001);
        place(&mut e, 2, 3, 0.999);
        e.tel.car_left_right = 1;
        let r = build(&e);
        assert_eq!(r.cars.iter().find(|c| c.idx == 2).map(|c| c.y), Some(-8.0));
    }
}
