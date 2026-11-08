# Published data

## iracing-track-maps.json

Where iRacing's official track map of every track layout lives: track ID → map folder and SVG
layer files (`active`, `start-finish`, `turns`), plus `mirror`, a copy of the same files kept
by the [racing-track-maps-vector](https://github.com/meowmachine/racing-track-maps-vector)
project, used when iRacing's server does not answer. The SVG files are public.

This file is built into the app, so every track has its official map with no iRacing login
and nothing to record. Older versions of the app download it from here. Tracks added after a
version was built are looked up in the racing-track-maps-vector catalogue at run time.

To refresh it (new tracks or layouts), download that project's
`from-iracing/iracing-tracks-metadata.json` and run:

    cargo run -p apex-core --example track_map_index -- iracing-tracks-metadata.json > data/iracing-track-maps.json

To check every layout still builds a good outline, download each entry's three layers into
`<dir>/<trackId>/<layer>.svg` and run
`APEX_TRACK_SVGS=<dir> cargo test -p apex-core every_indexed_track -- --ignored`.

## trackmaps/

The shared track map library: outlines recorded by drivers (one clean lap each), so other
users get a map for these tracks without recording one. `index.json` lists the tracks;
each `<trackId>.json` is one outline in Apex Overlays' own format. A user's own recording
always takes priority, and so does iRacing's official map when it is available.

To add a track: drive a clean lap so Apex Overlays records it, then take
`%APPDATA%\Apex Overlays\trackmaps\<trackId>.json`, copy it here (set `"source"` to
`"library"`) and add the track to `index.json`.
