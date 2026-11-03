import { h, show, text } from '../lib/dom';
import type { OverlayDef } from '../lib/overlay';
import type { SettingDef } from '../lib/settings';
import type { Session } from '../lib/types';

/** Every flag iRacing can raise (irsdk_Flags), highest priority first. */
export const FLAGS: { id: string; name: string; label: string; detail: string; on: boolean; blink?: boolean }[] = [
  { id: 'red', name: 'Red flag', label: 'RED FLAG', detail: 'Session stopped', on: true },
  { id: 'disqualify', name: 'Disqualified', label: 'DISQUALIFIED', detail: 'Black flag — disqualified', on: true },
  { id: 'black', name: 'Black flag', label: 'BLACK FLAG', detail: 'Serve your penalty in the pits', on: true },
  { id: 'repair', name: 'Meatball (repair)', label: 'REPAIR', detail: 'Car damaged — pit for repairs', on: true },
  { id: 'furled', name: 'Slow down (furled black flag)', label: 'SLOW DOWN', detail: 'Slow-down penalty — give the time back', on: true },
  { id: 'checkered', name: 'Chequered flag', label: 'FINISH', detail: 'Chequered flag', on: true },
  { id: 'white', name: 'White flag', label: 'LAST LAP', detail: 'White flag — final lap', on: true },
  { id: 'cautionWaving', name: 'Caution (waving)', label: 'CAUTION', detail: 'Full course caution', on: true, blink: true },
  { id: 'caution', name: 'Caution', label: 'CAUTION', detail: 'Full course caution', on: true },
  { id: 'yellowWaving', name: 'Waving yellow', label: 'YELLOW', detail: 'Danger — slow down, no overtaking', on: true, blink: true },
  { id: 'randomWaving', name: 'Random waving', label: 'YELLOW', detail: 'Hazard on track', on: true, blink: true },
  { id: 'yellow', name: 'Yellow flag', label: 'YELLOW', detail: 'Caution — no overtaking', on: true },
  { id: 'debris', name: 'Debris (surface) flag', label: 'DEBRIS', detail: 'Slippery surface / debris ahead', on: true },
  { id: 'blue', name: 'Blue flag', label: 'BLUE FLAG', detail: 'Let the faster car pass', on: true, blink: true },
  { id: 'oneLapToGreen', name: 'One lap to green', label: '1 LAP TO GREEN', detail: 'Restart next lap', on: true },
  { id: 'greenHeld', name: 'Green held', label: 'GREEN HELD', detail: 'Green flag held', on: true },
  { id: 'crossed', name: 'Crossed flags (halfway)', label: 'HALFWAY', detail: 'Crossed flags — race halfway', on: true },
  { id: 'tenToGo', name: '10 to go', label: '10 TO GO', detail: 'Ten laps remaining', on: true },
  { id: 'fiveToGo', name: '5 to go', label: '5 TO GO', detail: 'Five laps remaining', on: true },
  { id: 'green', name: 'Green flag', label: 'GREEN', detail: 'Track clear', on: true },
  { id: 'servicible', name: 'Pit service allowed', label: 'PIT OPEN', detail: 'Car can be serviced', on: false },
];

const START = ['startReady', 'startSet', 'startGo'];

export const flags: OverlayDef = {
  kind: 'flags',
  name: 'Flags',
  description: 'A digital flag panel showing the flags being waved right now: green, yellow (waving), caution, blue, white, chequered, red, black, meatball, debris and more, plus the start lights.',
  topics: ['session'],
  size: [260, 150],
  settings: [
    {
      key: 'style',
      label: 'Style',
      type: 'select',
      default: 'led',
      options: [
        ['led', 'LED panel (digital flag)'],
        ['solid', 'Solid flag'],
        ['badge', 'Compact badge'],
      ],
      group: 'Look',
    },
    { key: 'ledSize', when: (v) => v.style === 'led', label: 'LED size', type: 'number', default: 7, min: 3, max: 20, unit: 'px', group: 'Look' },
    { key: 'showLabel', label: 'Flag name', type: 'bool', default: true, group: 'Look' },
    { key: 'showDetail', label: 'What it means', type: 'bool', default: false, group: 'Look' },
    { key: 'blink', label: 'Flash waving flags', type: 'bool', default: true, group: 'Look' },
    { key: 'maxFlags', label: 'Flags shown at once', type: 'number', default: 2, min: 1, max: 4, group: 'Behaviour', help: 'e.g. blue and yellow together' },
    { key: 'hideWhenNone', label: 'Hide when no flag is out', type: 'bool', default: true, group: 'Behaviour' },
    { key: 'greenSeconds', label: 'Show green for', type: 'number', default: 5, min: 0, max: 60, unit: 's', group: 'Behaviour', help: 'After the start or a restart; 0 = keep showing' },
    { key: 'startLights', label: 'Start lights', type: 'bool', default: true, group: 'Behaviour' },
    ...FLAGS.map((f): SettingDef => ({ key: `flag_${f.id}`, label: f.name, type: 'bool', default: f.on, group: 'Flags shown' })),
  ],
  create(root, ctx) {
    const s = ctx.settings;
    root.classList.add('flags', `style-${s.style}`);
    root.style.setProperty('--led', `${s.ledSize}px`);
    const row = h('div', 'flag-row');
    const lights = h('div', 'start-lights');
    const bulbs = Array.from({ length: 5 }, () => {
      const b = h('span', 'bulb');
      lights.append(b);
      return b;
    });
    root.append(row, lights);
    show(lights, false);
    let greenSince = -1;
    let lastKey = '';

    return {
      update(topic, data) {
        if (topic !== 'session') return;
        const d = data as Session;
        const active = new Set(d.flags);
        const now = performance.now() / 1000;
        if (active.has('green')) {
          if (greenSince < 0) greenSince = now;
        } else {
          greenSince = -1;
        }
        const greenExpired = (s.greenSeconds as number) > 0 && greenSince >= 0 && now - greenSince > (s.greenSeconds as number);
        let shown = FLAGS.filter((f) => active.has(f.id) && s[`flag_${f.id}`] && !(f.id === 'green' && greenExpired));
        // A waving flag already covers its steady version.
        if (shown.some((f) => f.id === 'yellowWaving' || f.id === 'randomWaving')) shown = shown.filter((f) => f.id !== 'yellow');
        if (shown.some((f) => f.id === 'cautionWaving')) shown = shown.filter((f) => f.id !== 'caution');
        shown = shown.slice(0, s.maxFlags as number);

        const start = START.find((x) => active.has(x));
        show(lights, !!s.startLights && !!start);
        bulbs.forEach((b, i) => {
          b.className = `bulb${start === 'startGo' ? ' go' : start === 'startSet' || (start === 'startReady' && i < 2) ? ' red' : ''}`;
        });

        const key = shown.map((f) => f.id).join();
        root.classList.toggle('auto-hidden', !!s.hideWhenNone && !shown.length && !(s.startLights && start));
        if (key === lastKey) return;
        lastKey = key;
        row.replaceChildren(
          ...shown.map((f) => {
            const el = h('div', `flag f-${f.id}${f.blink && s.blink ? ' blink' : ''}`);
            const face = h('div', 'face');
            const label = h('div', 'label');
            text(label, f.label);
            show(label, s.showLabel as boolean);
            const detail = h('div', 'detail', f.detail);
            show(detail, s.showDetail as boolean);
            el.append(face, label, detail);
            return el;
          }),
        );
      },
    };
  },
};
