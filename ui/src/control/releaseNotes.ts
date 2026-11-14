// Release notes (a version's section of CHANGELOG.md) shown as sections — New features,
// Improvements, Fixes — with bullet points. Built from text only: notes are never inserted
// as HTML.

import { h } from '../lib/dom';

/** Section kind from its heading, for its colour and icon. */
function kind(title: string): { cls: string; icon: string } {
  const t = title.toLowerCase();
  if (/new|feature|added/.test(t)) return { cls: 'new', icon: '✦' };
  if (/fix|bug/.test(t)) return { cls: 'fixed', icon: '✓' };
  if (/perf|faster|speed/.test(t)) return { cls: 'perf', icon: '⚡' };
  if (/remov|deprecat/.test(t)) return { cls: 'removed', icon: '−' };
  return { cls: 'improved', icon: '↑' };
}

/** `**bold**` and `*italic*` as elements; everything else as plain text. */
function inline(text: string): Node[] {
  const out: Node[] = [];
  const re = /\*\*(.+?)\*\*|\*(.+?)\*/g;
  let last = 0;
  for (let m = re.exec(text); m; m = re.exec(text)) {
    if (m.index > last) out.push(document.createTextNode(text.slice(last, m.index)));
    out.push(m[1] != null ? h('strong', '', m[1]) : h('em', '', m[2]));
    last = m.index + m[0].length;
  }
  if (last < text.length) out.push(document.createTextNode(text.slice(last)));
  return out;
}

/** Notes of several versions, newest first, each under its version number; one version is
 *  shown as before. */
export function releaseHistory(releases: { version: string; notes: string }[]): HTMLElement {
  if (releases.length === 1) return releaseNotes(releases[0].notes);
  const box = h('div', 'release-history');
  for (const r of releases) {
    const part = h('div', 'rn-version');
    part.append(h('div', 'rn-version-head', `Version ${r.version}`), releaseNotes(r.notes));
    box.append(part);
  }
  return box;
}

export function releaseNotes(notes: string): HTMLElement {
  const box = h('div', 'release-notes');
  let list: HTMLElement | null = null;
  const ensureList = () => {
    if (!list) {
      list = h('ul');
      box.append(list);
    }
    return list;
  };
  for (const raw of notes.split(/\r?\n/)) {
    const line = raw.trim();
    if (!line) continue;
    const heading = /^#{2,4}\s+(.*)$/.exec(line);
    const bullet = /^[-*]\s+(.*)$/.exec(line);
    if (heading) {
      const k = kind(heading[1]);
      const head = h('div', `rn-head rn-${k.cls}`);
      head.append(h('span', 'rn-icon', k.icon), h('span', '', heading[1]));
      box.append(head);
      list = null;
    } else if (bullet) {
      const li = h('li');
      li.append(...inline(bullet[1]));
      ensureList().append(li);
    } else {
      const p = h('p');
      p.append(...inline(line));
      box.append(p);
      list = null;
    }
  }
  return box;
}
