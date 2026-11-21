# Changelog

The section for each version is shown in the in-app update pop-up and on the GitHub release.
Each version groups its changes under `### New features`, `### Improvements` and `### Fixes`
(plus `### Performance` or `### Removed` when needed).

## Unreleased

### New features
- Race length in timed races now reads "Lap 6/12.89" (was "6-12.89"), and what goes between the two is yours to set (Between the lap and the race length: /, -, "of"…)
- Fuel calculator: Average every lap, clean or not (in the fuel overlay and Strategy → Fuel). Off, as before, only clean laps count; on, pit-lane and caution laps count too. A lap you refuelled on never does, as what it used cannot be measured. It applies at once, to the laps already driven too

- Session details and overlay headers: the incident count is coloured by how close it is to the next limit: green under half way, orange from 50% to 74%, red from 75% (the Data box already did). Past a penalty limit it starts again towards the next one (e.g. after 17x, from 17x towards 25x). The three colours are settings, and it can be switched off
- The Car settings overlay is now called In-Car Setup

### Fixes
- Track map: "Turn numbers" showed corner names at about half the iRacing tracks, whose corner data has names but no numbers. The numbers now come from iRacing's own track map (built into the app for every layout, and taken from the map download), so each corner shows the number iRacing gives it (Spa's Raidillon T3-4, the Bus Stop T18-19), turns the corner data leaves out get a number of their own, and "Turn numbers" shows only numbers. Your own corners in Settings → General keep the numbers you gave them
- Start lights: the countdown was a second behind the timer. It rounded the seconds left up (9.4 s showed 10) where iRacing's timer and the app's own clock cut them down (0:09); it now counts like them, and the first number shows for a whole second
- Driver inputs: the steering wheel no longer runs off the bottom of the overlay. It was sized before the steering angle above it had its first reading, so it ended up too tall; it now sizes itself to the window, and again whenever the angle line or the window changes
- Car settings: the pop-up after a change fills the whole overlay, however big or small: it covers it edge to edge and its text and meter scale up or down to the room there is, so nothing is cut off and a big overlay gets a big read-out

## 1.5.1

### New features
- Lap count in timed races: a new choice, "Lap 6-12.89": the lap you are on, and how long the race is in laps at the leader's pace (laps done plus what the time left allows). It counts up with the laps rather than down, and the length moves as the leader speeds up or slows down. Decimals as set for the estimated laps (headers and the session overlay)
- Standings and Relative: a Pit lap column, the lap each car last came into the pits on (L12)
- Standings and Relative: flags and penalties can go at the end of each row (Flags & penalties: where to show → At the end of each row), as well as next to the name or, for the standings, outside the overlay
- Track map: Hide cars in the pits (Cars). Cars on pit road disappear from the map until they rejoin; your own car always shows
- Safety alert: "Private qualifying: hide other cars" (Behaviour), the same switch as the relative, track map, radar, spotter and battle box: in LMU's online qualifying and iRacing's Lone Qualify it stays quiet
- Safety alert: Warn about: clear dangers only (new default: stopped or crawling cars, and cars stuck off the track) or every car off the track or slowing
- The fuel calculator writes each lap to the log (Open logs): the fuel at the line when the lap began and ended, what it used, whether it counts towards the average (and if not, why), and the average, to check its figures against the game's

### Fixes
- Yellow flags (the safety alert and the yellow marks by drivers) were too eager: a car running wide or through a run-off at speed for a second counted. It now takes 2.5 seconds off the track, or being off it below about 43 km/h (was 1 second, or below 72 km/h)
- Car settings (iRacing): the ABS could show an absurd number and the brake bias stop following changes made in the car. After changing car or session the app could keep reading the settings from where the previous car had them; it now looks them up again with every new session. A value no setting has is left out rather than shown
- Car settings (iRacing): brake bias is read against the setup's bias, whatever scale the car uses (a %, a fraction, hundredths of a %, or an adjustment from the setup), so it follows each click of the adjuster; an adjustment of +0.5 no longer reads as 50%
- Track map: corner numbers were wrong on tracks whose data names its corners (Spa's La Source, Eau Rouge…) rather than numbering them: they were numbered in list order, so Spa's Bus Stop read T15. Those corners now show their name; real turn numbers are kept. Track data is fetched again so the fix applies to tracks already seen
- Manufacturer logos: Ligier shows Ligier Automotive's current logo instead of the old Ligier Automobiles one

## 1.5.0

### New features
- Overlay settings, reorganised (every setting and what it does stays the same, and saved layouts keep working):
  - The same tabs on every overlay, always in this order: Essentials, Content, Layout, Style, Behaviour and Window (a tab with nothing in it for that overlay is left out). The settings that used to be spread over 45 differently named tabs are now sections inside these
  - Essentials: the handful of settings most people change for that overlay, plus Show and Text size, on one page
  - Options sit under the switch that turns them on, indented with a line, instead of appearing elsewhere in the list
  - A section that one switch turns on and off (Shift lights, Corner speeds…) has that switch in its header; switched off, it folds down to the header
  - A chevron on each section folds its options away to save space while leaving it on ("3 settings hidden"); folded sections are remembered per overlay type
  - What you have changed is easy to see: a dot by each changed setting, "1 changed" and Reset section on each section, a count on each tab, and Changed only to list every change on the overlay
  - Search covers every tab of the overlay, with results under "Tab › Section"
  - Click a part of the preview (the gear, the wheel, the speed boxes, a row of the relative, a header…) to open its settings; hovering outlines what you can click
- General, reorganised the same way (every setting and button is still there):
  - Tabs instead of one long page: Essentials (game, units, update rate, the show/hide hotkey, and how updates and your licence stand), Racing data (units, short class names, ratings, private qualifying, game, track map & corners), Performance (the live readings, update rate, this window, monitoring), Hotkeys, VR and App (licence, updates, server, backup, logs)
  - Sections fold away, change dots, counts on tabs and sections, Reset section, Changed only, and a search across every tab that finds buttons too (e.g. "export")
  - Hotkeys: "Set only" shows just the hotkeys you use, and a hotkey used twice is named in a banner at the top
  - VR: "Show overlays in VR" is in the section header; off, the whole tab is one line
  - The live readings are on the Performance tab, so the other tabs are not redrawing graphs; Reset everything sits apart, at the end of Backup

### Fixes
- Incident tracker: "Lap and session time" could not be changed (it shared its name with the Show setting)
- Relative: the colour choice under "Colour bar by each driver" is now called "Bar colour from" instead of repeating the same name

## 1.4.0

### New features
- Driver inputs, new layout: the steering wheel on its own with its angle above it, your speed in a box between the top speed into the last corner and the lowest speed through it, and a big gear, side by side next to the pedals (as on a wheel-and-dash setup). The wheel is sized to the room it has
- Driver inputs: choose the layout under Layout: side by side or stacked, the order of the wheel, speeds and gear, and whether they sit right or left of the pedals
- Driver inputs: the steering wheel is now a formula-style sim wheel (a photo on a see-through background, turning with your steering). Wheel style also offers a drawn round wheel (rim with textured grips, three spokes, a hub with buttons, a top marker) and the old simple outline
- Input trace (Driver inputs and Input graph): faint lines across at 25%, 50% and 75% of input (Trace → Lines at 25%, 50% and 75%)
- Input trace: the steering line now fills the graph at the car's own steering lock rather than your wheel's full range (in iRacing that was often 900°, so the line barely moved). The lock is learned as you drive: the furthest you have turned in this car, at least 90° each way, growing if you turn further and starting again in another car. Or pick your wheel's full range, or a set angle (Trace → Steering fills the graph at)
- Driver inputs: colours for the wheel's top marker, the gear, the speed box and the two corner-speed boxes. The steering angle and corner speeds are now on by default

## 1.3.9

### New features
- "Private qualifying: hide other cars" now works in iRacing too: in Lone Qualify (each driver alone on track, as in official races) the other cars are left off the relative, track map, radar, spotter and battle box, as they already were in Le Mans Ultimate's online qualifying. Open Qualify, with everyone out together, is not affected. The switch is no longer marked LMU only

### Fixes
- "Private qualifying: hide other cars" is now also in the settings of the overlays it affects (Relative → Layout, Track map → Cars, Radar → View, Spotter → Behaviour, Battle box → Cars), not only under General → Display. It is the same switch everywhere: changing it in one place changes it for all

## 1.3.8

### Performance
- Relative, modern look: the gap bar moves in 5% steps and is drawn without re-laying out the row, so the modern look costs the same as the classic one (measured with both overlays running: 13.5% vs 13.9% of a CPU core in the test browser; before this, the bar alone doubled the time spent restyling)

### Fixes
- The hotkey on the Move & resize button (and other main buttons) was a dark box on the red button; it is now in the button's text colour on a light tint

## 1.3.7

### New features
- A modern look for the Relative and Standings (Layout → Look; Classic keeps the old one): positions, numbers, gaps and lap times in Barlow Condensed (or your overlay font), car numbers in slanted chips of their class colour, slanted session badge and PIT tags, class headers as a slanted class chip with a fading line, and your row marked down its left edge in your highlight colour. The iRating, licence and class chips are slanted too; the iRating still fills with the licence colour when the licence column is hidden
- Relative, modern look: a gap bar fills in behind a car as it gets closer to you and turns the "close" colour within a second (or the gap you set), with the gap itself in that colour; an arrow by the gap shows whether it is moving your way (green) or not (red). The bar, its range and both colours are settings (Colours)
- Standings, modern look: a line joins cars within a second (or the gap you set) of the car ahead, with their interval in the same colour (its colour is a setting), and the top three positions in gold, silver and bronze
- Relative and Standings: set the driver column's width (Column style → Set the driver column width); longer names are cut short with …
- Shift lights overlay: the car's rev lights on their own, with the RPM if you want it (under, above or beside the lights). The same lights as in Driver inputs, with all their settings: the car's own pattern and per-gear shift points where known, the light order, number, size, flash speed and colours; the RPM turns the "shift now" colour at the car's shift point
- Choose the monitor your overlays are on: the screen map on the Overlays page lists your monitors with their resolution (and scaling, e.g. Monitor 2 · 1920 × 1080 · 125%); picking one moves every overlay there, each at the same place on the screen and kept inside it. The map shows the monitor they are on
- Little moments in the settings window, each played once when it happens and then still (nothing loops): the lights go out as the game connects; a red-and-white kerb runs in under the page title and the page settles in when you open it; the page you are on lights three sector lights; switches sweep their colour across and light an LED in the knob; the line under the open tab glides to the one you pick; hovering an overlay draws a racing line under it and lights it on the screen map; General's readings roll to their new values like a timing board; a pit board saying SAVED slides in when a change is saved. Slanted main buttons and tags, and a cut corner and accent mark on the panels. Turn them off under General → Display → Animations in this window (Windows' "Animation effects: off" turns them off too)
- A new look for the settings window: the app's own Inter for everything you read and Barlow Condensed for titles, labels and readings, flatter panels, one accent colour and cyan for what is live. Colours still follow your theme
- Sidebar: a connection card at the top (the game, the session type, track, cars and time left, and the game switch), the pages in groups (On screen, Race weekend, App), the version under the name, and a switch at the bottom to show or hide every overlay, with its hotkey. The window opens on Overlays
- Every page has a header: its title, with its main buttons on the right (on Overlays: Layouts, Move & share and Move & resize with its hotkey; on General: Open logs and Back up settings)
- Overlays page: a small map of your screen shows where every overlay sits (click one to open it; switched-off ones are dashed); each overlay in the list shows its type and size and why it may not be on screen (Hidden in pits, In the car only, Races only); the editor shows the overlay's name with what it does under it, the settings search at the end of the tab row, and values in fixed boxes. "When it shows" is one choice (Show: whenever the game is running, not on pit road, only in the car, in the car not on pit road, or always) instead of three switches; it sets the same settings, so overlays behave as before. Showing and hiding every overlay is the switch at the foot of the sidebar
- General page: live readings across the top (app CPU, memory, data out, engine time, each with its last minute as a small graph), then Display and Server on the left and the overlay update rate (three cards to pick from), Licence and Updates on the right; everything else (game, hotkeys, VR, monitoring, logs, corners, backup) below
- The settings window goes to the tray when you drive out of the pits (General → Performance, on by default), so it uses no CPU or memory while you race; your overlays keep running. Opened again from the tray while on track, it stays open until you next leave the pits
- Closing the settings window during a session asks whether to minimise it to the tray instead, as that keeps your overlays and frees what the window uses

### Performance
- The settings window uses far less CPU while open: a pulsing "connected" dot redrew the window 60 times a second, and the General page received your car's data 60 times a second for one button. Measured with a game connected: about 15% of a CPU core down to 1.4% on most pages, General 19% to about 3.5%, Overlays 22% to 12–14% (its live preview). The time left in the sidebar changes once a minute, so it does not redraw the window every second. Blur and gradient effects are gone too
- Minimising to the tray now closes the settings window rather than hiding it: hidden, it still used about 5% of a core and 180 MB of memory; closed, nothing

### Fixes
- The D and W in the tyre circles (Relative and Standings) sit in the middle of the circle with every overlay font; they were a little high or low depending on the font
- The Ray FF1600's class showed an F1 badge in the Standings: its class is called "F1.6" in the car data, and "F1" matched. It now shows the FF1600 badge (so does a class named "Formula 1600")
- Le Mans Ultimate private qualifying: the other drivers (each in their own session) are now also left off the track map, the spotter and the battle box, as they already were off the relative and radar (General → Private qualifying: hide other cars)

## 1.3.6

### New features
- Driver inputs: corner speeds (Corner speeds, off by default). Above your speed, in the brake colour, the top speed going into the last corner, stored when you brake; below it, in the throttle colour, the lowest speed through the corner, stored when you are back on the throttle (dimmed while you are still in the next corner). The brake and throttle levels that mark a corner can be set (10% and 20%); light touches of the brake, the pit lane and spins are not counted
- Standings and relative: header text size (50–250%: the title, session badge and details together) and header height (0 fits the text; taller keeps the text centred), under Header

## 1.3.5

### Improvements
- The log file (General → Open logs) now says what the overlays are doing, so a report can be looked into from the log alone: at start-up which overlays are on and any setting that hides one (hidden in pits, only in the car, races only…) or hotkey for one; each overlay being hidden and why (on pit road, not in the car, no game connected, all overlays hidden…) and shown again; overlays switched on or off, added or removed, and layouts applied; hotkeys pressed; edit mode; overlay windows opened and closed; an overlay's window fitted to almost nothing; an overlay losing its connection to the app; and every 10 minutes a line saying the app is still running and what it is reading

### Fixes
- Standings: no longer shrink to almost nothing (looking as if switched off) for the moments there are no cars to show, such as between sessions or while the game reloads; the last table stays until there are cars again
- Standings, My class only: keeps showing your class while you are in the garage or the menus, or after a reset (it emptied when your car was not on track)

## 1.3.4

### Improvements
- Licence (General page): your key is hidden once it's added, so it isn't on screen while you stream. Press Show to see it

### Fixes
- Licence: Get your key opens the Apex Labs website's key page again (it went to an old address that no longer works)

## 1.3.3

### New features
- Estimated laps for timed sessions: a new header detail, Estimated laps left (~31.7 laps: the time left over your lap time), for the session details, standings and relative, and in the fuel calculator (Extra details) and the virtual energy calculator. Its decimals can be set (1 by default)
- Lap count in timed races can show the estimated length: Lap 12/~43 (the lap the clock runs out on, which the leader finishes), as well as Lap 12/∞ and Lap 12
- Flags & penalties (standings and relative) are set with two choices: where to show them (next to the driver's name, outside the overlay as an extension of it on the standings, or not at all) and on which side (left or right of the name, or the overlay's left or right side). These replace the separate "Flags next to drivers" and "Penalties next to drivers" switches
- Theme: session badge colour (RACE, QUALIFYING, PRACTICE in headers): the accent colour for every session (default), or a colour per session type as before

### Fixes
- Track map: the built-in iRacing map no longer disappears after leaving a session, iRacing disconnecting or joining a new session (a test session, say) at the same track, which asked for a clean lap instead. It is kept for the track and sent again whenever you join it
- Standings: flags and penalties set to show outside the table no longer disappear when the "next to drivers" switches were off
- The PRACTICE and QUALIFYING badges in headers no longer take the Gaining / good and Fastest lap colours unless you choose that (they follow the accent colour)
- Headers no longer show "Lap -1" when the leading car is in the garage
- Monitoring: on Windows, the first CPU reading after opening the Monitoring section or the performance widget showed the app at close to 0%. It now shows the real use from the first reading

## 1.3.2

### New features
- Licence (General page): paste your free Apex Labs key, which you get by signing in with Discord on the Apex Labs website. It is optional during early access, nothing is locked without one, and the same key works in Apex Commentator

## 1.3.1

### New features
- Endurance races: the incident tracker keeps your last 300 incidents instead of 100 once a race over 5 hours is detected (by its time limit, or its laps at your lap time). Only the newest 30 (the most it shows) are sent to the overlay, so its updates stay small
- Fuel calculator: the number of laps in the average can be set in the calculator's own settings (the same setting as Strategy → Fuel), and the average is labelled AVG / LAP
- Fuel calculator: the compact layout can show details under the bar too (Details under the bar): the average, laps to go and the extra details you pick
- Standings: flags and penalties (meatball, black flag, SLOW, DT, S&G…) can sit outside the table, just past the right edge of each driver's row, instead of next to the name (Columns → Flags & penalties)
- Pit time column for the relative and standings (PIT TIME): how long each car stood still in its pit box at its last stop, counting live (in the pit colour) during a stop. A penalty stop is told apart from a service stop: a stop-and-go served under the black flag shows as "S&G 30.0" in red; a stop for fuel, tyres or repairs shows its time; a stop that was both shows the service time marked +P (hover for both). iRacing and Le Mans Ultimate, every car
- Fuel calculator: estimates the race before it starts. In practice, qualifying and on the grid it shows how many laps the race will be (RACE LAPS: exact for a race set by laps, ~ for a timed race, worked out from its time and your lap time), the fuel for all of it with your safety margin (RACE FUEL; hover for what to start with) and the stops it takes starting with a full tank (RACE STOPS). On the grid the whole calculator (laps to go, fuel at the finish, pit window) plans the race. iRacing gives the race length from the start of the event; Le Mans Ultimate only once in the race session, so for its practice and qualifying you can set the race length (minutes or laps) in the calculator's settings or Strategy → Fuel
- Track map: rotation and mirroring (horizontal and vertical) are saved for each track. Set them while in a session and they apply to that track only; other tracks keep their own, or the overlay's setting if they have none

### Improvements
- iRacing track maps show the moment a track loads, with no lap to drive: the official outline of every track layout (421 of them) is now built into the app, so nothing has to be downloaded first (the app still fetches the latest map in the background when it can)
- Le Mans Ultimate track maps draw much sooner when you are alone on track (practice, testing): a fast car's samples are joined up instead of leaving gaps, so one clean lap is enough rather than several
- SOF in overlay headers shows in full below 10,000 (1539, 8320) and as 10.0k, 12.3k… from 10,000 up, the same in every header

### Fixes
- Car settings: brake bias no longer shows as 5500% on cars where iRacing gives it in hundredths (the Audi RS 3 LMS TCR and others); it is read in the right scale on every car and follows the changes you make in the car. On cars whose live value is not a percentage, the bias from the car's setup is shown instead. Values outside a believable range are hidden rather than shown wrong
- Text badges next to drivers have their words centred: SLOW, penalties (DT, S&G 40s, +5s…) and PIT
- The flag markers without text (meatball, black, blue, yellow, chequered, disqualified) are bigger and redrawn as sharp vector flags with a light sheen, crisp at any size or screen scaling
- Flags next to a driver's name no longer have the left edge of their outline cut off
- Le Mans Ultimate no longer flickers between "connected" and "waiting" when it is not running but another program (SimHub, CrewChief…) keeps its shared memory open: the app checks the game is really running before connecting, and waits a few seconds before looking again

## 1.3.0

### New features
- Game switch at the bottom of the sidebar, beside the Apex Labs logo: Auto (whichever game is running), iRacing or LMU, one click from any page. In Auto, the game being read is marked in green
- A fresh install starts with no overlays: add the ones you want (Add overlay) or start from a shared layout (Layout Market). Each new overlay goes to its usual spot on your screen rather than all in one place. Updating keeps your overlays as they are
- Layout Market (new page): browse overlay layouts other drivers have shared, each with a preview image of how it looks in the game (rendered with sample data once it is shared), the game it is for (iRacing or Le Mans Ultimate), the screen resolution it was made at, the sessions it is used in (all, or e.g. only qualifying) and who made it. Filter by game and resolution, search, and add one to your layouts in a click: its positions are fitted to your screen, its sessions are kept where your own layouts do not already have them, and your theme stays as it is. Share your own with Share my layout: choose the game, give your name, and the app makes a share code and opens the share form on GitHub (a free GitHub account), where it is checked and listed automatically within a few minutes. Layouts made with a newer version of the app are shown but wait for you to update
- Overlays fit their windows to their content by default, so nothing is cut off: the height follows what is shown, and the window widens when something would not fit (back to your width when it does). Not for the overlays drawn to fill their window (track map, radar, inputs, input graph, flags, spotter)
- Text size for each overlay (its General tab): 0 keeps the theme's size
- Fuel / energy left column for the relative and standings (FUEL): each car’s fuel, or virtual energy for the cars that race on it (shown in blue), as a % in Le Mans Ultimate; your own fuel in iRacing. Red when under 10%
- Tyre data: show one temperature per tyre instead of three: the core (carcass) temperature in Le Mans Ultimate, or the average across the tyre

### Improvements
- An overlay you delete stays deleted: updates no longer bring back overlay types you removed (each new type is offered once, switched off)
- A tidier, simpler settings window: overlay descriptions, settings and help texts are shorter and plainer (long explanations cut down, e.g. on General, VR, Race control and the League builder); every page and every overlay's settings checked at three window sizes so no text overlaps or is cut off
- Smaller settings windows: the sidebar and overlay list take less room, long overlay names wrap instead of being cut, the column picker shows each column's short code as a tag, and drop-down menus open towards the side with room
- The live preview in each overlay's settings fits the editor and shows the whole overlay (it was cut off on the right in narrower windows)
- Escape closes dialogs (Add overlay, Share a layout, previews)
- Sliders line up, with or without a unit next to them
- The overlay's name at the top of its settings no longer looks greyed out
- "Fit the window to the content" is no longer offered on overlays drawn to fill their window (track map, radar, inputs…), where it did nothing

### Fixes
- Radar: no longer stops drawing with an error ("radius is negative") when its window is very small or hidden for a moment
- The log no longer records the browser's harmless "ResizeObserver loop" note from the track map as an error
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
