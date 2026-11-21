// Customisable session header for table overlays (relative, standings).

import { h, show, text } from './dom';
import type { Units } from './format';
import { fieldValue, flagKind, headerFields, incidentColorSettings, incidentField, lapSettings, sessionType, type TimedLaps } from './sessionFields';
import { enabledColumns, type SettingDef, type Values } from './settings';
import type { Session } from './types';

export function headerSettings(on: string[]): SettingDef[] {
  return [
    { key: 'showHeader', label: 'Show header', type: 'bool', default: true, group: 'Header' },
    {
      key: 'headerTitle', when: (v) => v.showHeader !== false,
      label: 'Header title',
      type: 'select',
      default: 'overlay',
      options: [
        ['overlay', 'Overlay name (Relative / Standings)'],
        ['session', 'Session name'],
        ['track', 'Track name'],
        ['league', 'League name'],
        ['custom', 'Custom text'],
        ['none', 'No title'],
      ],
      group: 'Header',
    },
    { key: 'headerCustomTitle', when: (v) => v.showHeader !== false && v.headerTitle === 'custom', label: 'Custom title', type: 'text', default: '', placeholder: 'My League · Round 3', group: 'Header' },
    { key: 'sessionBadge', when: (v) => v.showHeader !== false, label: 'Session type badge', type: 'bool', default: true, group: 'Header' },
    {
      key: 'headerTextSize', when: (v) => v.showHeader !== false,
      label: 'Header text size',
      type: 'number',
      default: 100,
      min: 50,
      max: 250,
      step: 5,
      unit: '%',
      group: 'Header',
      help: 'The title, session badge and details, compared with the rest of the overlay',
    },
    {
      key: 'headerHeight', when: (v) => v.showHeader !== false,
      label: 'Header height',
      type: 'number',
      default: 0,
      min: 0,
      max: 160,
      unit: 'px',
      group: 'Header',
      help: '0: just tall enough for its text. Taller keeps the text centred',
    },
    { key: 'headerFields', when: (v) => v.showHeader !== false, label: 'Session details in the header', type: 'columns', ...headerFields(on), group: 'Header' },
    { key: 'headerSeparator', when: (v) => v.showHeader !== false, label: 'Separator', type: 'text', default: '  ·  ', group: 'Header' },
    ...lapSettings('Header', (v) => v.showHeader !== false),
    ...incidentColorSettings('Header', 'headerFields', (v) => v.showHeader !== false),
  ];
}

export function sessionHeader(s: Values, fallbackTitle: string, units: () => Units) {
  const el = h('div', 'header');
  const left = h('span', 'header-left');
  const title = h('span', 'title', fallbackTitle);
  const badge = h('span', 'session-badge');
  left.append(title, badge);
  show(badge, s.sessionBadge as boolean);
  const info = h('span', 'info');
  el.append(left, info);
  show(el, s.showHeader as boolean);
  // Its own text size and height.
  const size = (s.headerTextSize as number) ?? 100;
  if (size !== 100) el.style.fontSize = `${size}%`;
  const height = (s.headerHeight as number) ?? 0;
  if (height > 0) {
    el.style.minHeight = `${height}px`;
    el.style.boxSizing = 'border-box';
    el.style.alignItems = 'center';
  }
  show(title, s.headerTitle !== 'none');
  const fields = enabledColumns(s.headerFields);
  // The incident count, coloured, as its own element among the header's details.
  const incEl = h('span', 'inc-field');
  return {
    el,
    update(d: Session) {
      const t =
        s.headerTitle === 'custom' ? (s.headerCustomTitle as string) : s.headerTitle === 'none' || s.headerTitle === 'overlay' ? '' : fieldValue(s.headerTitle as string, d, units(), { compact: true });
      text(title, t || fallbackTitle);
      const type = sessionType(d);
      text(badge, type);
      badge.dataset.type = type.toLowerCase();
      show(badge, (s.sessionBadge as boolean) && !!type);
      const sep = s.headerSeparator as string;
      const parts = fields
        .map((id) => (id === 'incidents' ? incEl : fieldValue(id, d, units(), { compact: true, timedLaps: s.timedLaps as TimedLaps, estDecimals: s.estDecimals as number, lengthSeparator: s.raceLengthSeparator as string })))
        .filter((p) => p !== '');
      if (!fields.includes('incidents')) text(info, (parts as string[]).join(sep));
      else {
        incidentField(incEl, d, s);
        // Laid out again only when a text part changes; the count updates itself.
        const key = parts.map((p) => (typeof p === 'string' ? p : '\u0000')).join(sep);
        if (info.dataset.key !== key) {
          info.dataset.key = key;
          info.replaceChildren(...parts.flatMap((p, i) => (i ? [sep, p] : [p])));
        }
      }
      info.dataset.flag = fields.includes('flags') ? flagKind(d) : '';
    },
  };
}
