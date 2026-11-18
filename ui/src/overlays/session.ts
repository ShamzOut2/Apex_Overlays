import { h, text, toggle } from '../lib/dom';
import type { OverlayDef } from '../lib/overlay';
import { FIELD_LABELS, fieldValue, flagKind, headerFields, lapSettings, type TimedLaps } from '../lib/sessionFields';
import { enabledColumns } from '../lib/settings';
import type { Session } from '../lib/types';

export const session: OverlayDef = {
  kind: 'session',
  name: 'Session info',
  description: 'Track & air temperature (with trend), wetness, rain, wind, sky, time remaining, sim clock, SOF, incidents and flags. Pick and order any fields.',
  topics: ['session'],
  size: [900, 52],
  settings: [
    { key: 'fields', label: 'Fields', type: 'columns', ...headerFields(['session', 'time', 'clock', 'trackTemp', 'airTemp', 'wetness', 'rain', 'wind', 'sof', 'incidents']), group: 'Fields' },
    { key: 'layout', label: 'Layout', type: 'select', default: 'bar', options: [['bar', 'Horizontal bar'], ['stack', 'Vertical list']], group: 'Layout' },
    { key: 'showLabels', label: 'Show labels', type: 'bool', default: true, group: 'Layout' },
    { key: 'clock24', label: '24-hour clock', type: 'bool', default: true, group: 'Format' },
    ...lapSettings('Format', undefined, 'fields'),
    { key: 'showTempTrend', label: 'Track temperature trend arrow', type: 'bool', default: true, group: 'Format' },
  ],
  create(root, ctx) {
    const s = ctx.settings;
    root.classList.add('session-info', `layout-${s.layout}`);
    const cells: Record<string, HTMLElement> = {};
    for (const id of enabledColumns(s.fields)) {
      const item = h('div', `item f-${id}`);
      if (s.showLabels) item.append(h('div', 'label', FIELD_LABELS[id] ?? id));
      const v = h('div', 'value');
      item.append(v);
      root.append(item);
      cells[id] = v;
    }
    return {
      update(topic, data) {
        if (topic !== 'session') return;
        const d = data as Session;
        const u = ctx.units;
        for (const [id, el] of Object.entries(cells)) text(el, fieldValue(id, d, u, { clock24: s.clock24 as boolean, tempTrend: s.showTempTrend as boolean, timedLaps: s.timedLaps as TimedLaps, estDecimals: s.estDecimals as number }) || '–');
        if (cells.wetness) {
          toggle(cells.wetness, 'wet', d.wetness >= 4);
          toggle(cells.wetness, 'damp', d.wetness === 2 || d.wetness === 3);
        }
        if (cells.flags) cells.flags.dataset.flag = flagKind(d);
      },
    };
  },
};
