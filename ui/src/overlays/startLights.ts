// Start lights: a row of light pods that only appears for the start. Optionally a countdown
// to the start first, as a warning; then red lights come on (ready), all red (set), green
// (or lights out) for the go, and it hides again. Rolling starts (no lights in iRacing): amber
// on the pace laps, pulsing on the last one, green when the green flag drops.

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
  size: [400, 110],
  // Nothing behind the lights: the window is see-through.
  commonDefaults: { bgOpacity: 0 },
  settings: [
    { key: 'countdown', label: 'Countdown before the start', type: 'bool', default: true, group: 'Countdown', help: 'A warning that the race is about to start, before the lights come on' },
    { key: 'countdownFrom', when: (v) => v.countdown !== false, label: 'Count down from', type: 'number', default: 10, min: 3, max: 60, unit: 's', group: 'Countdown' },
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
    { key: 'showLabel', label: 'COUNTDOWN / READY / SET / GO wording', type: 'bool', default: true, group: 'Lights' },
    { key: 'rolling', label: 'Show rolling starts', type: 'bool', default: true, group: 'Rolling starts', help: 'Amber on the pace laps, pulsing on the last one ("1 lap to green"), green when the green flag drops' },
    { key: 'redColor', label: 'Red', type: 'color', default: '#ff2a3d', group: 'Colours' },
    { key: 'greenColor', label: 'Green', type: 'color', default: '#22e07a', group: 'Colours' },
    { key: 'paceColor', when: (v) => v.rolling !== false, label: 'Pace laps', type: 'color', default: '#ffb020', group: 'Colours' },
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
    const gantry = h('div', 'sl-gantry');
    const pods = Array.from({ length: n }, () => {
      const pod = h('div', 'sl-pod');
      pod.append(h('span', 'sl-lamp'), h('span', 'sl-lamp'));
      gantry.append(pod);
      return pod;
    });
    const count = h('div', 'sl-count');
    const label = h('div', 'sl-label');
    toggle(label, 'hidden-label', !s.showLabel);
    root.append(count, gantry, label);

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

    const draw = () => {
      // In edit mode (and the settings preview) a sample shows, so it can be placed.
      const sample = (editing || preview) && !phase && startIn == null;
      const lights: Phase = sample ? 'set' : phase;
      const now = performance.now();
      const goLeft = (s.goSeconds as number) * 1000 - (now - goAt);
      const counting = !!s.countdown && !lights && startIn != null && startIn <= (s.countdownFrom as number);
      // Rolling start: amber while pacing (pulsing on the last lap), green for the go.
      const pace = !!s.rolling && !lights && (rolling === 'pacing' || rolling === 'oneToGreen') ? rolling : '';
      const rollGo = !!s.rolling && !lights && rollingGo && goLeft > 0;
      const getReady = !!s.countdown && !lights && !counting && !pace && !rollGo && countdownEnded > 0 && gridding && now - countdownEnded < GET_READY_MAX;
      const lit = lights === 'ready' || lights === 'set' || (lights === 'go' && goLeft > 0);
      toggle(root, 'auto-hidden', !(lit || counting || getReady || pace || rollGo));
      // The countdown takes the place of the lights; "GET READY" shows them, still dark.
      show(count, counting);
      show(gantry, !counting);
      toggle(root, 'counting', counting);
      if (counting) text(count, String(Math.max(1, Math.ceil(startIn as number))));
      // Ready: the first lights red (three of five); set: all red; go: green, or all out.
      const red = lights === 'ready' ? Math.ceil(n * 0.6) : lights === 'set' ? n : 0;
      const green = (lights === 'go' && s.goStyle !== 'out') || rollGo;
      pods.forEach((p, i) => {
        toggle(p, 'red', i < red);
        toggle(p, 'green', green);
        toggle(p, 'amber', !!pace);
      });
      toggle(root, 'go', lights === 'go' || rollGo);
      toggle(root, 'pacing', !!pace);
      toggle(root, 'pulse', pace === 'oneToGreen');
      const words = counting ? 'RACE STARTS IN' : pace === 'oneToGreen' ? '1 LAP TO GREEN' : pace ? 'ROLLING START' : rollGo ? 'GO' : getReady ? 'GET READY' : lights === 'go' ? 'GO' : lights === 'set' ? 'SET' : lights === 'ready' ? 'READY' : '';
      text(label, words);
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
        const shownBefore = startIn == null ? null : Math.ceil(startIn);
        const roll = (p.rolling ?? '') as Rolling;
        const changed = next !== phase || (left == null) !== (startIn == null) || (left != null && Math.ceil(left) !== shownBefore) || !!p.gridding !== gridding || roll !== rolling;
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
