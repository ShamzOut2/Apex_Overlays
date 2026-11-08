import { h } from '../lib/dom';
import * as f from '../lib/format';
import { commonFor, merge, type SettingDef } from '../lib/settings';
import { applyUiTheme, PRESETS, THEME } from '../lib/theme';
import type { OverlayInstance, SavedLayout } from '../lib/types';
import { byKind, OVERLAYS } from '../overlays/registry';
import { CHOICES, field, form, relevance } from './form';
import { hotkeyInput } from './hotkey';
import { releaseNotes } from './releaseNotes';
import { STATS } from '../lib/monitorStats';
import { client, download, listen, save, state, uid, upload } from './state';

export type Page = { id: string; title: string; render(el: HTMLElement): void };

const card = (title: string, ...children: (Node | string)[]) => {
  const c = h('section', 'card');
  c.append(h('h2', '', title), ...children);
  return c;
};

const button = (label: string, onClick: () => void, cls = '') => {
  const b = h('button', cls, label);
  b.onclick = (e) => {
    e.preventDefault();
    onClick();
  };
  return b;
};

export const overlayUrl = (id: string) => `${location.origin}/o/${encodeURIComponent(id)}`;

let selected = '';

/** Shared files: one overlay, or a whole layout (overlays + theme). No personal data. */
type SharedOverlay = Pick<OverlayInstance, 'kind' | 'name' | 'enabled' | 'x' | 'y' | 'width' | 'height' | 'settings'>;
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

/** A menu to show open again after the page redraws. */
let openMenu = '';

const shareable = (o: OverlayInstance): SharedOverlay => ({ kind: o.kind, name: o.name, enabled: o.enabled, x: o.x, y: o.y, width: o.width, height: o.height, settings: structuredClone(o.settings) });
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
  const overlays = (d.overlays as SharedOverlay[]).filter((o) => o && typeof o.kind === 'string' && byKind(o.kind));
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

const instanceFrom = (o: SharedOverlay): OverlayInstance => {
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

/** Overlays name their groups differently; the editor shows them as tabs with common names. */
const TAB_ALIAS: Record<string, string> = { Elements: 'What to show', Fields: 'What to show', Readouts: 'What to show', Style: 'Look', Format: 'Look', Warnings: 'Alerts', Options: 'Settings' };
const SIZE_TAB = 'Size & visibility';
const ADVANCED_TAB = 'Advanced';
const VR_TAB = 'VR';
const tabOf = (s: SettingDef, common: boolean) =>
  common ? (s.group === 'Advanced' ? ADVANCED_TAB : s.group === 'VR' ? VR_TAB : SIZE_TAB) : (TAB_ALIAS[s.group ?? 'Options'] ?? s.group ?? 'Settings');

/** What to show and how it is laid out first; looks and behaviour after; window options last. */
const FIRST = ['Reference lap', 'What to show', 'Layout', 'Columns', 'Column style', 'Header'];
const LAST = ['Look', 'Colours', 'Alerts', 'Behaviour', SIZE_TAB, VR_TAB, ADVANCED_TAB];
const tabRank = (t: string) => (FIRST.includes(t) ? FIRST.indexOf(t) - 100 : LAST.includes(t) ? LAST.indexOf(t) + 100 : 0);

/** Editor state that survives the page being redrawn. */
const ui = { tab: new Map<string, string>(), query: '' };

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
  head.append(title, enabled, copyBtn, more);

  // Live preview of this overlay with its current settings.
  const preview = h('div', 'preview');
  const frame = h('iframe');
  frame.src = `${url}?preview`;
  const scale = Math.min(1, 700 / o.width, 220 / o.height);
  frame.style.width = `${o.width}px`;
  frame.style.height = `${o.height}px`;
  frame.style.transform = `scale(${scale})`;
  preview.style.height = `${o.height * scale + 16}px`;
  preview.append(frame);

  const common = commonFor(def.commonDefaults, def.fitsWidth);
  const values = merge([...common, ...def.settings], o.settings);
  const entries = [...def.settings.map((s) => ({ s, tab: tabOf(s, false) })), ...common.map((s) => ({ s, tab: tabOf(s, true) }))];
  const tabs = [...new Set(entries.map((e) => e.tab))].sort((a, b) => tabRank(a) - tabRank(b));
  let tab = ui.tab.get(o.id) ?? tabs[0];
  if (!tabs.includes(tab)) tab = tabs[0];

  const changed = (t: string) => entries.some((e) => e.tab === t && e.s.key in o.settings && JSON.stringify(o.settings[e.s.key]) !== JSON.stringify(e.s.default));
  // Settings that only apply with other choices are shown or hidden as those change.
  let relevant = () => {};
  const onChange = (k: string, v: unknown) => {
    values[k] = v;
    o.settings = { ...o.settings, [k]: v };
    save();
    drawTabs();
    relevant();
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
    for (const k of ['x', 'y', 'width', 'height'] as const) {
      const l = h('label', 'inline', { x: 'Left', y: 'Top', width: 'Width', height: 'Height' }[k]);
      const i = h('input');
      i.type = 'number';
      i.value = String(o[k]);
      i.oninput = () => {
        const v = parseInt(i.value, 10);
        if (Number.isFinite(v)) {
          o[k] = v;
          save();
        }
      };
      l.append(i);
      geo.append(l);
    }
    const box = h('div', 'size-extras');
    box.append(
      h('div', 'field-name', 'Position and size (pixels)'),
      geo,
      h('p', 'dim small', 'Easier: press Edit mode at the top and drag the overlay on screen.'),
      button('Use this overlay’s transparency for every overlay', () => {
        for (const other of state.config!.overlays) other.settings = { ...other.settings, opacity: values.opacity, bgOpacity: values.bgOpacity };
        save();
      }),
    );
    return box;
  };

  const drawTabs = () => {
    tabBar.replaceChildren(
      ...tabs.map((t) => {
        const b = button(
          t,
          () => {
            tab = t;
            ui.tab.set(o.id, t);
            ui.query = '';
            search.value = '';
            drawTabs();
            drawPanel();
          },
          `tab${t === tab && !ui.query ? ' sel' : ''}`,
        );
        if (changed(t)) {
          b.append(h('span', 'tab-dot'));
          b.title = 'Some settings here differ from the default';
        }
        return b;
      }),
    );
  };

  const drawPanel = () => {
    const q = ui.query.trim().toLowerCase();
    if (q) {
      const hits = entries.filter(({ s, tab: t }) => `${s.label} ${s.help ?? ''} ${t}`.toLowerCase().includes(q));
      const parts: HTMLElement[] = [];
      for (const t of tabs) {
        const inTab = hits.filter((e) => e.tab === t);
        if (!inTab.length) continue;
        // Searching finds every setting, including ones that only apply with other choices.
        parts.push(h('h3', '', t), ...inTab.map((e) => field(e.s, values, onChange)));
      }
      relevant = () => {};
      panel.replaceChildren(...(parts.length ? parts : [h('p', 'dim', `No setting matches “${ui.query}”.`)]));
      return;
    }
    const rows: [SettingDef, HTMLElement][] = entries.filter((e) => e.tab === tab).map((e) => [e.s, field(e.s, values, onChange)]);
    const none = h('p', 'dim', 'Nothing here applies with your current choices (for example the columns shown, or a feature switched off).');
    const apply = relevance(rows, values);
    relevant = () => {
      apply();
      none.hidden = rows.some((r) => !r[1].hidden);
    };
    relevant();
    panel.replaceChildren(...rows.map((r) => r[1]), none, ...(tab === SIZE_TAB ? [sizeExtras()] : []));
  };

  search.oninput = () => {
    ui.query = search.value;
    drawTabs();
    drawPanel();
  };
  drawTabs();
  drawPanel();
  wrap.append(head, h('p', 'dim desc', def.description), preview, search, tabBar, panel);
  return wrap;
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
  for (const d of OVERLAYS) {
    const card = h('button', 'gallery-item');
    const n = count(d.kind);
    card.append(h('span', 'g-name', d.name), h('span', 'g-desc', d.description));
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
    if (!cfg.overlays.some((o) => o.id === selected)) selected = cfg.overlays[0]?.id ?? '';
    const editing = state.status?.editMode ?? false;
    const visible = state.status?.visible ?? true;

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
        ? [layoutList, h('p', 'dim small', 'P Q W R: switch to that layout automatically when a Practice, Qualifying, Warmup or Race session starts. ⇩ exports a layout to share; hotkeys are under General → Hotkeys.')]
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

    const bar = h('div', 'toolbar');
    bar.append(
      button(editing ? 'Done moving' : 'Edit mode (move & resize)', () => client.send({ op: 'setEditMode', value: !editing }), editing ? 'primary active' : 'primary'),
      button(visible ? 'Hide all overlays' : 'Show all overlays', () => client.send({ op: 'setVisible', value: !visible })),
      h('span', 'spacer'),
      layoutsMenu,
      moveMenu,
    );

    const addOverlay = (kind: string) => {
      const d = byKind(kind)!;
      const o: OverlayInstance = { id: uid(d.kind), kind: d.kind, name: '', enabled: true, x: 200, y: 200, width: d.size[0], height: d.size[1], settings: {} };
      cfg.overlays.push(o);
      selected = o.id;
      ui.tab.delete(o.id);
      save();
      rerender();
    };
    const list = h('div', 'ov-list');
    list.append(button('＋ Add overlay', () => addOverlayDialog(addOverlay), 'add-btn'));
    for (const o of cfg.overlays) {
      const d = byKind(o.kind);
      const item = h('div', `ov-item${o.id === selected ? ' sel' : ''}${o.enabled ? '' : ' off'}`);
      item.tabIndex = 0;
      const names = h('span', 'ov-names');
      names.append(h('span', 'ov-name', o.name || d?.name || o.kind));
      if (o.name && d) names.append(h('span', 'ov-kind', d.name));
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
    const sel = cfg.overlays.find((o) => o.id === selected);
    const main = h('div', 'ov-main');
    main.append(sel ? overlayEditor(sel, rerender) : h('p', 'dim', 'No overlays yet — press “Add overlay”.'));
    const wrap = h('div', 'ov-layout');
    wrap.append(list, main);
    el.replaceChildren(bar, wrap);
  },
};

function swatch(name: string, theme: Record<string, unknown>, onApply: () => void, onDelete?: () => void) {
  const t = merge(THEME, theme);
  const el = h('button', 'swatch');
  el.style.background = t.background as string;
  el.style.color = t.textColor as string;
  const bars = h('div', 'swatch-bars');
  for (const k of ['accent', 'playerColor', 'goodColor', 'badColor', 'lapBehindColor']) {
    const b = h('span');
    b.style.background = t[k] as string;
    bars.append(b);
  }
  el.append(h('span', 'swatch-name', name), bars);
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
    const values = merge(THEME, cfg.theme);
    cfg.savedThemes ??= [];
    const apply = (theme: Record<string, unknown>, coloursOnly: boolean) => {
      // Presets only carry colours, so fonts and sizes the user picked survive.
      const base = coloursOnly ? Object.fromEntries(Object.entries(cfg.theme).filter(([k]) => !COLOUR_KEYS.includes(k) && k !== 'backgroundOpacity' && k !== 'shadow')) : {};
      cfg.theme = { ...base, ...theme };
      save();
      rerender();
    };
    const presets = h('div', 'swatches');
    for (const p of PRESETS) presets.append(swatch(p.name, p.values, () => apply(p.values, true)));
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
          'Shows a live delta to your Garage 61 personal best (or your team’s best lap), sector by sector, plus your personal best and your teams’ leaderboard for the car and track you are driving. Create a personal access token on garage61.net (Developer → My applications) with the “driving data” permission.',
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
  if (u?.status === 'available' && u.notes) c.append(h('h3', '', `What’s new in ${u.version}`), releaseNotes(u.notes));
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
      tr.append(h('td', '', String(corner.number)), h('td', '', corner.name || '–'), h('td', '', (corner.start * 100).toFixed(1)), h('td', '', (corner.end * 100).toFixed(1)), h('td'));
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
        () => {
          const p = state.player?.lapPct ?? 0;
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

/** Every hotkey: editing, layouts, and one per overlay. All but the first two start blank. */
function hotkeysCard(): HTMLElement {
  const cfg = state.config!;
  const hk = cfg.hotkeys;
  hk.layouts ??= {};
  hk.overlays ??= {};
  const c = card('Hotkeys');
  c.append(h('p', 'dim', 'Click a box and press the keys you want (Ctrl, Alt or Shift plus a key, or an F-key / number-pad key on its own). Backspace clears a hotkey. Desktop app only; changes apply straight away.'));
  const all = (): string[] => [hk.toggleEdit, hk.toggleVisibility, hk.nextLayout, ...Object.values(hk.layouts), ...Object.values(hk.overlays)].filter((k) => k?.trim());
  const rows: { warn: HTMLElement; get: () => string }[] = [];
  const checkClashes = () => {
    const used = all().map((k) => k.toLowerCase());
    for (const r of rows) {
      const k = r.get().toLowerCase();
      const clash = !!k && used.filter((u) => u === k).length > 1;
      r.warn.hidden = !clash;
    }
  };
  const row = (label: string, get: () => string, set: (v: string) => void, sub = '') => {
    const el = h('div', 'field hotkey-row');
    const lab = h('span', 'field-label');
    lab.append(h('span', 'field-name', label));
    if (sub) lab.append(h('small', 'help', sub));
    const warn = h('small', 'help bad', 'Also used by another hotkey');
    warn.hidden = true;
    lab.append(warn);
    rows.push({ warn, get });
    el.append(
      lab,
      hotkeyInput(get() ?? '', (v) => {
        set(v);
        save(true);
        checkClashes();
      }),
    );
    return el;
  };
  const group = (title: string, ...children: (Node | string)[]) => {
    const fs = h('fieldset');
    fs.append(h('legend', '', title), ...children);
    return fs;
  };
  const layouts = cfg.layouts ?? [];
  const grid = h('div', 'form hotkeys');
  grid.append(
    group(
      'Overlays',
      row('Edit mode (move & resize)', () => hk.toggleEdit, (v) => (hk.toggleEdit = v)),
      row('Show / hide all overlays', () => hk.toggleVisibility, (v) => (hk.toggleVisibility = v)),
    ),
    group(
      'Layouts',
      row('Next layout', () => hk.nextLayout ?? '', (v) => (hk.nextLayout = v), 'Cycles through your saved layouts'),
      row('Recentre VR', () => hk.vrRecenter ?? '', (v) => (hk.vrRecenter = v), 'Resets SteamVR’s seated position, which the overlays fixed in the car are placed from'),
      ...layouts.map((l) =>
        row(
          `Switch to “${l.name}”`,
          () => hk.layouts[l.id] ?? '',
          (v) => {
            if (v) hk.layouts[l.id] = v;
            else delete hk.layouts[l.id];
          },
        ),
      ),
      ...(layouts.length ? [] : [h('p', 'dim small', 'Save layouts under Overlays → Layouts, then give each one a hotkey here.')]),
    ),
    group(
      'Show / hide one overlay',
      ...cfg.overlays.map((o) =>
        row(
          o.name || byKind(o.kind)?.name || o.kind,
          () => hk.overlays[o.id] ?? '',
          (v) => {
            if (v) hk.overlays[o.id] = v;
            else delete hk.overlays[o.id];
          },
          o.name ? byKind(o.kind)?.name : '',
        ),
      ),
    ),
  );
  c.append(grid);
  checkClashes();
  return c;
}

/** Live readings of what Apex Overlays uses, updated in place once a second. */
const monitorView = { tiles: new Map<string, HTMLElement>(), spark: null as SVGPolylineElement | null, cpu: [] as number[] };

function drawMonitor() {
  const m = state.monitor;
  if (!m) return;
  monitorView.cpu.push(m.cpu);
  if (monitorView.cpu.length > 60) monitorView.cpu.shift();
  for (const s of STATS) {
    const el = monitorView.tiles.get(s.id);
    if (!el) continue;
    el.textContent = s.value(m);
    const lvl = s.level?.(m, 5) ?? '';
    el.classList.toggle('warn', lvl === 'warn');
    el.classList.toggle('bad', lvl === 'bad');
  }
  // CPU over the last minute, scaled to its own peak (at least 5%).
  const pts = monitorView.cpu;
  const top = Math.max(5, ...pts);
  monitorView.spark?.setAttribute('points', pts.map((v, i) => `${(i / 59) * 100},${30 - (v / top) * 28}`).join(' '));
}

listen((t) => {
  if (t === 'monitor' && monitorView.tiles.size && document.contains([...monitorView.tiles.values()][0])) drawMonitor();
});

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
      'Shows your overlays in the headset, looking and updating exactly as on your screen. Each overlay’s VR tab says whether it shows in VR, whether it is fixed in the car or follows your head, and where it goes.',
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
          help: 'Automatic draws inside iRacing when it runs in OpenXR mode (Virtual Desktop, Quest Link, Pimax, Varjo, SteamVR…), otherwise uses SteamVR overlays',
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
  c.append(
    h(
      'p',
      'dim small',
      'Set a “Recentre VR” hotkey under Hotkeys: with SteamVR it resets the seated position; with OpenXR it places the overlays fixed in the car from where your head is when you press it.',
    ),
    h(
      'p',
      'dim small',
      'Virtual Desktop: in the Virtual Desktop Streamer, set the OpenXR runtime to VDXR, then pick OpenXR as iRacing’s VR mode. No SteamVR needed; the status above says when the overlays are showing inside iRacing.',
    ),
    h(
      'p',
      'dim small',
      'OpenXR: the first time, Windows asks for permission to register Apex Overlays’ OpenXR layer, so every game can load it. Restart iRacing after turning VR on. It works alongside other OpenXR layers such as NextLevelXR or OpenXR Toolkit (Direct3D 11 and 12).',
    ),
    h(
      'p',
      'dim small',
      'Using another OpenXR layer (NextLevelXR, OpenXR Toolkit, OpenKneeboard)? They work together. If the overlays do not show or get cropped, change the order of the layers with the free “OpenXR API Layers GUI” tool: move “XR_APILAYER_APEX_overlays” to the other end of the list.',
    ),
  );
  return c;
}

function monitoringCard(): HTMLElement {
  const cfg = state.config!;
  const c = card('Monitoring');
  c.append(h('p', 'dim', 'What Apex Overlays uses right now: the app and all its overlay windows. Hover a reading for what it means.'));
  const grid = h('div', 'mon-tiles');
  monitorView.tiles.clear();
  for (const s of STATS) {
    const tile = h('div', 'mon-tile');
    tile.title = s.help;
    const v = h('div', 'mon-tile-val', '–');
    tile.append(h('div', 'mon-tile-label', s.label), v);
    if (s.id === 'cpu') {
      const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
      svg.setAttribute('viewBox', '0 0 100 30');
      svg.setAttribute('preserveAspectRatio', 'none');
      svg.classList.add('mon-spark');
      const line = document.createElementNS('http://www.w3.org/2000/svg', 'polyline');
      svg.append(line);
      monitorView.spark = line;
      tile.append(svg);
      tile.classList.add('wide');
    }
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

const generalPage: Page = {
  id: 'general',
  title: 'General',
  render(el) {
    const cfg = state.config!;
    const opts = {
      units: cfg.units,
      port: cfg.server.port,
      lanAccess: cfg.server.lanAccess,
      ratingsOfficialOnly: cfg.ratingsOfficialOnly,
      shortClassNames: cfg.shortClassNames,
      updateRate: cfg.updateRate ?? 'balanced',
      closeAction: cfg.closeAction ?? 'ask',
    };
    el.replaceChildren(
      card(
        'General',
        form(
          [
            { key: 'units', label: 'Units', type: 'select', default: 'auto', options: [['auto', 'Same as iRacing'], ['metric', 'Metric'], ['imperial', 'Imperial']], group: 'Display' },
            {
              key: 'ratingsOfficialOnly',
              label: 'Only estimate iRating / SR in official races',
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
              key: 'updateRate',
              label: 'Overlay update rate',
              type: 'select',
              default: 'balanced',
              options: [
                ['fast', 'Fast (lightest on CPU, 20–30/s)'],
                ['balanced', 'Balanced (recommended)'],
                ['full', 'Every tick (60/s, most CPU)'],
              ],
              help: 'Fast: inputs, track map, spotter and pit 30/s; relative and safety alert 20/s; standings and battle 10/s. Balanced: as Fast but inputs, track map and spotter 60/s. Telemetry is always read 60 times a second, so nothing is missed.',
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
            { key: 'port', label: 'Port', type: 'number', default: 8182, min: 1024, max: 65535, group: 'Server (restart to apply)' },
            { key: 'lanAccess', label: 'Allow other PCs on my network (streaming PC)', type: 'bool', default: false, group: 'Server (restart to apply)' },
          ],
          opts,
          (k, v) => {
            if (k === 'units') cfg.units = v as typeof cfg.units;
            if (k === 'port') cfg.server.port = v as number;
            if (k === 'lanAccess') cfg.server.lanAccess = v as boolean;
            if (k === 'ratingsOfficialOnly') cfg.ratingsOfficialOnly = v as boolean;
            if (k === 'shortClassNames') cfg.shortClassNames = v as boolean;
            if (k === 'updateRate') cfg.updateRate = v as 'fast' | 'balanced' | 'full';
            if (k === 'closeAction') cfg.closeAction = v as 'ask' | 'quit' | 'keep';
            save();
          },
        ),
      ),
      hotkeysCard(),
      vrCard(),
      monitoringCard(),
      updatesCard(),
      cornersCard(() => this.render(el)),
      card(
        'Backup',
        h('p', 'dim', 'Export everything (overlays, theme, leagues) to a file, or import a file shared by someone else. Your Garage 61 token is never exported.'),
        button('Export settings', () => download('apex-overlays.json', { ...cfg, garage61: { ...cfg.garage61, token: '' } })),
        button('Import settings', async () => {
          const data = (await upload()) as typeof cfg;
          if (!data || !Array.isArray(data.overlays)) return alert('That file is not an Apex Overlays export.');
          state.config = { ...data, garage61: { ...data.garage61, token: cfg.garage61.token } };
          save(true);
          this.render(el);
        }),
        button(
          'Reset everything',
          () => {
            if (!confirm('Reset all settings to defaults?')) return;
            fetch('/api/config', { method: 'PUT', headers: { 'content-type': 'application/json' }, body: '{}' }).then(() => location.reload());
          },
          'danger',
        ),
      ),
    );
  },
};

const streamingPage: Page = {
  id: 'streaming',
  title: 'Streaming / OBS',
  render(el) {
    const rows = state.config!.overlays.map((o) => {
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

export const PAGES: Page[] = [overlaysPage, themePage, strategyPage, garage61Page, generalPage, streamingPage];
