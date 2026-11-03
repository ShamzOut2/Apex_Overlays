// Payload shapes of the topics streamed by the core (see crates/core/src/topics.rs).

export interface ClassInfo {
  id: string;
  name: string;
  color: string;
  count: number;
  sof: number;
}

export interface Session {
  connected: boolean;
  source: string;
  units: 'metric' | 'imperial';
  trackId: number;
  trackName: string;
  trackConfig: string;
  trackLengthKm: number;
  trackCity: string;
  trackCountry: string;
  sessionType: string;
  sessionName: string;
  sessionState: string;
  isRace: boolean;
  timeRemain: number | null;
  timeTotal: number | null;
  lapsRemain: number | null;
  lapsTotal: number | null;
  leaderLap: number;
  timeOfDay: number;
  trackTemp: number;
  trackTempDelta: number;
  airTemp: number;
  wetness: number;
  wetnessLabel: string;
  precipitation: number;
  declaredWet: boolean;
  weatherTrend: 'drying' | 'stable' | 'wetting';
  windSpeed: number;
  windDir: number;
  humidity: number;
  fog: number;
  skies: string;
  airPressure: number;
  flags: string[];
  sof: number;
  classes: ClassInfo[];
  incidents: number;
  incidentLimit: number | null;
  numCars: number;
  league: string | null;
  leagueId: number;
  official: boolean;
  subsessionId: number;
}

export interface Player {
  onTrack: boolean;
  speed: number;
  rpm: number;
  gear: number;
  throttle: number;
  brake: number;
  clutch: number;
  steering: number;
  steeringMax: number;
  abs: boolean;
  shiftRpm: number;
  redline: number;
  lap: number;
  lapPct: number;
  lapTime: number;
  lastLap: number | null;
  bestLap: number | null;
  deltaBest: number | null;
  fuel: number;
  onPitRoad: boolean;
  position: number;
  classPosition: number;
  incidents: number;
  latG: number;
  longG: number;
  /** Compass heading of the followed car (you, or the camera car when spectating), degrees; null if unknown. */
  heading: number | null;
}

export interface CarDot {
  i: number;
  p: number;
  n: string;
  c: string;
  cls: string;
  pos: number;
  cpos: number;
  pit: boolean;
  off: boolean;
}

export interface Cars {
  focus: number | null;
  player: number | null;
  sectors: number[];
  cars: CarDot[];
  sectorTimes: SectorTimes;
}

export interface SectorTime {
  lap: number;
  time: number;
  /** Against the previous best (negative = new best); null for the first time. */
  delta: number | null;
}

export interface SectorTimes {
  starts: number[];
  last: (SectorTime | null)[];
  lap: number;
  lapEndAgo: number | null;
}

export interface Driver {
  idx: number;
  number: string;
  name: string;
  abbrev: string;
  team: string;
  car: string;
  classId: string;
  className: string;
  classColor: string;
  userId: number;
  carId: number;
  irating: number;
  license: string;
  licenseColor: string;
  club: string;
  /** Flair (country flag choice) exactly as iRacing sent it. */
  flair: string;
  /** ISO country code ('' if unknown); may be 'GB-SCT' style for UK nations. */
  country: string;
  make: string;
  isPlayer: boolean;
  isFocus: boolean;
}

/** Per-car extras shared by relative and standings rows. */
export interface Extras {
  lap: number;
  stintState: 'pit' | 'out' | 'lap';
  stintLap: number | null;
  bestLap: number | null;
  lastLap: number | null;
  /** Best lap is the fastest of the class. */
  classBest: boolean;
  /** Last lap was the driver's personal best. */
  lastIsPb: boolean;
  /** Has taken the chequered flag. */
  finished: boolean;
  lapsLed: number;
  /** Flags for this car: yellow (caused one), black, disqualify, repair (meatball), slow (slow-down penalty), blue. */
  flags: string[];
  /** "DT", "S&G 40s", "+5s"…, and whether it was just served. */
  penalty: string | null;
  penaltyServed: boolean;
  incidents: number | null;
  irDelta: number | null;
  srDelta: number | null;
  points: number | null;
  seasonPoints: number | null;
  /** Serving a slow-down penalty: for how long, and the time given up so far (estimated). */
  slowdown: { seconds: number; lost: number } | null;
}

export interface RelRow extends Driver, Extras {
  /** On-track gap to the focus car (positive = ahead). */
  gap: number;
  gapLeader: number | null;
  gapLeaderLaps: number;
  interval: number | null;
  intervalLaps: number;
  position: number;
  classPosition: number;
  lapDiff: number;
  onPitRoad: boolean;
  offTrack: boolean;
  tireWet: boolean | null;
}

export interface Relative {
  rows: RelRow[];
}

export interface StandRow extends Driver, Extras {
  position: number;
  classPosition: number;
  gap: number | null;
  gapLaps: number;
  interval: number | null;
  intervalLaps: number;
  /** Gap and interval overall (to P1 and the car ahead), for a single overall list. */
  gapOverall: number | null;
  gapOverallLaps: number;
  intervalOverall: number | null;
  intervalOverallLaps: number;
  isClassBest: boolean;
  laps: number;
  onPitRoad: boolean;
  inWorld: boolean;
  pitStops: number;
  stintLaps: number;
  positionsGained: number | null;
  tireWet: boolean | null;
  progress: number;
}

export interface StandClass extends ClassInfo {
  bestLap: number | null;
  rows: StandRow[];
}

export interface Standings {
  isRace: boolean;
  focusClass: string | null;
  classes: StandClass[];
}

export interface Fuel {
  level: number;
  capacity: number;
  pct: number;
  perLapAvg: number | null;
  perLapLast: number | null;
  perLapMax: number | null;
  lapsSampled: number;
  lapsInTank: number | null;
  usePerHour: number;
  lapsRemaining: number | null;
  lapTime: number | null;
  fuelToFinish: number | null;
  fuelToAdd: number | null;
  stops: number | null;
  lapsPerTank: number | null;
  targetPerLap: number | null;
  savePerLap: number | null;
  windowOpensLap: number | null;
  windowClosesLap: number | null;
  marginLaps: number;
  history: number[];
  /** What this car used at this track in earlier sessions. */
  expected: { car: string; track: string; perLap: number; max: number; lapTime: number; laps: number; updated: number } | null;
  estimateFrom: 'session' | 'history' | 'none';
}

export interface BattleCar extends Driver {
  position: number;
  classPosition: number;
  gap: number | null;
  gapLaps: number;
  trend: number | null;
  lapsToCatch: number | null;
  lastLap: number | null;
  bestLap: number | null;
  onPitRoad: boolean;
  tireWet: boolean | null;
  inBattle: boolean;
}

export interface BattlePair {
  ahead: BattleCar | null;
  behind: BattleCar | null;
}

export interface Battle {
  /** Same class position battle, overall running order, nearest cars on track. */
  class: BattlePair;
  overall: BattlePair;
  track: BattlePair;
  myLastLap: number | null;
  myBestLap: number | null;
  maxGap: number;
}

export type TyreRecommendation = 'stayOut' | 'considerWets' | 'boxForWets' | 'considerSlicks' | 'boxForSlicks' | 'noWets';

export interface Tyres {
  current: 'dry' | 'wet' | 'unknown';
  carHasWets: boolean;
  recommendation: TyreRecommendation;
  confidence: number;
  wetsAllowed: boolean;
  wetness: number;
  wetnessLabel: string;
  precipitation: number;
  trend: 'drying' | 'stable' | 'wetting';
  fieldOnWets: number;
  fieldTotal: number;
  wetPace: number | null;
  dryPace: number | null;
  reasons: string[];
}

export interface TrackMap {
  trackId: number;
  trackName: string;
  points: [number, number][] | null;
  source: string | null;
  recording: boolean;
  progress: number;
  sectors: number[];
  corners: Corner[];
  cornersSource: 'custom' | 'lovely' | 'mock' | 'none';
  /** Named straights from Lovely Track Data. */
  straights: Corner[];
  pitEntry: number | null;
  pitExit: number | null;
}

/** Shift-light data of the player's car. */
export interface CarInfo {
  carPath: string;
  carName: string;
  source: 'lovely' | 'iracing';
  lovely: {
    carName: string;
    carClass: string;
    leds: number;
    blinkMs: number;
    /** Index 0 = redline colour ('' = none), then one per light. */
    colors: string[];
    /** Gear ('R', 'N', '1'…) → [redline RPM, light 1 RPM, …]. */
    rpm: Record<string, number[]>;
  } | null;
  slFirst: number;
  slShift: number;
  slLast: number;
  slBlink: number;
  redline: number;
}

export interface Corner {
  name: string;
  number: number;
  start: number;
  end: number;
}

export interface G61Lap {
  driver: string;
  lapTime: number;
  date: string;
  isMe: boolean;
}

export interface G61Reference {
  driver: string;
  lapTime: number;
  /** personal, team; for the delta bar also teams, team:<slug>, world, session, allTime. */
  source: string;
  /** Seconds into the lap at equal steps of lap distance (first = start, last = finish). */
  times: number[];
}

export interface Garage61 {
  status: 'disabled' | 'noToken' | 'waiting' | 'loading' | 'ok' | 'unmapped' | 'error';
  message: string;
  user: string;
  car: string;
  track: string;
  personalBest: G61Lap | null;
  teamBest: G61Lap | null;
  leaderboard: G61Lap[];
  /** Which laps were searched, e.g. "Current season · 25–35 °C · dry". */
  filters: string;
  reference: G61Reference | null;
  referenceNote: string;
  teams: { slug: string; name: string }[];
  /** Reference laps the delta bars asked for: personal, teams, team:<slug>, world. */
  references: Record<string, G61Reference>;
  referenceNotes: Record<string, string>;
  /** Fastest lap you can see on Garage 61, telemetry or not. */
  worldBest: G61Lap | null;
}

/** Your own reference laps, recorded by the app (`delta` topic). */
export interface DeltaRefs {
  session: G61Reference | null;
  allTime: G61Reference | null;
}

export interface Status {
  connected: boolean;
  source: string;
  editMode: boolean;
  visible: boolean;
  version: string;
}

export interface OverlayInstance {
  id: string;
  kind: string;
  name: string;
  enabled: boolean;
  x: number;
  y: number;
  width: number;
  height: number;
  settings: Record<string, unknown>;
}

export interface ClassRules {
  carIds: number[];
  carNames: string[];
  carNumbers: string[];
  customerIds: number[];
  teamNames: string[];
  driverNames: string[];
  iracingClassIds: number[];
}

export interface LeagueClass {
  id: string;
  name: string;
  color: string;
  rules: ClassRules;
}

export interface DriverOverride {
  customerId: number;
  /** iRacing name, pulled from a session (reference only). */
  name: string;
  /** League car number; replaces the sim number in overlays when set. */
  carNumber: string;
  /** League class id this driver races in (overrides class rules). */
  classId: string;
  displayName: string;
  teamName: string;
  country: string;
  hidden: boolean;
}

export interface PointsConfig {
  table: number[];
  defaultPoints: number;
  basis: 'class' | 'overall';
  fastestLapBonus: number;
  poleBonus: number;
  lapLedBonus: number;
  mostLapsLedBonus: number;
  finishBonus: number;
  systemName: string;
  description: string;
}

export interface SeasonEntry {
  customerId: number;
  name: string;
  points: number;
  position: number;
}

export interface League {
  id: string;
  name: string;
  classes: LeagueClass[];
  driverOverrides: DriverOverride[];
  fallback: 'iracing' | 'other';
  iracingLeagueId: number;
  seasonId: number;
  seasonName: string;
  points: PointsConfig;
  seasonPoints: SeasonEntry[];
  importedAt: string;
  /** Leave out class tags in names: "(P) Max Power" shows as "Max Power". */
  stripNameClass: boolean;
}

export interface LeagueImport {
  status: 'working' | 'ok' | 'error';
  message: string;
  leagueId: number;
  seasons: { seasonId: number; name: string; active: boolean }[];
  id: string | null;
}

export interface SavedTheme {
  name: string;
  theme: Record<string, unknown>;
}

export interface Config {
  version: number;
  server: { port: number; lanAccess: boolean };
  units: 'auto' | 'metric' | 'imperial';
  theme: Record<string, unknown>;
  overlays: OverlayInstance[];
  /** VR (desktop app with SteamVR). */
  vr: { enabled: boolean; mode: 'steamvr' | 'openxr'; distance: number; width: number; height: number; offset: number };
  hotkeys: {
    toggleEdit: string;
    toggleVisibility: string;
    /** Switch to the next saved layout. */
    nextLayout: string;
    /** Layout id → hotkey. */
    layouts: Record<string, string>;
    /** Overlay id → hotkey that shows / hides it. */
    overlays: Record<string, string>;
    /** Recentre VR (SteamVR's seated position). */
    vrRecenter: string;
  };
  /** Saved arrangements of overlays, and the one in use. */
  layouts: SavedLayout[];
  activeLayout: string;
  fuel: { avgLaps: number; marginLaps: number; includeCautionLaps: boolean; includePitLaps: boolean };
  tyres: { wetThreshold: number; dryThreshold: number; useField: boolean; useCrossover: boolean; crossoverMargin: number };
  battle: { maxGap: number };
  leagues: League[];
  activeLeague: string | null;
  /** 'auto': the league matching the session's iRacing league ID (none otherwise); 'always': activeLeague. */
  leagueMode: 'auto' | 'always';
  garage61: {
    enabled: boolean;
    token: string;
    includeTeam: boolean;
    refreshMinutes: number;
    period: string;
    reference: 'personal' | 'team' | 'off';
    matchConditions: boolean;
    tempRange: number;
  };
  savedThemes: SavedTheme[];
  trackCorners: Record<string, Corner[]>;
  iracingApi: { clientId: string; clientSecretMasked: string; username: string; passwordMasked: string };
  ratingsOfficialOnly: boolean;
  autoCheckUpdates: boolean;
  /** "Remind me later" / "skip this version" from the update pop-up (until: unix seconds, null = skip). */
  updatePostpone: { version: string; until: number | null } | null;
  penalties: Penalty[];
  moveAllTogether: boolean;
  shortClassNames: boolean;
  /** Track map outline: iRacing's official map when available, or the one recorded from your driving. */
  trackOutline: 'iracing' | 'recorded';
  /** 'balanced': timing tables a few times a second; 'full': everything 60 times a second. */
  updateRate: 'fast' | 'balanced' | 'full';
  /** Closing the settings window: ask, close everything, or keep the overlays running. */
  closeAction: 'ask' | 'quit' | 'keep';
}

export interface Penalty {
  id: string;
  customerId: number;
  name: string;
  kind: 'dt' | 'sg' | 'time' | 'other';
  seconds: number;
  note: string;
}

export interface UpdateState {
  status: 'unsupported' | 'idle' | 'checking' | 'none' | 'available' | 'downloading' | 'installing' | 'error';
  current?: string;
  version?: string;
  notes?: string;
  progress?: number;
  message?: string;
}

export interface Spotter {
  /** 'left', 'right', 'both', 'left2' (two cars left), 'right2', or '' when clear. */
  state: '' | 'left' | 'right' | 'both' | 'left2' | 'right2';
  /** Metres each car alongside is ahead of you (negative = behind), when known. */
  left: number[];
  right: number[];
}

export interface IncidentCar {
  idx: number;
  name: string;
  number: string;
  /** Metres ahead of you (negative = behind) when it happened. */
  dist: number;
  /** Their own incident count rose by 4 or more just after. */
  confirmed: boolean;
}

export interface Incident {
  points: number;
  kind: 'offTrack' | 'lossOfControl' | 'objectContact' | 'carContact' | 'heavyContact';
  label: string;
  total: number;
  lap: number;
  sessionNum: number;
  sessionName: string;
  sessionTime: number;
  involved: IncidentCar[];
}

export interface Incidents {
  total: number;
  limit: number | null;
  /** A replay is playing: the replay buttons work. */
  canReplay: boolean;
  sessionNum: number;
  list: Incident[];
}

export interface Pit {
  phase: '' | 'entry' | 'lane' | 'box' | 'exit' | 'out';
  /** Metres to the pit entry / exit line. */
  toEntry: number | null;
  toExit: number | null;
  /** m/s */
  speed: number;
  /** Pit speed limit, m/s (0 = unknown). */
  limit: number;
  limiter: boolean;
  service: '' | 'wait' | 'go';
  repair: number;
  optRepair: number;
  /** Seconds since crossing the pit exit line. */
  sinceExit: number | null;
}

/** What Apex Overlays itself uses (the `monitor` topic, once a second). */
export interface Monitor {
  cpu: number;
  cpuApp: number;
  cpuWindows: number;
  systemCpu: number;
  memoryMb: number;
  memoryAppMb: number;
  memoryWindowsMb: number;
  processes: number;
  sentKbps: number;
  messagesPerSec: number;
  clients: number;
  telemetryHz: number;
  tickUs: number;
  engineCpu: number;
  uptime: number;
  cores: number;
}

export interface SavedLayout {
  id: string;
  name: string;
  overlays: OverlayInstance[];
  /** Session types it is switched to automatically: practice, qualifying, warmup, race. */
  sessions: string[];
}

/** A car ahead causing a yellow or carrying the meatball (see crates/core/src/safety.rs). */
export interface Hazard extends Driver, Extras {
  flag: 'yellow' | 'repair';
  reason: 'stopped' | 'slow' | 'offTrack' | 'rejoining' | 'damaged';
  /** Seconds and metres ahead of you. */
  gap: number;
  distance: number;
  /** m/s */
  speed: number;
  position: number;
  classPosition: number;
  tireWet: boolean | null;
}

/** A car of a faster class behind you. */
export interface Approach extends Driver, Extras {
  /** Seconds and metres behind you. */
  gap: number;
  distance: number;
  /** Seconds a lap faster than you. */
  fasterBy: number;
  /** Seconds until they reach you. */
  catchIn: number | null;
  speed: number;
  position: number;
  classPosition: number;
  tireWet: boolean | null;
}

/** A car ahead serving a slow-down penalty: a chance to pass it. */
export interface Overtake extends Driver, Extras {
  /** Seconds and metres ahead of you. */
  gap: number;
  distance: number;
  /** Their speed and yours, m/s. */
  speed: number;
  mySpeed: number;
  position: number;
  classPosition: number;
  tireWet: boolean | null;
}

export interface Safety {
  hazards: Hazard[];
  approaching: Approach[];
  overtakes: Overtake[];
}

/** VR status from the desktop app (`vr` topic). */
export interface VrState {
  status: 'off' | 'waiting' | 'on' | 'error';
  message: string;
  overlays: number;
}

export interface TopicMap {
  session: Session;
  player: Player;
  cars: Cars;
  relative: Relative;
  standings: Standings;
  fuel: Fuel;
  battle: Battle;
  tyres: Tyres;
  trackmap: TrackMap;
  car: CarInfo;
  spotter: Spotter;
  incidents: Incidents;
  pit: Pit;
  safety: Safety;
  delta: DeltaRefs;
  monitor: Monitor;
  vr: VrState;
  garage61: Garage61;
  status: Status;
  config: Config;
  leagueImport: LeagueImport;
  update: UpdateState;
}

export type TopicName = keyof TopicMap;
