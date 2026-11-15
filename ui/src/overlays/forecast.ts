// Weather forecast: the conditions now and at the times ahead you choose (every 15 or 30
// minutes, or every hour). Le Mans Ultimate publishes a forecast for each session (points
// through the session, blended here into your steps); iRacing shares only the weather now.

import { h, show, style, text, toggle } from '../lib/dom';
import * as f from '../lib/format';
import type { OverlayDef } from '../lib/overlay';
import type { Forecast, ForecastPoint, Sky, Status } from '../lib/types';

const SKY_NAME: Record<Sky, string> = {
  clear: 'Clear',
  light: 'Light clouds',
  partly: 'Partly cloudy',
  mostly: 'Mostly cloudy',
  overcast: 'Overcast',
  drizzle: 'Drizzle',
  lightRain: 'Light rain',
  rain: 'Rain',
  heavyRain: 'Heavy rain',
  storm: 'Storm',
};

const WET: Sky[] = ['drizzle', 'lightRain', 'rain', 'heavyRain', 'storm'];

// Icons (64×64): a sun or moon, clouds (light, or dark when it rains hard), drops, a bolt.
const sun = (x: number, y: number, r: number) =>
  `<g fill="#ffc83d">${Array.from({ length: 8 }, (_, i) => `<rect x="${x - 1.6}" y="${y - r - 7}" width="3.2" height="5" rx="1.6" transform="rotate(${i * 45} ${x} ${y})"/>`).join('')}<circle cx="${x}" cy="${y}" r="${r}"/></g>`;
const moon = (x: number, y: number, r: number) =>
  `<path fill="#d6dcff" d="M${x + r * 0.35} ${y - r} A${r} ${r} 0 1 0 ${x + r} ${y + r * 0.45} A${r * 0.8} ${r * 0.8} 0 1 1 ${x + r * 0.35} ${y - r} Z"/>`;
const cloud = (dx: number, dy: number, scale: number, fill: string) =>
  `<g transform="translate(${dx} ${dy}) scale(${scale})" fill="${fill}"><circle cx="22" cy="38" r="8"/><circle cx="32" cy="31" r="11"/><circle cx="43" cy="36" r="9"/><rect x="14" y="37" width="38" height="9" rx="4.5"/></g>`;
const drops = (xs: number[], y: number, color = '#5aaefc') =>
  xs.map((x, i) => `<path fill="${color}" d="M${x} ${y + (i % 2) * 4} c-2.2 3.4 -3.2 5.4 -3.2 6.6 a3.2 3.2 0 0 0 6.4 0 c0 -1.2 -1 -3.2 -3.2 -6.6z"/>`).join('');
const dots = (xs: number[], y: number) => xs.map((x, i) => `<circle cx="${x}" cy="${y + (i % 2) * 4}" r="1.8" fill="#8cc8ff"/>`).join('');
const LIGHT = '#e6ebf2';
const GREY = '#aab3c0';
const DARK = '#7d8798';

function icon(sky: Sky, night: boolean): string {
  const orb = (x: number, y: number, r: number) => (night ? moon(x, y, r) : sun(x, y, r));
  const body = (() => {
    switch (sky) {
      case 'clear':
        return orb(32, 32, 12);
      case 'light':
        return orb(28, 28, 11) + cloud(10, 10, 0.75, LIGHT);
      case 'partly':
        return orb(24, 24, 10) + cloud(4, 4, 0.92, LIGHT);
      case 'mostly':
        return orb(20, 20, 8) + cloud(0, 0, 1, LIGHT);
      case 'overcast':
        return cloud(-6, -6, 0.9, GREY) + cloud(2, 2, 1, LIGHT);
      case 'drizzle':
        return cloud(0, -6, 1, LIGHT) + dots([24, 32, 40], 48);
      case 'lightRain':
        return cloud(0, -6, 1, GREY) + drops([26, 38], 45);
      case 'rain':
        return cloud(0, -6, 1, GREY) + drops([22, 32, 42], 45);
      case 'heavyRain':
        return cloud(0, -8, 1, DARK) + drops([18, 26, 34, 42, 50], 42, '#3f95f5');
      case 'storm':
        return cloud(0, -8, 1, DARK) + drops([20, 44], 44, '#3f95f5') + '<path fill="#ffd23f" d="M34 38 L26 51 L32 51 L28 62 L41 46 L34 46 L38 38 Z"/>';
    }
  })();
  return `<svg viewBox="0 0 64 64" aria-hidden="true">${body}</svg>`;
}

/** The forecast at `m` minutes from now: temperature and rain chance blended between the
 *  points either side, the sky of the nearer one. */
export function forecastAt(points: ForecastPoint[], m: number): { sky: Sky; temp: number; rain: number } | null {
  if (!points.length) return null;
  let i = -1;
  for (let k = 0; k < points.length; k++) if (points[k].minutes <= m) i = k;
  if (i < 0) return { ...points[0] };
  const a = points[i];
  const b = points[i + 1];
  if (!b || b.minutes <= a.minutes) return { sky: a.sky, temp: a.temp, rain: a.rain };
  const k = (m - a.minutes) / (b.minutes - a.minutes);
  return { sky: k < 0.5 ? a.sky : b.sky, temp: a.temp + (b.temp - a.temp) * k, rain: a.rain + (b.rain - a.rain) * k };
}

/** A sample (edit mode, settings preview): clouds building to rain and clearing again. */
const SAMPLE: Forecast = {
  game: 'lmu',
  minutesLeft: 150,
  timeOfDay: 14 * 3600,
  now: { sky: 'partly', air: 23.4, track: 31.2, rain: 0, wetness: 1 },
  points: [
    { minutes: -10, sky: 'partly', temp: 23.6, rain: 0 },
    { minutes: 20, sky: 'mostly', temp: 22.4, rain: 0.2 },
    { minutes: 50, sky: 'rain', temp: 19.5, rain: 0.75 },
    { minutes: 80, sky: 'lightRain', temp: 19.8, rain: 0.5 },
    { minutes: 110, sky: 'partly', temp: 21, rain: 0.1 },
  ],
  note: '',
};

export const forecast: OverlayDef = {
  kind: 'forecast',
  name: 'Weather forecast',
  description:
    'The weather now and to come, every 15 or 30 minutes or every hour: sky, temperature and chance of rain. Le Mans Ultimate publishes a forecast for each session; iRacing shares only the conditions now.',
  topics: ['forecast', 'status'],
  size: [460, 150],
  settings: [
    {
      key: 'interval',
      label: 'Show the forecast every',
      type: 'select',
      default: '30',
      options: [
        ['15', '15 minutes'],
        ['30', '30 minutes'],
        ['60', 'Hour'],
      ],
      group: 'Forecast',
    },
    {
      key: 'tiles',
      label: 'Tiles',
      type: 'select',
      default: '5',
      options: [
        ['4', '4 tiles'],
        ['5', '5 tiles'],
      ],
      group: 'Forecast',
      help: 'The weather now is the first tile, then one per step: 4 tiles every 15 minutes covers the next 45 minutes',
    },
    { key: 'showNow', label: 'Weather now as the first tile', type: 'bool', default: true, group: 'Forecast', help: 'Off: every tile is a time ahead' },
    { key: 'showHeader', label: 'Title and time left', type: 'bool', default: true, group: 'Forecast' },
    { key: 'showTemp', label: 'Temperature', type: 'bool', default: true, group: 'What to show' },
    { key: 'showTrackTemp', when: (v) => v.showNow !== false, label: 'Track temperature (weather now)', type: 'bool', default: true, group: 'What to show' },
    { key: 'showRain', label: 'Chance of rain', type: 'bool', default: true, group: 'What to show' },
    { key: 'showSkyName', label: 'Sky in words', type: 'bool', default: false, group: 'What to show' },
    { key: 'wetFrom', when: (v) => v.showRain !== false, label: 'Highlight from', type: 'number', default: 40, min: 5, max: 100, step: 5, unit: '% rain', group: 'Colours', help: 'Times with at least this chance of rain glow in the rain colour' },
    { key: 'rainColor', label: 'Rain', type: 'color', default: '#4da3ff', group: 'Colours' },
  ],
  commonDefaults: { autoWidth: false },
  create(root, ctx) {
    const s = ctx.settings;
    const preview = new URLSearchParams(location.search).has('preview');
    root.classList.add('forecast');
    style(root, '--fc-rain', s.rainColor as string);
    const head = h('div', 'fc-head');
    const title = h('span', 'fc-title', 'WEATHER');
    const sub = h('span', 'fc-sub');
    head.append(title, sub);
    const row = h('div', 'fc-row');
    const note = h('div', 'fc-note');
    root.append(head, row, note);
    show(head, s.showHeader !== false);

    let data: Forecast | null = null;
    let editing = false;
    let key = '';

    const card = (label: string, sky: Sky, night: boolean, temp: number | null, rain: number | null, extra = '') => {
      const c = h('div', 'fc-card');
      const l = h('div', 'fc-label', label);
      const i = h('div', 'fc-icon');
      i.innerHTML = icon(sky, night);
      i.title = SKY_NAME[sky];
      c.append(l, i);
      if (s.showSkyName) c.append(h('div', 'fc-sky', SKY_NAME[sky]));
      if (s.showTemp !== false && temp != null) {
        const t = h('div', 'fc-temp', f.temp(temp, ctx.units).replace(/\.\d/, ''));
        if (extra) t.append(h('span', 'fc-extra', extra));
        c.append(t);
      }
      if (s.showRain !== false && rain != null) {
        const pct = Math.round(rain * 100);
        const r = h('div', 'fc-rain');
        const drop = h('span', 'fc-drop');
        drop.innerHTML = '<svg viewBox="0 0 10 13" aria-hidden="true"><path fill="currentColor" d="M5 0C2.2 4 0 6.6 0 8.6a5 5 0 0 0 10 0C10 6.6 7.8 4 5 0z"/></svg>';
        r.append(drop, h('span', 'fc-pct', `${pct}%`));
        const bar = h('div', 'fc-bar');
        const fill = h('div', 'fc-fill');
        style(fill, 'width', `${pct}%`);
        bar.append(fill);
        c.append(r, bar);
        toggle(c, 'wet', pct >= (s.wetFrom as number));
      }
      if (WET.includes(sky)) c.classList.add('raining');
      return c;
    };

    const isNight = (tod: number, minutes: number) => {
      const hour = (((tod + minutes * 60) % 86400) + 86400) % 86400 / 3600;
      return hour < 6.5 || hour >= 20;
    };
    const label = (m: number) => (m < 60 ? `+${m} MIN` : m % 60 ? `+${Math.floor(m / 60)}H ${m % 60}` : `+${m / 60}H`);

    const draw = () => {
      const d = data && (data.points.length || data.now) && data.game ? data : null;
      const fc = d ?? (editing || preview ? SAMPLE : null);
      toggle(root, 'auto-hidden', !fc);
      if (!fc) return;
      const step = Number(s.interval) || 30;
      const tiles = Number(s.tiles) === 4 ? 4 : 5;
      const slots = s.showNow !== false ? tiles - 1 : tiles;
      const k = JSON.stringify([fc, step, slots, ctx.units]);
      if (k === key) return;
      key = k;
      const cards: HTMLElement[] = [];
      if (s.showNow !== false) {
        const now = cards.length;
        const track = s.showTrackTemp !== false ? ` / ${f.temp(fc.now.track, ctx.units).replace(/\.\d/, '')}` : '';
        // Rain falling now; the chance of rain from the forecast when there is one.
        const chance = forecastAt(fc.points, 0)?.rain ?? null;
        const rainNow = fc.now.rain > 0.01 ? Math.max(fc.now.rain, chance ?? 0) : chance;
        cards.push(card('NOW', fc.now.sky, isNight(fc.timeOfDay, 0), fc.now.air, rainNow, track));
        cards[now].classList.add('now');
      }
      // Always the same number of tiles. Past the end of the session the forecast runs on as
      // the session's last (the game says nothing beyond): those tiles are dimmed.
      for (let n = 1; n <= slots && fc.points.length; n++) {
        const m = n * step;
        const p = forecastAt(fc.points, m);
        if (!p) break;
        const c = card(label(m), p.sky, isNight(fc.timeOfDay, m), p.temp, p.rain);
        if (fc.minutesLeft != null && m > fc.minutesLeft) {
          c.classList.add('after');
          c.title = 'After the end of this session: the session’s last forecast';
        }
        cards.push(c);
      }
      row.replaceChildren(...cards);
      text(sub, fc.minutesLeft != null ? `${Math.round(fc.minutesLeft)} min left` : '');
      text(note, fc.note);
      show(note, !!fc.note);
    };
    draw();

    return {
      update(topic, d) {
        if (topic === 'status') editing = !!(d as Status).editMode;
        if (topic === 'forecast') data = d as Forecast;
        draw();
      },
    };
  },
};
