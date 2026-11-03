import { h, show, style, text, toggle } from '../lib/dom';
import * as f from '../lib/format';
import type { OverlayDef } from '../lib/overlay';
import { NAME_STYLES } from '../lib/settings';
import { chip } from '../lib/table';
import type { Approach, Driver, Extras, Hazard, Overtake, Safety, Status } from '../lib/types';
import { flagEmoji, iratingCell } from './common';

const driverSample = (d: Partial<Driver & Extras>): Driver & Extras => ({
  idx: 0, number: '', name: '', abbrev: '', team: '', car: '', classId: '', className: '', classColor: '#888888', userId: 0, carId: 0,
  irating: 0, license: '', licenseColor: '#888888', club: '', flair: '', country: '', make: '', isPlayer: false, isFocus: false,
  lap: 0, stintState: 'lap', stintLap: null, bestLap: null, lastLap: null, classBest: false, lastIsPb: false, finished: false, lapsLed: 0,
  flags: [], penalty: null, penaltyServed: false, incidents: null, irDelta: null, srDelta: null, points: null, seasonPoints: null, slowdown: null, ...d,
});

/** Shown while positioning the overlay: a stopped car ahead and a faster class behind. */
const SAMPLE: Safety = {
  hazards: [
    {
      ...driverSample({ idx: 61, number: '27', name: 'Tom Walker', car: 'BMW M2 CS', className: 'M2', classColor: '#3aa0ff', irating: 2140, license: 'A 3.12', licenseColor: '#0153db', country: 'GB', incidents: 6 }),
      flag: 'yellow', reason: 'stopped', gap: 4.6, distance: 212, speed: 0.4, position: 14, classPosition: 6, tireWet: null,
    },
  ],
  approaching: [
    {
      ...driverSample({ idx: 62, number: '3', name: 'Lena Fischer', car: 'Porsche 911 GT3 R (992)', className: 'GT3', classColor: '#ff5a36', irating: 3380, license: 'A 4.40', licenseColor: '#0153db', country: 'DE' }),
      gap: 2.1, distance: 104, fasterBy: 6.4, catchIn: 28, speed: 61, position: 2, classPosition: 2, tireWet: null,
    },
  ],
  overtakes: [
    {
      ...driverSample({ idx: 64, number: '51', name: 'Marco Rossi', car: 'BMW M2 CS', className: 'M2', classColor: '#3aa0ff', irating: 1960, license: 'B 3.05', licenseColor: '#00c702', country: 'IT', flags: ['slow'], slowdown: { seconds: 5, lost: 1.8 } }),
      gap: 1.6, distance: 71, speed: 38, mySpeed: 47, position: 12, classPosition: 5, tireWet: null,
    },
  ],
};

/** The same yellow and meatball marks as next to drivers in the standings, shown large. */
function flagMark(kind: 'yellow' | 'repair'): HTMLElement {
  return h('span', `mk mk-${kind} sa-flag`);
}

const CHEVRONS = `<svg viewBox="0 0 40 40" aria-hidden="true"><path d="M8 22 20 10l12 12M8 33 20 21l12 12" fill="none" stroke="currentColor" stroke-width="5" stroke-linecap="round" stroke-linejoin="round"/></svg>`;

const REASONS: Record<Hazard['reason'], string> = { stopped: 'Stopped on track', slow: 'Slow car', offTrack: 'Off track', rejoining: 'Rejoining the track', damaged: 'Damaged, told to pit' };

interface Card {
  el: HTMLElement;
  icon: HTMLElement;
  title: HTMLElement;
  tag: HTMLElement;
  num: HTMLElement;
  nat: HTMLElement;
  name: HTMLElement;
  lic: HTMLElement;
  ir: HTMLElement;
  slow: HTMLElement;
  stats: HTMLElement;
  more: HTMLElement;
  gap: HTMLElement;
  dist: HTMLElement;
}

function card(kind: string): Card {
  const el = h('div', `sa-card sa-${kind}`);
  const icon = h('div', 'sa-icon');
  const body = h('div', 'sa-body');
  const head = h('div', 'sa-title');
  const title = h('span', 'sa-head');
  const tag = h('span', 'sa-tag');
  head.append(title, tag);
  const drv = h('div', 'sa-driver');
  const num = h('span', 'sa-num');
  const nat = h('span', 'sa-nat');
  const name = h('span', 'sa-name');
  const lic = h('span', 'sa-lic');
  const ir = h('span', 'sa-ir');
  const slow = h('span', 'slow-tag', 'SLOW');
  slow.title = 'Serving a slow-down penalty';
  drv.append(num, nat, name, slow, lic, ir);
  const stats = h('div', 'sa-stats');
  const more = h('div', 'sa-stats');
  body.append(head, drv, stats, more);
  const side = h('div', 'sa-side');
  const gap = h('div', 'sa-gap');
  const dist = h('div', 'sa-dist');
  side.append(gap, dist);
  el.append(icon, body, side);
  return { el, icon, title, tag, num, nat, name, lic, ir, slow, stats, more, gap, dist };
}

export const safety: OverlayDef = {
  kind: 'safety',
  name: 'Safety alert',
  description:
    'Stays hidden until something needs your attention: a car ahead causing a yellow (stopped, slow or off track) or carrying the meatball shows a waving flag with the driver and how far ahead they are, until you have passed them. A car just ahead serving a slow-down penalty shows as a possible overtake. A car of a faster class closing from behind shows its class, car, driver, gap and distance.',
  topics: ['safety', 'status'],
  size: [400, 230],
  settings: [
    { key: 'showHazards', label: 'Warn about cars ahead', type: 'bool', default: true, group: 'Yellow & meatball ahead' },
    { key: 'aheadSeconds', when: (v) => v.showHazards !== false, label: 'Look this far ahead', type: 'number', default: 10, min: 1, max: 30, step: 0.5, unit: 's', group: 'Yellow & meatball ahead' },
    { key: 'yellow', when: (v) => v.showHazards !== false, label: 'Cars causing a yellow', type: 'bool', default: true, group: 'Yellow & meatball ahead', help: 'Stopped, crawling or off track' },
    { key: 'meatball', when: (v) => v.showHazards !== false, label: 'Cars with the meatball', type: 'bool', default: true, group: 'Yellow & meatball ahead', help: 'Damaged cars told to pit for repairs' },
    { key: 'maxHazards', when: (v) => v.showHazards !== false, label: 'Cars shown at once', type: 'number', default: 2, min: 1, max: 4, group: 'Yellow & meatball ahead' },
    { key: 'showOvertake', label: 'Tell me when a car ahead is serving a slow-down', type: 'bool', default: true, group: 'Slow-down ahead (possible overtake)' },
    {
      key: 'overtakeSeconds', when: (v) => v.showOvertake !== false,
      label: 'When it is this close',
      type: 'number',
      default: 3,
      min: 0.5,
      max: 15,
      step: 0.5,
      unit: 's',
      group: 'Slow-down ahead (possible overtake)',
      help: 'iRacing does not say how long a slow-down lasts, so the alert shows how long the car has been slowing and roughly how much time it has given up',
    },
    { key: 'maxOvertake', when: (v) => v.showOvertake !== false, label: 'Cars shown at once', type: 'number', default: 1, min: 1, max: 4, group: 'Slow-down ahead (possible overtake)' },
    { key: 'showSlowBadge', label: 'SLOW badge next to drivers serving a slow-down', type: 'bool', default: true, group: 'What to show' },
    { key: 'showApproach', label: 'Warn about faster classes behind', type: 'bool', default: true, group: 'Faster class behind' },
    { key: 'behindSeconds', when: (v) => v.showApproach !== false, label: 'Look this far behind', type: 'number', default: 5, min: 1, max: 30, step: 0.5, unit: 's', group: 'Faster class behind' },
    { key: 'maxApproach', when: (v) => v.showApproach !== false, label: 'Cars shown at once', type: 'number', default: 1, min: 1, max: 4, group: 'Faster class behind' },
    { key: 'nameStyle', label: 'Driver name', type: 'select', default: 'full', options: NAME_STYLES, group: 'What to show' },
    { key: 'showNumber', label: 'Car number', type: 'bool', default: true, group: 'What to show' },
    { key: 'showCountry', label: 'Country flag', type: 'bool', default: true, group: 'What to show' },
    { key: 'showLicense', label: 'Licence', type: 'bool', default: true, group: 'What to show' },
    { key: 'showIrating', label: 'iRating', type: 'bool', default: true, group: 'What to show' },
    { key: 'showCar', label: 'Car model', type: 'bool', default: true, group: 'What to show' },
    { key: 'showPosition', label: 'Position (overall and in class)', type: 'bool', default: true, group: 'What to show' },
    { key: 'showDistance', label: 'Distance', type: 'bool', default: true, group: 'What to show' },
    { key: 'showSpeed', label: 'Their speed (cars ahead)', type: 'bool', default: true, group: 'What to show' },
    { key: 'showIncidents', label: 'Their incidents (cars ahead)', type: 'bool', default: false, group: 'What to show' },
    { key: 'showPace', label: 'How much faster and when they reach you', type: 'bool', default: true, group: 'What to show' },
    { key: 'animate', label: 'Wave the flag', type: 'bool', default: true, group: 'Look' },
    { key: 'pulse', label: 'Pulse the border', type: 'bool', default: true, group: 'Look' },
    { key: 'holdMs', label: 'Keep showing after it clears', type: 'number', default: 800, min: 0, max: 5000, step: 100, unit: 'ms', group: 'Look', help: 'Stops the alert flickering at the edge of the window' },
  ],
  commonDefaults: { hideWhenNotDriving: true },
  create(root, ctx) {
    const s = ctx.settings;
    const preview = new URLSearchParams(location.search).has('preview');
    root.classList.add('safety', 'transparent');
    toggle(root, 'still', !s.animate);
    toggle(root, 'pulse', !!s.pulse);
    const list = h('div', 'sa-list');
    root.append(list);

    let live: Safety | null = null;
    let editing = false;
    const cards = new Map<string, Card>();
    let clearTimer = 0;

    const units = () => ctx.units;
    const dist = (m: number) => (units() === 'imperial' ? `${Math.round(m * 3.28084)} ft` : `${Math.round(m)} m`);
    const spd = (ms: number) => `${Math.round(f.speed(ms, units()))} ${f.speedUnit(units())}`;

    const fillDriver = (c: Card, d: Driver) => {
      show(c.num, !!s.showNumber && !!d.number);
      if (s.showNumber) chip(c.num, `#${d.number}`, d.classColor);
      const nat = s.showCountry ? flagEmoji(d.country) : '';
      text(c.nat, nat);
      show(c.nat, !!nat);
      text(c.name, f.name(d.name, s.nameStyle as string));
      const lic = s.showLicense && d.license ? `${d.license.split(' ')[0]} ${(d.license.split(' ')[1] ?? '').slice(0, 3)}` : '';
      show(c.lic, !!lic);
      if (lic) chip(c.lic, lic, d.licenseColor);
      show(c.ir, !!s.showIrating && d.irating > 0);
      if (s.showIrating) iratingCell(c.ir, d.irating, null);
      show(c.slow, !!s.showSlowBadge && (d as Partial<Extras>).flags?.includes('slow') === true);
    };

    const pos = (p: number, cp: number) => (s.showPosition && p > 0 ? [`P${p}`, cp > 0 && cp !== p ? `class P${cp}` : ''] : []);
    const line = (el: HTMLElement, bits: string[]) => {
      const v = bits.filter(Boolean).join(' · ');
      text(el, v);
      show(el, !!v);
    };
    const side = (c: Card, gap: number, m: number) => {
      text(c.gap, `${gap.toFixed(1)}s`);
      text(c.dist, s.showDistance ? dist(m) : '');
    };

    const fillHazard = (c: Card, z: Hazard) => {
      if (c.icon.dataset.k !== z.flag) {
        c.icon.dataset.k = z.flag;
        c.icon.replaceChildren(flagMark(z.flag));
      }
      toggle(c.el, 'repair', z.flag === 'repair');
      text(c.title, z.flag === 'repair' ? 'MEATBALL AHEAD' : 'YELLOW AHEAD');
      text(c.tag, REASONS[z.reason]);
      fillDriver(c, z);
      line(c.stats, [s.showCar ? z.car : '', ...pos(z.position, z.classPosition)]);
      line(c.more, [
        s.showSpeed && z.flag === 'yellow' ? (z.speed < 2 ? 'Not moving' : `Doing ${spd(z.speed)}`) : '',
        s.showIncidents && z.incidents != null ? `${z.incidents}x incidents` : '',
      ]);
      side(c, z.gap, z.distance);
    };

    const fillApproach = (c: Card, a: Approach) => {
      if (!c.icon.dataset.k) {
        c.icon.dataset.k = 'chev';
        c.icon.innerHTML = CHEVRONS;
      }
      style(c.el, '--cls', a.classColor);
      text(c.title, 'FASTER CLASS BEHIND');
      chip(c.tag, a.className || 'Class', a.classColor);
      fillDriver(c, a);
      line(c.stats, [s.showCar ? a.car : '', ...pos(a.position, a.classPosition)]);
      line(c.more, [
        s.showPace && a.fasterBy > 0 ? `${a.fasterBy.toFixed(1)}s a lap faster` : '',
        s.showPace && a.catchIn != null ? (a.catchIn < 3 ? 'reaching you now' : `with you in ~${a.catchIn}s`) : '',
      ]);
      side(c, a.gap, a.distance);
    };

    const fillOvertake = (c: Card, o: Overtake) => {
      if (!c.icon.dataset.k) {
        c.icon.dataset.k = 'slow';
        c.icon.replaceChildren(h('span', 'slow-tag sa-slow', 'SLOW'));
      }
      text(c.title, 'POSSIBLE OVERTAKE');
      text(c.tag, 'Serving a slow-down');
      fillDriver(c, o);
      show(c.slow, false);
      line(c.stats, [s.showCar ? o.car : '', ...pos(o.position, o.classPosition)]);
      const sd = o.slowdown;
      line(c.more, [
        sd ? `${Math.round(sd.seconds)}s slowing` : '',
        sd && sd.lost >= 0.1 ? `~${sd.lost.toFixed(1)}s given up` : '',
        s.showSpeed ? `${spd(o.speed)}${o.mySpeed > 0 ? ` (you ${Math.round(f.speed(o.mySpeed, units()))})` : ''}` : '',
      ]);
      side(c, o.gap, o.distance);
    };

    /** What to show now: the cars within the chosen windows. */
    const pick = (d: Safety | null) => {
      const hz = s.showHazards
        ? (d?.hazards ?? []).filter((z) => z.gap <= (s.aheadSeconds as number) && (z.flag === 'repair' ? s.meatball : s.yellow)).slice(0, s.maxHazards as number)
        : [];
      const ap = s.showApproach ? (d?.approaching ?? []).filter((a) => a.gap <= (s.behindSeconds as number)).slice(0, s.maxApproach as number) : [];
      const ot = s.showOvertake ? (d?.overtakes ?? []).filter((o) => o.gap <= (s.overtakeSeconds as number)).slice(0, s.maxOvertake as number) : [];
      return { hz, ap, ot };
    };

    const draw = (hz: Hazard[], ap: Approach[], ot: Overtake[]) => {
      const seen = new Set<string>();
      const order: HTMLElement[] = [];
      const use = (key: string, kind: string) => {
        seen.add(key);
        let c = cards.get(key);
        if (!c) {
          c = card(kind);
          cards.set(key, c);
        }
        order.push(c.el);
        return c;
      };
      for (const z of hz) fillHazard(use(`h${z.idx}-${z.flag}`, 'hazard'), z);
      for (const o of ot) fillOvertake(use(`o${o.idx}`, 'overtake'), o);
      for (const a of ap) fillApproach(use(`a${a.idx}`, 'approach'), a);
      for (const [k, c] of cards) {
        if (!seen.has(k)) {
          c.el.remove();
          cards.delete(k);
        }
      }
      // Re-append only when the order changed, so the flag animations keep running.
      if (order.some((el, i) => list.children[i] !== el)) list.replaceChildren(...order);
      root.classList.toggle('auto-hidden', !order.length);
    };

    const update = () => {
      let { hz, ap, ot } = pick(live);
      if ((editing || preview) && !hz.length && !ap.length && !ot.length) ({ hz, ap, ot } = pick(SAMPLE));
      if (hz.length || ap.length || ot.length) {
        clearTimeout(clearTimer);
        clearTimer = 0;
        draw(hz, ap, ot);
      } else if (cards.size && !clearTimer) {
        clearTimer = window.setTimeout(() => {
          clearTimer = 0;
          draw([], [], []);
        }, s.holdMs as number);
      } else if (!cards.size) {
        root.classList.add('auto-hidden');
      }
    };

    return {
      update(topic, data) {
        if (topic === 'safety') live = data as Safety;
        if (topic === 'status') editing = !!(data as Status).editMode;
        update();
      },
    };
  },
};
