import { h, show, text } from '../lib/dom';
import * as f from '../lib/format';
import type { OverlayDef } from '../lib/overlay';
import type { Pit, Status } from '../lib/types';

/** Shown while positioning the overlay. */
const SAMPLE: Pit = { phase: 'entry', toEntry: 84, toExit: null, speed: 52, limit: 22.2, limiter: false, service: '', repair: 0, optRepair: 0, sinceExit: null };

const BRAKE_SVG = `<svg viewBox="0 0 100 100" aria-hidden="true"><circle cx="50" cy="50" r="45" fill="none" stroke="currentColor" stroke-width="6"/><circle cx="50" cy="50" r="38" fill="none" stroke="currentColor" stroke-width="2" stroke-dasharray="5 6"/><text x="50" y="56" text-anchor="middle" font-size="16" font-weight="900" fill="currentColor" font-family="Segoe UI, sans-serif">BRAKE</text></svg>`;
const LIMITER_SVG = `<svg viewBox="0 0 100 100" aria-hidden="true"><circle cx="50" cy="50" r="44" fill="none" stroke="currentColor" stroke-width="7"/><path d="M24 62a28 28 0 1 1 52 0" fill="none" stroke="currentColor" stroke-width="6" stroke-linecap="round"/><path d="M50 58 66 38" stroke="currentColor" stroke-width="6" stroke-linecap="round"/><text x="50" y="84" text-anchor="middle" font-size="17" font-weight="900" fill="currentColor" font-family="Segoe UI, sans-serif">PIT</text></svg>`;
const EXIT_SVG = `<svg viewBox="0 0 130 70" aria-hidden="true"><rect x="3" y="3" width="124" height="64" rx="8" fill="currentColor"/><text x="50" y="31" text-anchor="middle" font-size="18" font-weight="900" fill="#0b0e13" font-family="Segoe UI, sans-serif">PIT LANE</text><text x="50" y="54" text-anchor="middle" font-size="18" font-weight="900" fill="#0b0e13" font-family="Segoe UI, sans-serif">EXIT</text><path d="M98 35h20M109 25l10 10-10 10" fill="none" stroke="#0b0e13" stroke-width="6" stroke-linecap="round" stroke-linejoin="round"/></svg>`;

export const pit: OverlayDef = {
  kind: 'pit',
  name: 'Pit lane',
  description:
    'Shows when you head for the pits: when to brake for the pit lane, your speed and limiter, WAIT then GO in your box, and a reminder to switch the limiter off.',
  topics: ['pit', 'status'],
  size: [230, 200],
  settings: [
    { key: 'decel', label: 'Braking for the pit entry', type: 'number', default: 1.0, min: 0.3, max: 3, step: 0.1, unit: 'g', group: 'Pit entry', help: 'How hard you brake: the BRAKE sign lights up when you need this much to reach the limit at the line' },
    { key: 'margin', label: 'Safety margin', type: 'number', default: 15, min: 0, max: 100, unit: 'm', group: 'Pit entry', help: 'Brake this much earlier' },
    { key: 'hold', label: 'Keep showing after the pit exit', type: 'number', default: 5, min: 0, max: 30, unit: 's', group: 'Behaviour' },
    { key: 'showLimit', label: 'Show the speed limit', type: 'bool', default: true, group: 'Behaviour' },
    { key: 'showRepairs', label: 'Repair time left in the box', type: 'bool', default: true, group: 'Behaviour' },
  ],
  commonDefaults: { hideWhenNotDriving: true },
  create(root, ctx) {
    const s = ctx.settings;
    const preview = new URLSearchParams(location.search).has('preview');
    root.classList.add('pit');

    const sign = h('div', 'pit-sign');
    const big = h('div', 'pit-big');
    const sub = h('div', 'pit-sub');
    const limiter = h('div', 'pit-limiter');
    limiter.innerHTML = LIMITER_SVG;
    const row = h('div', 'pit-row');
    row.append(sign, limiter);
    root.append(row, big, sub);

    let live: Pit | null = null;
    let editing = false;

    const units = () => ctx.units;
    const dist = (m: number) => (units() === 'imperial' ? `${Math.round(m * 3.28084)} ft` : `${Math.round(m)} m`);
    const spd = (ms: number) => Math.round(f.speed(ms, units()));
    const setSign = (cls: string, svg: string) => {
      if (sign.dataset.svg !== cls) {
        sign.innerHTML = svg;
        sign.dataset.svg = cls;
      }
    };

    const render = () => {
      const p = live?.phase || !(editing || preview) ? live : SAMPLE;
      const phase = p?.phase ?? '';
      const visible = !!phase && !(phase === 'out' && (p!.sinceExit ?? 99) > (s.hold as number));
      root.classList.toggle('auto-hidden', !visible);
      if (!p || !visible) return;
      root.dataset.phase = phase;
      const over = p.limit > 0 && p.speed > p.limit + 0.3;
      let signCls = '';
      let bigText = '';
      let bigCls = '';
      let subText = '';
      show(limiter, phase === 'lane' || phase === 'exit' || phase === 'out');
      limiter.className = `pit-limiter ${p.limiter ? 'on' : 'off'}`;
      limiter.title = p.limiter ? 'Pit limiter on' : 'Pit limiter OFF';
      show(sign, phase !== 'box');
      switch (phase) {
        case 'entry': {
          // Distance needed to slow to the limit, from how hard the driver brakes.
          const a = (s.decel as number) * 9.81;
          const need = p.limit > 0 ? Math.max(0, (p.speed ** 2 - p.limit ** 2) / (2 * a)) + (s.margin as number) : 0;
          const ok = p.limit > 0 && p.speed <= p.limit + 0.3;
          const brake = !ok && p.toEntry != null && p.toEntry <= need;
          signCls = ok ? 'ok' : brake ? 'brake' : 'ready';
          setSign('brake', BRAKE_SVG);
          bigText = p.toEntry != null ? dist(p.toEntry) : 'Pit entry';
          subText = p.toEntry != null ? 'to the pit entry line' : 'Pit entry line not known yet';
          if (s.showLimit && p.limit > 0) subText += ` · limit ${spd(p.limit)} ${f.speedUnit(units())}`;
          break;
        }
        case 'lane':
          show(sign, false);
          bigText = `${spd(p.speed)}`;
          bigCls = `speed ${over ? 'bad' : 'good'}`;
          subText = f.speedUnit(units()) + (s.showLimit && p.limit > 0 ? ` · limit ${spd(p.limit)}` : '');
          break;
        case 'box':
          bigText = p.service === 'go' ? 'GO' : 'WAIT';
          bigCls = `logo ${p.service === 'go' ? 'go' : 'wait'}`;
          if (s.showRepairs && p.service !== 'go') subText = p.repair > 0 ? `Repairs ${Math.ceil(p.repair)} s` : p.optRepair > 0 ? `Optional repairs ${Math.ceil(p.optRepair)} s` : '';
          break;
        case 'exit':
          signCls = 'exit';
          setSign('exit', EXIT_SVG);
          bigText = p.toExit != null ? dist(p.toExit) : '';
          subText = p.toExit != null ? 'to the pit exit line' : '';
          break;
        case 'out':
          bigText = p.limiter ? 'TURN OFF PIT LIMITER' : 'PIT LIMITER OFF';
          bigCls = `banner ${p.limiter ? 'remind' : 'done'}`;
          show(sign, false);
          break;
      }
      sign.className = `pit-sign ${signCls}`;
      text(big, bigText);
      big.className = `pit-big ${bigCls}`;
      text(sub, subText);
      show(sub, !!subText);
    };

    return {
      update(topic, data) {
        if (topic === 'status') editing = !!(data as Status).editMode;
        if (topic === 'pit') live = data as Pit;
        render();
      },
    };
  },
};
