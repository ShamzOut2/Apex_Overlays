// Start lights: a slim gantry of light pods that only appears for the start. Red lights
// come on (ready), all red (set), then green, or lights out, for the go, and it hides again.

import { h, style, text, toggle } from '../lib/dom';
import type { OverlayDef } from '../lib/overlay';
import type { Player, Status } from '../lib/types';

type Phase = '' | 'ready' | 'set' | 'go';

export const startLights: OverlayDef = {
  kind: 'startLights',
  name: 'Start lights',
  description: 'Race start lights: a row of light pods that appears only while the red lights are on, all red when set, green (or lights out) for the go, then it disappears. Hidden for rolling starts, where iRacing shows no lights.',
  topics: ['player', 'status'],
  size: [400, 92],
  // The gantry carries its own dark capsule, so the window behind it is see-through.
  commonDefaults: { bgOpacity: 0 },
  settings: [
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
    { key: 'showLabel', label: 'READY / SET / GO', type: 'bool', default: true, group: 'Lights' },
    { key: 'redColor', label: 'Red', type: 'color', default: '#ff2a3d', group: 'Colours' },
    { key: 'greenColor', label: 'Green', type: 'color', default: '#22e07a', group: 'Colours' },
  ],
  create(root, ctx) {
    const s = ctx.settings;
    const preview = new URLSearchParams(location.search).has('preview');
    root.classList.add('start-lights');
    style(root, '--sl-red', s.redColor as string);
    style(root, '--sl-green', s.greenColor as string);
    const n = Math.max(3, Math.min(5, Math.round(s.lights as number)));
    const gantry = h('div', 'sl-gantry');
    const pods = Array.from({ length: n }, () => {
      const pod = h('div', 'sl-pod');
      pod.append(h('span', 'sl-lamp'), h('span', 'sl-lamp'));
      gantry.append(pod);
      return pod;
    });
    const label = h('div', 'sl-label');
    toggle(label, 'hidden-label', !s.showLabel);
    root.append(gantry, label);

    let phase: Phase = '';
    let editing = false;
    let goAt = 0;
    let timer: ReturnType<typeof setTimeout> | null = null;

    const draw = () => {
      // In edit mode (and the settings preview) a sample shows, so it can be placed.
      const sample = (editing || preview) && !phase;
      const shown: Phase = sample ? 'set' : phase;
      const goLeft = (s.goSeconds as number) * 1000 - (performance.now() - goAt);
      const visible = shown === 'ready' || shown === 'set' || (shown === 'go' && goLeft > 0);
      toggle(root, 'auto-hidden', !visible);
      // Ready: the first lights red (three of five); set: all red; go: green, or all out.
      const red = shown === 'ready' ? Math.ceil(n * 0.6) : shown === 'set' ? n : 0;
      const green = shown === 'go' && s.goStyle !== 'out';
      pods.forEach((p, i) => {
        toggle(p, 'red', i < red);
        toggle(p, 'green', green);
      });
      toggle(root, 'go', shown === 'go');
      text(label, shown === 'go' ? 'GO' : shown === 'set' ? 'SET' : shown === 'ready' ? 'READY' : '');
      if (timer) clearTimeout(timer);
      timer = shown === 'go' && goLeft > 0 ? setTimeout(draw, goLeft + 20) : null;
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
        const next = ((data as Player).start ?? '') as Phase;
        if (next === phase) return;
        if (next === 'go') goAt = performance.now();
        phase = next;
        draw();
      },
    };
  },
};
