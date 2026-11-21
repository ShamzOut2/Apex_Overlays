import { otherGameOnly, overlayBadges, withLmuBadge } from '../lib/badges';
import { h, text } from '../lib/dom';
import * as f from '../lib/format';
import { commonFor, merge, type SettingDef, SHOW_WHEN, type Values, showWhenOf, type TrackValues, trackValue } from '../lib/settings';
import { essentials, isFolded, masterOf, type SettingNode, sectionOf, setFolded, type Tab, TABS, tabOf, tree } from './sections';
import { applyUiTheme, PRESETS, THEME, themeValues } from '../lib/theme';
import type { OverlayInstance, Player, SavedLayout, ScreenMonitor } from '../lib/types';
import { byKind, hiddenKind, OVERLAYS } from '../overlays/registry';
import { CHOICES, field, form } from './form';
import { hotkeyInput } from './hotkey';
import { releaseHistory } from './releaseNotes';
import { STATS } from '../lib/monitorStats';
import { client, download, fetchOnce, listen, save, state, uid, upload } from './state';
import { pageActions } from './chrome';

export type Page = { id: string; title: string; render(el: HTMLElement): void };

export const card = (title: string, ...children: (Node | string)[]) => {
  const c = h('section', 'card');
  c.append(h('h2', '', title), ...children);
  return c;
};

export const button = (label: string, onClick: () => void, cls = '') => {
  const b = h('button', cls, label);
  b.onclick = (e) => {
    e.preventDefault();
    onClick();
  };
  return b;
};

export const overlayUrl = (id: string) => `${location.origin}/o/${encodeURIComponent(id)}`;

let selected = '';
/** Overlays listed in the settings: not internal ones, nor ones only the other game has. */
const listed = (o: OverlayInstance) => !hiddenKind(o.kind) && !otherGameOnly(byKind(o.kind), state.config?.layoutGame);

/** Shared files: one overlay, or a whole layout (overlays + theme). No personal data. */
export type SharedOverlay = Pick<OverlayInstance, 'kind' | 'name' | 'enabled' | 'x' | 'y' | 'width' | 'height' | 'settings'>;
interface ShareFile {
  apexOverlays: 'overlay' | 'layout';
  version: string;
  /** A saved layout's name and the session types it is used for. */
  name?: string;
  sessions?: string[];
  overlays: SharedOverlay[];
  theme?: Record<string, unknown>;
}

/** Session types a layout can be switched to automatically (id, button, name). */
const SESSION_TYPES: [string, string, string][] = [
  ['practice', 'P', 'Practice'],
  ['qualifying', 'Q', 'Qualifying'],
  ['warmup', 'W', 'Warmup'],
  ['race', 'R', 'Race'],
];

/** Keeps the overlay preview fitted to the editor (one at a time). */
let previewObserver: ResizeObserver | null = null;

/** A menu to show open again after the page redraws. */
let openMenu = '';

export const shareable = (o: OverlayInstance): SharedOverlay => ({ kind: o.kind, name: o.name, enabled: o.enabled, x: o.x, y: o.y, width: o.width, height: o.height, settings: structuredClone(o.settings) });
const fileName = (s: string) => s.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || 'overlay';

function exportOverlay(o: OverlayInstance) {
  const file: ShareFile = { apexOverlays: 'overlay', version: state.status?.version ?? '', overlays: [shareable(o)] };
  download(`apex-overlay-${fileName(o.name || byKind(o.kind)?.name || o.kind)}.json`, file);
}

function exportLayout() {
  const cfg = state.config!;
  const file: ShareFile = { apexOverlays: 'layout', version: state.status?.version ?? '', overlays: cfg.overlays.map(shareable), theme: structuredClone(cfg.theme) };
  download('apex-overlays-layout.json', file);
}

/** Reads a shared file; also accepts a full settings backup (its overlays and theme). */
async function readShare(): Promise<ShareFile | null> {
  let data: unknown;
  try {
    data = await upload();
  } catch {
    alert('That file could not be read.');
    return null;
  }
  const d = data as Partial<ShareFile> & { overlays?: unknown };
  if (!d || !Array.isArray(d.overlays)) {
    alert('That file is not an Apex Overlays overlay or layout.');
    return null;
  }
  const overlays = (d.overlays as SharedOverlay[]).filter((o) => o && typeof o.kind === 'string' && byKind(o.kind) && !hiddenKind(o.kind));
  if (!overlays.length) {
    alert('That file has no overlays this version of Apex Overlays knows.');
    return null;
  }
  return {
    apexOverlays: d.apexOverlays === 'overlay' ? 'overlay' : 'layout',
    version: String(d.version ?? ''),
    name: typeof d.name === 'string' ? d.name : undefined,
    sessions: Array.isArray(d.sessions) ? d.sessions.filter((x): x is string => typeof x === 'string') : undefined,
    overlays,
    theme: d.theme as Record<string, unknown> | undefined,
  };
}

export const instanceFrom = (o: SharedOverlay): OverlayInstance => {
  const d = byKind(o.kind)!;
  return {
    id: uid(o.kind),
    kind: o.kind,
    name: o.name ?? '',
    enabled: o.enabled !== false,
    x: Number.isFinite(o.x) ? o.x : 200,
    y: Number.isFinite(o.y) ? o.y : 200,
    width: o.width > 0 ? o.width : d.size[0],
    height: o.height > 0 ? o.height : d.size[1],
    settings: o.settings && typeof o.settings === 'object' ? o.settings : {},
  };
};

function copy(text: string, btn: HTMLButtonElement) {
  navigator.clipboard?.writeText(text).then(() => {
    const old = btn.textContent;
    btn.textContent = 'Copied';
    setTimeout(() => (btn.textContent = old), 1200);
  });
}

/** A small drop-down menu of actions. */
function menu(label: string, items: [string, () => void, string?][], cls = ''): HTMLElement {
  const d = h('details', `menu ${cls}`);
  const sum = h('summary', '', label);
  const pop = h('div', 'menu-pop');
  for (const [text, fn, c] of items) {
    pop.append(
      button(
        text,
        () => {
          d.open = false;
          fn();
        },
        c ?? '',
      ),
    );
  }
  d.append(sum, pop);
  return d;
}

// Close open menus when clicking elsewhere.
document.addEventListener('click', (e) => {
  // A click that redrew the page landed on an element that is gone: keep menus as they are.
  if (!(e.target as Node).isConnected) return;
  for (const d of document.querySelectorAll<HTMLDetailsElement>('details.menu[open]')) if (!d.contains(e.target as Node)) d.open = false;
});

/** Editor state that survives the page being redrawn. */
const ui = { tab: new Map<string, string>(), query: '', changedOnly: false, focus: '' };
/** The General page's tab, search and filters, kept while it is drawn again. */
const gen = { tab: 'Essentials', query: '', changedOnly: false, setOnly: false };
const same = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b);
/** Parts of every overlay that open their settings from the preview (see OverlayDef.regions). */
const GENERIC_REGIONS: [string, string][] = [['.header', 'showHeader'], ['.class-head', 'showClassHeader'], ['.row.head', 'showColumnTitles'], ['.table .row', 'columns']];

function overlayEditor(o: OverlayInstance, rerender: () => void): HTMLElement {
  const def = byKind(o.kind)!;
  const wrap = h('div', 'editor');
  const head = h('div', 'editor-head');
  const title = h('input', 'title-input');
  title.value = o.name;
  title.placeholder = def.name;
  title.title = 'Name this overlay (handy when you have several of the same kind)';
  title.oninput = () => {
    o.name = title.value;
    save();
  };
  const enabled = h('label', 'inline');
  const cb = h('input');
  cb.type = 'checkbox';
  cb.className = 'switch';
  cb.checked = o.enabled;
  cb.onchange = () => {
    o.enabled = cb.checked;
    save();
    rerender();
  };
  enabled.append(cb, o.enabled ? ' On' : ' Off');
  const url = overlayUrl(o.id);
  const copyBtn = button('Copy OBS link', () => copy(url, copyBtn));
  copyBtn.title = `Browser source URL for OBS / streaming: ${url}`;
  const more = menu('More ▾', [
    [
      'Duplicate',
      () => {
        const c = structuredClone(o);
        c.id = uid(o.kind);
        c.name = `${o.name || def.name} copy`;
        c.x += 30;
        c.y += 30;
        state.config!.overlays.push(c);
        selected = c.id;
        save();
        rerender();
      },
    ],
    ['Export settings to a file…', () => exportOverlay(o)],
    [
      'Load settings from a file…',
      async () => {
        const f = await readShare();
        if (!f) return;
        const src = f.overlays.find((x) => x.kind === o.kind);
        if (!src) return alert(`That file has no ${def.name} overlay.`);
        o.settings = structuredClone(src.settings ?? {});
        save();
        rerender();
      },
    ],
    [
      'Reset all settings',
      () => {
        if (!confirm(`Reset every setting of ${o.name || def.name} to the default?`)) return;
        o.settings = {};
        save();
        rerender();
      },
    ],
    [
      'Delete overlay',
      () => {
        if (!confirm(`Delete ${o.name || def.name}?`)) return;
        state.config!.overlays = state.config!.overlays.filter((x) => x !== o);
        selected = '';
        save();
        rerender();
      },
      'danger',
    ],
  ]);
  // The name (editable) with what the overlay does under it; switch and actions on the right.
  const titleCol = h('div', 'editor-title');
  const titleRow = h('div', 'editor-title-row');
  titleRow.append(title, ...overlayBadges(def));
  titleCol.append(titleRow, h('p', 'dim desc', def.description));
  const headActions = h('div', 'editor-actions');
  headActions.append(enabled, copyBtn, more);
  head.append(titleCol, headActions);

  // Live preview of this overlay with its current settings.
  const preview = h('div', 'preview');
  const frame = h('iframe');
  frame.src = `${url}?preview`;
  frame.style.width = `${o.width}px`;
  frame.style.height = `${o.height}px`;
  // Scaled to fit the editor, however wide the window is.
  const fitPreview = () => {
    // The overlay as wide and tall as its content (the real window fits itself to it).
    const panel = frame.contentDocument?.querySelector<HTMLElement>('.panel');
    const w = Math.max(o.width, panel?.scrollWidth ?? 0);
    const hgt = o.height;
    frame.style.width = `${w}px`;
    const room = (preview.clientWidth || 700) - 16;
    const scale = Math.min(1, room / w, 220 / hgt);
    frame.style.transform = `scale(${scale})`;
    preview.style.height = `${Math.ceil(hgt * scale) + 16}px`;
  };
  // Content grows as data comes in: check now and then while the editor is open.
  const fitTimer = window.setInterval(() => (preview.isConnected ? fitPreview() : clearInterval(fitTimer)), 1000);
  frame.addEventListener('load', () => setTimeout(fitPreview, 300));
  fitPreview();
  previewObserver?.disconnect();
  previewObserver = new ResizeObserver(fitPreview);
  previewObserver.observe(preview);
  preview.append(frame);

  const common = commonFor(def.commonDefaults, def.fitsWidth, def.fill);
  const values = merge([...common, ...def.settings], o.settings);
  values.visibleWhen = showWhenOf(values);
  // App-wide settings shown in this overlay's settings: their value lives in the config.
  const sharedPath = (k: string) => def.settings.find((s) => s.key === k)?.shared;
  const sharedAt = (path: string): [Record<string, unknown>, string] => {
    const keys = path.split('.');
    let obj = state.config as unknown as Record<string, unknown>;
    for (const k of keys.slice(0, -1)) obj = obj[k] as Record<string, unknown>;
    return [obj, keys[keys.length - 1]];
  };
  for (const s of def.settings) {
    if (!s.shared) continue;
    const [obj, k] = sharedAt(s.shared);
    if (obj?.[k] !== undefined) values[s.key] = obj[k];
  }
  // Settings saved per track (track map orientation): while in a session, these show and edit
  // the current track's own values; otherwise the overlay's, used on tracks with none.
  const track = state.session?.connected && state.session.trackId ? state.session : null;
  const isPerTrack = (k: string) => !!track && !!def.settings.find((s) => s.key === k)?.perTrack;
  const trackOwn = () => ((o.settings.tracks as TrackValues | undefined) ?? {})[String(track?.trackId)] ?? {};
  for (const s of def.settings) if (isPerTrack(s.key)) values[s.key] = trackValue(values, track?.trackId, s.key);
  const shown = def.settings.map((s): SettingDef => {
    if (!s.perTrack) return s;
    const note = track ? `Saved for ${track.trackName || 'this track'} only` : 'Saved per track: join a session to set it for that track. This is used on tracks without their own';
    return { ...s, help: s.help ? `${s.help}. ${note}` : note };
  });
  // Every setting with its tab and section; Essentials repeats the hand-picked ones.
  type Entry = { s: SettingDef; tab: Tab; section: string };
  const all: Entry[] = [...shown, ...common].map((s) => ({ s, tab: tabOf(s), section: sectionOf(s) }));
  const ess = essentials(def.kind)
    .map((k) => all.find((e) => e.s.key === k))
    .filter((e): e is Entry => !!e)
    .map((e) => ({ ...e, tab: 'Essentials' as Tab, section: 'The essentials' }));
  const entries: Entry[] = [...ess, ...all];
  const tabs = TABS.filter((t) => entries.some((e) => e.tab === t));
  let tab = (ui.tab.get(o.id) as Tab | undefined) ?? tabs[0];
  if (!tabs.includes(tab)) tab = tabs[0];
  const defOf = (k: string) => all.find((e) => e.s.key === k)?.s;
  const applies = (s: SettingDef) => !s.when || s.when(values);
  /** Differs from its default, and applies with the other choices. */
  const isChanged = (s: SettingDef) => applies(s) && !same(values[s.key], s.default);
  const changedIn = (list: Entry[]) => new Set(list.filter((e) => isChanged(e.s)).map((e) => e.s.key)).size;

  // After a change the markers and counts update; a switch or a choice can add or remove
  // other settings, so the panel is drawn again for those.
  let refresh = () => {};
  /** Stores a value where it lives (this overlay, the app's config, or this track). */
  const store = (k: string, v: unknown) => {
    values[k] = v;
    const path = sharedPath(k);
    if (path) {
      const [obj, key] = sharedAt(path);
      obj[key] = v;
    } else if (k === 'visibleWhen') {
      // One choice, stored as the three switches the overlays read.
      const sw = SHOW_WHEN.find(([id]) => id === v)?.[2];
      if (sw) {
        Object.assign(values, sw);
        o.settings = { ...o.settings, ...sw };
      }
    } else if (isPerTrack(k)) {
      const tracks = { ...((o.settings.tracks as TrackValues | undefined) ?? {}) };
      tracks[String(track!.trackId)] = { ...trackOwn(), [k]: v };
      o.settings = { ...o.settings, tracks };
    } else o.settings = { ...o.settings, [k]: v };
  };
  const onChange = (k: string, v: unknown) => {
    store(k, v);
    save();
    // The overlay list's tags (Hidden in pits…) follow "Show".
    if (k === 'visibleWhen') return rerender();
    drawTabs();
    const t = defOf(k)?.type;
    if (t === 'bool' || t === 'select') drawPanel();
    else refresh();
  };
  /** Puts settings back to their defaults, saved once. */
  const resetAll = (list: SettingDef[]) => {
    for (const s of list) if (isChanged(s)) store(s.key, structuredClone(s.default));
    save();
    if (list.some((s) => s.key === 'visibleWhen')) return rerender();
    drawTabs();
    drawPanel();
  };

  const search = h('input', 'search');
  search.type = 'search';
  search.placeholder = `Search ${def.name} settings…`;
  search.value = ui.query;
  const tabBar = h('div', 'tabs');
  const panel = h('div', 'tab-panel');

  /** Position and size, plus copying the transparency to every overlay. */
  const sizeExtras = () => {
    const geo = h('div', 'geo');
    // While the overlays show in VR, these are the VR position and size (the screen ones stay).
    const inVr = state.vr?.status === 'on';
    const screenRect = () => ({ x: o.x, y: o.y, width: o.width, height: o.height });
    for (const k of ['x', 'y', 'width', 'height'] as const) {
      const l = h('label', 'inline', { x: 'Left', y: 'Top', width: 'Width', height: 'Height' }[k]);
      const i = h('input');
      i.type = 'number';
      i.value = String(inVr ? (o.vrRect ?? o)[k] : o[k]);
      i.oninput = () => {
        const v = parseInt(i.value, 10);
        if (Number.isFinite(v)) {
          if (inVr) o.vrRect = { ...(o.vrRect ?? screenRect()), [k]: v };
          else o[k] = v;
          save();
        }
      };
      l.append(i);
      geo.append(l);
    }
    const box = h('div', 'size-extras');
    box.append(
      h('div', 'field-name', inVr ? 'Position and size in VR (pixels)' : 'Position and size (pixels)'),
      geo,
      h('p', 'dim small', 'Easier: press Edit mode at the top and drag the overlay on screen.'),
    );
    if (inVr) box.append(h('p', 'dim small', 'You are in VR: moving or resizing the overlay only changes where it is in VR. Its screen position is kept for when you leave VR.'));
    else if (o.vrRect)
      box.append(
        h('p', 'dim small', 'This overlay has its own position in VR (it was moved while in VR).'),
        button('Use the screen position in VR too', () => {
          o.vrRect = null;
          save();
        }),
      );
    box.append(
      button('Use this overlay’s transparency for every overlay', () => {
        for (const other of state.config!.overlays) other.settings = { ...other.settings, opacity: values.opacity, bgOpacity: values.bgOpacity };
        save();
      }),
    );
    return box;
  };

  // The accent line under the open tab, which glides to the tab you pick.
  const tabInd = h('span', 'tab-ind');
  let tabIndPlaced = false;
  const placeTabInd = () => {
    const sel = tabBar.querySelector<HTMLElement>('.tab.sel');
    tabInd.hidden = !sel;
    if (!sel) return;
    // The first time it is just put there; after that it slides.
    if (!tabIndPlaced) tabInd.style.transition = 'none';
    tabInd.style.left = `${sel.offsetLeft}px`;
    tabInd.style.top = `${sel.offsetTop + sel.offsetHeight - 2}px`;
    tabInd.style.width = `${sel.offsetWidth}px`;
    if (!tabIndPlaced) {
      void tabInd.offsetWidth;
      tabInd.style.transition = '';
      tabIndPlaced = true;
    }
  };
  const drawTabs = () => {
    requestAnimationFrame(placeTabInd);
    tabBar.replaceChildren(
      ...tabs.map((t) => {
        const b = button(
          t,
          () => {
            tab = t;
            ui.tab.set(o.id, t);
            ui.query = '';
            ui.changedOnly = false;
            ui.focus = '';
            search.value = '';
            drawTabs();
            drawPanel();
          },
          `tab${t === tab && !ui.query && !ui.changedOnly ? ' sel' : ''}`,
        );
        // How many settings here differ from the default.
        const n = changedIn(entries.filter((e) => e.tab === t));
        if (n) {
          const c = h('span', 'tab-count', String(n));
          c.title = n === 1 ? 'One setting here differs from the default' : `${n} settings here differ from the default`;
          b.append(c);
        }
        return b;
      }),
    );
    onlyChanged.classList.toggle('on', ui.changedOnly);
    onlyChanged.setAttribute('aria-pressed', String(ui.changedOnly));
  };

  /** A setting's row: its marker dot, indented by how deep it is nested. */
  const rowFor = (s: SettingDef, depth: number) => {
    const el = field(s, values, onChange);
    el.classList.add('sec-row');
    if (depth) {
      el.classList.add('nested');
      el.style.setProperty('--depth', String(depth));
    }
    const dot = h('span', 'chg-dot');
    dot.title = 'Changed from the default';
    el.prepend(dot);
    return el;
  };

  interface Sec {
    el: HTMLElement;
    title: string;
    list: SettingDef[];
    rows: [SettingDef, HTMLElement][];
    master: SettingDef | null;
    count: HTMLElement;
    reset: HTMLElement;
    chevron: HTMLButtonElement | null;
    hiddenNote: HTMLElement;
  }

  /**
   * A section: its title, how many of its settings are changed, Reset section, the switch that
   * turns it on and off (when it has one), a chevron folding its options away, and its
   * settings with options nested under the switch they depend on.
   */
  const sectionEl = (title: string, list: SettingDef[], opts: { nest: boolean; fold: boolean }): Sec => {
    const el = h('section', 'sec');
    const head = h('div', 'sec-head');
    const titles = h('div', 'sec-titles');
    const name = h('div', 'sec-title-row');
    name.append(h('span', 'sec-title', title));
    if (opts.fold && ui.focus === title) {
      el.classList.add('focus');
      name.append(h('span', 'sec-from', 'Opened from the preview'));
    }
    titles.append(name);
    const count = h('span', 'sec-count');
    const hiddenNote = h('span', 'sec-hidden dim');
    const reset = button('Reset section', () => resetAll(list), 'subtle sec-reset');
    head.append(h('span', 'chg-dot'), titles, count, hiddenNote, reset);
    const body = h('div', 'sec-body');
    const rows: [SettingDef, HTMLElement][] = [];
    let master: SettingDef | null = null;
    const add = (nodes: SettingNode[], depth: number) => {
      for (const n of nodes) {
        rows.push([n.s, rowFor(n.s, depth)]);
        add(n.kids, depth + 1);
      }
    };
    if (opts.nest) {
      const roots = tree(list, values);
      master = masterOf(roots, list.length);
      if (master) {
        // The section's own switch, in its header.
        const ctl = field(master, values, onChange).querySelector('.field-end > :first-child');
        if (master.help) titles.append(h('small', 'help', master.help));
        if (ctl) head.append(ctl);
        el.classList.add('has-master');
        add(roots[0].kids, 0);
      } else add(roots, 0);
    } else for (const s of list) rows.push([s, rowFor(s, 0)]);
    body.append(...rows.map((r) => r[1]));
    let chevron: HTMLButtonElement | null = null;
    if (opts.fold) {
      chevron = button(
        '',
        () => {
          const folded = !el.classList.contains('folded');
          setFolded(def.kind, title, folded);
          el.classList.toggle('folded', folded);
          refresh();
        },
        'sec-fold',
      );
      chevron.innerHTML = '<svg viewBox="0 0 12 12" width="12" height="12" aria-hidden="true"><path d="M2.5 4.5 6 8l3.5-3.5" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>';
      head.append(chevron);
      // Opened from the preview: shown even if folded.
      if (isFolded(def.kind, title) && ui.focus !== title) el.classList.add('folded');
    }
    el.append(head, body);
    return { el, title, list, rows, master, count, reset, chevron, hiddenNote };
  };

  /** Brings each section's rows, markers, counts and fold up to date with the values. */
  const refreshSections = (secs: Sec[], filter: boolean) => {
    for (const sec of secs) {
      if (filter) for (const [s, el] of sec.rows) el.hidden = !applies(s);
      for (const [s, el] of sec.rows) el.classList.toggle('changed', isChanged(s));
      const masterOn = !sec.master || (values[sec.master.key] as boolean);
      const masterShown = !!sec.master && applies(sec.master);
      const visible = sec.rows.filter(([, el]) => !el.hidden).length;
      sec.el.hidden = filter && !masterShown && visible === 0;
      sec.el.classList.toggle('off', !!sec.master && !masterOn);
      sec.el.classList.toggle('changed', !!sec.master && isChanged(sec.master));
      sec.el.classList.toggle('empty', visible === 0);
      const n = new Set(sec.list.filter(isChanged).map((s) => s.key)).size;
      text(sec.count, n ? `${n} changed` : '');
      sec.count.hidden = !n;
      sec.reset.hidden = !n;
      const folded = sec.el.classList.contains('folded');
      if (sec.chevron) {
        sec.chevron.hidden = visible === 0 || !masterOn;
        sec.chevron.title = folded ? `Show the ${sec.title} settings` : `Hide the ${sec.title} settings (they stay on)`;
        sec.chevron.setAttribute('aria-expanded', String(!folded));
      }
      text(sec.hiddenNote, folded && masterOn && visible ? (visible === 1 ? '1 setting hidden' : `${visible} settings hidden`) : '');
    }
  };

  const drawPanel = () => {
    const q = ui.query.trim().toLowerCase();
    // Search and "Changed only" list settings from every tab, under "Tab › Section".
    if (q || ui.changedOnly) {
      const hits = all.filter((e) => (q ? `${e.s.label} ${e.s.help ?? ''} ${e.section} ${e.tab}`.toLowerCase().includes(q) : isChanged(e.s)));
      const groups = new Map<string, SettingDef[]>();
      for (const e of hits) {
        const k = `${e.tab} › ${e.section}`;
        groups.set(k, [...(groups.get(k) ?? []), e.s]);
      }
      const secs = [...groups].map(([title, list]) => sectionEl(title, list, { nest: false, fold: false }));
      // Searching finds every setting, including ones that only apply with other choices.
      refresh = () => refreshSections(secs, !q);
      refresh();
      const empty = q ? `No setting matches “${ui.query}”.` : 'Nothing changed: every setting here is at its default.';
      const note = q ? 'Results from every tab.' : 'Everything you have changed on this overlay, from every tab. ↺ puts one back.';
      panel.replaceChildren(...(secs.length ? [h('p', 'dim small sec-note', note), ...secs.map((x) => x.el)] : [h('p', 'dim', empty)]));
      return;
    }
    const groups = new Map<string, SettingDef[]>();
    for (const e of entries.filter((x) => x.tab === tab)) groups.set(e.section, [...(groups.get(e.section) ?? []), e.s]);
    const essentialsTab = tab === 'Essentials';
    const secs = [...groups].map(([title, list]) => sectionEl(title, list, { nest: !essentialsTab, fold: !essentialsTab }));
    const none = h('p', 'dim', 'Nothing here applies with your current choices (for example the columns shown, or a feature switched off).');
    refresh = () => {
      refreshSections(secs, true);
      none.hidden = secs.some((x) => !x.el.hidden);
    };
    refresh();
    const lead = essentialsTab ? [h('p', 'dim small sec-note', 'The settings most people change. Everything else is in the other tabs, the same on every overlay.')] : [];
    panel.replaceChildren(...lead, ...secs.map((x) => x.el), none, ...(tab === 'Window' ? [sizeExtras()] : []));
    // Opened from the preview: brought into view.
    const focused = panel.querySelector<HTMLElement>('.sec.focus');
    if (focused) requestAnimationFrame(() => focused.scrollIntoView({ block: 'nearest', behavior: 'smooth' }));
  };

  /** Opens the settings of a part of the overlay clicked in the preview. */
  const jumpTo = (key: string) => {
    const e = all.find((x) => x.s.key === key);
    if (!e) return;
    tab = e.tab;
    ui.tab.set(o.id, tab);
    ui.focus = e.section;
    ui.query = '';
    ui.changedOnly = false;
    search.value = '';
    drawTabs();
    drawPanel();
  };

  // The preview: hovering a part outlines it, clicking it opens its settings.
  const regions = [...(def.regions ?? []), ...GENERIC_REGIONS].filter(([, k]) => all.some((e) => e.s.key === k));
  const idleHint = 'Click a part of the preview to open its settings.';
  const hint = h('p', 'dim small preview-hint', regions.length ? idleHint : '');
  hint.hidden = !regions.length;
  const wirePreview = () => {
    const doc = frame.contentDocument;
    if (!doc || !regions.length || doc.documentElement.dataset.apexEdit) return;
    doc.documentElement.dataset.apexEdit = '1';
    const css = doc.createElement('style');
    css.textContent = '.apex-hot { outline: 1px dashed var(--accent, #ff3d57) !important; outline-offset: 2px; cursor: pointer; }';
    doc.head.append(css);
    const find = (t: EventTarget | null): [Element, string] | null => {
      const el = t as Element | null;
      if (!el || typeof el.closest !== 'function') return null;
      for (const [sel, k] of regions) {
        const m = el.closest(sel);
        if (m) return [m, k];
      }
      return null;
    };
    let hot: Element | null = null;
    const mark = (r: [Element, string] | null) => {
      const m = r?.[0] ?? null;
      if (m !== hot) {
        hot?.classList.remove('apex-hot');
        m?.classList.add('apex-hot');
        hot = m;
      }
      const e = r && all.find((x) => x.s.key === r[1]);
      text(hint, e ? `Click to open: ${e.section} (${e.tab})` : idleHint);
    };
    doc.addEventListener('mouseover', (ev) => mark(find(ev.target)));
    doc.documentElement.addEventListener('mouseleave', () => mark(null));
    doc.addEventListener('click', (ev) => {
      const r = find(ev.target);
      if (!r) return;
      ev.preventDefault();
      jumpTo(r[1]);
    });
  };
  frame.addEventListener('load', () => setTimeout(wirePreview, 200));

  const onlyChanged = button(
    'Changed only',
    () => {
      ui.changedOnly = !ui.changedOnly;
      ui.query = '';
      ui.focus = '';
      search.value = '';
      drawTabs();
      drawPanel();
    },
    'subtle chg-only',
  );
  onlyChanged.title = 'Every setting you have changed on this overlay, from every tab';

  search.oninput = () => {
    ui.query = search.value;
    ui.focus = '';
    if (ui.query) ui.changedOnly = false;
    drawTabs();
    drawPanel();
  };
  drawTabs();
  drawPanel();
  const tabRow = h('div', 'tab-row');
  tabRow.append(tabBar, tabInd);
  // The search and "Changed only" on a row of their own under the tabs.
  const tools = h('div', 'tab-tools');
  tools.append(search, onlyChanged);
  wrap.append(head, preview, hint, tabRow, tools, panel);
  return wrap;
}

/** Where each kind of overlay goes by default on a 1920×1080 screen (as the app's catalogue). */
const PLACES: Record<string, [number, number]> = {
  relative: [40, 380], standings: [40, 40], trackmap: [1500, 40], inputs: [760, 860], shiftlights: [760, 790], inputGraph: [760, 740], fuel: [1540, 420],
  battle: [1200, 860], session: [510, 10], tyres: [1540, 680], flags: [860, 70], startLights: [760, 190], forecast: [1240, 520],
  laptimes: [40, 700], energy: [1540, 420], radar: [1220, 420], carsettings: [760, 770], tyredata: [1200, 600], spotter: [850, 520],
  pit: [845, 300], safety: [760, 160], delta: [760, 400], incidents: [40, 700], weather: [1240, 680], databox: [1240, 300], monitor: [1680, 40],
};

/** A new overlay's place: its usual spot (fitted to this screen), moved along a little if another overlay is already there. */
function placeFor(kind: string, existing: OverlayInstance[]): [number, number] {
  const [px, py] = PLACES[kind] ?? [200, 200];
  const sx = window.screen.width / 1920;
  const sy = window.screen.height / 1080;
  let x = Math.round(px * sx);
  let y = Math.round(py * sy);
  for (let i = 0; i < 10 && existing.some((o) => Math.abs(o.x - x) < 24 && Math.abs(o.y - y) < 24); i++) {
    x += 32;
    y += 32;
  }
  return [x, y];
}

/** Pick an overlay to add, with what each one does. */
function addOverlayDialog(onAdd: (kind: string) => void) {
  const back = h('div', 'modal-back');
  const dlg = h('div', 'modal');
  const close = () => back.remove();
  back.onclick = (e) => {
    if (e.target === back) close();
  };
  const head = h('div', 'modal-head');
  head.append(h('h2', '', 'Add an overlay'), button('✕', close, 'subtle'));
  const grid = h('div', 'gallery');
  const count = (kind: string) => state.config!.overlays.filter((o) => o.kind === kind).length;
  for (const d of OVERLAYS.filter((x) => !otherGameOnly(x, state.config!.layoutGame))) {
    const card = h('button', 'gallery-item');
    const n = count(d.kind);
    const gname = h('span', 'g-name', d.name);
    gname.append(...overlayBadges(d));
    card.append(gname, h('span', 'g-desc', d.description));
    if (n) card.append(h('span', 'g-count', n === 1 ? '✓ In your layout' : `✓ ${n} in your layout`));
    card.onclick = (e) => {
      e.preventDefault();
      close();
      onAdd(d.kind);
    };
    grid.append(card);
  }
  dlg.append(head, h('p', 'dim', 'You can add any overlay more than once — each copy has its own settings, size and position.'), grid);
  back.append(dlg);
  document.body.append(back);
}

const overlaysPage: Page = {
  id: 'overlays',
  title: 'Overlays',
  render(el) {
    const cfg = state.config!;
    const rerender = () => this.render(el);
    if (!cfg.overlays.some((o) => o.id === selected && listed(o))) selected = cfg.overlays.find(listed)?.id ?? '';
    const editing = state.status?.editMode ?? false;

    cfg.layouts ??= [];
    const version = state.status?.version ?? '';
    // Keep a menu open when an action inside it redraws the page.
    const reopen = (menu: string) => {
      openMenu = menu;
      rerender();
    };
    const remember = (d: HTMLDetailsElement, name: string) => {
      d.open = openMenu === name;
      d.addEventListener('toggle', () => {
        if (d.open) openMenu = name;
        else if (openMenu === name) openMenu = '';
      });
    };

    // Saved layouts: switch, share, and pick the session types each one is used for.
    const layoutsMenu = h('details', 'menu layout-menu');
    const active = cfg.layouts.find((l) => l.id === cfg.activeLayout);
    layoutsMenu.append(h('summary', '', active ? `Layout: ${active.name} ▾` : 'Layouts ▾'));
    const lpop = h('div', 'menu-pop wide');
    const layoutList = h('div', 'layout-list');
    const exportSaved = (l: SavedLayout) => {
      const overlays = l.id === cfg.activeLayout ? cfg.overlays : l.overlays;
      const file: ShareFile = { apexOverlays: 'layout', version, name: l.name, sessions: l.sessions ?? [], overlays: overlays.map(shareable), theme: structuredClone(cfg.theme) };
      download(`apex-layout-${fileName(l.name)}.json`, file);
    };
    for (const l of cfg.layouts) {
      l.sessions ??= [];
      const isActive = l.id === cfg.activeLayout;
      const item = h('div', `layout-item${isActive ? ' active' : ''}`);
      const chips = h('span', 'session-chips');
      for (const [id, short, long] of SESSION_TYPES) {
        const on = l.sessions.includes(id);
        const b = button(
          short,
          () => {
            // A session type belongs to one layout at most.
            for (const other of cfg.layouts) other.sessions = (other.sessions ?? []).filter((s) => s !== id);
            if (!on) l.sessions.push(id);
            save(true);
            reopen('layouts');
          },
          `chip-toggle${on ? ' on' : ''}`,
        );
        b.title = on ? `Used automatically for ${long} sessions — click to stop` : `Use this layout automatically when a ${long} session starts`;
        chips.append(b);
      }
      item.append(
        h('span', 'layout-name', `${isActive ? '● ' : ''}${l.name}`),
        chips,
        button(
          isActive ? 'In use' : 'Use',
          () => {
            applyLayout(l.id);
            layoutsMenu.open = false;
            openMenu = '';
            rerender();
          },
          'mini',
        ),
        button('⇩', () => exportSaved(l), 'mini subtle'),
        button(
          '✎',
          () => {
            const name = prompt('Layout name', l.name)?.trim();
            if (!name) return;
            l.name = name;
            save(true);
            reopen('layouts');
          },
          'mini subtle',
        ),
        button(
          '✕',
          () => {
            if (!confirm(`Delete the layout “${l.name}”? Your overlays on screen stay as they are.`)) return;
            cfg.layouts = cfg.layouts.filter((x) => x !== l);
            if (cfg.activeLayout === l.id) cfg.activeLayout = '';
            delete cfg.hotkeys.layouts?.[l.id];
            save(true);
            reopen('layouts');
          },
          'mini subtle',
        ),
      );
      const [, , , exp, ren, del] = [...item.children] as HTMLElement[];
      exp.title = 'Export this layout to a file to share';
      ren.title = 'Rename';
      del.title = 'Delete';
      layoutList.append(item);
    }
    const layoutActions = h('div', 'toolbar');
    layoutActions.append(
      button('Save current overlays as a layout…', () => {
        const name = prompt('Name this layout (e.g. Race, Qualifying, Streaming)')?.trim();
        if (!name) return;
        const l: SavedLayout = { id: uid('layout'), name, overlays: structuredClone(cfg.overlays), sessions: [] };
        cfg.layouts.push(l);
        cfg.activeLayout = l.id;
        save(true);
        reopen('layouts');
      }),
      button('Import a layout file…', async () => {
        const f = await readShare();
        if (!f) return;
        const name = (prompt('Name for the imported layout', f.name || 'Imported layout') ?? '').trim();
        if (!name) return;
        const l: SavedLayout = { id: uid('layout'), name, overlays: f.overlays.map(instanceFrom), sessions: [] };
        // Session types from the file, where no layout of yours already has them.
        for (const s of f.sessions ?? []) if (SESSION_TYPES.some((t) => t[0] === s) && !cfg.layouts.some((o) => o.sessions?.includes(s))) l.sessions.push(s);
        cfg.layouts.push(l);
        if (f.theme && confirm('Also use the colour theme from this layout?')) cfg.theme = f.theme;
        if (confirm(`Use “${name}” now?`)) applyLayout(l.id);
        else save(true);
        reopen('layouts');
      }),
    );
    lpop.append(
      ...(cfg.layouts.length
        ? [layoutList, h('p', 'dim small', 'P Q W R: use this layout automatically in Practice, Qualifying, Warmup or Race. ⇩ exports it.')]
        : [h('p', 'dim small', 'Save the overlays you have now as a layout, set up another, and switch between them here, with a hotkey, or automatically by session type.')]),
      layoutActions,
    );
    layoutsMenu.append(lpop);
    remember(layoutsMenu, 'layouts');

    // Moving every overlay at once, and sharing single overlays.
    const moveMenu = h('details', 'menu layout-menu');
    moveMenu.append(h('summary', '', 'Move & share ▾'));
    const pop = h('div', 'menu-pop wide');
    const together = h('label', 'inline');
    const tcb = h('input');
    tcb.type = 'checkbox';
    tcb.className = 'switch';
    tcb.checked = !!cfg.moveAllTogether;
    tcb.onchange = () => {
      cfg.moveAllTogether = tcb.checked;
      save(true);
    };
    together.append(tcb, ' Drag all overlays together in edit mode');
    const step = h('select');
    for (const v of [1, 10, 50]) {
      const opt = h('option', '', `${v} px`);
      opt.value = String(v);
      step.append(opt);
    }
    step.value = '10';
    const nudge = (dx: number, dy: number) => () => {
      const n = parseInt(step.value, 10);
      for (const o of cfg.overlays) {
        o.x += dx * n;
        o.y += dy * n;
      }
      save(true);
    };
    const arrows = h('div', 'toolbar');
    arrows.append(h('span', 'dim small', 'Shift everything'), button('←', nudge(-1, 0), 'mini'), button('↑', nudge(0, -1), 'mini'), button('↓', nudge(0, 1), 'mini'), button('→', nudge(1, 0), 'mini'), step);
    const sharing = h('div', 'toolbar');
    sharing.append(
      button('Import an overlay…', async () => {
        const f = await readShare();
        if (!f) return;
        const added = f.overlays.map(instanceFrom);
        cfg.overlays.push(...added);
        selected = added[0].id;
        moveMenu.open = false;
        openMenu = '';
        save(true);
        rerender();
      }),
      button('Export all current overlays…', exportLayout),
    );
    pop.append(
      h('h3', '', 'Move'),
      together,
      arrows,
      h('h3', '', 'Share'),
      sharing,
      h('p', 'dim small', 'Shared files contain overlays, their settings and the theme — never tokens, logins or leagues.'),
    );
    moveMenu.append(pop);
    remember(moveMenu, 'move');

    // In the page header: layouts, moving and sharing, and Move & resize. (Showing and hiding
    // every overlay is the switch at the foot of the sidebar.)
    pageActions(layoutsMenu, moveMenu, editButton(editing));

    const addOverlay = (kind: string) => {
      const d = byKind(kind)!;
      const [x, y] = placeFor(kind, cfg.overlays);
      const o: OverlayInstance = { id: uid(d.kind), kind: d.kind, name: '', enabled: true, x, y, width: d.size[0], height: d.size[1], settings: {} };
      cfg.overlays.push(o);
      selected = o.id;
      ui.tab.delete(o.id);
      save();
      rerender();
    };
    const list = h('div', 'ov-list');
    list.append(
      screenMap(
        cfg.overlays.filter(listed),
        selected,
        (id) => {
          selected = id;
          ui.query = '';
          rerender();
        },
        rerender,
      ),
    );
    for (const o of cfg.overlays.filter(listed)) {
      const d = byKind(o.kind);
      const item = h('div', `ov-item${o.id === selected ? ' sel' : ''}${o.enabled ? '' : ' off'}`);
      item.tabIndex = 0;
      item.dataset.id = o.id;
      // Hovering an overlay lights it on the screen map (and the other way round).
      item.onmouseenter = () => hot(o.id, true);
      item.onmouseleave = () => hot(o.id, false);
      const names = h('span', 'ov-names');
      const oname = h('span', 'ov-name', o.name || d?.name || o.kind);
      oname.append(...overlayBadges(d));
      names.append(oname);
      // What it is and its size: "Relative · 600 × 320".
      names.append(h('span', 'ov-kind', `${d?.name ?? o.kind} · ${o.width} × ${o.height}`));
      // Why it may not be on screen: its own visibility settings, in the list.
      const st = o.settings as Record<string, unknown>;
      const tags = [st.hideInPits === true && 'Hidden in pits', st.hideWhenNotDriving === true && 'In the car only', st.raceOnly === true && 'Races only'].filter(Boolean) as string[];
      if (tags.length) {
        const t = h('span', 'ov-tags');
        t.append(...tags.map((x) => h('span', 'ov-tag', x)));
        names.append(t);
      }
      const sw = h('input');
      sw.type = 'checkbox';
      sw.className = 'switch small';
      sw.checked = o.enabled;
      sw.title = o.enabled ? 'Shown — click to switch off' : 'Off — click to show';
      sw.onclick = (e) => e.stopPropagation();
      sw.onchange = () => {
        o.enabled = sw.checked;
        save();
        rerender();
      };
      item.append(names, sw);
      const pick = () => {
        selected = o.id;
        ui.query = '';
        rerender();
      };
      item.onclick = pick;
      item.onkeydown = (e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          pick();
        }
      };
      list.append(item);
    }
    list.append(button('＋ Add overlay', () => addOverlayDialog(addOverlay), 'add-btn'));
    const sel = cfg.overlays.find((o) => o.id === selected);
    const main = h('div', 'ov-main');
    if (sel) main.append(overlayEditor(sel, rerender));
    else {
      // A fresh start: nothing yet, and two ways in.
      const empty = h('section', 'card ov-empty');
      const actions = h('div', 'toolbar');
      actions.append(button('＋ Add overlay', () => addOverlayDialog(addOverlay), 'primary'), button('Browse the Layout Market', () => (location.hash = '#market')));
      empty.append(
        h('h2', '', 'No overlays yet'),
        h('p', 'dim', 'Add the overlays you want, one at a time, or start from a whole layout someone has shared. You can move and resize them in Edit mode.'),
        actions,
      );
      main.append(empty);
    }
    const wrap = h('div', 'ov-layout');
    wrap.append(list, main);
    el.replaceChildren(...gameStrip(rerender), wrap);
  },
};

function swatch(name: string, theme: Record<string, unknown>, onApply: () => void, onDelete?: () => void, logo?: string) {
  const t = merge(THEME, theme);
  const el = h('button', `swatch${logo ? ' has-logo' : ''}`);
  el.style.background = t.background as string;
  el.style.color = t.textColor as string;
  el.title = name;
  if (logo) {
    // A sponsored team's card: its logo in place of the name and colour bars.
    const img = h('img', 'swatch-logo') as HTMLImageElement;
    img.src = logo;
    img.alt = name;
    el.append(img);
  } else {
    const bars = h('div', 'swatch-bars');
    for (const k of ['accent', 'playerColor', 'goodColor', 'badColor', 'lapBehindColor']) {
      const b = h('span');
      b.style.background = t[k] as string;
      bars.append(b);
    }
    el.append(h('span', 'swatch-name', name), bars);
  }
  el.onclick = (e) => {
    e.preventDefault();
    onApply();
  };
  if (onDelete) {
    const x = h('span', 'swatch-x', '✕');
    x.title = 'Delete';
    x.onclick = (e) => {
      e.stopPropagation();
      onDelete();
    };
    el.append(x);
  }
  return el;
}

const COLOUR_KEYS = THEME.filter((x) => x.type === 'color').map((x) => x.key);

const themePage: Page = {
  id: 'theme',
  title: 'Theme',
  render(el) {
    const cfg = state.config!;
    const rerender = () => this.render(el);
    applyUiTheme(cfg.theme, document.documentElement);
    const values = themeValues(cfg.theme);
    cfg.savedThemes ??= [];
    const apply = (theme: Record<string, unknown>, coloursOnly: boolean) => {
      // Presets only carry colours, so fonts and sizes the user picked survive.
      const base = coloursOnly ? Object.fromEntries(Object.entries(cfg.theme).filter(([k]) => !COLOUR_KEYS.includes(k) && k !== 'backgroundOpacity' && k !== 'shadow')) : {};
      cfg.theme = { ...base, ...theme };
      save();
      rerender();
    };
    const presets = h('div', 'swatches');
    const sponsored = h('div', 'swatches');
    for (const p of PRESETS) (p.sponsored ? sponsored : presets).append(swatch(p.name, p.values, () => apply(p.values, true), undefined, p.logo));
    const saved = h('div', 'swatches');
    cfg.savedThemes.forEach((t, i) =>
      saved.append(
        swatch(
          t.name,
          t.theme,
          () => apply(t.theme, false),
          () => {
            cfg.savedThemes.splice(i, 1);
            save();
            rerender();
          },
        ),
      ),
    );
    if (!cfg.savedThemes.length) saved.append(h('p', 'dim small', 'No saved themes yet.'));
    const nameInput = h('input');
    nameInput.placeholder = 'Theme name';
    const saveRow = h('div', 'toolbar');
    saveRow.append(
      nameInput,
      button(
        'Save current theme',
        () => {
          const name = nameInput.value.trim() || `Theme ${cfg.savedThemes.length + 1}`;
          cfg.savedThemes = cfg.savedThemes.filter((t) => t.name !== name);
          cfg.savedThemes.push({ name, theme: { ...cfg.theme } });
          save();
          rerender();
        },
        'primary',
      ),
      button('Export theme', () => download('apex-overlays-theme.json', { name: nameInput.value.trim() || 'My theme', theme: cfg.theme })),
      button('Import theme', async () => {
        const t = (await upload()) as { name?: string; theme?: Record<string, unknown> };
        if (!t?.theme) return alert('That file is not an Apex Overlays theme.');
        cfg.savedThemes.push({ name: t.name || 'Imported theme', theme: t.theme });
        save();
        rerender();
      }),
    );
    el.replaceChildren(
      card(
        'Colour themes',
        h('p', 'dim', 'A theme colours every overlay and this settings window. Pick a preset, tweak it below, then save it as your own.'),
        h('h3', '', 'Presets'),
        presets,
        h('h3', '', 'Sponsored by Us'),
        h('p', 'dim small', 'The colours of the teams and leagues Apex Overlays sponsors.'),
        sponsored,
        h('h3', '', 'My themes'),
        saved,
        saveRow,
      ),
      card(
        'Customise',
        form(THEME, values, (k, v) => {
          cfg.theme = { ...cfg.theme, [k]: v };
          save();
          applyUiTheme(cfg.theme, document.documentElement);
        }),
        button('Reset to default', () => apply({}, false)),
      ),
    );
  },
};

const WETNESS: [string, string][] = [
  ['1', '1 – Dry'],
  ['2', '2 – Mostly dry'],
  ['3', '3 – Very lightly wet'],
  ['4', '4 – Lightly wet'],
  ['5', '5 – Moderately wet'],
  ['6', '6 – Very wet'],
  ['7', '7 – Extremely wet'],
];

const strategyPage: Page = {
  id: 'strategy',
  title: 'Strategy',
  render(el) {
    const cfg = state.config!;
    const section = <T extends object>(title: string, obj: T, schema: SettingDef[], intro: string, map?: (k: string, v: unknown) => unknown) =>
      card(
        title,
        h('p', 'dim', intro),
        form(schema, obj as Record<string, unknown>, (k, v) => {
          (obj as Record<string, unknown>)[k] = map ? map(k, v) : v;
          save();
        }),
      );
    el.replaceChildren(
      section(
        'Fuel calculator',
        cfg.fuel,
        [
          { key: 'avgLaps', label: 'Laps averaged', type: 'number', default: 5, min: 1, max: 20 },
          { key: 'marginLaps', label: 'Safety margin', type: 'number', default: 1, min: 0, max: 5, step: 0.1, unit: 'laps' },
          { key: 'raceMinutes', label: 'Race length (minutes)', type: 'number', default: 0, min: 0, max: 1440, unit: 'min', help: 'For the race estimate before the start, when the game does not say (Le Mans Ultimate before the race session). 0: none' },
          { key: 'raceLaps', label: 'Race length (laps)', type: 'number', default: 0, min: 0, max: 2000, unit: 'laps', help: 'For a race set by laps. 0: none' },
          { key: 'includePitLaps', label: 'Include in/out laps', type: 'bool', default: false },
          { key: 'includeCautionLaps', label: 'Include caution laps', type: 'bool', default: false },
        ],
        'Consumption is measured per lap. Laps with refuelling, off-track resets or (optionally) pit/caution laps are ignored.',
      ),
      section(
        'Tyre advisor',
        { ...cfg.tyres, wetThreshold: String(cfg.tyres.wetThreshold), dryThreshold: String(cfg.tyres.dryThreshold) },
        [
          { key: 'wetThreshold', label: 'Advise wets from', type: 'select', default: '4', options: WETNESS },
          { key: 'dryThreshold', label: 'Advise slicks at or below', type: 'select', default: '2', options: WETNESS },
          { key: 'useField', label: 'Use what my class is running', type: 'bool', default: true },
          { key: 'useCrossover', label: 'Compare wet vs dry lap times', type: 'bool', default: true },
          { key: 'crossoverMargin', label: 'Crossover margin', type: 'number', default: 0.5, min: 0, max: 5, step: 0.1, unit: 's/lap' },
        ],
        'The advisor scores track wetness, the rain trend, the field and lap-time crossover. Lower thresholds = earlier calls.',
        (k, v) => {
          const n = k.endsWith('Threshold') ? parseInt(v as string, 10) : v;
          (cfg.tyres as Record<string, unknown>)[k] = n;
          return v;
        },
      ),
      section(
        'Battle box',
        cfg.battle,
        [
          { key: 'maxGap', label: 'Battle range', type: 'number', default: 3, min: 0.2, max: 20, step: 0.1, unit: 's' },
        ],
        'Cars within the battle range are highlighted and their gap trend is tracked. Which cars the box shows (class, overall or on track) is set per Battle box overlay.',
      ),
    );
  },
};

const garage61Page: Page = {
  id: 'garage61',
  title: 'Garage 61',
  render(el) {
    const g = state.config!.garage61;
    const status = state.garage61;
    const statusLine = h('p', `status-line s-${status?.status ?? 'disabled'}`, status ? `${status.status}${status.message ? ' — ' + status.message : ''}${status.user ? ` (signed in as ${status.user})` : ''}` : 'disabled');
    const token = h('input');
    token.type = 'password';
    token.placeholder = g.token === '__keep__' ? 'Token saved — paste a new one to replace' : 'Paste your personal access token';
    token.value = '';
    token.onchange = () => {
      g.token = token.value.trim();
      save(true);
    };
    const tokRow = h('label', 'field');
    tokRow.append(h('span', 'field-label', 'Access token'), token);
    el.replaceChildren(
      card(
        'Garage 61',
        h(
          'p',
          'dim',
          'Live delta to your Garage 61 best (or your team’s), sector by sector, plus your team’s leaderboard. Make a personal access token on garage61.net (Developer → My applications) and paste it below.',
        ),
        form(
          [
            { key: 'enabled', label: 'Enable integration', type: 'bool', default: false },
            {
              key: 'reference',
              label: 'Live delta compares with',
              type: 'select',
              default: 'personal',
              options: [
                ['personal', 'My best lap'],
                ['team', 'The fastest lap in my teams'],
                ['off', 'Nothing (no live delta)'],
              ],
              help: 'Uses your own best if a teammate’s telemetry isn’t shared',
            },
            { key: 'includeTeam', label: 'Include team leaderboard', type: 'bool', default: true },
            {
              key: 'period',
              label: 'Laps from',
              type: 'select',
              default: 'all',
              options: [
                ['all', 'All time'],
                ['season', 'The current iRacing season'],
                ['seasons2', 'This and last season'],
                ['90', 'The last 90 days'],
                ['30', 'The last 30 days'],
                ['7', 'The last 7 days'],
              ],
            },
            {
              key: 'matchConditions',
              label: 'Only laps in similar conditions',
              type: 'bool',
              default: false,
              help: 'Track temperature close to now, and the same dry or wet track',
            },
            { key: 'tempRange', label: 'Track temperature within ±', type: 'number', default: 6, min: 1, max: 30, unit: '°C' },
            { key: 'refreshMinutes', label: 'Refresh every', type: 'number', default: 5, min: 1, max: 60, unit: 'min' },
          ],
          g as unknown as Record<string, unknown>,
          (k, v) => {
            (g as unknown as Record<string, unknown>)[k] = v;
            save();
          },
        ),
        tokRow,
        statusLine,
        ...(status?.status === 'ok'
          ? [
              h('p', 'dim small', `Laps searched: ${status.filters || 'all'}`),
              h(
                'p',
                'dim small',
                status.reference
                  ? `Live delta: ${status.reference.source === 'team' ? `${status.reference.driver}’s team best` : 'your best'} (${f.lapTime(status.reference.lapTime)})${status.referenceNote ? ` — ${status.referenceNote}` : ''}`
                  : `Live delta: ${status.referenceNote || 'off'}`,
              ),
            ]
          : []),
        h('p', 'dim small', 'Remember to enable the “Garage 61” overlay on the Overlays page.'),
      ),
    );
  },
};

/** In-app updates (desktop app only). */
interface LicenceStatus {
  hasKey: boolean;
  key: string;
  valid: boolean;
  user: string;
  tierName: string;
  error: string;
  manageUrl: string;
  getKeyUrl: string;
}

/** The optional Apex Labs key: free during early access, the same key as Apex Commentator. */
function licenceCard(): HTMLElement {
  const c = card('Licence');
  const status = h('div', 'notice', 'Checking…');
  const error = h('p', 'small');
  error.style.color = 'var(--bad)';
  const input = h('input');
  input.placeholder = 'APEX-XXXX-XXXX-XXXX-XXXX';
  input.autocomplete = 'off';
  input.spellcheck = false;
  input.setAttribute('aria-label', 'Apex Labs key');
  input.style.flex = '1 1 260px';
  const bar = h('div', 'toolbar');
  const call = async (method: string, path: string, body?: unknown) => {
    bar.querySelectorAll('button').forEach((b) => (b.disabled = true));
    try {
      const r = await fetch(path, { method, headers: body ? { 'content-type': 'application/json' } : {}, body: body ? JSON.stringify(body) : undefined });
      const data = await r.json().catch(() => ({}));
      if (!r.ok) throw new Error(data.error || `Request failed (${r.status})`);
      render(data as LicenceStatus);
      return true;
    } catch (e) {
      text(error, (e as Error).message);
      bar.querySelectorAll('button').forEach((b) => (b.disabled = false));
      return false;
    }
  };
  // Once added, the key is hidden (APEX-••••-••••-••••-••••) until Show is pressed, so it isn't on
  // screen while streaming. A new key starts hidden again.
  let shownKey = '';
  let keyShown = false;
  const mask = (key: string) => key.split('-').map((part, i) => (i === 0 && part === 'APEX' ? part : '•'.repeat(part.length))).join('-');
  const render = (st: LicenceStatus) => {
    if (st.key !== shownKey) keyShown = false;
    shownKey = st.key;
    text(status, st.valid ? `${st.tierName} · registered to ${st.user || 'you'}` : st.hasKey ? 'Key added, not confirmed yet.' : 'No key added.');
    status.classList.toggle('good', st.valid);
    text(error, st.error);
    const open = (url: string) => () => client.send({ op: 'openUrl', url });
    if (st.hasKey) {
      const code = h('code', 'licence-key');
      code.setAttribute('aria-label', 'Your key');
      const show = button('Show', () => {
        keyShown = !keyShown;
        paint();
      });
      const paint = () => {
        text(code, keyShown ? st.key : mask(st.key));
        code.style.userSelect = keyShown ? 'all' : 'none';
        code.style.opacity = keyShown ? '' : '0.7';
        text(show, keyShown ? 'Hide' : 'Show');
        show.title = keyShown ? 'Hide the key' : 'Show the full key';
        show.setAttribute('aria-pressed', String(keyShown));
      };
      paint();
      bar.replaceChildren(
        code,
        show,
        button('Check now', () => void call('POST', '/api/licence/check')),
        button('Manage key', open(st.manageUrl)),
        button('Remove from this PC', () => void call('DELETE', '/api/licence'), 'danger'),
      );
    } else {
      input.value = '';
      const add = () => {
        if (!input.value.trim()) return text(error, 'Paste your key first.');
        void call('POST', '/api/licence', { key: input.value });
      };
      input.onkeydown = (e) => {
        if (e.key === 'Enter') add();
      };
      bar.replaceChildren(input, button('Add key', add, 'primary'), button('Get your key', open(st.getKeyUrl)));
    }
  };
  c.append(
    h('p', 'dim', 'Apex Overlays is free during early access and works without a key. A free Apex Labs key registers you, works in Apex Commentator too, and is how paid plans and team or league perks will reach you later.'),
    status,
    bar,
    error,
  );
  fetch('/api/licence')
    .then((r) => r.json())
    .then(render)
    .catch(() => text(status, 'Licence status unavailable.'));
  return c;
}

function updatesCard(): HTMLElement {
  const cfg = state.config!;
  const u = state.update;
  const c = card('Updates');
  const current = u?.current ? `You have version ${u.current}.` : '';
  const line: Record<string, string> = {
    unsupported: u?.message ?? 'Updates are installed by the Apex Overlays desktop app.',
    idle: 'Not checked yet.',
    checking: 'Checking for updates…',
    none: 'You are on the latest version.',
    available: `Version ${u?.version} is available.`,
    downloading: `Downloading version ${u?.version}… ${u?.progress ?? 0}%`,
    installing: 'Installing — Apex Overlays will restart.',
    error: `Update check failed: ${u?.message ?? 'unknown error'}`,
  };
  const status = h('div', `notice${u?.status === 'available' ? ' good' : ''}`, `${current} ${line[u?.status ?? 'idle'] ?? ''}`);
  const bar = h('div', 'toolbar');
  if (u?.status !== 'unsupported') {
    bar.append(button('Check for updates', () => client.send({ op: 'checkUpdate' })));
    if (u?.status === 'available') bar.append(button(`Install ${u.version} and restart`, () => client.send({ op: 'installUpdate' }), 'primary'));
  }
  const auto = h('label', 'inline');
  const cb = h('input');
  cb.type = 'checkbox';
  cb.className = 'switch';
  cb.checked = cfg.autoCheckUpdates ?? true;
  cb.onchange = () => {
    cfg.autoCheckUpdates = cb.checked;
    save();
  };
  auto.append(cb, ' Check for updates when the app starts (and offer them in a pop-up)');
  c.append(status, bar, auto);
  const pp = cfg.updatePostpone;
  if (pp?.version) {
    const until = pp.until == null ? `Version ${pp.version} is skipped — the pop-up won't offer it again.` : `The pop-up for version ${pp.version} is postponed until ${new Date(pp.until * 1000).toLocaleString()}.`;
    if (pp.until == null || pp.until * 1000 > Date.now()) {
      c.append(
        h('p', 'dim small', until),
        button('Remind me again on start-up', () => {
          cfg.updatePostpone = null;
          save(true);
        }),
      );
    }
  }
  if (u?.status === 'available') {
    const releases = u.releases?.length ? u.releases : u.notes ? [{ version: u.version ?? '', notes: u.notes }] : [];
    if (releases.length > 1) c.append(h('h3', '', `What’s new in the ${releases.length} versions since yours`), releaseHistory(releases));
    else if (releases.length) c.append(h('h3', '', `What’s new in ${u.version}`), releaseHistory(releases));
  }
  return c;
}

/** Track outline + corner list editor for the current track. */
function cornersCard(rerender: () => void): HTMLElement {
  const cfg = state.config!;
  const tm = state.trackmap;
  const c = card('Track map & corners');
  const outline = h('select');
  for (const [v, t] of [
    ['iracing', 'iRacing’s official track map (recorded from my driving if a track has none)'],
    ['recorded', 'Recorded from my own driving'],
  ]) {
    const o = h('option', '', t);
    o.value = v;
    outline.append(o);
  }
  outline.value = cfg.trackOutline ?? 'iracing';
  outline.onchange = () => {
    cfg.trackOutline = outline.value as 'iracing' | 'recorded';
    save(true);
  };
  const outlineRow = h('label', 'field');
  outlineRow.append(h('span', 'field-label', 'Track outline'), outline);
  c.append(outlineRow);
  if (!tm?.trackId) {
    c.append(h('p', 'dim', 'Join a session to manage the track outline and corner names for that track.'));
    return c;
  }
  const key = String(tm.trackId);
  const custom = cfg.trackCorners?.[key];
  const sourceText: Record<string, string> = { custom: 'your custom list', lovely: 'Lovely Track Data (community)', mock: 'the built-in simulator', none: 'none found yet' };
  c.append(
    h(
      'p',
      'dim',
      `${tm.trackName}: outline ${({ iracing: 'from iRacing’s official track map', recorded: 'recorded from your driving', library: 'from the shared map library', mock: 'from the simulator' } as Record<string, string>)[tm.source ?? ''] ?? (tm.recording ? 'recording…' : 'not recorded yet')}. Corners from ${sourceText[tm.cornersSource] ?? tm.cornersSource}.`,
    ),
    button('Re-record track outline', () => client.send({ op: 'resetTrackMap' })),
  );
  const list = custom ?? tm.corners;
  const table = h('table', 'corners');
  const head = h('tr');
  for (const t of ['Turn', 'Name', 'Start %', 'End %', '']) head.append(h('th', '', t));
  table.append(head);
  list.forEach((corner, i) => {
    const tr = h('tr');
    const cell = (node: Node) => {
      const td = h('td');
      td.append(node);
      tr.append(td);
    };
    if (!custom) {
      tr.append(h('td', '', corner.number > 0 ? String(corner.number) : '–'), h('td', '', corner.name || '–'), h('td', '', (corner.start * 100).toFixed(1)), h('td', '', (corner.end * 100).toFixed(1)), h('td'));
      table.append(tr);
      return;
    }
    const num = h('input');
    num.type = 'number';
    num.value = String(corner.number);
    num.oninput = () => {
      corner.number = parseInt(num.value, 10) || 0;
      save();
    };
    const name = h('input', 'cname');
    name.value = corner.name;
    name.placeholder = 'e.g. Eau Rouge';
    name.oninput = () => {
      corner.name = name.value;
      save();
    };
    const pct = (field: 'start' | 'end') => {
      const inp = h('input');
      inp.type = 'number';
      inp.step = '0.1';
      inp.value = (corner[field] * 100).toFixed(1);
      inp.oninput = () => {
        const v = parseFloat(inp.value);
        if (Number.isFinite(v)) {
          corner[field] = Math.min(1, Math.max(0, v / 100));
          save();
        }
      };
      return inp;
    };
    cell(num);
    cell(name);
    cell(pct('start'));
    cell(pct('end'));
    cell(
      button(
        '✕',
        () => {
          custom.splice(i, 1);
          save();
          rerender();
        },
        'mini',
      ),
    );
    table.append(tr);
  });
  const bar = h('div', 'toolbar');
  if (!custom) {
    bar.append(
      button(
        list.length ? 'Edit these corners' : 'Create a corner list',
        () => {
          cfg.trackCorners = { ...(cfg.trackCorners ?? {}), [key]: list.map((x) => ({ ...x })) };
          save();
          rerender();
        },
        'primary',
      ),
    );
  } else {
    bar.append(
      button(
        'Add corner at my car’s position',
        async () => {
          // Your car's position, fetched now (the page does not stream it, to save CPU).
          const p = (await fetchOnce<Player>('player'))?.lapPct ?? state.player?.lapPct ?? 0;
          custom.push({ name: '', number: (custom.reduce((m, x) => Math.max(m, x.number), 0) || 0) + 1, start: Math.max(0, p - 0.005), end: Math.min(1, p + 0.005) });
          custom.sort((a, b) => a.start - b.start);
          save();
          rerender();
        },
        'primary',
      ),
      button('Revert to downloaded corners', () => {
        const next = { ...cfg.trackCorners };
        delete next[key];
        cfg.trackCorners = next;
        save();
        rerender();
      }),
    );
  }
  c.append(table, bar, h('p', 'dim small', 'Tip: drive to a corner, then click “Add corner at my car’s position”. Corner labels are switched on in the Track map overlay settings.'));
  return c;
}

/** Switch to a saved layout, keeping the arrangement in use in the layout it came from. */
// Your Garage 61 teams, for the delta bar's team choice.
CHOICES.g61Teams = () => (state.garage61?.teams ?? []).map((t) => [t.slug, t.name]);

export function applyLayout(id: string) {
  const cfg = state.config!;
  const next = cfg.layouts?.find((l) => l.id === id);
  if (!next) return;
  const active = cfg.layouts.find((l) => l.id === cfg.activeLayout);
  if (active) active.overlays = structuredClone(cfg.overlays);
  // Overlays the layout does not have come along switched off (same id), so their hotkeys
  // work in every layout. Matches Config::apply_layout in the app.
  const missing = cfg.overlays.filter((o) => !next.overlays.some((n) => n.id === o.id)).map((o) => ({ ...structuredClone(o), enabled: false }));
  cfg.overlays = [...structuredClone(next.overlays), ...missing];
  cfg.activeLayout = id;
  save(true);
}

/** Live readings of what Apex Overlays uses, updated in place once a second. */
const monitorView = {
  tiles: new Map<string, HTMLElement>(),
  /** The headline readings' graphs (last minute), and their history. */
  sparks: new Map<string, SVGPolylineElement>(),
  hist: new Map<string, number[]>(),
};

/** The readings at the top of General, each with its last minute as a small graph. */
const HEADLINE: [string, (m: NonNullable<typeof state.monitor>) => number, number][] = [
  ['cpu', (m) => m.cpu, 5],
  ['memory', (m) => m.memoryMb, 50],
  ['sent', (m) => m.sentKbps, 10],
  ['tick', (m) => m.tickUs, 100],
];

function headlineTiles(): HTMLElement {
  const row = h('section', 'gen-tiles');
  row.setAttribute('aria-label', 'Live readings');
  for (const [id] of HEADLINE) {
    const s = STATS.find((x) => x.id === id)!;
    const tile = h('div', 'gen-tile');
    tile.title = s.help;
    const v = h('div', 'gen-tile-val', '–');
    const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    svg.setAttribute('viewBox', '0 0 100 24');
    svg.setAttribute('preserveAspectRatio', 'none');
    svg.setAttribute('aria-hidden', 'true');
    svg.classList.add('gen-spark');
    const line = document.createElementNS('http://www.w3.org/2000/svg', 'polyline');
    svg.append(line);
    tile.append(h('div', 'gen-tile-label', s.label), v, svg);
    monitorView.tiles.set(id, v);
    monitorView.sparks.set(id, line);
    row.append(tile);
  }
  return row;
}

/** Shows the latest readings; `record`: a new reading arrived (added to the graphs). */
function drawMonitor(record = false) {
  const m = state.monitor;
  if (!m) return;
  for (const [id, get] of HEADLINE) {
    if (!record) break;
    const hst = monitorView.hist.get(id) ?? [];
    hst.push(get(m));
    if (hst.length > 60) hst.shift();
    monitorView.hist.set(id, hst);
  }
  for (const s of STATS) {
    const el = monitorView.tiles.get(s.id);
    if (!el) continue;
    // The headline readings roll to their new value; the rest change in place.
    if (monitorView.sparks.has(s.id)) roll(el, s.value(m));
    else el.textContent = s.value(m);
    const lvl = s.level?.(m, 5) ?? '';
    el.classList.toggle('warn', lvl === 'warn');
    el.classList.toggle('bad', lvl === 'bad');
  }
  // Each headline reading over the last minute, scaled to its own peak (with a floor).
  for (const [id, , floor] of HEADLINE) {
    const pts = monitorView.hist.get(id) ?? [];
    const top = Math.max(floor, ...pts);
    monitorView.sparks.get(id)?.setAttribute('points', pts.map((v, i) => `${(i / 59) * 100},${23 - (v / top) * 21}`).join(' '));
  }
}

listen((t) => {
  if (t === 'monitor' && monitorView.tiles.size && document.contains([...monitorView.tiles.values()][0])) drawMonitor(true);
});

/** Overlays page: whose overlays these are, and copying the other game's. */
function gameStrip(rerender: () => void): HTMLElement[] {
  const cfg = state.config!;
  const game = cfg.layoutGame ?? '';
  if (!game) return [];
  const others = Object.entries(cfg.gameSets ?? {}).filter(([g, set]) => g !== game && set.overlays.length);
  const strip = h('div', 'notice game-strip');
  strip.append(
    h('span', '', `Overlays for ${gameName(game)}. Each game keeps its own overlays and layouts: pick the other game under General → Game to set it up.`),
  );
  for (const [g, set] of others) {
    strip.append(
      button(`Copy ${gameName(g)}’s overlays here`, () => {
        if (!confirm(`Replace ${gameName(game)}’s overlays and layouts with a copy of ${gameName(g)}’s?`)) return;
        cfg.overlays = structuredClone(set.overlays);
        cfg.layouts = structuredClone(set.layouts);
        cfg.activeLayout = set.activeLayout;
        save(true);
        rerender();
      }),
    );
  }
  return [strip];
}

/** A game's name for people ('iracing' → iRacing). */
export function gameName(id: string): string {
  return id === 'mock' ? 'Mock sim' : id === 'lmu' ? 'Le Mans Ultimate' : 'iRacing';
}

/** Which game to read: automatic, or one chosen by hand. */
function gameCard(): HTMLElement {
  const cfg = state.config!;
  const c = card('Game');
  c.append(
    h('p', 'dim', 'Apex Overlays reads iRacing and Le Mans Ultimate. Automatic switches to whichever game you start; pick one to stay with it.'),
    form(
      [
        {
          key: 'game',
          label: 'Game',
          type: 'select',
          default: 'auto',
          options: [
            ['auto', 'Automatic'],
            ['iracing', 'iRacing'],
            ['lmu', 'Le Mans Ultimate'],
          ],
          help: 'Automatic: whichever game is running',
        },
      ],
      { game: cfg.game ?? 'auto' },
      (_k, v) => {
        cfg.game = v as 'auto' | 'iracing' | 'lmu';
        save();
      },
    ),
  );
  const s = state.status;
  const auto = (cfg.game ?? 'auto') === 'auto';
  const line = !s
    ? 'Not connected to the app.'
    : s.connected
      ? `Reading ${gameName(s.source)}${auto ? ' (found automatically)' : ''}.`
      : s.source
        ? `Waiting for ${gameName(s.source)} — start a session in the game.`
        : 'Looking for iRacing or Le Mans Ultimate — start a session in either game.';
  c.append(h('div', `notice${s?.connected ? ' good' : ''}`, line));
  c.append(
    h(
      'p',
      'dim small',
      'Le Mans Ultimate: its built-in shared memory must be on. Turn on “Enable Plugins” in the game’s settings, then restart the game. No plugin needs installing.',
    ),
    h(
      'p',
      'dim small',
      'LMU has no iRating, licences or incident points, so those columns stay empty. Track maps draw themselves within about a lap.',
    ),
    withLmuBadge(button('Export my LMU track maps', async () => {
      // For the shared map library, so other drivers get these tracks without mapping them.
      try {
        const r = await fetch('/api/lmu-track-maps');
        const d = (await r.json()) as { maps: unknown[] };
        if (!d.maps?.length) {
          alert('No Le Mans Ultimate track maps yet: they are made as you go on track in LMU.');
          return;
        }
        download('apex-lmu-track-maps.json', d);
      } catch {
        alert('Could not read the track maps.');
      }
    })),
  );
  return c;
}

/** VR (SteamVR overlays or the OpenXR layer, desktop app on Windows). */
function vrCard(): HTMLElement {
  const cfg = state.config!;
  cfg.vr ??= { enabled: false, mode: 'auto', distance: 0.9, width: 1.2, height: -0.05, offset: 0 };
  cfg.vr.mode ??= 'auto';
  const c = card('VR');
  c.append(
    h(
      'p',
      'dim',
      'Shows your overlays in the headset, as on your screen. Each overlay’s VR tab sets where it goes.',
    ),
    form(
      [
        { key: 'enabled', label: 'Show overlays in VR', type: 'bool', default: false },
        {
          key: 'mode',
          label: 'How',
          type: 'select',
          default: 'auto',
          options: [
            ['auto', 'Automatic (recommended)'],
            ['openxr', 'OpenXR only (no SteamVR needed)'],
            ['steamvr', 'SteamVR overlays only'],
          ],
          help: 'Automatic: drawn inside the game with OpenXR (Virtual Desktop, Quest Link, Pimax…), otherwise SteamVR overlays',
        },
        { key: 'distance', label: 'Screen layout: distance in front', type: 'number', default: 0.9, min: 0.2, max: 5, step: 0.05, unit: 'm', help: 'Overlays placed “as on my screen” sit on an imaginary screen this far away' },
        { key: 'width', label: 'Screen layout: width', type: 'number', default: 1.2, min: 0.2, max: 6, step: 0.05, unit: 'm' },
        { key: 'height', label: 'Screen layout: raise / lower', type: 'number', default: -0.05, min: -2, max: 2, step: 0.01, unit: 'm' },
        { key: 'offset', label: 'Screen layout: move left / right', type: 'number', default: 0, min: -2, max: 2, step: 0.01, unit: 'm', help: '+ moves every overlay placed “as on my screen” to the right' },
      ],
      cfg.vr as unknown as Record<string, unknown>,
      (k, v) => {
        (cfg.vr as unknown as Record<string, unknown>)[k] = v;
        save();
      },
    ),
  );
  const v = state.vr;
  const line: Record<string, string> = {
    off: 'Off.',
    waiting: v?.message ? (cfg.vr.mode === 'steamvr' ? `Waiting for SteamVR — ${v.message}.` : `${v.message}.`) : 'Waiting for SteamVR.',
    on: `In VR: ${v?.overlays ?? 0} overlay${v?.overlays === 1 ? '' : 's'} showing.${v?.message ? ` ${v.message}.` : ''}`,
    error: `VR is not working: ${v?.message ?? ''}`,
  };
  c.append(h('div', `notice${v?.status === 'on' ? ' good' : ''}`, v ? (line[v.status] ?? v.status) : 'VR works in the Apex Overlays desktop app on Windows, with SteamVR or any OpenXR runtime.'));
  // Separate VR positions: moving overlays in VR leaves the screen layout alone.
  const ownVr = cfg.overlays.filter((o) => o.vrRect).length;
  c.append(
    h(
      'p',
      'dim small',
      'In VR, moving overlays only changes their VR positions; your screen layout is kept.',
    ),
  );
  if (ownVr)
    c.append(
      button(`Reset VR positions to the screen layout (${ownVr} overlay${ownVr === 1 ? '' : 's'})`, () => {
        for (const o of cfg.overlays) o.vrRect = null;
        save();
      }),
    );
  c.append(
    h(
      'p',
      'dim small',
      'Set a “Recentre VR” hotkey under Hotkeys to re-centre the overlays on where your head is.',
    ),
    h(
      'p',
      'dim small',
      'Virtual Desktop: set the OpenXR runtime to VDXR in the Streamer, then pick OpenXR in iRacing. No SteamVR needed.',
    ),
    h(
      'p',
      'dim small',
      'Le Mans Ultimate works the same way, and keeps its own overlays and VR positions.',
    ),
    h(
      'p',
      'dim small',
      'The first time, Windows asks to register the OpenXR layer. Restart the game after turning VR on. Works with NextLevelXR and OpenXR Toolkit.',
    ),
    h(
      'p',
      'dim small',
      'Overlays missing or cropped with other OpenXR layers? Use the free “OpenXR API Layers GUI” to move “XR_APILAYER_APEX_overlays” to the other end of the list.',
    ),
  );
  return c;
}

function monitoringCard(): HTMLElement {
  const cfg = state.config!;
  const c = card('Monitoring');
  c.append(h('p', 'dim', 'What Apex Overlays uses right now: the app and all its overlay windows. Hover a reading for what it means.'));
  const grid = h('div', 'mon-tiles');
  // The headline readings are at the top of the page; the rest here.
  for (const s of STATS.filter((x) => !HEADLINE.some(([id]) => id === x.id))) {
    const tile = h('div', 'mon-tile');
    tile.title = s.help;
    const v = h('div', 'mon-tile-val', '–');
    tile.append(h('div', 'mon-tile-label', s.label), v);
    monitorView.tiles.set(s.id, v);
    grid.append(tile);
  }
  // The on-screen widget: one Performance monitor overlay.
  const widget = cfg.overlays.find((o) => o.kind === 'monitor');
  const row = h('div', 'toolbar mon-widget');
  const sw = h('label', 'inline');
  const cb = h('input');
  cb.type = 'checkbox';
  cb.className = 'switch';
  cb.checked = !!widget?.enabled;
  cb.onchange = () => {
    let o = cfg.overlays.find((x) => x.kind === 'monitor');
    if (!o && cb.checked) {
      const d = byKind('monitor')!;
      o = { id: uid('monitor'), kind: 'monitor', name: '', enabled: true, x: 1680, y: 40, width: d.size[0], height: d.size[1], settings: {} };
      cfg.overlays.push(o);
    } else if (o) o.enabled = cb.checked;
    save(true);
  };
  sw.append(cb, ' Show a performance widget on screen');
  row.append(
    sw,
    button('Choose what it shows…', () => {
      let o = cfg.overlays.find((x) => x.kind === 'monitor');
      if (!o) {
        const d = byKind('monitor')!;
        o = { id: uid('monitor'), kind: 'monitor', name: '', enabled: false, x: 1680, y: 40, width: d.size[0], height: d.size[1], settings: {} };
        cfg.overlays.push(o);
        save(true);
      }
      selected = o.id;
      ui.tab.set(o.id, 'What to show');
      location.hash = '#overlays';
    }),
  );
  c.append(grid, row);
  drawMonitor();
  return c;
}

/** Where the log file is, to open its folder or copy it for a bug report. */
function logsCard(): HTMLElement {
  const where = h('code', '', '…');
  const note = h('p', 'dim small');
  const copyBtn = button('Copy log', async () => {
    const r = await fetch('/api/log').then((x) => x.json()).catch(() => null);
    if (!r?.text) return alert('The log is empty or could not be read.');
    copy(r.text, copyBtn);
  });
  const c = card(
    'Logs',
    h('p', 'dim', 'Everything the app does, and every error, goes in apex.log. If something goes wrong, send that file with your report.'),
    (() => {
      const p = h('p', '', 'Log folder: ');
      p.append(where);
      return p;
    })(),
    button('Open log folder', () => client.send({ op: 'openLogs' })),
    copyBtn,
    note,
  );
  fetch('/api/log')
    .then((x) => x.json())
    .then((r) => {
      text(where, r.folder);
      text(note, r.previous ? 'Copy log copies the end of this run’s log; the previous run’s is in the folder.' : '');
    })
    .catch(() => text(where, 'unknown'));
  return c;
}

const generalPage: Page = {
  id: 'general',
  title: 'General',
  render(el) {
    const cfg = state.config!;
    const backup = () => download('apex-overlays.json', { ...cfg, garage61: { ...cfg.garage61, token: '' } });
    pageActions(button('Open logs', () => client.send({ op: 'openLogs' })), button('Back up settings', backup));
    const opts = {
      units: cfg.units,
      port: cfg.server.port,
      lanAccess: cfg.server.lanAccess,
      ratingsOfficialOnly: cfg.ratingsOfficialOnly,
      shortClassNames: cfg.shortClassNames,
      updateRate: cfg.updateRate ?? 'balanced',
      closeAction: cfg.closeAction ?? 'ask',
      trayOnPitExit: cfg.trayOnPitExit ?? true,
      lmuPrivateQualifying: cfg.lmuPrivateQualifying ?? true,
      uiAnimations: cfg.uiAnimations ?? true,
    };
    const SCHEMA: SettingDef[] = [
            { key: 'units', label: 'Units', type: 'select', default: 'auto', options: [['auto', 'Same as the game'], ['metric', 'Metric'], ['imperial', 'Imperial']], group: 'Display' },
            {
              key: 'ratingsOfficialOnly',
              label: 'iRating / SR only in official races',
              type: 'bool',
              default: true,
              help: 'Leagues and hosted sessions do not change iRating or SR',
              group: 'Display',
            },
            {
              key: 'shortClassNames',
              label: 'Short class names (GT3, TCR, LMP2…)',
              type: 'bool',
              default: true,
              help: 'From the community Lovely Car Classes and Car Data lists; leagues use their own class names',
              group: 'Display',
            },
            {
              key: 'lmuPrivateQualifying',
              label: 'Private qualifying: hide other cars',
              type: 'bool',
              default: true,
              help: 'Le Mans Ultimate’s online qualifying and iRacing’s Lone Qualify put each driver alone on track, so the other cars are left off the relative, track map, radar, spotter and battle box. Open qualifying (everyone out together) is not affected',
              group: 'Display',
            },
            {
              key: 'updateRate',
              label: 'Overlay update rate',
              type: 'select',
              default: 'balanced',
              options: [
                ['fast', 'Light (least CPU and FPS impact)'],
                ['balanced', 'Balanced (recommended)'],
                ['full', 'Every tick (60/s, most CPU)'],
              ],
              help: 'Light: least CPU and FPS impact. Balanced: smooth driving aids, timing tables a little less often. Every tick: everything 60 times a second.',
              group: 'Performance',
            },
            {
              key: 'closeAction',
              label: 'When I close this window',
              type: 'select',
              default: 'ask',
              options: [
                ['ask', 'Ask me'],
                ['quit', 'Close the overlays too'],
                ['keep', 'Keep the overlays running (tray icon)'],
              ],
              group: 'Display',
            },
            {
              key: 'uiAnimations',
              label: 'Animations in this window',
              type: 'bool',
              default: true,
              help: 'The lights as the game connects, pages settling in, the “saved” board. Each plays once and stops; Windows’ “Animation effects: off” turns them off too',
              group: 'Display',
            },
            {
              key: 'trayOnPitExit',
              label: 'Minimise to the tray when I leave the pits',
              type: 'bool',
              default: true,
              help: 'This window closes to the tray as you drive out, freeing its CPU and memory for the game. Opened again from the tray while on track, it stays open',
              group: 'Performance',
            },
            { key: 'port', label: 'Port', type: 'number', default: 8182, min: 1024, max: 65535, group: 'Server (restart to apply)' },
            { key: 'lanAccess', label: 'Allow other PCs on my network', type: 'bool', default: false, group: 'Server (restart to apply)' },
          ];
    const rerender = () => this.render(el);
    let refresh = () => {};
    /** Settings shown in Essentials and in their own tab: the page is drawn again so both agree. */
    const TWICE = new Set(['units', 'updateRate']);
    const onChange = (k: string, v: unknown) => {
      if (k === 'units') cfg.units = v as typeof cfg.units;
      if (k === 'port') cfg.server.port = v as number;
      if (k === 'lanAccess') cfg.server.lanAccess = v as boolean;
      if (k === 'ratingsOfficialOnly') cfg.ratingsOfficialOnly = v as boolean;
      if (k === 'shortClassNames') cfg.shortClassNames = v as boolean;
      if (k === 'updateRate') cfg.updateRate = v as 'fast' | 'balanced' | 'full';
      if (k === 'closeAction') cfg.closeAction = v as 'ask' | 'quit' | 'keep';
      if (k === 'trayOnPitExit') cfg.trayOnPitExit = v as boolean;
      if (k === 'uiAnimations') {
        cfg.uiAnimations = v as boolean;
        document.body.classList.toggle('no-motion', v === false);
      }
      if (k === 'lmuPrivateQualifying') cfg.lmuPrivateQualifying = v as boolean;
      save();
      if (TWICE.has(k)) rerender();
      else refresh();
    };

    /** A row the page can mark as changed and put back to its default. */
    interface Item {
      el: HTMLElement;
      changed: () => boolean;
      reset: () => void;
      hot?: { get: () => string };
    }
    const dotted = (row: HTMLElement) => {
      row.classList.add('sec-row');
      const d = h('span', 'chg-dot');
      d.title = 'Changed from the default';
      row.prepend(d);
      return row;
    };
    const values = opts as unknown as Values;
    const settingRow = (key: string): Item => {
      const s = SCHEMA.find((x) => x.key === key)!;
      return {
        el: dotted(field(s, values, onChange)),
        changed: () => !same(values[key], s.default),
        reset: () => {
          values[key] = structuredClone(s.default);
          onChange(key, values[key]);
        },
      };
    };
    const GAME: SettingDef = {
      key: 'game',
      label: 'Game',
      type: 'select',
      default: 'auto',
      options: [
        ['auto', 'Automatic'],
        ['iracing', 'iRacing'],
        ['lmu', 'Le Mans Ultimate'],
      ],
      help: 'Automatic: whichever game is running',
    };
    const gameRow = (): Item => ({
      el: dotted(
        field(GAME, { game: cfg.game ?? 'auto' }, (_k, v) => {
          cfg.game = v as 'auto' | 'iracing' | 'lmu';
          save();
          rerender();
        }),
      ),
      changed: () => (cfg.game ?? 'auto') !== 'auto',
      reset: () => {
        cfg.game = 'auto';
        save();
        rerender();
      },
    });
    // The update rate as three cards to pick from.
    const rateDef = SCHEMA.find((x) => x.key === 'updateRate')!;
    const RATE_TEXT: Record<string, [string, string]> = { fast: ['Light', 'Least CPU and FPS impact'], balanced: ['Balanced', 'Recommended'], full: ['Every tick', '60 times a second'] };
    const rateRow = (): Item => {
      const rates = h('div', 'rate-cards');
      rates.setAttribute('role', 'radiogroup');
      rates.setAttribute('aria-label', 'Overlay update rate');
      for (const [id] of rateDef.type === 'select' ? rateDef.options : []) {
        const b = h('button', `rate-card${opts.updateRate === id ? ' sel' : ''}`);
        b.setAttribute('role', 'radio');
        b.setAttribute('aria-checked', String(opts.updateRate === id));
        b.append(h('span', 'rate-name', RATE_TEXT[id]?.[0] ?? id), h('span', 'rate-help', RATE_TEXT[id]?.[1] ?? ''));
        b.onclick = () => {
          opts.updateRate = id as typeof opts.updateRate;
          onChange('updateRate', id);
        };
        rates.append(b);
      }
      const row = h('div', 'field wide rate-row');
      row.dataset.key = 'updateRate';
      const lab = h('span', 'field-label');
      lab.append(h('span', 'field-name', 'Overlay update rate'), h('small', 'help', 'How often overlays redraw. Lower uses less CPU and leaves more frames for the game.'));
      row.append(lab, rates);
      return {
        el: dotted(row),
        changed: () => opts.updateRate !== 'balanced',
        reset: () => {
          opts.updateRate = 'balanced';
          onChange('updateRate', 'balanced');
        },
      };
    };

    // Hotkeys: each row with its clash warning; all of them checked for clashes.
    const hk = cfg.hotkeys;
    const keyName = (k: string) => k.replace('CmdOrCtrl', 'Ctrl').replace('Shift', '⇧').split('+').join(' ');
    // One entry per hotkey setting (Show / hide all appears on two tabs but is one hotkey).
    const hots = new Map<string, { label: string; get: () => string }>();
    const warns: [HTMLElement, () => string][] = [];
    const hotRow = (id: string, label: string, get: () => string, set: (v: string) => void, def = '', sub = '', twice = false): Item => {
      hots.set(id, { label, get });
      const row = h('div', 'field hotkey-row');
      const lab = h('span', 'field-label');
      lab.append(h('span', 'field-name', label));
      if (sub) lab.append(h('small', 'help', sub));
      const warn = h('small', 'help bad', 'Also used by another hotkey');
      warn.hidden = true;
      lab.append(warn);
      warns.push([warn, get]);
      row.append(
        lab,
        hotkeyInput(get() ?? '', (v) => {
          set(v);
          save(true);
          if (twice) rerender();
          else refresh();
        }),
      );
      return {
        el: dotted(row),
        hot: { get },
        changed: () => (get() ?? '') !== def,
        reset: () => {
          set(def);
          save(true);
          rerender();
        },
      };
    };
    const editHot = () => hotRow('edit', 'Edit mode (move & resize)', () => hk.toggleEdit, (v) => (hk.toggleEdit = v), 'CmdOrCtrl+Shift+E');
    const visHot = () => hotRow('visibility', 'Show / hide all overlays', () => hk.toggleVisibility, (v) => (hk.toggleVisibility = v), 'CmdOrCtrl+Shift+H', '', true);
    const clashBanner = h('div', 'notice bad gen-clash');
    const clashCheck = () => {
      const list = [...hots.values()];
      const used = list.map((x) => (x.get() ?? '').trim().toLowerCase()).filter(Boolean);
      const dup = new Set(used.filter((k, i) => used.indexOf(k) !== i));
      for (const [w, get] of warns) w.hidden = !dup.has((get() ?? '').trim().toLowerCase());
      const first = [...dup][0];
      const who = list.filter((x) => (x.get() ?? '').trim().toLowerCase() === first).map((x) => `“${x.label}”`);
      const combo = first ? keyName(list.find((x) => (x.get() ?? '').trim().toLowerCase() === first)!.get()) : '';
      text(clashBanner, first ? `${combo} is used by ${who.join(' and ')}. Change one of them, or clear one with its ×.` : '');
      return !!first;
    };

    // Sections: the same look as an overlay's settings.
    interface Sec {
      tab: string;
      id: string;
      title: string;
      el: HTMLElement;
      body: HTMLElement;
      items: Item[];
      master?: { item: Item; on: () => boolean };
      count: HTMLElement;
      reset: HTMLElement;
      chevron: HTMLButtonElement | null;
      hiddenNote: HTMLElement;
      tabName: HTMLElement;
    }
    const secs: Sec[] = [];
    const masterSwitch = (get: () => boolean, set: (v: boolean) => void, def: boolean, help: string) => {
      const input = h('input');
      input.type = 'checkbox';
      input.className = 'switch';
      input.checked = get();
      input.onchange = () => {
        set(input.checked);
        save();
        refresh();
      };
      const item: Item = {
        el: input,
        changed: () => get() !== def,
        reset: () => {
          set(def);
          save();
          rerender();
        },
      };
      return { input, item, on: get, help };
    };
    const section = (tab: string, id: string, title: string, items: Item[], extra: Node[] = [], o: { master?: ReturnType<typeof masterSwitch>; fold?: boolean } = {}) => {
      const sec = h('section', 'sec gen-sec');
      const head = h('div', 'sec-head');
      const titles = h('div', 'sec-titles');
      const name = h('div', 'sec-title-row');
      const tabName = h('span', 'sec-tab dim', `${tab} ›`);
      name.append(tabName, h('span', 'sec-title', title));
      titles.append(name);
      if (o.master?.help) titles.append(h('small', 'help', o.master.help));
      const count = h('span', 'sec-count');
      const hiddenNote = h('span', 'sec-hidden dim');
      const all = [...(o.master ? [o.master.item] : []), ...items];
      const reset = button(
        'Reset section',
        () => {
          for (const it of all) if (it.changed()) it.reset();
          rerender();
        },
        'subtle sec-reset',
      );
      head.append(h('span', 'chg-dot'), titles, count, hiddenNote, reset);
      if (o.master) {
        head.append(o.master.input);
        sec.classList.add('has-master');
      }
      const body = h('div', 'sec-body gen-body');
      body.append(...items.map((i) => i.el), ...extra);
      let chevron: HTMLButtonElement | null = null;
      if (o.fold !== false) {
        chevron = button(
          '',
          () => {
            const f = !sec.classList.contains('folded');
            setFolded('general', id, f);
            sec.classList.toggle('folded', f);
            refresh();
          },
          'sec-fold',
        );
        chevron.innerHTML = '<svg viewBox="0 0 12 12" width="12" height="12" aria-hidden="true"><path d="M2.5 4.5 6 8l3.5-3.5" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>';
        head.append(chevron);
        if (isFolded('general', id)) sec.classList.add('folded');
      }
      sec.append(head, body);
      secs.push({ tab, id, title, el: sec, body, items, master: o.master, count, reset, chevron, hiddenNote, tabName });
    };
    /** A card's content without its title, to sit in a section. */
    const cardBody = (c: HTMLElement, drop: (x: Element) => boolean = () => false) => [...c.children].filter((x) => x.tagName !== 'H2' && !drop(x));
    const info = (label: string, value: HTMLElement, help = '') => {
      const row = h('div', 'field sec-row info-row');
      const lab = h('span', 'field-label');
      lab.append(h('span', 'field-name', label));
      if (help) lab.append(h('small', 'help', help));
      row.append(lab, value);
      return row;
    };

    // Essentials: what most people set once, plus how the app stands.
    const u = state.update;
    const updPill = h('span', `gen-pill${u?.status === 'none' ? ' good' : ''}`, u?.status === 'available' ? `Version ${u.version} available` : u?.status === 'none' ? `Up to date · ${u.current}` : u?.current ? `Version ${u.current}` : 'In the desktop app');
    const licPill = h('span', 'gen-pill', '…');
    fetch('/api/licence')
      .then((r) => r.json())
      .then((l: LicenceStatus) => {
        text(licPill, l.valid ? l.tierName : l.hasKey ? 'Key not confirmed yet' : 'Free early access');
        licPill.classList.toggle('good', l.valid);
      })
      .catch(() => text(licPill, 'Unavailable'));
    section('Essentials', 'ess', 'The essentials', [gameRow(), settingRow('units'), rateRow(), visHot()], [info('Updates', updPill, 'More under App'), info('Licence', licPill, 'More under App')], { fold: false });

    // Racing data: what changes what the overlays show.
    section('Racing data', 'names', 'Units, names & ratings', [settingRow('units'), settingRow('shortClassNames'), settingRow('ratingsOfficialOnly')]);
    const pqDef = SCHEMA.find((x) => x.key === 'lmuPrivateQualifying')!;
    section('Racing data', 'pq', 'Private qualifying: hide other cars', [], [], {
      master: masterSwitch(() => opts.lmuPrivateQualifying, (v) => (opts.lmuPrivateQualifying = cfg.lmuPrivateQualifying = v), true, pqDef.help ?? ''),
      fold: false,
    });
    section('Racing data', 'game', 'Game', [gameRow()], cardBody(gameCard(), (x) => x.classList.contains('form')));
    section('Racing data', 'corners', 'Track map & corners', [], cardBody(cornersCard(rerender)));

    // Performance: the live readings, then everything that sets how much the app uses.
    monitorView.tiles.clear();
    monitorView.sparks.clear();
    const tiles = headlineTiles();
    section('Performance', 'rate', 'Overlay update rate', [rateRow()]);
    section('Performance', 'window', 'This window', [settingRow('trayOnPitExit'), settingRow('closeAction'), settingRow('uiAnimations')]);
    section('Performance', 'monitoring', 'Monitoring', [], cardBody(monitoringCard()));

    // Hotkeys.
    const layouts = cfg.layouts ?? [];
    section('Hotkeys', 'hk-overlays', 'Overlays', [editHot(), visHot()]);
    section(
      'Hotkeys',
      'hk-layouts',
      'Layouts',
      [
        hotRow('next', 'Next layout', () => hk.nextLayout ?? '', (v) => (hk.nextLayout = v), '', 'Cycles through your saved layouts'),
        hotRow('recentre', 'Recentre VR', () => hk.vrRecenter ?? '', (v) => (hk.vrRecenter = v), '', 'Resets SteamVR’s seated position, which the overlays fixed in the car are placed from'),
        ...layouts.map((l) =>
          hotRow(
            `layout-${l.id}`,
            `Switch to “${l.name}”`,
            () => hk.layouts[l.id] ?? '',
            (v) => {
              if (v) hk.layouts[l.id] = v;
              else delete hk.layouts[l.id];
            },
          ),
        ),
      ],
      layouts.length ? [] : [h('p', 'dim small', 'Save layouts under Overlays → Layouts, then give each one a hotkey here.')],
    );
    section(
      'Hotkeys',
      'hk-each',
      'Show / hide one overlay',
      cfg.overlays.filter(listed).map((o) =>
        hotRow(
          `overlay-${o.id}`,
          o.name || byKind(o.kind)?.name || o.kind,
          () => hk.overlays[o.id] ?? '',
          (v) => {
            if (v) hk.overlays[o.id] = v;
            else delete hk.overlays[o.id];
          },
          '',
          o.name ? (byKind(o.kind)?.name ?? '') : '',
        ),
      ),
    );

    // VR: everything under its switch.
    cfg.vr ??= { enabled: false, mode: 'auto', distance: 0.9, width: 1.2, height: -0.05, offset: 0 };
    cfg.vr.mode ??= 'auto';
    const VR_DEFS: SettingDef[] = [
      {
        key: 'mode',
        label: 'How',
        type: 'select',
        default: 'auto',
        options: [
          ['auto', 'Automatic (recommended)'],
          ['openxr', 'OpenXR only (no SteamVR needed)'],
          ['steamvr', 'SteamVR overlays only'],
        ],
        help: 'Automatic: drawn inside the game with OpenXR (Virtual Desktop, Quest Link, Pimax…), otherwise SteamVR overlays',
      },
      { key: 'distance', label: 'Screen layout: distance in front', type: 'number', default: 0.9, min: 0.2, max: 5, step: 0.05, unit: 'm', help: 'Overlays placed “as on my screen” sit on an imaginary screen this far away' },
      { key: 'width', label: 'Screen layout: width', type: 'number', default: 1.2, min: 0.2, max: 6, step: 0.05, unit: 'm' },
      { key: 'height', label: 'Screen layout: raise / lower', type: 'number', default: -0.05, min: -2, max: 2, step: 0.01, unit: 'm' },
      { key: 'offset', label: 'Screen layout: move left / right', type: 'number', default: 0, min: -2, max: 2, step: 0.01, unit: 'm', help: '+ moves every overlay placed “as on my screen” to the right' },
    ];
    const vrVals = cfg.vr as unknown as Values;
    const vrRows = VR_DEFS.map(
      (s): Item => ({
        el: dotted(
          field(s, vrVals, () => {
            save();
            refresh();
          }),
        ),
        changed: () => !same(vrVals[s.key], s.default),
        reset: () => {
          vrVals[s.key] = s.default;
          save();
          rerender();
        },
      }),
    );
    section('VR', 'vr', 'Show overlays in VR', vrRows, cardBody(vrCard(), (x) => x.classList.contains('form')), {
      master: masterSwitch(() => !!cfg.vr.enabled, (v) => (cfg.vr.enabled = v), false, 'Shows your overlays in the headset, as on your screen. Each overlay’s Window tab sets where it goes'),
      fold: false,
    });

    // App: the app itself.
    section('App', 'licence', 'Licence', [], cardBody(licenceCard()));
    section('App', 'updates', 'Updates', [], cardBody(updatesCard()));
    section('App', 'server', 'Server', [settingRow('port'), settingRow('lanAccess')], [h('p', 'dim small', 'Restart Apex Overlays after changing these.')]);
    const danger = h('div', 'danger-zone');
    danger.append(
      h('p', 'dim small', 'Start again: every setting, overlay and layout back to how a new install has them.'),
      button(
        'Reset everything',
        () => {
          if (!confirm('Reset all settings to defaults?')) return;
          fetch('/api/config', { method: 'PUT', headers: { 'content-type': 'application/json' }, body: '{}' }).then(() => location.reload());
        },
        'danger',
      ),
    );
    section('App', 'backup', 'Backup', [], [
      h('p', 'dim', 'Export everything (overlays, theme, leagues, hotkeys) to a file, or import a file shared by someone else.'),
      button('Export settings', backup),
      button('Import settings', async () => {
        const data = (await upload()) as typeof cfg;
        if (!data || !Array.isArray(data.overlays)) return alert('That file is not an Apex Overlays export.');
        state.config = { ...data, garage61: { ...data.garage61, token: cfg.garage61.token } };
        save(true);
        rerender();
      }),
      danger,
    ]);
    section('App', 'logs', 'Logs', [], cardBody(logsCard()));

    // Tabs, search, "Set only" (hotkeys) and "Changed only".
    const TABS_GEN = ['Essentials', 'Racing data', 'Performance', 'Hotkeys', 'VR', 'App'];
    if (!TABS_GEN.includes(gen.tab)) gen.tab = 'Essentials';
    const tabBar = h('div', 'tabs gen-tabs');
    tabBar.setAttribute('role', 'tablist');
    const search = h('input', 'search');
    search.type = 'search';
    search.placeholder = 'Search General settings and hotkeys…';
    search.value = gen.query;
    const setOnly = button('Set only', () => {
      gen.setOnly = !gen.setOnly;
      applyView();
    }, 'subtle chg-only');
    setOnly.title = 'Only the hotkeys you have set';
    const onlyChanged = button('Changed only', () => {
      gen.changedOnly = !gen.changedOnly;
      gen.query = '';
      search.value = '';
      applyView();
    }, 'subtle chg-only');
    onlyChanged.title = 'Everything you have changed on this page, from every tab';
    search.oninput = () => {
      gen.query = search.value;
      if (gen.query) gen.changedOnly = false;
      applyView();
    };
    const tools = h('div', 'tab-tools');
    tools.append(search, setOnly, onlyChanged);
    const note = h('p', 'dim small sec-note');
    const empty = h('p', 'dim');
    const changedCount = (tab: string) => secs.filter((s) => s.tab === tab).flatMap((s) => [...(s.master ? [s.master.item] : []), ...s.items]).filter((i) => i.changed()).length;
    const drawTabs = () => {
      const filtering = !!gen.query.trim() || gen.changedOnly;
      tabBar.replaceChildren(
        ...TABS_GEN.map((t) => {
          const b = button(
            t,
            () => {
              gen.tab = t;
              gen.query = '';
              gen.changedOnly = false;
              search.value = '';
              applyView();
            },
            `tab${t === gen.tab && !filtering ? ' sel' : ''}`,
          );
          b.setAttribute('role', 'tab');
          b.setAttribute('aria-selected', String(t === gen.tab && !filtering));
          const n = changedCount(t);
          if (n) b.append(h('span', 'tab-count', String(n)));
          return b;
        }),
      );
    };
    const page = h('div', 'gen-page');
    const applyView = () => {
      const q = gen.query.trim().toLowerCase();
      const filtering = !!q || gen.changedOnly;
      page.classList.toggle('gen-filtering', filtering);
      tiles.hidden = filtering || gen.tab !== 'Performance';
      clashBanner.hidden = filtering || !['Hotkeys', 'Essentials'].includes(gen.tab) || !clashBanner.textContent;
      const hotkeys = !filtering && gen.tab === 'Hotkeys';
      setOnly.hidden = !hotkeys;
      setOnly.classList.toggle('on', gen.setOnly);
      setOnly.setAttribute('aria-pressed', String(gen.setOnly));
      onlyChanged.classList.toggle('on', gen.changedOnly);
      onlyChanged.setAttribute('aria-pressed', String(gen.changedOnly));
      let any = false;
      for (const s of secs) {
        const rows = [...s.body.children] as HTMLElement[];
        for (const r of rows) r.classList.remove('gen-filtered');
        let show: boolean;
        if (q) {
          const titleHit = `${s.tab} ${s.title}`.toLowerCase().includes(q);
          const hits = rows.filter((r) => (r.textContent ?? '').toLowerCase().includes(q));
          show = s.tab !== 'Essentials' && (titleHit || hits.length > 0);
          if (!titleHit) for (const r of rows) r.classList.toggle('gen-filtered', !hits.includes(r));
        } else if (gen.changedOnly) {
          const ch = s.items.filter((i) => i.changed());
          show = s.tab !== 'Essentials' && (ch.length > 0 || !!s.master?.item.changed());
          for (const r of rows) r.classList.toggle('gen-filtered', !ch.some((i) => i.el === r));
        } else {
          show = s.tab === gen.tab;
          if (hotkeys && gen.setOnly) {
            for (const i of s.items) if (i.hot && !i.hot.get()) i.el.classList.add('gen-filtered');
            for (const r of rows) if (!r.classList.contains('field')) r.classList.add('gen-filtered');
            show = show && rows.some((r) => !r.classList.contains('gen-filtered'));
          }
        }
        s.el.hidden = !show;
        s.tabName.hidden = !filtering;
        if (show) any = true;
      }
      text(
        note,
        q
          ? 'Results from every tab, buttons included.'
          : gen.changedOnly
            ? 'Everything you have changed on this page, from every tab. ↺ puts one back.'
            : gen.tab === 'Essentials'
              ? 'What most people set once. Everything else is in the other tabs.'
              : hotkeys
                ? gen.setOnly
                  ? 'Only the hotkeys you have set. Turn off Set only to add more.'
                  : 'Click a box and press the keys (Ctrl, Alt or Shift plus a key, or an F-key). Backspace clears it.'
                : '',
      );
      note.hidden = !note.textContent;
      text(empty, q ? `No setting matches “${gen.query}”.` : gen.changedOnly ? 'Nothing changed: every setting here is at its default.' : hotkeys && gen.setOnly ? 'No hotkeys set yet. Turn off Set only to add some.' : '');
      empty.hidden = any || !empty.textContent;
      drawTabs();
    };
    refresh = () => {
      for (const s of secs) {
        for (const i of s.items) i.el.classList.toggle('changed', i.changed());
        const n = [...(s.master ? [s.master.item] : []), ...s.items].filter((i) => i.changed()).length;
        text(s.count, n ? `${n} changed` : '');
        s.count.hidden = !n;
        s.reset.hidden = !n;
        const off = !!s.master && !s.master.on();
        s.el.classList.toggle('off', off);
        s.el.classList.toggle('changed', !!s.master?.item.changed());
        const kids = s.body.children.length;
        s.el.classList.toggle('empty', kids === 0);
        const folded = s.el.classList.contains('folded');
        if (s.chevron) {
          s.chevron.hidden = off || kids === 0;
          s.chevron.title = folded ? `Show ${s.title}` : `Hide ${s.title} (it stays on)`;
          s.chevron.setAttribute('aria-expanded', String(!folded));
        }
        text(s.hiddenNote, folded && !off && kids ? (s.items.length > 1 ? `${s.items.length} settings hidden` : 'Hidden') : '');
      }
      clashCheck();
      applyView();
    };
    page.append(tabBar, tools, note, clashBanner, tiles, ...secs.map((s) => s.el), empty);
    el.replaceChildren(page);
    refresh();
    drawMonitor();
  },
};

const streamingPage: Page = {
  id: 'streaming',
  title: 'Streaming / OBS',
  render(el) {
    const rows = state.config!.overlays.filter(listed).map((o) => {
      const tr = h('tr');
      const url = overlayUrl(o.id);
      const b = button('Copy', () => copy(url, b));
      const code = h('code', '', url);
      tr.append(h('td', '', o.name || byKind(o.kind)?.name || o.kind), h('td', '', `${o.width}×${o.height}`), h('td'), h('td'));
      tr.children[2].append(code);
      tr.children[3].append(b);
      return tr;
    });
    const table = h('table', 'urls');
    table.append(...rows);
    el.replaceChildren(
      card(
        'Overlays as browser sources',
        h('p', 'dim', 'Every overlay is a web page. In OBS add a Browser Source with the URL and size below — the same settings and data as in-game, transparent background included.'),
        table,
        h('p', 'dim small', 'Streaming from a second PC: enable “Allow other PCs on my network” under General, restart, and replace 127.0.0.1 with this PC’s IP address.'),
      ),
      card('Coming next', h('p', 'dim', 'Stream chat overlay (Twitch / YouTube), VR overlay support and more are on the roadmap — see docs/ROADMAP.md.')),
    );
  },
};

// Garage 61 (garage61Page) is held back until the integration is ready.
export const PAGES: Page[] = [overlaysPage, themePage, strategyPage, generalPage, streamingPage];
void garage61Page;


/** The edit-mode button with its hotkey ("Move & resize", or "Done moving" while moving). */
function editButton(editing: boolean) {
  const key = (state.config?.hotkeys?.toggleEdit ?? '').replace('CmdOrCtrl', 'Ctrl').replace('Shift', '⇧').split('+').join(' ');
  const b = button(editing ? 'Done moving' : 'Move & resize', () => client.send({ op: 'setEditMode', value: !editing }), editing ? 'primary active' : 'primary');
  if (key) b.append(' ', h('kbd', '', key));
  b.title = 'Drag overlays to move them, their corner to resize';
  return b;
}

/** The monitor an overlay is on: the one holding its centre, else the nearest. */
function monitorOf(o: { x: number; y: number; width: number; height: number }, mons: ScreenMonitor[]): ScreenMonitor | undefined {
  const cx = o.x + o.width / 2, cy = o.y + o.height / 2;
  const inside = mons.find((m) => cx >= m.x && cx < m.x + m.width && cy >= m.y && cy < m.y + m.height);
  if (inside) return inside;
  const dist = (m: ScreenMonitor) => Math.hypot(Math.max(m.x - cx, 0, cx - (m.x + m.width)), Math.max(m.y - cy, 0, cy - (m.y + m.height)));
  return [...mons].sort((a, b) => dist(a) - dist(b))[0];
}

/** How a monitor reads in the picker (under "Monitor"): "2 · 2560 × 1440 · 125% · main". */
const monitorLabel = (m: ScreenMonitor) => `${m.index} · ${m.resolution[0]} × ${m.resolution[1]}${Math.abs(m.scale - 1) > 0.01 ? ` · ${Math.round(m.scale * 100)}%` : ''}${m.primary ? ' · main' : ''}`;

/**
 * Where each overlay sits on the screen, small; click one to open its settings. Off ones are
 * dashed, the selected one in the accent. In the desktop app, the monitor the overlays are on
 * (with its resolution) can be changed here: every overlay moves to it, keeping its place.
 */
function screenMap(overlays: OverlayInstance[], selected: string, pick: (id: string) => void, rerender: () => void) {
  const mons = state.monitors ?? [];
  // The monitor most of the overlays are on (the main one when there are none).
  const counts = new Map<number, number>();
  for (const o of overlays.filter((x) => x.enabled)) {
    const m = monitorOf(o, mons);
    if (m) counts.set(m.index, (counts.get(m.index) ?? 0) + 1);
  }
  const best = [...counts].sort((a, b) => b[1] - a[1])[0]?.[0];
  const current = mons.find((m) => m.index === best) ?? mons.find((m) => m.primary) ?? mons[0];
  const bounds = current ?? { x: 0, y: 0, width: Math.max(window.screen?.width || 1920, ...overlays.map((o) => o.x + o.width)), height: Math.max(window.screen?.height || 1080, ...overlays.map((o) => o.y + o.height)) };
  const wrap = h('div', 'screen-map');
  const head = h('div', 'screen-map-head');
  if (current) {
    const label = h('label', 'screen-map-pick');
    label.append(h('span', '', 'Monitor'));
    const sel = h('select') as HTMLSelectElement;
    sel.setAttribute('aria-label', 'Monitor the overlays are on');
    for (const m of mons) {
      const opt = h('option', '', monitorLabel(m)) as HTMLOptionElement;
      opt.value = String(m.index);
      opt.title = m.name;
      opt.selected = m.index === current.index;
      sel.append(opt);
    }
    sel.onchange = () => {
      const target = mons.find((m) => String(m.index) === sel.value);
      if (target) moveToMonitor(target, mons, current);
      rerender();
    };
    label.append(sel);
    head.append(label);
  } else head.append(h('span', '', 'Your screen'), h('span', '', `${bounds.width} × ${bounds.height}`));
  const box = h('div', 'screen-map-box');
  box.style.aspectRatio = `${bounds.width} / ${bounds.height}`;
  let elsewhere = 0;
  for (const o of overlays) {
    if (current && monitorOf(o, mons)?.index !== current.index) {
      elsewhere++;
      continue;
    }
    const name = o.name || byKind(o.kind)?.name || o.kind;
    const b = button('', () => pick(o.id), `${o.enabled ? '' : 'off '}${o.id === selected ? 'sel' : ''}`.trim());
    b.title = `${name}${o.enabled ? '' : ' (off)'}`;
    b.setAttribute('aria-label', b.title);
    b.dataset.id = o.id;
    b.onmouseenter = () => hot(o.id, true);
    b.onmouseleave = () => hot(o.id, false);
    const pct = (v: number, of: number) => `${Math.max(0, Math.min(100, (v / of) * 100)).toFixed(2)}%`;
    Object.assign(b.style, { left: pct(o.x - bounds.x, bounds.width), top: pct(o.y - bounds.y, bounds.height), width: pct(Math.max(o.width, 8), bounds.width), height: pct(Math.max(o.height, 8), bounds.height) });
    box.append(b);
  }
  wrap.append(head, box);
  if (elsewhere) wrap.append(h('p', 'dim small screen-map-note', `${elsewhere} more on ${elsewhere === 1 ? 'another monitor' : 'other monitors'}: pick a monitor to bring them all here`));
  return wrap;
}

/** Every overlay to `target`, each at the same place relative to its monitor's corner, kept
 *  inside the new monitor (a smaller one pulls overlays in from its edges). */
function moveToMonitor(target: ScreenMonitor, mons: ScreenMonitor[], fallback: ScreenMonitor) {
  const cfg = state.config!;
  for (const o of cfg.overlays) {
    const from = monitorOf(o, mons) ?? fallback;
    const rx = Math.max(0, Math.min(o.x - from.x, target.width - o.width));
    const ry = Math.max(0, Math.min(o.y - from.y, target.height - o.height));
    o.x = Math.round(target.x + rx);
    o.y = Math.round(target.y + ry);
  }
  save(true);
}

/** Lights an overlay in both the list and the screen map (hover), or puts it out. */
function hot(id: string, on: boolean) {
  for (const el of document.querySelectorAll(`.ov-item[data-id="${CSS.escape(id)}"], .screen-map-box button[data-id="${CSS.escape(id)}"]`)) el.classList.toggle('hot', on);
}

/** One digit as a column of 0-9, shown at `ch` (it rolls when moved to another digit). */
function digitColumn(ch: string): HTMLElement {
  const box = h('span', 'roll-d');
  const col = h('span', 'roll-col');
  col.append(...'0123456789'.split('').map((d) => h('span', '', d)));
  col.style.transform = `translateY(${-Number(ch) * 1.1}em)`;
  box.append(col);
  return box;
}

/** A reading that rolls to its new value digit by digit, like a timing board. When the shape
 *  changes (99 to 100, say) it is drawn again instead. */
function roll(el: HTMLElement, value: string) {
  const prev = el.dataset.v;
  if (prev === value) return;
  el.dataset.v = value;
  el.setAttribute('aria-label', value);
  const shape = (t: string) => t.replace(/\d/g, '0');
  if (prev === undefined || shape(prev) !== shape(value) || el.children.length !== value.length) {
    el.replaceChildren(...[...value].map((ch) => (/\d/.test(ch) ? digitColumn(ch) : h('span', 'roll-ch', ch))));
    for (const c of el.children) c.setAttribute('aria-hidden', 'true');
    return;
  }
  [...value].forEach((ch, i) => {
    if (/\d/.test(ch)) (el.children[i].firstElementChild as HTMLElement).style.transform = `translateY(${-Number(ch) * 1.1}em)`;
  });
}
