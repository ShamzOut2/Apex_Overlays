// Cars around you, seen from above: your car in the middle, the others where they are. Le Mans
// Ultimate only, as it says where every car is (iRacing does not: its spotter overlay is there).

import { fitCanvas, h, toggle } from '../lib/dom';
import type { OverlayDef } from '../lib/overlay';
import type { Radar, Status } from '../lib/types';

/** Shown while placing the overlay: a car alongside on the right, one close behind, one ahead. */
const SAMPLE: Radar = {
  active: true,
  range: 40,
  carLength: 4.8,
  cars: [
    { idx: 1, x: 3.1, y: 1.2, color: '#ff5a36', number: '3' },
    { idx: 2, x: -0.4, y: -7.5, color: '#3aa0ff', number: '27' },
    { idx: 3, x: 0.6, y: 15, color: '#ffda59', number: '12' },
  ],
};

export const radar: OverlayDef = {
  kind: 'radar',
  fill: true,
  name: 'Radar',
  game: 'lmu',
  description:
    'The cars around you, seen from above, exactly where they are: your car in the middle, the others amber when close and red when alongside. Hides itself until a car is near. Le Mans Ultimate only, as it is the game that says where every car is; in iRacing, use the Spotter.',
  topics: ['radar', 'status'],
  size: [220, 260],
  settings: [
    { key: 'range', label: 'Distance shown ahead and behind', type: 'number', default: 20, min: 8, max: 40, unit: 'm', group: 'View' },
    { key: 'hideWithin', label: 'Show only when a car is within', type: 'number', default: 12, min: 0, max: 40, unit: 'm', group: 'View', help: '0: always shown' },
    { key: 'showGuides', label: 'Distance lines (every 10 m)', type: 'bool', default: true, group: 'View' },
    { key: 'showNumbers', label: 'Car numbers', type: 'bool', default: false, group: 'View' },
    { key: 'colorBy', label: 'Other cars', type: 'select', default: 'class', options: [['class', 'Class colour'], ['single', 'One colour']], group: 'Colours' },
    { key: 'carColor', when: (v) => v.colorBy === 'single', label: 'Car colour', type: 'color', default: '#d8dde6', group: 'Colours' },
    { key: 'closeColor', label: 'Close (within a car length or two)', type: 'color', default: '#ffb020', group: 'Colours' },
    { key: 'alongsideColor', label: 'Alongside (overlapping)', type: 'color', default: '#ff3b30', group: 'Colours' },
  ],
  create(root, ctx) {
    const s = ctx.settings;
    root.classList.add('radar');
    const canvas = h('canvas', 'rd-canvas') as HTMLCanvasElement;
    root.append(canvas);
    const g = canvas.getContext('2d')!;
    const dpr = () => window.devicePixelRatio || 1;

    const rect = (x: number, y: number, w: number, l: number, r: number) => {
      g.beginPath();
      g.roundRect(x - w / 2, y - l / 2, w, l, r);
    };

    let last: Radar | null = null;
    let live: Radar | null = null;
    let editing = false;
    const preview = new URLSearchParams(location.search).has('preview');
    const draw = () => {
      fitCanvas(canvas);
      const W = canvas.width;
      const H = canvas.height;
      g.clearRect(0, 0, W, H);
      const d = last;
      if (!d) return;
      const range = s.range as number;
      const scale = (H / 2 - 6 * dpr()) / range;
      const cx = W / 2;
      const cy = H / 2;
      const carW = 2.0 * scale;
      const carL = d.carLength * scale;
      if (s.showGuides !== false) {
        g.strokeStyle = 'rgba(255,255,255,0.10)';
        g.lineWidth = dpr();
        for (let m = 10; m <= range; m += 10) {
          for (const y of [cy - m * scale, cy + m * scale]) {
            g.beginPath();
            g.moveTo(cx - W * 0.32, y);
            g.lineTo(cx + W * 0.32, y);
            g.stroke();
          }
        }
        // The lanes either side of you.
        g.strokeStyle = 'rgba(255,255,255,0.06)';
        for (const x of [cx - 1.6 * scale, cx + 1.6 * scale]) {
          g.beginPath();
          g.moveTo(x, 0);
          g.lineTo(x, H);
          g.stroke();
        }
      }
      // You.
      const theme = getComputedStyle(root);
      rect(cx, cy, carW, carL, carW * 0.3);
      g.fillStyle = theme.getPropertyValue('--player').trim() || '#ffd400';
      g.fill();
      g.font = `700 ${Math.max(9, 11 * dpr())}px ${theme.getPropertyValue('--font') || 'sans-serif'}`;
      g.textAlign = 'center';
      g.textBaseline = 'middle';
      for (const c of d.cars) {
        if (Math.abs(c.y) > range + d.carLength || Math.abs(c.x) > range) continue;
        const x = cx + c.x * scale;
        const y = cy - c.y * scale;
        const overlap = Math.abs(c.y) < d.carLength && Math.abs(c.x) < 4.5;
        const close = Math.abs(c.y) < d.carLength * 2 && Math.abs(c.x) < 4.5;
        const base = s.colorBy === 'single' ? (s.carColor as string) : c.color || '#d8dde6';
        g.globalAlpha = Math.max(0.35, 1 - Math.hypot(c.x, c.y) / (range * 1.3));
        rect(x, y, carW, carL, carW * 0.3);
        g.fillStyle = base;
        g.fill();
        if (overlap || close) {
          g.lineWidth = 2.5 * dpr();
          g.strokeStyle = (overlap ? s.alongsideColor : s.closeColor) as string;
          g.stroke();
        }
        if (s.showNumbers && c.number) {
          g.fillStyle = '#000';
          g.fillText(c.number, x, y);
        }
        g.globalAlpha = 1;
      }
    };
    new ResizeObserver(draw).observe(canvas);

    return {
      update(topic, data) {
        if (topic === 'status') editing = !!(data as Status).editMode;
        else if (topic === 'radar') live = data as Radar;
        else return;
        const sample = (editing || preview) && !live?.cars.length;
        last = sample ? SAMPLE : live;
        const within = s.hideWithin as number;
        const near = !!last && last.active && last.cars.some((c) => Math.hypot(c.x, c.y) <= within);
        toggle(root, 'auto-hidden', !sample && within > 0 && !near);
        draw();
      },
    };
  },
};
