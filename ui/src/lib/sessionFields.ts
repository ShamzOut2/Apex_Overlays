// Session detail fields shared by the Session info overlay and the
// customisable headers of the relative and standings.

import * as f from './format';
import { columns, type ColumnDef } from './settings';
import type { Session } from './types';

export const SESSION_FIELDS: ColumnDef[] = [
  { id: 'sessionType', label: 'Session type (Practice, Qualifying, Race…)' },
  { id: 'session', label: 'Session name' },
  { id: 'track', label: 'Track' },
  { id: 'time', label: 'Time / laps left' },
  { id: 'lap', label: 'Leader lap' },
  { id: 'clock', label: 'Time of day (in sim)' },
  { id: 'trackTemp', label: 'Track temperature' },
  { id: 'airTemp', label: 'Air temperature' },
  { id: 'wetness', label: 'Track wetness' },
  { id: 'trackState', label: 'Track state (rubber: clean, low usage…)' },
  { id: 'rain', label: 'Rain' },
  { id: 'declared', label: 'Declared wet' },
  { id: 'skies', label: 'Sky' },
  { id: 'wind', label: 'Wind' },
  { id: 'humidity', label: 'Humidity' },
  { id: 'sof', label: 'Strength of field' },
  { id: 'cars', label: 'Cars' },
  { id: 'incidents', label: 'Incidents' },
  { id: 'flags', label: 'Flags' },
  { id: 'league', label: 'League name' },
];

/** Friendly session type: iRacing reports e.g. "Lone Qualify" or "Offline Testing". */
export function sessionType(d: Session): string {
  const t = (d.sessionType || '').toLowerCase();
  if (t.includes('qualify')) return 'Qualifying';
  if (t.includes('warm')) return 'Warmup';
  if (t.includes('test')) return 'Testing';
  if (t.includes('heat')) return 'Heat';
  if (t.includes('race')) return 'Race';
  if (t.includes('practice')) return 'Practice';
  return d.sessionType || '';
}

export const FIELD_LABELS: Record<string, string> = {
  sessionType: 'SESSION',
  track: 'TRACK',
  session: 'SESSION',
  time: 'REMAINING',
  lap: 'LAP',
  clock: 'TIME',
  trackTemp: 'TRACK',
  airTemp: 'AIR',
  wetness: 'SURFACE',
  trackState: 'TRACK STATE',
  rain: 'RAIN',
  declared: 'DECLARED WET',
  skies: 'SKY',
  wind: 'WIND',
  humidity: 'HUMIDITY',
  sof: 'SOF',
  cars: 'CARS',
  incidents: 'INC',
  flags: 'FLAG',
  league: 'LEAGUE',
};

/** Columns-type setting for picking and ordering header fields. */
export const headerFields = (on: string[]) => columns(SESSION_FIELDS, on);

export interface FieldOpts {
  clock24?: boolean;
  tempTrend?: boolean;
  /** Short forms suit compact headers ("42:10", "SOF 2.3k"). */
  compact?: boolean;
  /** Timed races: show the lap as "12/∞" or just "12" (iRacing may report a lap cap like 50). */
  timedLaps?: 'infinity' | 'count';
}

/** Display value of a field; '' when there is nothing to show. */
export function fieldValue(id: string, d: Session, u: f.Units, o: FieldOpts = {}): string {
  switch (id) {
    case 'sessionType':
      return sessionType(d);
    case 'session':
      return d.sessionName || d.sessionType || '';
    case 'track':
      return d.trackName + (d.trackConfig && !o.compact ? ` – ${d.trackConfig}` : '');
    case 'time':
      return d.timeRemain != null ? f.duration(d.timeRemain) : d.lapsRemain != null ? `${d.lapsRemain} laps` : '';
    case 'lap': {
      if (!d.leaderLap) return '';
      // In a timed race the lap count is open-ended even if iRacing reports a lap cap.
      const total = d.timeTotal != null ? ((o.timedLaps ?? 'infinity') === 'infinity' ? '/∞' : '') : d.lapsTotal ? `/${d.lapsTotal}` : '';
      return `${o.compact ? 'Lap ' : ''}${d.leaderLap}${total}`;
    }
    case 'clock':
      return f.clock(d.timeOfDay, o.clock24 ?? true);
    case 'trackTemp': {
      const trend = o.tempTrend !== false && Math.abs(d.trackTempDelta) >= 0.3 ? (d.trackTempDelta > 0 ? ' ▲' : ' ▼') : '';
      return `${o.compact ? 'Track ' : ''}${f.temp(d.trackTemp, u)}${trend}`;
    }
    case 'airTemp':
      return `${o.compact ? 'Air ' : ''}${f.temp(d.airTemp, u)}`;
    case 'wetness':
      return d.wetnessLabel;
    case 'trackState':
      return trackState(d.trackState, o.compact);
    case 'rain':
      return d.precipitation > 0 ? `${o.compact ? 'Rain ' : ''}${Math.round(d.precipitation * 100)}%` : o.compact ? '' : 'None';
    case 'declared':
      return o.compact ? (d.declaredWet ? 'Declared wet' : '') : d.declaredWet ? 'Yes' : 'No';
    case 'skies':
      return d.skies;
    case 'wind':
      return `${o.compact ? 'Wind ' : ''}${Math.round(f.speed(d.windSpeed, u))} ${f.speedUnit(u)} ${f.compass(d.windDir)}`;
    case 'humidity':
      return `${o.compact ? 'Hum ' : ''}${Math.round(d.humidity * 100)}%`;
    case 'sof':
      return d.sof ? (o.compact ? `SOF ${f.iRating(d.sof)}` : String(d.sof)) : '';
    case 'cars':
      return `${d.numCars}${o.compact ? ' cars' : ''}`;
    case 'incidents':
      return d.incidentLimit ? `${d.incidents}/${d.incidentLimit}x` : `${d.incidents}x`;
    case 'flags':
      return d.flags.map((x) => x.replace(/([A-Z])/g, ' $1').toLowerCase()).join(', ');
    case 'league':
      return d.league ?? '';
    default:
      return '';
  }
}

/** iRacing's rubber state in sentence case ("Moderately high usage"); compact headers get
 *  a short form ("Rubber: mod. high"). */
export function trackState(raw: string | undefined, compact = false): string {
  const t = (raw ?? '').trim().toLowerCase();
  if (!t) return '';
  if (compact) {
    const short = t.replace('moderately', 'mod.').replace(/\s*usage$/, '');
    return `Rubber: ${short}`;
  }
  return t.charAt(0).toUpperCase() + t.slice(1);
}

/** Data attribute used to colour the flags field. */
export function flagKind(d: Session): string {
  const fl = d.flags;
  if (fl.includes('red')) return 'red';
  if (fl.some((x) => x.startsWith('yellow') || x.startsWith('caution'))) return 'yellow';
  if (fl.includes('blue')) return 'blue';
  if (fl.includes('white')) return 'white';
  if (fl.includes('checkered')) return 'checkered';
  if (fl.includes('green')) return 'green';
  return '';
}
