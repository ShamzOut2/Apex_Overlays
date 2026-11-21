// Start lights: a gantry that only appears for the start. Optionally a countdown to the start
// first, as a warning; then red lights come on (ready), all red (set), green (or lights out)
// for the go, and it hides again. Rolling starts (no lights in iRacing): amber on the pace
// laps, flashing on the last one, green when the green flag drops.
//
// Each column has two red lenses and one below for green (go) and amber (pace laps); each
// lens is a cluster of LEDs behind a hooded visor. Or a compact strip, one lens per column.
// The whole thing scales to fit the window.

import { h, show, style, text, toggle } from '../lib/dom';
import type { OverlayDef } from '../lib/overlay';
import type { Player, Status } from '../lib/types';

type Phase = '' | 'ready' | 'set' | 'go';
type Rolling = '' | 'pacing' | 'oneToGreen' | 'green';

/** How long "GET READY" waits for the lights after the countdown (rolling starts have none). */
const GET_READY_MAX = 20_000;

export const startLights: OverlayDef = {
  kind: 'startLights',
  name: 'Start lights',
  description:
    'Start lights that show only for the start: an optional countdown, red lights, then green. For rolling starts: amber on the pace laps, green when the flag drops.',
  topics: ['player', 'status'],
  size: [340, 230],
  // Nothing behind the lights: the window is see-through.
  commonDefaults: { bgOpacity: 0 },
  settings: [
    { key: 'countdown', label: 'Countdown before the start', type: 'bool', default: true, group: 'Countdown', help: 'A warning that the race is about to start, before the lights come on' },
    { key: 'countdownFrom', when: (v) => v.countdown !== false, label: 'Count down from', type: 'number', default: 10, min: 3, max: 60, unit: 's', group: 'Countdown' },
    {
      key: 'layout',
      label: 'Look',
      type: 'select',
      default: 'gantry',
      options: [
        ['gantry', 'Gantry (two reds and a green per column)'],
        ['strip', 'Compact strip (one lens per column)'],
      ],
      group: 'Lights',
    },
    { key: 'lights', label: 'Lights', type: 'number', default: 5, min: 3, max: 5, group: 'Lights' },
    {
      key: 'goStyle',
      label: 'For the go',
      type: 'select',
      default: 'green',
      options: [
        ['green', 'Green lights'],
        ['out', 'Lights out'],
      ],
      group: 'Lights',
    },
    { key: 'goSeconds', label: 'Keep showing after the go', type: 'number', default: 2, min: 0, max: 10, step: 0.5, unit: 's', group: 'Lights' },
    { key: 'showLabel', label: 'Status (READY / SET / GO)', type: 'bool', default: true, group: 'Lights', help: 'The strip under the gantry, or the words beside the compact strip' },
    { key: 'showBrand', when: (v) => v.showLabel !== false && v.layout !== 'strip', label: 'Apex badge on the status strip', type: 'bool', default: true, group: 'Lights' },
    { key: 'rolling', label: 'Show rolling starts', type: 'bool', default: true, group: 'Rolling starts', help: 'Amber on the pace laps, flashing on the last one ("1 lap to green"), green when the green flag drops' },
    { key: 'redColor', label: 'Red', type: 'color', default: '#ff5442', group: 'Colours' },
    { key: 'greenColor', label: 'Green', type: 'color', default: '#46ff86', group: 'Colours' },
    { key: 'paceColor', when: (v) => v.rolling !== false, label: 'Pace laps', type: 'color', default: '#ffb22a', group: 'Colours' },
    { key: 'countColor', when: (v) => v.countdown !== false, label: 'Countdown', type: 'color', default: '#ffffff', group: 'Colours' },
  ],
  create(root, ctx) {
    const s = ctx.settings;
    const preview = new URLSearchParams(location.search).has('preview');
    root.classList.add('start-lights');
    style(root, '--sl-red', s.redColor as string);
    style(root, '--sl-green', s.greenColor as string);
    style(root, '--sl-count', s.countColor as string);
    style(root, '--sl-amber', s.paceColor as string);
    const n = Math.max(3, Math.min(5, Math.round(s.lights as number)));
    const compact = s.layout === 'strip';
    toggle(root, 'sl-compact', compact);
    /** A lens: LEDs, the glow when lit, the glass, under a visor. */
    const lens = (kind: 'r' | 'b') => {
      const cell = h('div', 'sl-cell');
      const l = h('div', `sl-lens ${kind}`);
      l.append(h('div', 'sl-leds'), h('div', 'sl-glow'), h('div', 'sl-glass'));
      cell.append(l);
      return { cell, l };
    };
    const stage = h('div', 'sl-stage');
    const count = h('div', 'sl-count');
    const status = h('span', 'sl-status');
    const num = h('span', 'sl-num');
    /** Per column: its red lenses and its green/amber one (the same lens in the compact strip). */
    const columns: { reds: HTMLElement[]; low: HTMLElement }[] = [];
    let strip: HTMLElement;
    if (compact) {
      const mini = h('div', 'sl-mini');
      const lenses = h('div', 'sl-lenses');
      for (let i = 0; i < n; i++) {
        const x = lens('r');
        x.cell.classList.add('sm');
        lenses.append(x.cell);
        columns.push({ reds: [x.l], low: x.l });
      }
      lenses.append(count);
      status.classList.add('sl-mini-label');
      mini.append(lenses, status);
      strip = status;
      stage.append(mini);
    } else {
      const beam = h('div', 'sl-beam');
      for (const b of ['tl', 'tr', 'bl', 'br']) beam.append(h('span', `sl-bolt ${b}`));
      for (let i = 0; i < n; i++) {
        const pod = h('div', 'sl-pod');
        const a = lens('r');
        const b = lens('r');
        const c = lens('b');
        pod.append(a.cell, b.cell, c.cell);
        beam.append(pod);
        columns.push({ reds: [a.l, b.l], low: c.l });
      }
      beam.append(count);
      const brand = h('span', 'sl-brand');
      brand.append(h('span', 'sl-dot'), 'Apex');
      show(brand, s.showBrand !== false);
      strip = h('div', 'sl-strip');
      strip.append(brand, status, num);
      stage.append(beam, strip);
    }
    show(strip, !!s.showLabel);
    root.append(stage);

    // Scaled to fit the window: its width and height, or only its width when the window's
    // height follows the overlay (and not at all when its width does too).
    const fit = () => {
      stage.style.fontSize = '';
      if (root.classList.contains('auto-width')) return;
      const cs = getComputedStyle(root);
      const roomW = root.clientWidth - parseFloat(cs.paddingLeft) - parseFloat(cs.paddingRight);
      const roomH = root.clientHeight - parseFloat(cs.paddingTop) - parseFloat(cs.paddingBottom);
      const needW = stage.offsetWidth;
      const needH = stage.offsetHeight;
      if (roomW <= 0 || needW <= 0 || needH <= 0) return;
      const k = root.classList.contains('auto-height') ? roomW / needW : Math.min(roomW / needW, roomH / needH);
      stage.style.fontSize = `${Math.floor(k * 1000) / 10}%`;
    };
    new ResizeObserver(fit).observe(root);
    void document.fonts?.ready.then(fit);
    let shownWords = '';

    let phase: Phase = '';
    let editing = false;
    let goAt = 0;
    let startIn: number | null = null;
    let gridding = false;
    let rolling: Rolling = '';
    // The green flag just dropped after the pace laps (not just "green" mid-race).
    let rollingGo = false;
    // When the countdown ran out (the grid forms next), for "GET READY".
    let countdownEnded = 0;
    let timer: ReturnType<typeof setTimeout> | null = null;
    /** No start under way: the player topic is only looked at a few times a second. */
    let idle = false;

    /** The whole seconds left as iRacing's own timer shows them: cut down, not rounded up
     *  (9.4 s left reads 9, as 0:09 does in the game, not 10). */
    const shownSecond = (secs: number) => Math.max(0, Math.floor(secs));

    const draw = () => {
      // In edit mode (and the settings preview) a sample shows, so it can be placed.
      const sample = (editing || preview) && !phase && startIn == null;
      const lights: Phase = sample ? 'set' : phase;
      const now = performance.now();
      const goLeft = (s.goSeconds as number) * 1000 - (now - goAt);
      // Starts as the shown number reaches the chosen one, so that number shows for a whole second.
      const counting = !!s.countdown && !lights && startIn != null && startIn < (s.countdownFrom as number) + 1;
      // Rolling start: amber while pacing (pulsing on the last lap), green for the go.
      const pace = !!s.rolling && !lights && (rolling === 'pacing' || rolling === 'oneToGreen') ? rolling : '';
      const rollGo = !!s.rolling && !lights && rollingGo && goLeft > 0;
      const getReady = !!s.countdown && !lights && !counting && !pace && !rollGo && countdownEnded > 0 && gridding && now - countdownEnded < GET_READY_MAX;
      const lit = lights === 'ready' || lights === 'set' || (lights === 'go' && goLeft > 0);
      toggle(root, 'auto-hidden', !(lit || counting || getReady || pace || rollGo));
      // The countdown shows in the gantry, over its dark lenses; "GET READY" shows them, still dark.
      show(count, counting);
      toggle(root, 'counting', counting);
      if (counting) text(count, String(shownSecond(startIn as number)));
      // Ready: the first lights red (three of five); set: all red; go: green, or all out.
      const red = lights === 'ready' ? Math.ceil(n * 0.6) : lights === 'set' ? n : 0;
      const green = (lights === 'go' && s.goStyle !== 'out') || rollGo;
      columns.forEach((c, i) => {
        for (const l of c.reds) toggle(l, 'on', i < red);
        toggle(c.low, 'go', green);
        toggle(c.low, 'amb', !!pace);
      });
      toggle(root, 'go', lights === 'go' || rollGo);
      toggle(root, 'pacing', !!pace);
      toggle(root, 'pulse', pace === 'oneToGreen');
      const words = counting
        ? 'Race starts in'
        : pace === 'oneToGreen'
          ? '1 lap to green'
          : pace
            ? 'Rolling start'
            : rollGo
              ? 'Green flag'
              : getReady
                ? 'Get ready'
                : lights === 'go'
                  ? s.goStyle === 'out'
                    ? 'Lights out'
                    : 'Go'
                  : lights === 'set'
                    ? 'Set'
                    : lights === 'ready'
                      ? 'Ready'
                      : '';
      text(status, words);
      text(num, lights ? `${red}/${n}` : counting ? 'sec' : '');
      if (words !== shownWords) {
        shownWords = words;
        if (compact) fit();
      }
      if (timer) clearTimeout(timer);
      timer = null;
      if ((lights === 'go' || rollGo) && goLeft > 0) timer = setTimeout(draw, goLeft + 20);
      else if (getReady) timer = setTimeout(draw, GET_READY_MAX - (now - countdownEnded) + 20);
    };
    draw();

    return {
      update(topic, data) {
        if (topic === 'status') {
          editing = !!(data as Status).editMode;
          draw();
          return;
        }
        if (topic !== 'player') return;
        const p = data as Player;
        const next = (p.start ?? '') as Phase;
        const left = p.raceStartIn ?? null;
        // The countdown has just run out: the grid forms next.
        if (left == null && startIn != null) countdownEnded = performance.now();
        if (next) countdownEnded = 0;
        const shownBefore = startIn == null ? null : shownSecond(startIn);
        const roll = (p.rolling ?? '') as Rolling;
        const changed = next !== phase || (left == null) !== (startIn == null) || (left != null && shownSecond(left) !== shownBefore) || !!p.gridding !== gridding || roll !== rolling;
        if (next === 'go' && phase !== 'go') goAt = performance.now();
        // Green straight after the pace laps is the start; green at any other time is not.
        if (roll === 'green' && (rolling === 'pacing' || rolling === 'oneToGreen')) {
          rollingGo = true;
          goAt = performance.now();
        } else if (roll !== 'green') rollingGo = false;
        rolling = roll;
        phase = next;
        startIn = left;
        gridding = !!p.gridding;
        if (changed) draw();
        // Mid-race nothing is coming: a few looks a second until a start, grid or countdown
        // shows up (always seconds before the lights), then every tick for exact timing.
        const quiet = !next && left == null && !gridding && (!roll || roll === 'green') && !rollingGo;
        if (quiet !== idle) {
          idle = quiet;
          ctx.limit('player', quiet ? 5 : 0);
        }
      },
    };
  },
};
