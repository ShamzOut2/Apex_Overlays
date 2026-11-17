# Changelog

The section for each version is shown in the in-app update pop-up and on the GitHub release.
Each version groups its changes under `### New features`, `### Improvements` and `### Fixes`
(plus `### Performance` or `### Removed` when needed).

## Unreleased

### New features
- Layout Market (new page): browse overlay layouts other drivers have shared, filter by game and search, and add one to your layouts in a click; its positions are fitted to your screen and your theme stays as it is. Share your own with Share my layout: the app makes a share code and opens the share form on GitHub (a free GitHub account), where it is checked and listed automatically within a few minutes. Layouts made with a newer version of the app are shown but wait for you to update
- Overlays fit their windows to their content by default, so nothing is cut off: the height follows what is shown, and the window widens when something would not fit (back to your width when it does). Not for the overlays drawn to fill their window (track map, radar, inputs, input graph, flags, spotter)
- Text size for each overlay (its General tab): 0 keeps the theme's size
- Fuel / energy left column for the relative and standings (FUEL): each car’s fuel, or virtual energy for the cars that race on it (shown in blue), as a % in Le Mans Ultimate; your own fuel in iRacing. Red when under 10%
- Tyre data: show one temperature per tyre instead of three: the core (carcass) temperature in Le Mans Ultimate, or the average across the tyre

### Fixes
- Closing Le Mans Ultimate now hides the overlays, even when another program (SimHub, CrewChief...) keeps its shared memory open; the app checks the game is still running when its data stops
- Switching games: closing one and starting the other (LMU to iRacing or back) now switches to it, instead of staying with the game that was closed. iRacing closing without saying so (a crash) is noticed too
- Le Mans Ultimate’s private qualifying: cars driving in their own qualifying sessions no longer show on the relative and radar as if they were around you. A setting under General turns this off for open qualifying, where everyone is on track together
- Tyre data: temperatures and tread % no longer spill out of their boxes on narrower overlays or with wider fonts; the numbers shrink to fit

## 1.2.0

### New features
- Lap times overlay: your last laps with their sector times, personal bests in green and the class’s fastest lap in purple, the gap to your best, and the best possible lap (your best sectors added up). The lap under way fills in sector by sector, laps through the pit lane are tagged, and in Le Mans Ultimate laps taken away for track limits are struck out
- Virtual energy overlay (Le Mans Ultimate): virtual energy planned like fuel, for the classes that race on it (Hypercar, LMGT3): laps of energy left against laps to go on a bar with the finish marked, energy per lap, what you will have at the flag, how much to add at the stop, the pit window and the hybrid battery. It hides itself in cars without virtual energy and in iRacing
- Radar overlay (Le Mans Ultimate): the cars around you seen from above, exactly where they are, amber when close and red when alongside. It hides itself until a car is near
- Car settings overlay: the settings you change from the wheel (brake bias, ABS, traction control, engine map, fuel mixture, hybrid deploy... whichever your car has, in the order you choose), each on a card coloured by kind with a meter of its steps (brake bias front / rear), and a big pop-up showing the new value and the one before whenever you change one. It can also run as pop-ups only. In Le Mans Ultimate it can show how many steps each setting has
- Tyre data overlay: a card per wheel with the temperatures across the tyre as tall cells (blue when cold, green in the window, amber when hot, red-orange when critical), the pressure, tread left and brake temperature (coloured the same way), and a pill naming the worst problem (e.g. FR INNER 118°, TYRES COLD, RL TREAD 14%). Live in Le Mans Ultimate, which also gives each tyre’s ideal temperature (used for the window) and carcass temperature; iRacing measures tyres only at pit stops, so there it shows your last stop’s readings
- LMU badges: overlays, settings and buttons for Le Mans Ultimate carry a small LMU badge in the settings, including the fuel-added columns in the relative and standings (every car in LMU, only you in iRacing)

### Improvements
- The Spotter is now iRacing only (with an iRacing badge), as the Radar is Le Mans Ultimate only: each is left out of the other game’s overlay list and stays hidden while the other game runs
- Settings → Theme → Sponsored by Us: “the teams and leagues Apex Overlays sponsors” (JWB is a league)

## 1.1.2

### Improvements
- VCR theme: VCR’s new colours, yellow (#E9B949) in place of red, with its new logo on the theme card. Cars a lap ahead are now soft red, so they stand apart from your car’s yellow

## 1.1.1

### New features
- Nimbus Motorsport theme: black, lime green and grey
- Settings → Theme: the team themes (Gassed Up Motorsports, VCR, JWB and Nimbus Motorsport) have a section of their own, Sponsored by Us, and the teams’ cards show their logos (VCR with its full Virtual Circuit Racing logo)

## 1.1.0

### New features
- Secondary classes (League builder), as in the Apex Commentator app: a class running across the main ones, e.g. a Rookie Cup or a Pro-Am trophy. Each class is a main or a secondary one (Kind, or Add secondary class); secondary classes are never picked by rules, but for each driver in the roster’s new 2nd class column, and the driver keeps their main class everywhere. Switching a class’s kind moves its drivers to the other column. Re-importing a league from iRacing keeps them
- Car numbers by secondary class: Number colour → Secondary class in the relative, standings, battle box and safety alert (drivers without one show just the number). The little colour bar by each driver in the relative and standings (Colours → Colour bar by each driver) can show the main class (as before), the secondary class (no bar for drivers without one), the team’s colour (the class colour for a team with no colour) or follow the car number. And a 2nd class column shows each driver’s secondary class
- Sign-up sheets can bring in secondary classes (“The sheet’s classes are: Secondary classes”), and league files from the Apex Commentator app keep their classes’ kind and drivers’ secondary classes
- Team builder (League builder → Team builder), with the features of the Apex Commentator app’s: each team of a league gets a name, a short name (for tight spaces), a colour (optional) and a full and a small logo; drivers are added to and taken out of teams from the team’s card, with a list of who is not in a team. Renaming a team moves its drivers with it. The roster’s Team column suggests the league’s teams
- Teams in more than one league: tick another league under “Also in” to put the team, and its drivers who race there too, in that league as well; changes to the team (name, short name, colour, logos, new drivers) reach it from then on. ✕ takes the team out of that league
- Export and import teams (with their logos and who drives for each), as files either app can read; importing a league file from the Apex Commentator app now brings its teams and logos too
- Import a sign-up sheet (CSV) into a league: iRacing IDs, names, car numbers, teams, team colours, classes and nicknames, with the columns worked out from the headings (and checked before importing). Drivers already on the roster are updated, the rest added; new teams and classes are made
- Car numbers in team colours: Number colour → Team in the relative, standings, battle box and safety alert. Drivers in a team with no colour, or in no team, keep their class colour
- Team logo column in the relative and standings (the full or the small logo), and the Team column can show teams’ short names
- In the Team builder and on the roster, the arrow keys move between the boxes, as in a spreadsheet
- Log file for bug reports: Apex Overlays now writes what it does and every error it meets to apex.log, in the logs folder of its data folder. That covers the game connecting and each session, overlays that fail to open, downloads (Garage 61, track and car data, the LMU forecast), settings that cannot be saved, updates, VR, crashes (with where they happened) and script errors inside the overlays and the settings window. The previous run’s log is kept as apex.previous.log, so the log of a crash survives the restart. Settings → General → Logs opens the folder or copies the log, ready to paste into a report. The same error repeating is written once with a count, so the file stays small

## 1.0.2

### Performance
- Driver inputs and Input graph: the scrolling trace draws only its newest sliver each tick and slides the rest along, instead of redrawing every line in full 60 times a second. It looks the same, for about a third of the drawing work (the Input graph overlay uses around 55% less CPU)
- The iRating pills on the relative and standings no longer blur what is behind them: the blur had to be redrawn by the graphics card for every pill on every update, taking GPU time from the game
- The delta bar, the Garage 61 delta bar and the spotter’s car markers move without re-laying out the overlay on every tick
- Track map: the car dots are updated 30 times a second in Balanced (20 in Light) instead of 60: still smooth, as a car moves well under a pixel between updates, for about a third less work
- Overlays only take the live car data as often as they show it: the start lights look a few times a second until a start, grid or countdown comes up (then every tick, for exact timing), the weather compass 15 times a second, Garage 61 and the data box 20, and overlays set to hide when you are not driving or in the pits check that 10 times a second
- The Light update rate (previously “Fast”) is lighter: relative and safety alert 10 updates a second, standings and battle 5, track map 20, inputs, spotter and pit 30
- The drivers’ names, teams, cars and classes are no longer sent out to the overlays again each time iRacing updates the session info (every few seconds in a race, with each new result), only when one of them changes
- Le Mans Ultimate: the session details are built when the game’s scoring updates (5 times a second) instead of with every telemetry update (60 times a second)
- Monitoring: the readings look for the app’s own processes every 30 seconds (or when an overlay opens) instead of every 5, as listing every process on the PC was itself a small spike every few seconds while the Monitoring numbers were shown

## 1.0.1

### New features
- Fuel added column on the relative and standings: how much fuel each car put in at its last pit stop (FUEL+), and in total this session (FUEL Σ), lit up while the car is filling. In Le Mans Ultimate it shows for every car, as a share of a full tank (LMU tells every car’s fuel level, or virtual energy for the classes that run on it); in iRacing it shows for you, in litres or gallons, as iRacing does not tell other cars’ fuel. Turn the columns on under Columns

### Fixes
- Le Mans Ultimate: the gaps on the relative and standings (gap, interval and the relative’s time to each car) work: they showed 0 for every car
- Le Mans Ultimate: a car crossing the line counts its new lap straight away, so the gaps no longer jump for a moment at the line
- Le Mans Ultimate: gaps work from the first laps of a session, before the game has an estimated lap time for the class (the class’s best lap is used)

## 1.0.0

### Improvements
- VR in Le Mans Ultimate: the overlays show in the headset in LMU as in iRacing, drawn inside the game in OpenXR (with Virtual Desktop, Quest Link, Pimax, SteamVR’s OpenXR…) or as SteamVR overlays, picked automatically. The VR status names the game the overlays are showing in (“Connected to Le Mans Ultimate through OpenXR on Virtual Desktop”)
- Each game keeps its own VR screen layout (distance, size, raise/lower, left/right) along with its own overlays and VR positions, switching with the game; VR on or off and how stay the same for both

## 0.9.9

### New features
- Weather forecast overlay, for iRacing and Le Mans Ultimate: 4 or 5 tiles (you choose), the weather now and then every 15 or 30 minutes or every hour ahead (you choose), as modern cards with a sky icon (sun or moon by the time of day at the track), temperature and chance of rain, glowing blue when rain is likely. Le Mans Ultimate publishes a forecast for each session, read from the game; iRacing does not share its forecast with apps, so there the overlay shows the conditions now and says so. Turn it on under Overlays (it is added switched off)
- Le Mans Ultimate support. Every overlay works in LMU: relative, standings, track map, inputs, fuel, delta bar, start lights (rolling starts), pit lane, spotter, weather and the rest. Apex Overlays reads LMU’s built-in shared memory (turn on “Enable Plugins” in the game’s settings; nothing to install). LMU has no iRating, licences or incident points, so those stay empty
- Le Mans Ultimate track maps are drawn from where the cars actually are: LMU publishes every car’s position, so the map is built from all the cars on track at once (usually within a minute of a busy session, even while you sit in the garage or spectate), exact rather than reckoned from one lap of yours, with the pit lane drawn from cars driving through it, and sector lines from where cars change sector (also used for sector times). It is kept, so the next visit shows it straight away. LMU tracks also come from the shared map library (so a track mapped once is there for everyone: Settings → General → Game → Export my LMU track maps), and get corner names, pit entry and exit from Lovely Sim Racing’s LMU track data, as iRacing tracks do. iRacing’s track maps are unchanged
- Game switch (Settings → General → Game): Automatic switches to whichever game you start, iRacing or Le Mans Ultimate; or pick one to stay with it. The status at the bottom of the settings window names the game being read
- Each game has its own overlays and layouts: positions, sizes, settings and saved layouts made for iRacing stay with iRacing, and Le Mans Ultimate keeps its own. The first time a game is used it starts from a copy of your current setup; the Overlays page says which game’s overlays you are editing, and can copy the other game’s over

## 0.9.8

### Performance
- Less CPU and fewer periodic spikes. The relative, standings, safety alert and battle box no longer resend every driver’s name, team, car, class and licence many times a second: those details are sent once (and again only when they change), so the relative sends about a third of the data it did and the standings two thirds. The relative only sends as many cars as your relative overlays show. The overlays create far less garbage to clean up (the clean-ups were the spikes): the track map draws the track and its sector colours once and reuses them instead of redrawing them every frame, and the timing tables stop re-reading their text, colours and flags on every update
- With VR switched on but SteamVR not running, the app no longer starts up SteamVR’s client every 5 seconds just to find it is not there (a CPU spike every few seconds): it first checks whether SteamVR is running
- The Monitoring readings (Settings → General and the performance widget) no longer read the CPU and memory of every program on the PC each second, only Apex Overlays’ own, so watching the numbers no longer adds to them

### Improvements
- VR: overlays have their own positions in VR. While they show in the headset, moving or resizing them (in Edit mode or with the position boxes) only changes where they are in VR; your screen layout stays exactly as it was and comes back when you leave VR. Settings → General → VR can reset the VR positions to the screen layout, and each overlay can go back to its screen position in VR on its own
- Driver inputs: engine RPM can be shown in large bold figures under the shift lights (Elements → RPM, off by default). It turns the shift lights’ “shift now” colour and glows at the car’s shift point
- Driver inputs: the steering angle can be shown above the steering wheel, in degrees with L or R (Elements → Steering angle, off by default)

## 0.9.7

### Fixes
- Pit lane overlay: being put on the grid for the race start no longer flashes “TURN OFF PIT LIMITER”. Moving from the pit box to the grid is not driving out of the pit lane; only crossing the exit line under your own power counts
- Taskbar icon: Windows could keep showing the old icon (on a pinned taskbar shortcut especially) after updating. The installer now tells Windows to refresh its icons, and has the app’s icon itself

## 0.9.6

### New features
- Start lights overlay: five realistic start-light housings (LED lamps behind glass lenses, with a visor and glow) that appear only for the race start. First a countdown as a warning that the race is about to start (“RACE STARTS IN 10, 9, …”, from 10 seconds by default; it can be turned off), then “GET READY” with the lights dark while the grid forms, then red lights, all five red when set, and green (or lights out, if you prefer) for the go, after which it disappears. Rolling starts (where iRacing shows no lights) get their own sequence: amber lights with “ROLLING START” on the pace laps, pulsing with “1 LAP TO GREEN” on the last one, and green “GO” when the green flag drops (it can be turned off, and the amber colour changed). Updated 60 times a second, so the lights change on the same frame as in the sim. The lights have no background, so nothing else shows on screen. Choose 3 to 5 lights, the colours, and whether the wording is shown. Turn it on under Overlays (it is added switched off)
- Apex Overlays is now a tool by Apex Labs: a new app icon (taskbar, tray, installer and browser tabs), the Apex Labs logo in the settings window and the update pop-up (light or dark lettering to suit your theme), and Apex Labs as the publisher in Windows

### Improvements
- Relative and standings: when the licence column is hidden, each iRating pill takes the colour of the driver’s licence (A blue, B green, C yellow, D orange, R red, Pro black), so you can still see licences at a glance. On by default; switch it off under Column style
- Data box, Incidents: shows the incident limits still ahead of you after your count (“9x / 10x / 20x”, then “10x / 20x” once the first is reached), from the session’s penalty and disqualification limits. Your count is green up to half way to the next limit, orange to three quarters, then red, starting again from each limit reached, and it blinks a few times whenever you reach one. The Incidents session field lists the limits the same way
- Theme → “Keep overlay sizes when changing font” (on by default): each font’s letters are sized to take the room the standard font’s do, and lines keep the standard spacing, so overlays, rows and columns stay exactly the same size whatever font you choose (as in Apex Commentator)
- Track map: the pit lane, drawn beside the track as on iRacing’s map (official maps; 407 of 421 layouts have one). It can be turned off or recoloured under Track
- Update pop-up: when you are a few versions behind, it now lists what is new in every version since yours (newest first, each under its version number), not only the latest one. The Updates card on the General page does the same

### Fixes
- Relative and standings: the IR and Driver column titles are centred over their columns (IR looked right-aligned), and the iRating pills are centred under theirs
- Delta bar: it could count down to zero over the lap. iRacing’s lap timer still shows the previous lap’s time for a moment after the line, and the recorded reference lap took that as its time for most of the lap. Laps are now timed from the moment the lap begins, and broken reference laps saved before are thrown away (the next clean lap replaces them). The bar also ignores that moment just after the line
- Overlays with a see-through background no longer have a light outline round them: the outline and the header strip fade with the background
- Track state: “Carry over” now shows the actual state it carries over (the last state iRacing named in an earlier session of the event)
- Manufacturer logos no longer have a white glow round them. They now use Apex Commentator’s versions, made for dark overlays (light lettering for Audi, Toyota, Nissan, McLaren, Porsche, Volkswagen and others), so they need no outline. The optional plate behind them is now dark, for light themes or a see-through overlay over bright scenery

### Removed
- Garage 61, for now, until the integration is ready: its settings page, its overlay and the Garage 61 reference laps of the delta bar (which now compares with your best this session or your all-time best). Your Garage 61 settings are kept for when it comes back

## 0.9.5

### New features
- Theme → Overlay font: pick the font every overlay uses from a list, or type any installed font under Custom. The settings window keeps its own font. A font you had typed in before stays as Custom
- 20 fonts come with the app, so they work on any PC without installing anything: Barlow Condensed, Rajdhani, Oswald, Titillium Web, Chakra Petch, Saira Semi Condensed, Roboto Condensed, Exo 2, Inter, Montserrat, Teko, Kanit, Orbitron, Sora, Space Grotesk, Archivo Narrow, Big Shoulders Display, IBM Plex Sans Condensed, Fira Sans Condensed and Antonio. Plus Standard (Segoe UI) and Bahnschrift Condensed from Windows

### Removed
- Fuel calculator: its own font option. It now follows Theme → Overlay font (choose Bahnschrift Condensed there for the previous look)

## 0.9.4

### New features
- Fuel calculator: a new design. Laps of fuel left in large figures, a segmented bar with a line at the finish, per lap, laps to go and fuel at the finish, and a status pill: cyan “Makes the finish”, amber “Marginal” (under half a lap spare, adjustable) or red “Pit · add 4.7 L”. The colours can be changed. A compact mode shows just the laps, the bar and the fuel at the finish. Last lap, stops and pit window show underneath, and every other figure (usual here, max per lap, needed to finish, target and save per lap, laps per tank, per hour, history) can be added in the same style
- Track state: how much rubber is down (Clean, Low usage, Moderately high usage…, as iRacing reports it) as a new session detail. Pick it in the Session info overlay’s fields, in the relative and standings headers, in the data box, or as a readout in the weather overlay (off until you turn it on)

## 0.9.3

### New features
- League builder: “Import league file” also takes a league exported from the commentator app: its classes and colours, every driver (car number, class, nickname, team), the points system and bonuses, and the championship so far, worked out from the rounds’ results (per class or overall, with multipliers, adjustments, dropped rounds and penalties). Importing a league you already have (same iRacing league ID) offers to update it

### Improvements
- League builder: the championship standings are shown per class when points are scored per class

### Fixes
- League builder: a file that is not a league export now says so instead of doing nothing

### Removed
- League builder: the “Import a league from iRacing” section (it needed an iRacing API login most people cannot get). Use “Import league file” or “Create from current session” instead

## 0.9.2

### New features
- Virtual Desktop support without SteamVR: with Virtual Desktop’s own OpenXR runtime (VDXR) on a Quest, the overlays now show inside iRacing in OpenXR mode
- New VR mode “Automatic”, now the default: it draws inside iRacing whenever iRacing runs in OpenXR mode (Virtual Desktop, Quest Link, Pimax, Varjo, SteamVR…), and otherwise uses SteamVR overlays while SteamVR runs. Settings saved with the old default (SteamVR) switch to Automatic once; “OpenXR only” and “SteamVR overlays only” are still there
- The VR status now says when the overlays are showing inside iRacing and on which runtime (for example “Virtual Desktop (VDXR)”), or why they are not
- Track map: choose your own colour for your car’s dot (Cars → turn off “Your car: use the theme’s highlight colour”)
- Track map: iRacing’s official map for every track and layout (424 of them), straight away, with no iRacing login and no lap to record first. Tracks added to iRacing later are found automatically

### Improvements
- A refreshed, modern look for the settings window, laid out exactly as before: icons in the sidebar, rounded cards with soft shadows, a pill highlight for the page you are on, and a pulsing dot when iRacing is connected
- Smoother buttons, drop-downs, text boxes and switches, with hover effects and a clear focus ring for keyboard use
- The overlay settings tabs are now a segmented bar, the overlay list has card-style rows, and the settings search box has a search icon
- Pop-up menus and windows fade in, with the page behind blurred
- Works with every colour theme, light ones included

### Fixes
- OpenXR: if a runtime ever refuses the overlays, iRacing’s own frame is now sent unchanged instead of being lost, and the overlays try again a few seconds later; the reason shows in the VR status
- OpenXR: overlay rotations are kept exact, as strict runtimes such as Virtual Desktop’s require
- Track map: official maps now come out right on tracks iRacing draws differently: Monza, Suzuka and Oran Park (drawn as a thin filled shape, which doubled the lap), 40 newer maps such as Adelaide, Navarra, Sachsenring, Cadwell Park and Sonoma (start/finish line in the wrong place), and maps whose corner names contain numbers or whose two-digit turn numbers are drawn digit by digit (Monza’s Lesmos, Algarve, Cadwell Park). Mount Washington runs from its start line to its finish
- Track map: the direction of travel now follows iRacing’s turn numbers first (Spa’s classic pits layout ran the wrong way)
- Track map: outlines recorded from your laps came out mirrored (the Charlotte Roval ran clockwise). They are now recorded the right way round, and ones you recorded before, plus the shared library’s, are turned round automatically
- Track map: if iRacing’s map server does not answer, a copy of the same map is used, and a failed download is retried instead of giving up for the session
- On narrow windows, the unit after a slider (such as “px” for the font size) no longer spills past the edge of its box

## 0.9.1

### Improvements
- VR with OpenXR: works with NextLevelXR and other OpenXR layers. The overlays are now drawn in Direct3D 12 sessions as well as Direct3D 11 (NextLevelXR supports both), and the overlay images are shared in a way both can open. If the overlays do not show or get cropped with another layer, the VR card explains how to change the order of the layers

## 0.9.0

### New features
- **VR with OpenXR, no SteamVR needed**: General → VR → How → "OpenXR, inside iRacing". Your overlays are drawn inside iRacing's own OpenXR VR (Quest Link, Virtual Desktop, Pimax, Varjo, WMR and other OpenXR runtimes), with the same per-overlay VR placement (fixed in the car or following your head, as on your screen or where you set). The first time, Windows asks for permission to register Apex Overlays' OpenXR layer; restart iRacing after turning it on. "Recentre VR" places the overlays fixed in the car from where your head is

### Improvements
- Overlay settings only show what applies: options that depend on another choice appear when it is made and hide when it is not (for example the VR custom position only with "Where I set below", a custom header title only with "Custom text", shift-light options only with shift lights on, the delta bar's team only with "one of my teams"). Searching still finds every setting
- Relative and standings: column styling (flag style, manufacturer logos, class badges, iRating gain/loss, out-lap label, PIT tag…) has its own "Column style" tab, and only lists the options for columns you show
- VR: move the whole screen layout left or right (General → VR → "Screen layout: move left / right"), alongside raise / lower
- VR: each overlay's rotation (turn, tilt and a new roll) now works however it is placed, including "as on my screen", in SteamVR and OpenXR

## 0.8.0

### New features
- **VR (SteamVR)**: General → VR shows your overlays in the headset, looking and updating exactly as on your screen. Each overlay's new VR tab chooses whether it shows in VR, whether it is fixed in the car or follows your head, and where it goes: as on your screen (your monitor's layout on an imaginary screen in front of you, whose distance, size and height you set) or at a position, angle and size you set, with its own opacity. A "Recentre VR" hotkey resets the seated position. Needs SteamVR running; OpenXR without SteamVR comes later
- **Input graph overlay**: the scrolling throttle / brake / clutch trace on its own, so you can place and size it anywhere. Switch off "Input trace graph" in Driver inputs to split it from the pedal bars, wheel, gear and speed. It has the same trace options and colours, and is added (switched off) to existing setups

## 0.7.2

### New features
- **Remove Driver Class in Name** (League builder → Drivers): for leagues that put the class in front of names, such as “(P) Max Power” or “(PA) Max Power”. Tick it and the brackets and what is in them come out, the space in front goes, and the display name is filled with the driver's full name, so the overlays show “Max Power”. A nickname of the driver's own is kept (without a tag), drivers added later are cleaned up too, and drivers not on the roster show without the tag as well

### Fixes
- Standings as a single overall list: only overall P1 is the leader. Gap and interval are measured to the overall leader and the car ahead overall (they were per class, so every class leader showed "Leader"), and the list's header says Overall instead of showing a class badge

## 0.7.1

### New features
- **SLOW badge**: drivers serving a slow-down penalty get an orange SLOW badge next to their name in the standings, relative and safety alert. Hover it to see how long they have been slowing and roughly how much time they have given up
- **Possible overtake** (safety alert): when a car just ahead (3 s by default) is serving a slow-down, the alert shows it with how long it has been slowing, the time it has given up so far and its speed against yours. iRacing does not say how long a slow-down lasts, so the app measures the time given up from the car's own pace

### Improvements
- Standings list every driver in the session in practice, qualifying and the race, also those still in the garage or loading in (shown dimmed with OUT)
- Relative and standings: column titles are centred over their columns, numbers and times are centred under them, and the spacing between columns is tighter and even
- Manufacturer logos sit straight on the overlay (no white box), with a thin light outline so dark logos stay visible; the white background is still an option under Columns
- The Flags overlay calls iRacing's furled black flag what it is used for now: SLOW DOWN

### Fixes
- Yellow flags: cutting a chicane or running over the kerbs (Monza's turn 5, for example) no longer counts as causing a yellow. A car has to stay off track for over a second, or be slow off the track
- Dragging columns to reorder them works in the app on Windows (it showed a "not allowed" cursor)
- Manufacturer logos: the standings and relative could get stuck showing short names (BMW, CHE, MCL) instead of the logos. Each logo is now found in one request (your own image in the logos folder first, then the built-in one) and is tried again if it ever fails to load

## 0.7.0

### New features
- **Safety alert overlay**: stays hidden until something needs your attention
  - **Yellow and meatball ahead**: a car up to 10 s ahead (you choose how far) that is stopped, slow, off track or rejoining, or carrying the meatball, shows the waving yellow or meatball flag (the same flags as in the standings) with the driver (number, country, licence, iRating), their car, position, how far ahead they are in seconds and metres, and their speed. It disappears once you have passed them
  - **Faster class behind**: a car of a faster class closing on you shows its class, car, driver, the gap in seconds and metres, how much faster a lap it is and roughly when it will reach you
- **Delta bar overlay**: your live delta to a reference lap, with a bar (grows right when you are faster, left when slower), your predicted lap time and a gaining / losing arrow. Compare with:
  - your best lap this session
  - your all-time best: the fastest of the laps recorded by Apex Overlays (saved for every car and track) and your Garage 61 best
  - the fastest lap in one of your Garage 61 teams (pick the team), or in all your teams
  - the fastest lap on Garage 61 that you can see
  - until the chosen lap is available it can use your best this session
- **Fast update rate** (General → Performance): overlays update 20–30 times a second instead of up to 60, for the least CPU. Telemetry is still read 60 times a second

### Improvements
- The safety alert and delta bar have their own show / hide hotkeys (General → Hotkeys) and are added, switched off, to existing setups
- Layouts: overlays a saved layout does not have come along switched off when you switch to it, so every overlay's hotkey works in every layout

### Fixes
- "Fit the window width to the columns" only appears for overlays that have columns (relative and standings), not for the track map and others

## 0.6.0

### New features
- **Saved layouts**: save the overlays you have as a named layout (Race, Qualifying, Streaming…), set up others, and switch between them from Overlays → Layouts. Each layout remembers its own overlays, positions and settings
- **Session layouts**: tick P, Q, W or R next to a saved layout and it is switched to automatically when a Practice, Qualifying, Warmup or Race session starts
- **Share layouts**: export a saved layout to a file (⇩) and send it to other drivers; they import it as a new saved layout of their own, with its session types and, if they want, its colour theme
- **More hotkeys** (General → Hotkeys): next layout, one per saved layout, and one per overlay to show or hide it — all blank until you set them. Click a box (it shows Recording…), press the keys and it is saved straight away; a hotkey used twice is flagged
- **Monitoring** (General): live CPU and memory use of Apex Overlays and its overlay windows (with a one-minute CPU graph), your PC's CPU, data sent to the overlays, telemetry rate, engine time per update, windows open and uptime
- **Performance monitor overlay**: a small on-screen widget with the readings you pick and order; CPU turns amber or red above a level you choose
- **Flag abbreviations** (League builder → Drivers): every flag code the Flag box accepts, with the flag and country name — searchable, click a row to copy the code

### Improvements
- General is now the first page in the settings window
- Hotkeys apply straight away, no restart needed
- The update pop-up shows what's new grouped into new features, improvements and fixes

### Fixes
- iRating pill: the gain/loss (green ▲ / red ▼) now sits inside the pill with the rating

## 0.5.8

- iRating is shown in a soft frosted pill, with the estimated race gain (green ▲) or loss (red ▼) right next to it instead of in a separate column; switch it off with "iRating gain/loss next to iRating" in the Columns tab
- Columns: drag columns to rearrange them, or drag a hidden column into place (the ▲▼ buttons still work)
- Settings: long drop-down options no longer squash the setting's name (e.g. the overlay update rate under General)
- Closing the settings window now asks whether to close the overlays too or keep them running (from the tray icon), with "don't ask again"; the choice can be changed under General → When I close this window

## 0.5.7

- Shared track map library: tracks other drivers have recorded get their outline straight away, no clean lap needed (Road Atlanta, Brands Hatch, Virginia International Raceway and Charlotte Motor Speedway to start with). Your own recording always takes priority
- iRacing's official track maps: the list used to share them is now written as soon as the app starts (for when iRacing reopens API access)

## 0.5.6

- Track map: iRacing's official track maps no longer need the iRacing login — the app downloads the list of maps published with Apex Overlays, so every track has its real outline straight away instead of waiting for a clean lap (tracks missing from the list are still recorded from your driving)
- Overlay settings redesigned: each overlay's settings are split into tabs (What to show, Layout, Columns, Colours, Size & visibility…), with a search box, a live preview at the top and a ↺ button to put any setting back to its default; tabs with changed settings are marked
- Columns: the columns you show are listed in order (move or remove them), the others are one click away to add
- Overlay list: switch overlays on and off right in the list; "Add overlay" shows every overlay with what it does; moving all overlays and sharing layouts are in one "Layout & sharing" menu
- Faster: timing tables are sent at a sensible rate instead of 60 times a second (standings 10/s, relative 20/s, fuel and session 4/s; inputs, track map and spotter stay at 60/s) — about 70% less data, and the standings window uses about a fifth of the CPU it did; *General → Performance → Overlay update rate* switches back to everything 60 times a second
- Faster: overlays only read the newest update of each kind per screen refresh, skip drawing while hidden, and no longer receive the 60/s input data unless they need it; the settings window only receives live race data on the pages that show it

## 0.5.5

- New Pit lane overlay: a BRAKE sign and the distance to the pit entry line on the way in; your speed and a pit limiter light in the lane (solid green on, flashing red off); WAIT and a flashing GO in your box; the distance to the pit exit line; and a flashing "turn off pit limiter" once you cross it. It only appears when you head for the pits and hides 5 s after you leave
- New Incident tracker overlay: each incident with its likely type (off track, loss of control, wall, car contact), the points, who you made contact with, your total against the limit, and a replay button that jumps the iRacing replay to the moment (clickable while a replay is playing)
- Inputs: while ABS is working, the brake trace (thicker line and filled area) and the brake bar change to the ABS colour
- New Spotter overlay: left and right bars that only appear when a car is alongside you, showing where it overlaps your car; red when three wide or two cars on one side
- Garage 61: live delta to your Garage 61 best lap or your team's fastest lap, with sector deltas, using the lap's telemetry (downloaded once and cached)
- Garage 61: choose which laps are searched (all time, this season, this and last season, last 90/30/7 days); all time by default
- Garage 61: optionally only compare with laps driven in similar conditions (track temperature within a range, and the same dry or wet track)
- Garage 61: the team leaderboard shows the gap to the fastest lap, and the settings page tells you if your token is missing the driving data permission
- League class badges: Pro and Semi-Pro (also Semi Pro, SemiPro, Professional)
- League classes are matched by their name only: a league class with no matching badge shows its name instead of the badge of the cars in it (Pro and Semi Pro classes of GT3 cars showed the GT3 badge)

## 0.5.4

- League class badges: Gold, Silver, Bronze, Am and Pro-Am, coloured to match the league class colour; Amateur, Amatuer and Amz show as AM, Pro Am / ProAm / Pro-Amateur as PRO-AM
- Wide class badges are scaled to fit the class column instead of being cut off
- Shift lights: blank spots in a car's light bar no longer show as lights that are always on (Mercedes-AMG GT3 and GT4, BMW M4 GT3, Audi R8, McLaren 720S, Lamborghini Huracán, Acura NSX, Aston Martin Vantage GT4, Corvette C8.R and BMW M8 GTE, Mercedes W12/W13, Mustang GT4, Porsche 992 Cup); checked against all 84 cars in Lovely Car Data

## 0.5.3

- Fuel calculator remembers what each car uses at each track: in practice (or any session) it shows the usual consumption per lap, lap time and laps counted from earlier sessions, and plans with it until you've driven a clean lap
- Standings and other auto-sized overlays resize again after the app restarts or reconnects
- Team themes: Gassed Up Motorsports, VCR and JWB
- Standings class headers show just the class badge when there is one (the class name still shows for classes without a badge; the old badge-and-name look is an option)
- Shift lights are back to each car's own lights and shift points (the extra blue shift-point lights from 0.5.2 are gone), with a new light pattern option: as the car has it, left to right, right to left, outside in, or inside out

## 0.5.2

- Track map: iRacing's official track map outline, downloaded with your iRacing login (League builder → Import from iRacing), so the full circuit shows straight away; the outline recorded from your driving is still used without a login, or if you prefer it
- Class badges for 52 iRacing classes (GT3, LMP2, TCR, GTP, MX-5, oval and dirt classes…), coloured to match each class's colour in the session, in the standings class headers and the class column
- Shift lights finish with blue shift-point lights at the ideal shift RPM (in the middle for centre-out patterns), and only flash once you go past it
- Flags overlay: the writing on every flag is now readable (yellow, white and LED-style flags)
- iRating gain/loss estimate counts the whole field from the start, so it no longer shows large negative numbers while drivers are still loading in
- Before the start, the standings show the qualifying grid, including drivers who haven't loaded in yet; the grid is remembered if the app restarts

## 0.5.1

- Update pop-up: when a new version is out, Apex Overlays offers it at start-up — update now, remind me tomorrow or in a week, or skip that version
- Gaps in the relative, standings and battle box no longer jump when a car crosses the start/finish line
- League builder follows the session: a league is used only in sessions with its iRacing league ID (or always, if you choose)
- Shift lights in the driver inputs overlay, with per-gear shift points from Lovely Car Data
- Track map: corner names for more tracks, pit entry/exit markers and optional straight names
- Short class names (TCR, GT3, LMP2…), class logos in the standings, built-in manufacturer logos
- Flags for every country and territory, and a globe for the Global flair
- Column lists show each column's title (e.g. "— PTS")

## 0.5.0

- Track map sector colours (purple / green / yellow), driver input percentages, readable text on bright theme colours
- Spectated-car compass, D/W tyre badges, PIT tag next to names, auto-fit width, move all overlays together, overlay sharing

## 0.4.0

- Flags, weather and data box overlays, battle box modes, auto-fit height, timed-race laps (x/∞), in-app updates
- Driver flags and penalty box, race control page
