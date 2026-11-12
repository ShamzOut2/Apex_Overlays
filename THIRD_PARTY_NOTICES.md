# Third-party notices

## Twemoji Country Flags font
`ui/dist/assets/fonts/TwemojiCountryFlags.woff2` comes from the
[country-flag-emoji-polyfill](https://github.com/talkjs/country-flag-emoji-polyfill) package (MIT, © 2022 TalkJS).
The flag artwork is from [Twemoji](https://github.com/twitter/twemoji), © Twitter, Inc and other contributors,
licensed under [CC-BY 4.0](https://creativecommons.org/licenses/by/4.0/).

## Overlay fonts
The fonts offered under Theme → Overlay font are bundled in `ui/static/assets/fonts/overlay/` (Latin and
Latin Extended subsets, from Google Fonts, fetched with `tools/fetch-overlay-fonts.py`). Each is licensed
under the [SIL Open Font License 1.1](https://openfontlicense.org); the licence and copyright notice of each
font is in `ui/static/assets/fonts/overlay/licenses/`:
Antonio, Archivo Narrow, Barlow Condensed, Big Shoulders Display, Chakra Petch, Exo 2, Fira Sans Condensed,
IBM Plex Sans Condensed, Inter, Kanit, Montserrat, Orbitron, Oswald, Rajdhani, Roboto Condensed,
Saira Semi Condensed, Sora, Space Grotesk, Teko and Titillium Web.

## Lovely Sim Racing data
The following data is downloaded at runtime from community projects by [Lovely Sim Racing](https://lsr.gg), licensed under
[CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/). None of it is bundled with this application.

- [Lovely Track Data](https://github.com/Lovely-Sim-Racing/lovely-track-data): corner names and turn positions, straights,
  pit entry and exit on the track map.
- [Lovely Car Data](https://github.com/Lovely-Sim-Racing/lovely-car-data) (with ATSR and Gomez Sim Industries): shift-light
  RPMs and colours per gear for the driver inputs overlay, and car class shorthands.
- [Lovely Car Classes](https://github.com/Lovely-Sim-Racing/lovely-car-classes): short car class names.

## Car manufacturer logos
`ui/static/assets/makes/` contains the logos of the manufacturers that appear in iRacing, taken from the
[car-logos-dataset](https://github.com/filippofilip95/car-logos-dataset) (MIT License, © filippofilip95; thumbnails).
The Dallara, Oreca, Tatuus and Williams logos (not in that dataset) were supplied by the project owner.
The logos themselves are trademarks of their respective owners and are shown only to identify each car's manufacturer.

## Class badges
`ui/static/assets/classes/badges.json` holds the iRacing class badges designed for Apex Overlays by the project owner,
stored as one SVG template per class plus the badge colour palette.

## OpenVR (SteamVR overlays)

`apps/desktop/vr/openvr_api.dll` is Valve's OpenVR API library from
https://github.com/ValveSoftware/openvr (`bin/win64`), used by the desktop app to show
overlays in VR. It is distributed under the BSD 3-Clause licence in
`apps/desktop/vr/OPENVR_LICENSE.txt` (Copyright (c) 2015, Valve Corporation).

## openxr-sys

The OpenXR layer (`apps/xr-layer`) uses the `openxr-sys` crate (https://github.com/Ralith/openxrs),
MIT or Apache-2.0, generated from the Khronos OpenXR registry (Apache-2.0).
