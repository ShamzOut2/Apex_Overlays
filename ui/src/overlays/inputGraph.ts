import { h } from '../lib/dom';
import type { OverlayDef } from '../lib/overlay';
import type { Player } from '../lib/types';
import { inputTrace, TRACE_SETTINGS } from './inputTrace';

/** The input trace on its own, to place apart from the pedal bars and wheel. */
export const inputGraph: OverlayDef = {
  kind: 'inputGraph',
  name: 'Input graph',
  description:
    'The scrolling throttle, brake and clutch trace on its own, so it can sit anywhere on screen. Switch the graph off in Driver inputs if you use this instead.',
  topics: ['player'],
  size: [360, 110],
  settings: [
    ...TRACE_SETTINGS,
    { key: 'showClutch', label: 'Clutch', type: 'bool', default: true, group: 'Trace' },
    { key: 'background', label: 'Darker background behind the graph', type: 'bool', default: true, group: 'Look' },
    { key: 'throttleColor', label: 'Throttle', type: 'color', default: '#3ddc84', group: 'Colours' },
    { key: 'brakeColor', label: 'Brake', type: 'color', default: '#ff4d4d', group: 'Colours' },
    { key: 'clutchColor', label: 'Clutch', type: 'color', default: '#4da3ff', group: 'Colours' },
    { key: 'absColor', label: 'ABS', type: 'color', default: '#ffd84d', group: 'Colours' },
    { key: 'steeringColor', label: 'Steering', type: 'color', default: '#e6e9ef', group: 'Colours' },
  ],
  commonDefaults: { hideWhenNotDriving: true },
  create(root, ctx) {
    const s = ctx.settings;
    root.classList.add('input-graph');
    root.classList.toggle('plain', !s.background);
    const canvas = h('canvas', 'trace');
    root.append(canvas);
    const trace = inputTrace(canvas, s, () => !!s.showClutch);
    return {
      update(topic, data) {
        if (topic === 'player') trace.push(data as Player);
      },
    };
  },
};
