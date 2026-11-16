import { h, style } from '../lib/dom';
import type { OverlayDef } from '../lib/overlay';
import type { Spotter, Status } from '../lib/types';

/** Shown while positioning the overlay: a car half a length ahead on your left. */
const SAMPLE: Spotter = { state: 'left', left: [2.2], right: [] };

export const spotter: OverlayDef = {
  kind: 'spotter',
  name: 'Spotter',
  game: 'iracing',
  description:
    'Left and right spotter bars that stay invisible until a car is alongside you. A block shows where the other car overlaps yours, and the bar turns red when you are three wide or have two cars on one side. iRacing only; in Le Mans Ultimate, use the Radar.',
  topics: ['spotter', 'status'],
  size: [220, 200],
  settings: [
    {
      key: 'style',
      label: 'Style',
      type: 'select',
      default: 'blocks',
      options: [
        ['blocks', 'Bars showing where the car overlaps yours'],
        ['full', 'Whole side lights up'],
      ],
      group: 'Look',
    },
    { key: 'barWidth', label: 'Bar width', type: 'number', default: 16, min: 4, max: 80, unit: 'px', group: 'Look' },
    { key: 'gap', label: 'Gap between the bars', type: 'number', default: 90, min: 0, max: 600, unit: 'px', group: 'Look' },
    { key: 'showCar', label: 'Your car between the bars', type: 'bool', default: true, group: 'Look' },
    { key: 'color', label: 'Car alongside', type: 'color', default: '#ffb400', group: 'Look' },
    { key: 'dangerColor', label: 'Three wide / two cars on one side', type: 'color', default: '#ff3b30', group: 'Look' },
    { key: 'glow', label: 'Glow', type: 'bool', default: true, group: 'Look' },
    { key: 'holdMs', label: 'Keep showing after clear', type: 'number', default: 250, min: 0, max: 3000, step: 50, unit: 'ms', group: 'Behaviour', help: 'Stops the bars flickering when a car hovers at the edge' },
    { key: 'carLength', label: 'Car length', type: 'number', default: 4.6, min: 2.5, max: 8, step: 0.1, unit: 'm', group: 'Behaviour', help: 'The bars cover three car lengths: one ahead, yours and one behind' },
  ],
  commonDefaults: { hideWhenNotDriving: true },
  create(root, ctx) {
    const s = ctx.settings;
    const preview = new URLSearchParams(location.search).has('preview');
    root.classList.add('spotter', 'transparent');
    root.classList.toggle('glow', !!s.glow);
    style(root, '--sp-color', s.color as string);
    style(root, '--sp-danger', s.dangerColor as string);
    style(root, '--sp-bar', `${s.barWidth}px`);
    style(root, '--sp-gap', `${s.gap}px`);
    const L = s.carLength as number;
    const range = L * 1.5;

    const wrap = h('div', 'sp-wrap');
    const side = () => {
      const el = h('div', 'sp-side');
      const blocks = [h('div', 'sp-block'), h('div', 'sp-block')];
      el.append(...blocks);
      return { el, blocks };
    };
    const left = side();
    const right = side();
    const car = h('div', 'sp-car');
    car.append(h('div', 'sp-glass'));
    style(car, 'height', `${(L / (2 * range)) * 100}%`);
    if (!s.showCar) car.hidden = true;
    wrap.append(left.el, car, right.el);
    root.append(wrap);

    let live: Spotter | null = null;
    let editing = false;
    let shown: Spotter | null = null;
    let clearTimer = 0;

    /** Blocks for one side: one per car with a known overlap, else the whole bar. */
    const paint = (sd: ReturnType<typeof side>, on: boolean, offsets: number[], danger: boolean) => {
      sd.el.classList.toggle('on', on);
      sd.el.classList.toggle('danger', on && danger);
      const full = s.style === 'full' || !offsets.length;
      sd.blocks.forEach((b, i) => {
        const d = offsets[i];
        const visible = on && (full ? i === 0 : d != null);
        b.hidden = !visible;
        if (!visible) return;
        // Moved with a transform (no new layout each tick); a % of the block's own height.
        if (full) {
          style(b, 'height', '100%');
          style(b, 'transform', 'translateY(0%)');
        } else {
          // Positive = ahead of you = higher up the bar.
          const hf = L / (2 * range);
          const top = Math.max(0, Math.min(1, (range - (d + L / 2)) / (2 * range)));
          style(b, 'height', `${hf * 100}%`);
          style(b, 'transform', `translateY(${((top / hf) * 100).toFixed(2)}%)`);
        }
      });
    };

    const render = (sp: Spotter | null) => {
      const st = sp?.state ?? '';
      const l = st === 'left' || st === 'left2' || st === 'both';
      const r = st === 'right' || st === 'right2' || st === 'both';
      const danger = st === 'both' || st === 'left2' || st === 'right2';
      paint(left, l, sp?.left ?? [], danger);
      paint(right, r, sp?.right ?? [], danger);
      car.classList.toggle('danger', danger);
      root.classList.toggle('auto-hidden', !st);
    };

    const update = () => {
      const want = (editing || preview) && !live?.state ? SAMPLE : live;
      if (want?.state) {
        clearTimeout(clearTimer);
        clearTimer = 0;
        shown = want;
        render(shown);
      } else if (shown && !clearTimer) {
        clearTimer = window.setTimeout(() => {
          clearTimer = 0;
          shown = null;
          render(null);
        }, s.holdMs as number);
      } else if (!shown) {
        render(null);
      }
    };

    return {
      update(topic, data) {
        if (topic === 'spotter') live = data as Spotter;
        if (topic === 'status') editing = !!(data as Status).editMode;
        update();
      },
    };
  },
};
