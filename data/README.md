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

## iracing-track-outlines.json

The official outline of every layout in `iracing-track-maps.json`, already built, so the map
shows the moment a track loads, with no download and no lap to drive. Built into the app;
the app still downloads the SVGs when it can and replaces the built-in outline with its own
(in case iRacing redraws a map). Format: `{"version": 2, "tracks": {"<trackId>": {"p": ...,
"pit": ...}}}`, where `p` is the 500 outline points and `pit` the pit lane dashes (pairs of
points), each as base64 of little-endian u16 x, y pairs (0 to 65535 for 0 to 1).

To rebuild it, run the `every_indexed_track` check above (it writes each outline to
`<dir>/out/<trackId>.json`), then pack them:

    python3 tools/pack_track_outlines.py <dir>/out data/iracing-track-outlines.json

## trackmaps/

The shared track map library: outlines recorded by drivers (one clean lap each), so other
users get a map for these tracks without recording one. `index.json` lists the tracks;
each `<trackId>.json` is one outline in Apex Overlays' own format. A user's own recording
always takes priority, and so does iRacing's official map when it is available.

To add a track: drive a clean lap so Apex Overlays records it, then take
`%APPDATA%\Apex Overlays\trackmaps\<trackId>.json`, copy it here (set `"source"` to
`"library"`) and add the track to `index.json`.

### Le Mans Ultimate tracks

LMU outlines go in the same library, under the LMU track ids (from 1,000,000,000 up; Apex
Overlays derives them from the track's name). The app builds them from the cars on track and
keeps them; *Settings → General → Game → Export my LMU track maps* saves every one it has as
`apex-lmu-track-maps.json` (`{"maps": [...]}`). For each map in it: save it here as
`<trackId>.json` with `"source"` set to `"library"`, and add `"<trackId>": "<track name>"` to
`index.json`. From then on every LMU driver gets that track's map straight away.

