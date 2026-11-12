import { h, show, style, text } from '../lib/dom';
import * as f from '../lib/format';
import type { OverlayDef } from '../lib/overlay';
import { trackState } from '../lib/sessionFields';
import type { Player, Session } from '../lib/types';

// Compass ring (rotates with the car) and a fixed car marker pointing up.
const RING = `<svg viewBox="-60 -60 120 120">
  <circle r="54" fill="none" stroke="currentColor" stroke-opacity="0.35" stroke-width="2"/>
  ${Array.from({ length: 36 }, (_, i) => {
    const a = (i * 10 * Math.PI) / 180;
    const r1 = i % 9 === 0 ? 44 : 49;
    return `<line x1="${Math.sin(a) * r1}" y1="${-Math.cos(a) * r1}" x2="${Math.sin(a) * 54}" y2="${-Math.cos(a) * 54}" stroke="currentColor" stroke-opacity="${i % 9 === 0 ? 0.9 : 0.4}" stroke-width="${i % 9 === 0 ? 2.5 : 1.2}"/>`;
  }).join('')}
  <g class="cardinals" font-size="11" font-weight="800" text-anchor="middle" dominant-baseline="central" fill="currentColor">
    <text x="0" y="-34" class="n">N</text><text x="34" y="0">E</text><text x="0" y="34">S</text><text x="-34" y="0">W</text>
  </g>
  <g class="wind"><path d="M0 -54 L0 -14" stroke="var(--wind)" stroke-width="5" stroke-linecap="round"/><path d="M-9 -24 L0 -10 L9 -24 Z" fill="var(--wind)"/></g>
</svg>`;
const CAR = `<svg viewBox="-60 -60 120 120"><path d="M0 -12 L7 9 L0 5 L-7 9 Z" fill="var(--accent)" stroke="#000" stroke-width="1"/></svg>`;

export const weather: OverlayDef = {
  kind: 'weather',
  name: 'Weather',
  description: 'Live weather with a wind compass that turns with your car, so you can see head, tail and crosswind at a glance. Plus temperatures, humidity, pressure, sky, rain and track wetness.',
  topics: ['session', 'player'],
  size: [300, 170],
  settings: [
    { key: 'compassSize', label: 'Compass size', type: 'number', default: 8, min: 4, max: 20, step: 0.5, unit: 'em', group: 'Compass' },
    { key: 'carUp', label: 'Rotate with the car (car always points up)', type: 'bool', default: true, group: 'Compass' },
    { key: 'cardinals', label: 'N / E / S / W letters', type: 'bool', default: true, group: 'Compass' },
    { key: 'windColor', label: 'Wind arrow', type: 'color', default: '#4dc3ff', group: 'Compass' },
    { key: 'invertWind', label: 'Flip wind direction', type: 'bool', default: false, group: 'Compass', help: 'Use if the arrow points where the wind comes from instead of where it blows' },
    { key: 'invertHeading', label: 'Flip car heading', type: 'bool', default: false, group: 'Compass' },
    { key: 'showComponents', label: 'Head/tail and crosswind', type: 'bool', default: true, group: 'Readouts' },
    { key: 'showTrack', label: 'Track temperature', type: 'bool', default: true, group: 'Readouts' },
    { key: 'showAir', label: 'Air temperature', type: 'bool', default: true, group: 'Readouts' },
    { key: 'showHumidity', label: 'Humidity', type: 'bool', default: true, group: 'Readouts' },
    { key: 'showPressure', label: 'Air pressure', type: 'bool', default: false, group: 'Readouts' },
    { key: 'showSky', label: 'Sky', type: 'bool', default: true, group: 'Readouts' },
    { key: 'showRain', label: 'Rain and track wetness', type: 'bool', default: true, group: 'Readouts' },
    { key: 'showFog', label: 'Fog', type: 'bool', default: false, group: 'Readouts' },
    { key: 'showTrackState', label: 'Track state (rubber: clean, low usage…)', type: 'bool', default: false, group: 'Readouts' },
  ],
  create(root, ctx) {
    const s = ctx.settings;
    root.classList.add('weather');
    root.style.setProperty('--wind', s.windColor as string);
    root.style.setProperty('--compass', `${s.compassSize}em`);
    const compass = h('div', 'compass');
    const ring = h('div', 'ring');
    ring.innerHTML = RING;
    const car = h('div', 'car');
    car.innerHTML = CAR;
    const windWrap = ring.querySelector('.wind') as SVGGElement;
    show(ring.querySelector('.cardinals') as unknown as HTMLElement, s.cardinals as boolean);
    compass.append(ring, car);
    const side = h('div', 'readouts');
    const big = h('div', 'wind-speed');
    const comp = h('div', 'wind-comp');
    show(comp, s.showComponents as boolean);
    side.append(big, comp);
    const kv = h('div', 'kv');
    const rows: Record<string, HTMLElement> = {};
    for (const [key, label] of [
      ['showTrack', 'Track'],
      ['showAir', 'Air'],
      ['showHumidity', 'Humidity'],
      ['showPressure', 'Pressure'],
      ['showSky', 'Sky'],
      ['showRain', 'Rain'],
      ['showFog', 'Fog'],
      ['showTrackState', 'Rubber'],
    ]) {
      if (!s[key]) continue;
      const v = h('div', 'v');
      kv.append(h('div', 'k', label), v);
      rows[key] = v;
    }
    side.append(kv);
    root.append(compass, side);
    let session: Session | null = null;
    let heading = 0;
    let headingKnown = true;

    const draw = () => {
      if (!session) return;
      const u = ctx.units;
      const hdg = s.invertHeading ? -heading : heading;
      // Wind direction is where the wind comes from (meteorological).
      const from = (session.windDir + (s.invertWind ? 180 : 0)) % 360;
      style(ring, 'transform', `rotate(${s.carUp ? -hdg : 0}deg)`);
      style(car, 'transform', `rotate(${s.carUp ? 0 : hdg}deg)`);
      windWrap.setAttribute('transform', `rotate(${from})`);
      const speed = f.speed(session.windSpeed, u);
      text(big, `${Math.round(speed)} ${f.speedUnit(u)} ${f.compass(from)}`);
      const rel = ((from - hdg) * Math.PI) / 180;
      const head = speed * Math.cos(rel);
      const cross = speed * Math.sin(rel);
      show(car, headingKnown);
      if (!headingKnown) text(comp, 'Heading unknown — drive a lap to record the track');
      else text(
        comp,
        `${head >= 0 ? 'Head' : 'Tail'} ${Math.abs(head).toFixed(0)} · ${Math.abs(cross) < 0.5 ? 'no cross' : `cross ${Math.abs(cross).toFixed(0)} from ${cross > 0 ? 'right' : 'left'}`}`,
      );
      const set = (k: string, v: string) => rows[k] && text(rows[k], v);
      const trend = Math.abs(session.trackTempDelta) >= 0.3 ? (session.trackTempDelta > 0 ? ' ▲' : ' ▼') : '';
      set('showTrack', f.temp(session.trackTemp, u) + trend);
      set('showAir', f.temp(session.airTemp, u));
      set('showHumidity', `${Math.round(session.humidity * 100)}%`);
      // iRacing reports inches of mercury.
      const p = session.airPressure;
      set('showPressure', p > 0 && p < 200 ? (u === 'imperial' ? `${p.toFixed(2)} inHg` : `${Math.round(p * 33.8639)} hPa`) : `${Math.round(p)}`);
      set('showSky', session.skies);
      const arrow = session.weatherTrend === 'wetting' ? ' ↑' : session.weatherTrend === 'drying' ? ' ↓' : '';
      set('showRain', `${session.precipitation > 0 ? `${Math.round(session.precipitation * 100)}%` : 'None'} · ${session.wetnessLabel}${arrow}`);
      set('showFog', `${Math.round(session.fog * 100)}%`);
      set('showTrackState', trackState(session.trackState) || '–');
    };

    return {
      update(topic, data) {
        if (topic === 'session') session = data as Session;
        if (topic === 'player') {
          const raw = (data as Player).heading;
          const hd = raw ?? 0;
          if (Math.abs(hd - heading) < 0.2 && headingKnown === (raw != null)) return;
          heading = hd;
          headingKnown = raw != null;
        }
        draw();
      },
    };
  },
};
