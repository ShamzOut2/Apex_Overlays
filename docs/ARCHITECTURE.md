# Architecture

```
 iRacing ──shared memory──▶ telemetry thread ──topics──▶ hub ──WebSocket──▶ overlay pages
 (60 Hz, IRSDKDataValidEvent)  Engine::tick()       (dedupe,      (/ws)     (desktop windows,
                                                     only if                OBS browser sources,
 mock simulator ─────────────▶                       subscribed)            settings panel)
```

## Goals

- **Lightweight.** One small native process does the work. Overlays are static pages of about 45 KB that only update
  text and attributes that actually changed. No framework, no virtual DOM, no per-frame allocations in the hot UI path.
- **Smooth.** The telemetry thread wakes on iRacing's *data valid* event, so it runs at the sim's 60 Hz with no
  polling jitter. The browser batches messages into one render per animation frame.
- **Customisable.** Each overlay declares a settings schema. The settings panel renders forms from it, and the values live
  in `config.json` as free-form JSON, so adding an option never needs a backend change.

## Pieces

### `crates/irsdk`
- Parses the `irsdk_header`, variable headers and telemetry rows from byte slices. The same layout is used by `.ibt` files,
  which makes IBT replay a small step later.
- `SharedMemory` (Windows) maps `Local\IRSDKMemMapFileName`, waits on `Local\IRSDKDataValidEvent`, and copies the
  newest buffer. It re-checks the tick count after copying, so a row torn by the sim writing mid-copy is never used.
- `yaml` is a tiny parser for iRacing's session string. iRacing writes driver and team names unquoted, which can
  include `:` and `#`, so it reads every value to the end of the line instead of using a general YAML library.

### `crates/core`
- `model`: normalised `Telemetry` and `SessionInfo`. Sources fill these, and nothing downstream knows SDK variable names.
- `source::iracing`: the iRacing source. Variable handles are resolved once per connection.
- `source::lmu`: the Le Mans Ultimate source, reading the game's built-in shared memory (`LMU_Data`; `lmu::data` mirrors
  its layout, pinned by tests). It turns scoring and telemetry into the same telemetry rows and writes iRacing-style
  session info (drivers, classes, the session), so the engine and every overlay read both games the same way. LMU tracks
  get ids from 1,000,000,000 up, so nothing looks them up in iRacing's track lists. `lmu::map` builds their outline from
  every car's world position and lap distance (averaged per stretch of lap, pit lane from cars in the pits), hands it to
  the engine once complete (saved like a recorded outline), and the engine's heading-based recorder is not used for LMU.
  Sector boundaries are learned from cars changing sector and written into the session info.
  `lmu::extras` runs in the background while LMU is read: the weather forecast from the game's local REST API
  (`localhost:6397/rest/sessions/weather`, five points per session; the `forecast` topic places them on the session
  clock), Lovely's LMU track data (corners, pit entry/exit) and the shared map library under the LMU track ids.
- `source::games`: picks the game: the one chosen under General → Game, or (automatic) whichever is running. When the
  game changes the engine starts afresh and the status names the new game.
- `source::mock`: a deterministic three-class race on a synthetic circuit. It emits YAML that goes through the real parser.
- `engine`: derived state recomputed every tick:
  - running order and positions per class (live progress in races, best lap otherwise)
  - gaps to the leader and to the car ahead (estimated-time based, lap-aware)
  - per-car lap history, pit stops and stint length
  - `fuel` (consumption per lap and the race-length estimate for lap/time races and lapped cars)
  - `weather` (wetness and rain trend), `tyres` (wet/dry advisor), `battle` (gap trend by least squares)
  - `trackmap` (outline recorder: integrates heading over distance for a clean lap, closes the loop and caches per TrackID)
- `league`: user-defined classes and driver overrides that replace iRacing classes everywhere.
- `topics`: serialises each topic. Payloads are rounded so unchanged data serialises identically, and the hub drops duplicates.
- `hub`: topic fan-out, subscriber counts (a topic is only built while someone watches it) and host events for the desktop app.
- `server` (axum): serves the embedded UI and `/o/<id>` pages, plus the `/ws` protocol:
  - client → server: `sub`, `unsub`, `setEditMode`, `setVisible`, `drag`, `resize`, `setConfig`, `patchOverlay`, `resetTrackMap`,
    `setIracingApi`, `importLeague` (progress comes back on the `leagueImport` topic)
  - server → client: `{"t": topic, "d": payload}`
- `engine::ratings`: the live iRating estimate (community formula, computed per iRacing class), a rough SR estimate, and league points.
- `corners`: a background worker that downloads corner data for the current track from Lovely Track Data and caches it. Corners you edit in the config take precedence.
- `iracing_api`: the iRacing Data API client (OAuth password-limited grant with masked secrets, following the S3 `link` indirection) and the league importer.
- `lookup`: flair → ISO country code (for flags) and car → manufacturer.
- `garage61`: a background worker that maps the current iRacing car and track to Garage 61 IDs and fetches your PB and your teams' best laps.

### `apps/desktop` (Tauri 2)
- Runs the core in-process and opens one transparent, undecorated, always-on-top, **click-through** window per
  enabled overlay. In edit mode the windows take the mouse; the overlay page asks the host (over WebSocket) to start a native
  drag or resize, and new geometry is debounced into the config.
- Tray menu, global hotkeys, single instance, and a settings window that is the same page as `http://127.0.0.1:8182/`.

### `ui`
- `src/overlay.ts` is the host page for one overlay: it applies the theme and settings and feeds topics to the overlay.
- `src/overlays/*` hold one file per overlay, each exporting an `OverlayDef`, and `registry.ts` lists them.
- `src/control/*` is the settings panel (overlays with live previews, theme, strategy, league builder, Garage 61, general, OBS).

## Data budget

Topics nobody subscribes to cost nothing, and a topic whose payload has not changed is not re-sent.

Each car's fixed details (number, name, team, car, class, licence, iRating, country, make) are on the `drivers` topic,
sent only when they change (each session info update at most). The rows of `relative`, `standings`, `safety` and `battle`
carry just the car index, `isPlayer`, `isFocus` and live data; `ui/src/lib/client.ts` fills the details into them before
any page sees them, so overlays read rows as if everything were there. The relative only sends as many cars either side
as the relative overlays show (4 by default, at most 10). With 19 cars that is about 4.5 KB per relative message and
12.5 KB per standings message. The track outline (about 8 KB) is only re-sent when it changes.
