import { h, show } from '../lib/dom';
import type { OverlayDef } from '../lib/overlay';
import type { CarInfo, Player } from '../lib/types';
import { shiftLightSettings, shiftLights } from './shiftLights';

export const shiftlights: OverlayDef = {
  kind: 'shiftlights',
  name: 'Shift lights',
  description: 'The car’s rev lights on their own, with the RPM if you like. The same lights as in Driver inputs: the car’s own pattern and per-gear shift points where known.',
  topics: ['player', 'car'],
  size: [420, 70],
  regions: [['.shift-lights', 'shiftLayout'], ['.rpm', 'showRpm']],
  commonDefaults: { autoHeight: true },
  settings: [
    { key: 'showRpm', label: 'RPM', type: 'bool', default: false, group: 'Layout', help: 'The engine RPM with the lights. It turns the “shift now” colour at the car’s shift point' },
    {
      key: 'rpmPosition', when: (v) => !!v.showRpm,
      label: 'RPM goes',
      type: 'select',
      default: 'below',
      options: [
        ['below', 'Under the lights'],
        ['above', 'Above the lights'],
        ['left', 'Left of the lights'],
        ['right', 'Right of the lights'],
      ],
      group: 'Layout',
    },
    ...shiftLightSettings('Lights'),
  ],
  create(root, ctx) {
    const s = ctx.settings;
    const pos = s.rpmPosition as string;
    root.classList.add('shiftlights', `rpm-${pos}`);
    const sl = shiftLights(s, () => true, () => !!s.showRpm);
    show(sl.rpm, !!s.showRpm);
    // Until the car is known (the lights need its shift points): a short note.
    const wait = h('div', 'sl-wait dim', 'Shift lights appear once you are in a car');
    const row = h('div', 'sl-row');
    if (pos === 'above' || pos === 'left') row.append(sl.rpm, sl.lights);
    else row.append(sl.lights, sl.rpm);
    root.append(row, wait);
    let gear = 1;
    return {
      update(topic, data) {
        if (topic === 'car') {
          sl.setCar(data as CarInfo);
          show(wait, !sl.hasPlan(gear));
          return;
        }
        if (topic !== 'player') return;
        const p = data as Player;
        gear = p.gear;
        sl.draw(p);
        show(wait, !sl.hasPlan(p.gear) && !s.showRpm);
      },
    };
  },
};
