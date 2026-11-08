//! Rebuilds `data/iracing-track-maps.json` (the track map index built into the app and
//! published for older versions) from the public racing-track-maps-vector catalogue.
//!
//! cargo run -p apex-core --example track_map_index -- iracing-tracks-metadata.json > data/iracing-track-maps.json

fn main() {
    let path = std::env::args().nth(1).expect("usage: track_map_index <iracing-tracks-metadata.json>");
    let text = std::fs::read_to_string(&path).expect("cannot read the catalogue");
    let catalogue: serde_json::Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).expect("the catalogue is not JSON");
    let index = apex_core::svgmap::index_from_catalogue(&catalogue);
    eprintln!("{} track layouts", index.as_object().map_or(0, |o| o.len()));
    println!("{}", serde_json::to_string_pretty(&index).unwrap());
}
