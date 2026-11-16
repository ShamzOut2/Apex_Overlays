import { h, show, text, toggle } from '../lib/dom';
import * as f from '../lib/format';
import type { OverlayDef } from '../lib/overlay';
import { NAME_STYLES } from '../lib/settings';
import { chip } from '../lib/table';
import type { Battle, BattleCar } from '../lib/types';
import { NUMBER_COLOR_SETTING, numberChip } from './common';

export const battle: OverlayDef = {
  kind: 'battle',
  name: 'Battle box',
  description: 'The car ahead and behind in your class: gap, whether it is closing and how fast, laps until contact and lap time comparison.',
  topics: ['battle'],
  size: [360, 150],
  settings: [
    {
      key: 'mode',
      label: 'Battle against',
      type: 'select',
      default: 'overall',
      options: [
        ['overall', 'Car ahead/behind in the race, any class'],
        ['class', 'Car ahead/behind in my class'],
        ['track', 'Nearest car on track (any class, incl. lapped cars)'],
      ],
      group: 'Cars',
    },
    { key: 'showClass', label: 'Show the other car’s class', type: 'bool', default: true, group: 'Elements' },
    { ...NUMBER_COLOR_SETTING, group: 'Format' },
    { key: 'showAhead', label: 'Car ahead', type: 'bool', default: true, group: 'Elements' },
    { key: 'showBehind', label: 'Car behind', type: 'bool', default: true, group: 'Elements' },
    { key: 'showTrend', label: 'Gap trend per lap', type: 'bool', default: true, group: 'Elements' },
    { key: 'showCatch', label: 'Laps to catch', type: 'bool', default: true, group: 'Elements' },
    { key: 'showLaps', label: 'Lap time comparison', type: 'bool', default: true, group: 'Elements' },
    { key: 'showIRating', label: 'iRating', type: 'bool', default: true, group: 'Elements' },
    { key: 'showGapBar', label: 'Gap bar', type: 'bool', default: true, group: 'Elements' },
    { key: 'onlyInBattle', label: 'Only show cars within battle range', type: 'bool', default: false, group: 'Elements', help: 'Battle range is set under Settings → Strategy' },
    { key: 'nameStyle', label: 'Name format', type: 'select', default: 'full', options: NAME_STYLES, group: 'Format' },
    { key: 'gapDecimals', label: 'Gap decimals', type: 'number', default: 2, min: 0, max: 3, group: 'Format' },
  ],
  create(root, ctx) {
    const s = ctx.settings;
    root.classList.add('battle');
    const card = (label: string) => {
      const el = h('div', 'card');
      const top = h('div', 'top');
      const tag = h('span', 'tag', label);
      const num = h('span', 'num');
      const name = h('span', 'name');
      const cls = h('span', 'cls');
      show(cls, s.showClass as boolean);
      const ir = h('span', 'ir dim');
      show(ir, s.showIRating as boolean);
      const gap = h('span', 'big');
      top.append(tag, num, name, cls, ir, gap);
      const bar = h('div', 'gapbar');
      const fill = h('div', 'fill');
      bar.append(fill);
      show(bar, s.showGapBar as boolean);
      const bottom = h('div', 'bottom');
      const trend = h('span', 'trend');
      const caught = h('span', 'catch dim');
      const laps = h('span', 'laps dim');
      show(trend, s.showTrend as boolean);
      show(caught, s.showCatch as boolean);
      show(laps, s.showLaps as boolean);
      bottom.append(trend, caught, laps);
      el.append(top, bar, bottom);
      return { el, num, name, cls, ir, gap, fill, trend, caught, laps };
    };
    const ahead = card('AHEAD');
    const behind = card('BEHIND');
    root.append(ahead.el, behind.el);

    const fill = (c: ReturnType<typeof card>, b: BattleCar | null, isAhead: boolean, d: Battle) => {
      const visible = !!b && (!s.onlyInBattle || b.inBattle) && (isAhead ? (s.showAhead as boolean) : (s.showBehind as boolean));
      toggle(c.el, 'empty', !visible);
      if (!b || !visible) {
        text(c.name, isAhead ? 'Clear ahead' : 'Clear behind');
        text(c.gap, '');
        text(c.trend, '');
        text(c.caught, '');
        text(c.laps, '');
        c.num.replaceChildren();
        c.cls.replaceChildren();
        c.fill.style.transform = 'scaleX(0)';
        return;
      }
      numberChip(c.num, b.number, b, s);
      if (s.showClass) chip(c.cls, b.className, b.classColor);
      text(c.name, f.name(b.name, s.nameStyle as string));
      text(c.ir, b.irating ? f.iRating(b.irating) : '');
      text(c.gap, b.gapLaps ? `${b.gapLaps}L` : (b.gap ?? 0).toFixed(s.gapDecimals as number));
      toggle(c.el, 'close', b.inBattle);
      toggle(c.el, 'pit', b.onPitRoad);
      const ratio = b.gap == null ? 1 : Math.min(1, b.gap / Math.max(0.1, d.maxGap * 2));
      c.fill.style.transform = `scaleX(${(1 - ratio).toFixed(3)})`;
      // For the car ahead a shrinking gap is good; for the car behind it is bad.
      const tr = b.trend;
      if (tr == null) {
        text(c.trend, 'trend …');
        c.trend.className = 'trend dim';
      } else {
        const closing = tr < 0;
        text(c.trend, `${closing ? '▼' : '▲'} ${Math.abs(tr).toFixed(2)}s/lap`);
        c.trend.className = `trend ${closing === isAhead ? 'good' : 'bad'}`;
      }
      text(c.caught, b.lapsToCatch != null ? (isAhead ? `catch in ${b.lapsToCatch.toFixed(1)} laps` : `on you in ${b.lapsToCatch.toFixed(1)} laps`) : '');
      const delta = b.lastLap != null && d.myLastLap != null ? b.lastLap - d.myLastLap : null;
      text(c.laps, b.lastLap ? `last ${f.lapTime(b.lastLap)}${delta != null ? ` (${f.signed(delta, 2)})` : ''}` : '');
    };

    return {
      update(topic, data) {
        if (topic !== 'battle') return;
        const d = data as Battle;
        const pair = d[(s.mode as 'overall' | 'class' | 'track') ?? 'overall'] ?? d.overall;
        fill(ahead, pair.ahead, true, d);
        fill(behind, pair.behind, false, d);
      },
    };
  },
};
