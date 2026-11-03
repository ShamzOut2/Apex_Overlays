//! Apex Overlays OpenXR API layer. The OpenXR loader runs it inside OpenXR games (iRacing)
//! when it is registered as an implicit layer. Each frame the game submits, it adds the
//! overlays the Apex Overlays app publishes (see `apex_xr_shared`) as quad layers: fixed in
//! the car (LOCAL space) or following the head (VIEW space). Works with any OpenXR runtime
//! for Direct3D 11 games, SteamVR or not.
//!
//! It must never break the game: every hook catches panics (the layer is built with the
//! `layer` profile, which unwinds) and on any problem the frame goes through unchanged.

#[cfg(windows)]
mod layer;

#[cfg(windows)]
pub use layer::xrNegotiateLoaderApiLayerInterface;
