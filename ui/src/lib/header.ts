// Customisable session header for table overlays (relative, standings).

import { h, show, text } from './dom';
import type { Units } from './format';
import { fieldValue, flagKind, headerFields, lapSettings, sessionType, type TimedLaps } from './sessionFields';
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
    { key: 'headerFields', when: (v) => v.showHeader !== false, label: 'Session details in the header', type: 'columns', ...headerFields(on), group: 'Header' },
    { key: 'headerSeparator', when: (v) => v.showHeader !== false, label: 'Separator', type: 'text', default: '  ·  ', group: 'Header' },
    ...lapSettings('Header', (v) => v.showHeader !== false),
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
  show(title, s.headerTitle !== 'none');
  const fields = enabledColumns(s.headerFields);
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
      const parts = fields.map((id) => fieldValue(id, d, units(), { compact: true, timedLaps: s.timedLaps as TimedLaps, estDecimals: s.estDecimals as number })).filter(Boolean);
      text(info, parts.join(s.headerSeparator as string));
      info.dataset.flag = fields.includes('flags') ? flagKind(d) : '';
    },
  };
}
