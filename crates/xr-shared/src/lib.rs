//! What the Apex Overlays app and its OpenXR API layer (running inside iRacing) share: a
//! named block of memory listing the overlay panels to draw. The pixels themselves are in
//! Direct3D 11 textures the app shares by handle (with a keyed mutex, key 0).
//!
//! The app writes the block as a seqlock: `seq` is odd while it writes and bumped again
//! when done, so a reader that sees the same even `seq` before and after copying has a
//! consistent copy.

/// Name of the shared memory (per Windows session).
pub const NAME: &str = "Local\\ApexOverlaysOpenXR";
pub const MAGIC: u32 = 0x5258_5041; // "APXR"
pub const VERSION: u32 = 1;
pub const MAX_PANELS: usize = 32;
/// Panels older than this (ms since the app last wrote) are not drawn: the app has closed.
pub const STALE_MS: u64 = 2000;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Panel {
    /// Shared texture handle (legacy, from IDXGIResource::GetSharedHandle) and its size.
    pub texture: u64,
    pub width: u32,
    pub height: u32,
    /// Bumped when the texture holds a new frame.
    pub frame: u32,
    /// 1 = follows the head (view space), 0 = fixed in the car (local space).
    pub head: u32,
    /// Centre (m: right, up, back) and orientation (quaternion x, y, z, w).
    pub pos: [f32; 3],
    pub orient: [f32; 4],
    /// Width and height in metres.
    pub size: [f32; 2],
    /// Which overlay this is (stays the same while the overlay exists).
    pub key: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Shared {
    pub magic: u32,
    pub version: u32,
    pub seq: u64,
    /// Milliseconds since the Unix epoch at the last write.
    pub written_ms: u64,
    /// Bumped by the recentre hotkey: the layer takes the current head position as the
    /// seated position for panels fixed in the car.
    pub recenter: u32,
    pub count: u32,
    pub panels: [Panel; MAX_PANELS],
}

impl Default for Shared {
    fn default() -> Self {
        Self { magic: MAGIC, version: VERSION, seq: 0, written_ms: 0, recenter: 0, count: 0, panels: [Panel::default(); MAX_PANELS] }
    }
}

impl Shared {
    /// The panels in use.
    pub fn panels(&self) -> &[Panel] {
        &self.panels[..(self.count as usize).min(MAX_PANELS)]
    }
}

/// Quaternion (x, y, z, w) for a turn of `yaw` about the vertical axis, then a tilt of
/// `pitch`, then a roll in the panel's own plane (degrees).
pub fn orientation(yaw_deg: f32, pitch_deg: f32, roll_deg: f32) -> [f32; 4] {
    let half = |d: f32| ((d.to_radians() / 2.0).sin(), (d.to_radians() / 2.0).cos());
    let ((sy, cy), (sp, cp), (sr, cr)) = (half(yaw_deg), half(pitch_deg), half(roll_deg));
    let mul = |a: [f32; 4], b: [f32; 4]| {
        [
            a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
            a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
            a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
            a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
        ]
    };
    // q = q_yaw(about Y) * q_pitch(about X) * q_roll(about Z)
    mul(mul([0.0, sy, 0.0, cy], [sp, 0.0, 0.0, cp]), [0.0, 0.0, sr, cr])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rotate(q: [f32; 4], v: [f32; 3]) -> [f32; 3] {
        let (x, y, z, w) = (q[0], q[1], q[2], q[3]);
        let u = [x, y, z];
        let cross = |a: [f32; 3], b: [f32; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
        let t = cross(u, v).map(|c| 2.0 * c);
        let c = cross(u, t);
        [v[0] + w * t[0] + c[0], v[1] + w * t[1] + c[1], v[2] + w * t[2] + c[2]]
    }

    #[test]
    fn orientation_matches_yaw_then_pitch() {
        // Yaw 90°: the panel's facing direction (+z) turns to +x.
        let r = rotate(orientation(90.0, 0.0, 0.0), [0.0, 0.0, 1.0]);
        assert!((r[0] - 1.0).abs() < 1e-5 && r[1].abs() < 1e-5 && r[2].abs() < 1e-5, "{r:?}");
        // Pitch 90°: +z turns to -y (tilted to face down).
        let r = rotate(orientation(0.0, 90.0, 0.0), [0.0, 0.0, 1.0]);
        assert!(r[0].abs() < 1e-5 && (r[1] + 1.0).abs() < 1e-5 && r[2].abs() < 1e-5, "{r:?}");
        // Same as the SteamVR matrix (apex_core::vr::transform): yaw 30°, pitch 20°, roll 15°.
        let (y, p, z) = (30f32.to_radians(), 20f32.to_radians(), 15f32.to_radians());
        let m = [[y.cos(), y.sin() * p.sin(), y.sin() * p.cos()], [0.0, p.cos(), -p.sin()], [-y.sin(), y.cos() * p.sin(), y.cos() * p.cos()]];
        let r = |i: usize| [m[i][0] * z.cos() + m[i][1] * z.sin(), -m[i][0] * z.sin() + m[i][1] * z.cos(), m[i][2]];
        let full = [r(0), r(1), r(2)];
        for v in [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]] {
            let got = rotate(orientation(30.0, 20.0, 15.0), v);
            let e = [0, 1, 2].map(|i| full[i][0] * v[0] + full[i][1] * v[1] + full[i][2] * v[2]);
            assert!((0..3).all(|i| (got[i] - e[i]).abs() < 1e-5), "{v:?}: {got:?} vs {e:?}");
        }
    }

    #[test]
    fn layout_is_stable() {
        assert_eq!(std::mem::size_of::<Panel>(), 72);
        assert_eq!(std::mem::size_of::<Shared>(), 32 + 72 * MAX_PANELS);
    }
}
