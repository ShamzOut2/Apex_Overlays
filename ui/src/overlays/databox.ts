import { h, show, text, toggle } from '../lib/dom';
import * as f from '../lib/format';
import type { OverlayDef } from '../lib/overlay';
import { fieldValue, sessionType } from '../lib/sessionFields';
import type { Battle, Fuel, Relative, Session, TopicMap } from '../lib/types';

type Latest = Partial<TopicMap>;
type Out = { v: string; tone?: 'good' | 'bad' | '' };

interface Field {
  id: string;
  group: string;
  label: string;
  get(d: Latest, o: { units: f.Units; dec: number }): Out | null;
}

const num = (v: number | null | undefined, dec: number, suffix = '') => (v == null ? null : { v: `${v.toFixed(dec)}${suffix}` });
const signedTone = (v: number | null | undefined, dec: number, goodWhenNegative = false): Out | null =>
  v == null ? null : { v: f.signed(v, dec) || (0).toFixed(dec), tone: v === 0 ? '' : (v < 0) === goodWhenNegative ? 'good' : 'bad' };
const vol = (l: number | null | undefined, o: { units: f.Units; dec: number }) => (l == null ? null : { v: `${f.fuel(l, o.units).toFixed(o.dec)} ${f.fuelUnit(o.units)}` });
const focusRow = (d: Latest) => (d.relative as Relative | undefined)?.rows.find((r) => r.isFocus);
const sess = (id: string) => (d: Latest, o: { units: f.Units }) => (d.session ? { v: fieldValue(id, d.session as Session, o.units, { compact: false }) || '–' } : null);

/** Every value a data box can show. */
export const FIELDS: Field[] = [
  // Car
  { id: 'speed', group: 'Car', label: 'Speed', get: (d, o) => (d.player ? { v: `${Math.round(f.speed(d.player.speed, o.units))} ${f.speedUnit(o.units)}` } : null) },
  { id: 'gear', group: 'Car', label: 'Gear', get: (d) => (d.player ? { v: d.player.gear === -1 ? 'R' : d.player.gear === 0 ? 'N' : String(d.player.gear) } : null) },
  { id: 'rpm', group: 'Car', label: 'RPM', get: (d) => num(d.player?.rpm, 0) },
  { id: 'throttle', group: 'Car', label: 'Throttle', get: (d) => num(d.player ? d.player.throttle * 100 : null, 0, '%') },
  { id: 'brake', group: 'Car', label: 'Brake', get: (d) => num(d.player ? d.player.brake * 100 : null, 0, '%') },
  { id: 'latG', group: 'Car', label: 'Lateral G', get: (d, o) => num(d.player?.latG, Math.max(1, o.dec), ' G') },
  { id: 'longG', group: 'Car', label: 'Longitudinal G', get: (d, o) => num(d.player?.longG, Math.max(1, o.dec), ' G') },
  { id: 'heading', group: 'Car', label: 'Heading', get: (d) => (d.player?.heading != null ? { v: `${Math.round(d.player.heading)}° ${f.compass(d.player.heading)}` } : null) },
  // Laps
  { id: 'lapTime', group: 'Laps', label: 'Current lap', get: (d) => (d.player ? { v: f.lapTime(d.player.lapTime) } : null) },
  { id: 'lastLap', group: 'Laps', label: 'Last lap', get: (d) => (d.player ? { v: f.lapTime(d.player.lastLap) } : null) },
  { id: 'bestLap', group: 'Laps', label: 'Best lap', get: (d) => (d.player ? { v: f.lapTime(d.player.bestLap) } : null) },
  { id: 'delta', group: 'Laps', label: 'Delta to best lap', get: (d, o) => signedTone(d.player?.deltaBest, Math.max(2, o.dec), true) },
  { id: 'lap', group: 'Laps', label: 'Lap', get: (d) => (d.player ? { v: String(d.player.lap) } : null) },
  { id: 'stint', group: 'Laps', label: 'Stint lap', get: (d) => { const r = focusRow(d); return r ? { v: r.stintState === 'pit' ? 'PIT' : r.stintState === 'out' ? 'OUTLAP' : String(r.stintLap ?? '') } : null; } },
  // Race
  { id: 'position', group: 'Race', label: 'Position', get: (d) => (d.player?.position ? { v: `P${d.player.position}` } : null) },
  { id: 'classPosition', group: 'Race', label: 'Class position', get: (d) => (d.player?.classPosition ? { v: `P${d.player.classPosition}` } : null) },
  { id: 'gapAhead', group: 'Race', label: 'Gap to car ahead', get: (d, o) => { const a = (d.battle as Battle | undefined)?.overall.ahead; return a ? { v: a.gapLaps ? `${a.gapLaps}L` : (a.gap ?? 0).toFixed(Math.max(1, o.dec)) } : null; } },
  { id: 'gapBehind', group: 'Race', label: 'Gap to car behind', get: (d, o) => { const b = (d.battle as Battle | undefined)?.overall.behind; return b ? { v: b.gapLaps ? `${b.gapLaps}L` : (b.gap ?? 0).toFixed(Math.max(1, o.dec)) } : null; } },
  { id: 'incidents', group: 'Race', label: 'Incidents', get: (d) => (d.player ? { v: `${d.player.incidents}x` } : null) },
  { id: 'irDelta', group: 'Race', label: 'iRating gain/loss (est.)', get: (d) => signedTone(focusRow(d)?.irDelta, 0) },
  { id: 'srDelta', group: 'Race', label: 'SR gain/loss (est.)', get: (d) => signedTone(focusRow(d)?.srDelta, 2) },
  { id: 'points', group: 'Race', label: 'League points', get: (d) => num(focusRow(d)?.points, 0) },
  { id: 'seasonPoints', group: 'Race', label: 'Championship total', get: (d) => num(focusRow(d)?.seasonPoints, 0) },
  // Fuel
  { id: 'fuel', group: 'Fuel', label: 'Fuel', get: (d, o) => vol((d.fuel as Fuel | undefined)?.level, o) },
  { id: 'fuelPerLap', group: 'Fuel', label: 'Fuel per lap', get: (d, o) => vol((d.fuel as Fuel | undefined)?.perLapAvg, o) },
  { id: 'lapsInTank', group: 'Fuel', label: 'Laps of fuel', get: (d) => num((d.fuel as Fuel | undefined)?.lapsInTank, 1) },
  { id: 'fuelToFinish', group: 'Fuel', label: 'Fuel to finish', get: (d, o) => vol((d.fuel as Fuel | undefined)?.fuelToFinish, o) },
  { id: 'fuelToAdd', group: 'Fuel', label: 'Fuel to add', get: (d, o) => vol((d.fuel as Fuel | undefined)?.fuelToAdd, o) },
  { id: 'stops', group: 'Fuel', label: 'Pit stops needed', get: (d) => num((d.fuel as Fuel | undefined)?.stops, 0) },
  { id: 'raceLapsLeft', group: 'Fuel', label: 'Race laps left', get: (d) => num((d.fuel as Fuel | undefined)?.lapsRemaining, 1) },
  // Session
  { id: 'sessionType', group: 'Session', label: 'Session type', get: (d) => (d.session ? { v: sessionType(d.session as Session) } : null) },
  { id: 'time', group: 'Session', label: 'Time / laps left', get: sess('time') },
  { id: 'leaderLap', group: 'Session', label: 'Leader lap', get: sess('lap') },
  { id: 'clock', group: 'Session', label: 'Time of day (in sim)', get: sess('clock') },
  { id: 'sof', group: 'Session', label: 'Strength of field', get: sess('sof') },
  { id: 'cars', group: 'Session', label: 'Cars', get: sess('cars') },
  // Weather
  { id: 'trackTemp', group: 'Weather', label: 'Track temperature', get: sess('trackTemp') },
  { id: 'airTemp', group: 'Weather', label: 'Air temperature', get: sess('airTemp') },
  { id: 'wetness', group: 'Weather', label: 'Track wetness', get: sess('wetness') },
  { id: 'rain', group: 'Weather', label: 'Rain', get: sess('rain') },
  { id: 'wind', group: 'Weather', label: 'Wind', get: sess('wind') },
  { id: 'humidity', group: 'Weather', label: 'Humidity', get: sess('humidity') },
];

export const databox: OverlayDef = {
  kind: 'databox',
  name: 'Data box',
  description: 'A small box showing one value of your choice (speed, delta, fuel, position, track temperature, iRating change…). Add as many as you like, each with its own look and size.',
  topics: ['player', 'session', 'fuel', 'battle', 'relative'],
  size: [160, 70],
  settings: [
    { key: 'field', label: 'Show', type: 'select', default: 'delta', options: FIELDS.map((x) => [x.id, `${x.group}: ${x.label}`]), group: 'Data' },
    { key: 'label', label: 'Label', type: 'text', default: '', placeholder: 'Leave empty for the default name', group: 'Data' },
    { key: 'showLabel', label: 'Show label', type: 'bool', default: true, group: 'Look' },
    { key: 'labelPosition', when: (v) => v.showLabel !== false, label: 'Label position', type: 'select', default: 'top', options: [['top', 'Above the value'], ['left', 'Left of the value'], ['bottom', 'Below the value']], group: 'Look' },
    { key: 'align', label: 'Alignment', type: 'select', default: 'center', options: [['center', 'Centre'], ['left', 'Left'], ['right', 'Right']], group: 'Look' },
    { key: 'valueSize', label: 'Value size', type: 'number', default: 2, min: 0.6, max: 8, step: 0.1, unit: 'em', group: 'Look' },
    { key: 'labelSize', when: (v) => v.showLabel !== false, label: 'Label size', type: 'number', default: 0.75, min: 0.4, max: 3, step: 0.05, unit: 'em', group: 'Look' },
    { key: 'decimals', label: 'Decimals', type: 'number', default: 1, min: 0, max: 3, group: 'Look' },
    { key: 'valueColor', when: (v) => v.useThemeColor === false, label: 'Value colour', type: 'color', default: '#f2f4f8', group: 'Look' },
    { key: 'useThemeColor', label: 'Use the theme’s text colour', type: 'bool', default: true, group: 'Look' },
    { key: 'colorDeltas', label: 'Green/red for gains and losses', type: 'bool', default: true, group: 'Look' },
  ],
  create(root, ctx) {
    const s = ctx.settings;
    const field = FIELDS.find((x) => x.id === s.field) ?? FIELDS[0];
    root.classList.add('databox', `label-${s.labelPosition}`, `align-${s.align}`);
    const label = h('div', 'db-label', (s.label as string) || field.label);
    const value = h('div', 'db-value', '–');
    label.style.fontSize = `${s.labelSize}em`;
    value.style.fontSize = `${s.valueSize}em`;
    if (!s.useThemeColor) value.style.color = s.valueColor as string;
    show(label, s.showLabel as boolean);
    root.append(label, value);
    return {
      update() {
        const out = field.get(ctx.latest, { units: ctx.units, dec: s.decimals as number });
        text(value, out?.v ?? '–');
        toggle(value, 'good', !!s.colorDeltas && out?.tone === 'good');
        toggle(value, 'bad', !!s.colorDeltas && out?.tone === 'bad');
      },
    };
  },
};

