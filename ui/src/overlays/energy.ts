// Le Mans Ultimate's virtual energy (Hypercar, LMGT3...): energy left against the laps to go,
// like the fuel calculator, in % of the car's allowance.

import { h, show, style, text, toggle } from '../lib/dom';
import type { OverlayDef } from '../lib/overlay';
import { onColor } from '../lib/theme';
import type { Energy, Session, Status } from '../lib/types';

/** Shown while placing the overlay: a Hypercar mid-stint. */
const SAMPLE: Energy = {
  available: true, game: 'lmu', level: 61.4, battery: 58, perLapAvg: 5.82, perLapLast: 5.76, lapsInTank: 10.5, lapsRemaining: 14.2,
  toFinish: 88.5, toAdd: 27.1, stops: 1, targetPerLap: 4.04, windowOpensLap: 18, windowClosesLap: 27, marginLaps: 1, history: [],
};

type State = 'ok' | 'marginal' | 'short' | 'none';

const pct = (v: number | null | undefined, d = 1) => (v == null ? '–' : `${v.toFixed(d)}`);

export const energy: OverlayDef = {
  kind: 'energy',
  name: 'Virtual energy',
  lmu: true,
  description:
    'Virtual energy planned like fuel: laps left, use per lap, what you will have at the flag, how much to add and the pit window. Plus the hybrid battery. For Hypercar and LMGT3.',
  topics: ['energy', 'session', 'status'],
  size: [330, 230],
  commonDefaults: { autoHeight: true },
  settings: [
    { key: 'hideWithout', label: 'Hide when there is no virtual energy', type: 'bool', default: true, group: 'Layout', help: 'In iRacing, and in Le Mans Ultimate classes that race on fuel (LMP2...)', lmu: true },
    { key: 'showBar', label: 'Energy bar', type: 'bool', default: true, group: 'Layout' },
    { key: 'showBattery', label: 'Hybrid battery', type: 'bool', default: true, group: 'Layout', lmu: true },
    { key: 'showPlan', label: 'Add at stop, target and pit window', type: 'bool', default: true, group: 'Layout' },
    { key: 'showEstLaps', label: 'Estimated laps left (time left ÷ your lap time)', type: 'bool', default: false, group: 'Layout', help: 'Timed sessions' },
    { key: 'estDecimals', when: (v) => !!v.showEstLaps, label: 'Estimated laps: decimals', type: 'number', default: 1, min: 0, max: 3, group: 'Layout' },
    { key: 'marginalLaps', label: 'Amber when the spare is under', type: 'number', default: 0.5, min: 0, max: 5, step: 0.1, unit: 'laps', group: 'Status' },
    { key: 'okColor', label: 'Makes the finish', type: 'color', default: '#3dd6f5', group: 'Colours' },
    { key: 'marginalColor', label: 'Marginal', type: 'color', default: '#ffb020', group: 'Colours' },
    { key: 'shortColor', label: 'Short (pit needed)', type: 'color', default: '#ff5a3c', group: 'Colours' },
    { key: 'segments', when: (v) => v.showBar !== false, label: 'Bar segments', type: 'number', default: 20, min: 6, max: 40, group: 'Bar' },
  ],
  create(root, ctx) {
    const s = ctx.settings;
    root.classList.add('fuel', 'fl-full', 'energy');
    style(root, '--fl-ok', s.okColor as string);
    style(root, '--fl-marginal', s.marginalColor as string);
    style(root, '--fl-short', s.shortColor as string);

    const top = h('div', 'fl-top');
    const title = h('span', 'fl-label', 'Virtual energy');
    const pill = h('span', 'fl-pill');
    top.append(title, pill);
    const main = h('div', 'fl-main');
    const hero = h('span', 'fl-num fl-hero');
    const heroLabel = h('span', 'fl-label fl-hero-label', 'laps left');
    const tank = h('span', 'fl-num fl-tank');
    main.append(hero, heroLabel, tank);
    const bar = h('div', 'fl-bar');
    const segs: HTMLElement[] = [];
    for (let i = 0; i < (s.segments as number); i++) {
      const seg = h('span', 'fl-seg');
      segs.push(seg);
      bar.append(seg);
    }
    const finish = h('span', 'fl-finish');
    bar.append(finish);
    const stat = (label: string) => {
      const el = h('div', 'fl-stat');
      const val = h('span', 'fl-num fl-val');
      el.append(h('span', 'fl-label', label), val);
      return { el, val };
    };
    const perLap = stat('Per lap');
    const toGo = stat('Laps to go');
    const atFinish = stat('At finish');
    const stats = h('div', 'fl-stats');
    stats.append(perLap.el, toGo.el, atFinish.el);
    const last = stat('Last lap');
    const battery = stat('Battery');
    const toAdd = stat('Add at stop');
    const target = stat('Target / lap');
    const stops = stat('Stops');
    const pitWindow = stat('Pit window');
    const estLaps = stat('Est. laps');
    const extra = h('div', 'fl-stats fl-extra');
    extra.append(last.el, battery.el, toAdd.el, estLaps.el);
    show(estLaps.el, !!s.showEstLaps);
    const plan = h('div', 'fl-stats fl-extra');
    plan.append(target.el, stops.el, pitWindow.el);
    const none = h('div', 'en-none dim');
    root.append(top, main, bar, stats, extra, plan, none);
    show(bar, s.showBar !== false);
    show(battery.el, s.showBattery !== false);
    show(plan, s.showPlan !== false);

    let lastState: State | '' = '';
    let session: Session | null = null;
    let editing = false;
    let live: Energy | null = null;
    const preview = new URLSearchParams(location.search).has('preview');
    return {
      update(topic, data) {
        if (topic === 'session') session = data as Session;
        else if (topic === 'status') editing = !!(data as Status).editMode;
        else if (topic === 'energy') live = data as Energy;
        if (topic !== 'energy' && topic !== 'status') return;
        const e = (editing || preview) && !live?.available ? SAMPLE : live;
        if (!e) return;
        const on = e.available;
        // Nothing to show: hidden (or a short note while placing it).
        toggle(root, 'auto-hidden', !on && s.hideWithout !== false && !!session?.connected);
        for (const el of [main, stats, extra, plan]) show(el, on);
        show(bar, on && s.showBar !== false);
        show(none, !on);
        if (!on) {
          text(none, e.game === 'lmu' ? 'This car races on fuel, not virtual energy' : 'Virtual energy is a Le Mans Ultimate feature (Hypercar, LMGT3)');
          text(pill, '');
          show(pill, false);
          return;
        }
        show(pill, true);
        const level = e.level ?? 0;
        const spare = e.lapsInTank != null && e.lapsRemaining != null ? e.lapsInTank - e.lapsRemaining : null;
        const st: State = spare == null ? 'none' : spare < 0 ? 'short' : spare < (s.marginalLaps as number) ? 'marginal' : 'ok';
        if (st !== lastState) {
          root.classList.remove(`st-${lastState}`);
          root.classList.add(`st-${st}`);
          lastState = st;
          const tone = st === 'short' ? s.shortColor : st === 'marginal' ? s.marginalColor : s.okColor;
          style(root, '--fl-tone-text', onColor(tone as string));
        }
        text(pill, st === 'short' ? `ADD ${pct(e.toAdd)}%` : st === 'marginal' ? 'CLOSE' : st === 'ok' ? 'MAKES IT' : 'MEASURING');
        toggle(pill, 'neutral', st === 'none');
        text(hero, e.lapsInTank == null ? '–' : e.lapsInTank.toFixed(1));
        text(tank, `${pct(level)}%`);
        // The bar: energy left (one segment per lap where it fits), the finish marked.
        const n = segs.length;
        const lapsShown = Math.max(e.lapsRemaining ?? 0, e.lapsInTank ?? 0, 1);
        const per = lapsShown <= n ? 1 : Math.ceil(lapsShown / n);
        const filled = (e.lapsInTank ?? level / 100 * n) / per;
        segs.forEach((seg, i) => style(seg, '--p', String(Math.max(0, Math.min(1, filled - i)))));
        show(finish, e.lapsRemaining != null && e.lapsRemaining / per <= n);
        if (e.lapsRemaining != null) style(finish, 'left', `${Math.min(100, (e.lapsRemaining / per / n) * 100)}%`);
        text(perLap.val, e.perLapAvg == null ? '–' : `${e.perLapAvg.toFixed(2)}%`);
        text(toGo.val, e.lapsRemaining == null ? '–' : e.lapsRemaining.toFixed(1));
        const at = e.perLapAvg != null && e.lapsRemaining != null ? level - e.perLapAvg * e.lapsRemaining : null;
        text(atFinish.val, at == null ? '–' : `${at >= 0 ? '+' : '−'}${Math.abs(at).toFixed(1)}%`);
        text(last.val, e.perLapLast == null ? '–' : `${e.perLapLast.toFixed(2)}%`);
        text(battery.val, e.battery == null ? '–' : `${Math.round(e.battery)}%`);
        text(toAdd.val, e.toAdd == null || e.stops === 0 ? '–' : `${e.toAdd.toFixed(1)}%`);
        text(target.val, e.targetPerLap == null ? '–' : `${e.targetPerLap.toFixed(2)}%`);
        text(stops.val, e.stops == null ? '–' : String(e.stops));
        const est = session?.estLapsLeft;
        text(estLaps.val, est == null ? '–' : `~${est.toFixed(s.estDecimals as number)}`);
        text(pitWindow.val, e.windowOpensLap == null || e.windowClosesLap == null ? '–' : `L${e.windowOpensLap}–L${e.windowClosesLap}`);
      },
    };
  },
};
