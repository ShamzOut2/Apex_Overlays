// Session detail fields shared by the Session info overlay and the
// customisable headers of the relative and standings.

import * as f from './format';
import { columnOn, columns, type ColumnDef, type SettingDef } from './settings';
import type { Session } from './types';

export const SESSION_FIELDS: ColumnDef[] = [
  { id: 'sessionType', label: 'Session type (Practice, Qualifying, Race…)' },
  { id: 'session', label: 'Session name' },
  { id: 'track', label: 'Track' },
  { id: 'time', label: 'Time / laps left' },
  { id: 'lap', label: 'Leader lap' },
  { id: 'estLaps', label: 'Estimated laps left (timed sessions)' },
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
  estLaps: 'EST. LAPS',
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

export type TimedLaps = 'infinity' | 'estimate' | 'count';

/** Settings for the lap count in timed sessions and the estimated laps left (headers and the
 *  session overlay). */
export function lapSettings(group: string, when: (v: Record<string, unknown>) => boolean = () => true, fieldsKey = 'headerFields'): SettingDef[] {
  return [
    {
      key: 'timedLaps',
      when,
      label: 'Lap count in timed races',
      type: 'select',
      default: 'infinity',
      options: [
        ['infinity', 'Lap 12/∞'],
        ['estimate', 'Lap 12/~24 (estimated laps)'],
        ['count', 'Lap 12'],
      ],
      group,
    },
    {
      key: 'estDecimals',
      when: (v) => when(v) && columnOn(v, 'estLaps', fieldsKey),
      label: 'Estimated laps: decimals',
      type: 'number',
      default: 1,
      min: 0,
      max: 3,
      group,
      help: 'Estimated laps left: the time left over your lap time, in timed sessions',
    },
  ];
}

export interface FieldOpts {
  clock24?: boolean;
  tempTrend?: boolean;
  /** Short forms suit compact headers ("42:10", "SOF 2.3k"). */
  compact?: boolean;
  /** Timed races: show the lap as "12/∞", "12/~24" (estimated) or just "12" (iRacing may
   *  report a lap cap like 50). */
  timedLaps?: TimedLaps;
  /** Decimals of the estimated laps left. */
  estDecimals?: number;
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
      const timed = o.timedLaps ?? 'infinity';
      const total =
        d.timeTotal != null
          ? timed === 'estimate' && d.estLapsTotal != null
            ? `/~${d.estLapsTotal}`
            : timed === 'count'
              ? ''
              : '/∞'
          : d.lapsTotal
            ? `/${d.lapsTotal}`
            : '';
      return `${o.compact ? 'Lap ' : ''}${d.leaderLap}${total}`;
    }
    case 'estLaps':
      return d.estLapsLeft == null ? '' : `~${d.estLapsLeft.toFixed(o.estDecimals ?? 1)}${o.compact ? ' laps' : ''}`;
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
      return d.sof ? `${o.compact ? 'SOF ' : ''}${f.sof(d.sof)}` : '';
    case 'cars':
      return `${d.numCars}${o.compact ? ' cars' : ''}`;
    case 'incidents':
      return incidentText(d.incidents, d.incidentLimits ?? (d.incidentLimit ? [d.incidentLimit] : []));
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

/** Incident limits still ahead of `now`: the ones already reached drop out. */
export function limitsAhead(now: number, limits: number[]): number[] {
  return limits.filter((l) => l > now);
}

/** "9x / 10x / 20x": incidents now, then each limit still ahead. */
export function incidentText(now: number, limits: number[]): string {
  return [`${now}x`, ...limitsAhead(now, limits).map((l) => `${l}x`)].join(' / ');
}

/** How close `now` is to the next limit, counted from the last one reached (or 0): under half
 *  way 'ok', under three quarters 'warn', else 'bad'; past every limit 'bad'; '' when there are
 *  none. */
export function incidentTone(now: number, limits: number[]): 'ok' | 'warn' | 'bad' | '' {
  if (!limits.length) return '';
  const next = limitsAhead(now, limits)[0];
  if (next == null) return 'bad';
  const reached = limits.filter((l) => l <= now);
  const prev = reached.length ? reached[reached.length - 1] : 0;
  const share = (now - prev) / (next - prev);
  return share < 0.5 ? 'ok' : share < 0.75 ? 'warn' : 'bad';
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
