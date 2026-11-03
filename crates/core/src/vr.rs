//! VR placement: where an overlay goes in the headset (used by the desktop app, which shows
//! the overlays as SteamVR overlays).

use crate::config::{OverlayInstance, VrConfig};

/// How an overlay is placed in VR, from its settings (see `COMMON` in ui/src/lib/settings.ts).
#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    pub show: bool,
    /// Follows the head (true) or is fixed in the car.
    pub head: bool,
    /// Centre (m: right, up, back) and rotation (degrees: turn, tilt, roll) relative to the
    /// seated position or the head; width in metres.
    pub pos: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    pub width: f32,
    pub alpha: f32,
}

/// Where an overlay goes. `screen` is the monitor's logical size, `rect` the overlay's
/// logical position and size: "as on my screen" maps it onto the imaginary screen.
pub fn placement(o: &OverlayInstance, vr: &VrConfig, screen: (f64, f64), rect: (f64, f64, f64, f64)) -> Placement {
    let get = |k: &str| o.settings.get(k);
    let num = |k: &str, d: f32| get(k).and_then(|v| v.as_f64()).map(|v| v as f32).unwrap_or(d);
    let text = |k: &str, d: &str| get(k).and_then(|v| v.as_str()).unwrap_or(d).to_string();
    let show = get("vrShow").and_then(|v| v.as_bool()).unwrap_or(true);
    let head = text("vrAnchor", "world") == "head";
    let alpha = (num("vrOpacity", 100.0) / 100.0).clamp(0.05, 1.0);
    // The overlay's own rotation, wherever it is placed.
    let (yaw, pitch, roll) = (num("vrYaw", 0.0), num("vrPitch", 0.0), num("vrRoll", 0.0));
    if text("vrPlace", "screen") == "custom" {
        return Placement {
            show,
            head,
            pos: [num("vrX", 0.0), num("vrY", 0.0), -num("vrDistance", 0.9).max(0.1)],
            yaw,
            pitch,
            roll,
            width: num("vrWidth", 0.3).max(0.02),
            alpha,
        };
    }
    let (sw, sh) = (screen.0.max(1.0), screen.1.max(1.0));
    let (x, y, w, h) = rect;
    let plane_w = vr.width.max(0.1);
    let plane_h = plane_w * sh / sw;
    let cx = (x + w / 2.0) / sw - 0.5;
    let cy = 0.5 - (y + h / 2.0) / sh;
    Placement {
        show,
        head,
        pos: [(cx * plane_w + vr.offset) as f32, (cy * plane_h + vr.height) as f32, -vr.distance.max(0.1) as f32],
        yaw,
        pitch,
        roll,
        width: ((w / sw) * plane_w).max(0.02) as f32,
        alpha,
    }
}

/// OpenVR's 3x4 transform (rows of [rotation | translation]): turn (yaw) about the vertical
/// axis, then tilt (pitch), then roll in the overlay's own plane, then the position.
pub fn transform(p: &Placement) -> [[f32; 4]; 3] {
    let (y, x, z) = (p.yaw.to_radians(), p.pitch.to_radians(), p.roll.to_radians());
    let (sy, cy, sx, cx, sz, cz) = (y.sin(), y.cos(), x.sin(), x.cos(), z.sin(), z.cos());
    // M = Ry(yaw) * Rx(pitch); R = M * Rz(roll)
    let m = [[cy, sy * sx, sy * cx], [0.0, cx, -sx], [-sy, cy * sx, cy * cx]];
    let row = |i: usize, t: f32| [m[i][0] * cz + m[i][1] * sz, -m[i][0] * sz + m[i][1] * cz, m[i][2], t];
    [row(0, p.pos[0]), row(1, p.pos[1]), row(2, p.pos[2])]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_layout_maps_onto_the_vr_screen() {
        let vr = VrConfig { enabled: true, distance: 1.0, width: 1.2, height: 0.0, ..Default::default() };
        let o = OverlayInstance { settings: serde_json::json!({}), ..Default::default() };
        // A 480x270 overlay in the middle of a 1920x1080 screen: centred, a quarter as wide.
        let p = placement(&o, &vr, (1920.0, 1080.0), (720.0, 405.0, 480.0, 270.0));
        assert!(p.show && !p.head);
        assert!(p.pos[0].abs() < 1e-4 && p.pos[1].abs() < 1e-4 && (p.pos[2] + 1.0).abs() < 1e-4, "{:?}", p.pos);
        assert!((p.width - 0.3).abs() < 1e-4);
        // The whole layout moved 0.25 m to the right.
        let shifted = VrConfig { offset: 0.25, ..vr.clone() };
        assert!((placement(&o, &shifted, (1920.0, 1080.0), (720.0, 405.0, 480.0, 270.0)).pos[0] - 0.25).abs() < 1e-4);
        // Top-left corner: left and up.
        let p = placement(&o, &vr, (1920.0, 1080.0), (0.0, 0.0, 192.0, 108.0));
        assert!(p.pos[0] < -0.5 && p.pos[1] > 0.25, "{:?}", p.pos);
    }

    #[test]
    fn custom_placement_and_rotation() {
        let vr = VrConfig::default();
        let o = OverlayInstance {
            settings: serde_json::json!({ "vrPlace": "custom", "vrAnchor": "head", "vrX": 0.2, "vrY": -0.1, "vrDistance": 0.7, "vrYaw": 90, "vrWidth": 0.25, "vrOpacity": 80 }),
            ..Default::default()
        };
        let p = placement(&o, &vr, (1920.0, 1080.0), (0.0, 0.0, 100.0, 100.0));
        assert!(p.head);
        assert_eq!(p.pos, [0.2, -0.1, -0.7]);
        assert!((p.alpha - 0.8).abs() < 1e-6 && (p.width - 0.25).abs() < 1e-6);
        // Yaw 90°: the overlay's x axis points backwards (-z), its z axis to the right.
        let m = transform(&p);
        assert!((m[2][0] + 1.0).abs() < 1e-5 && (m[0][2] - 1.0).abs() < 1e-5, "{m:?}");
        assert_eq!([m[0][3], m[1][3], m[2][3]], [0.2, -0.1, -0.7]);
    }

    #[test]
    fn rotation_applies_to_screen_layout_and_rolls_in_plane() {
        let vr = VrConfig::default();
        let o = OverlayInstance { settings: serde_json::json!({ "vrYaw": 20, "vrRoll": 90 }), ..Default::default() };
        let p = placement(&o, &vr, (1920.0, 1080.0), (0.0, 0.0, 192.0, 108.0));
        assert_eq!((p.yaw, p.pitch, p.roll), (20.0, 0.0, 90.0));
        // Roll 90° alone: the overlay's x axis (its width) points up.
        let r = transform(&Placement { yaw: 0.0, pitch: 0.0, roll: 90.0, ..p.clone() });
        assert!(r[0][0].abs() < 1e-5 && (r[1][0] - 1.0).abs() < 1e-5 && (r[0][1] + 1.0).abs() < 1e-5, "{r:?}");
        // The facing direction (z) is not changed by roll.
        assert!((r[2][2] - 1.0).abs() < 1e-5);
    }
}
