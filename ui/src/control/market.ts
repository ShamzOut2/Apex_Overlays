// Layout Market: overlay layouts people share. They are kept in the app's GitHub repository
// (see crates/core/src/market.rs and .github/market/): browsing and downloading work for
// everyone; sharing makes a share code here and opens the repository's "Share a layout" form
// (a free GitHub account), where it is checked, listed and given a preview image automatically.

import { h, text } from '../lib/dom';
import type { SavedLayout } from '../lib/types';
import { byKind, hiddenKind } from '../overlays/registry';
import { applyLayout, button, card, gameName, instanceFrom, type Page, shareable, type SharedOverlay } from './pages';
import { client, save, state, uid } from './state';

const REPO = 'https://github.com/ShamzOut2/Apex_Overlays';

type Size = { w: number; h: number };

/** A layout as listed (index.json on the repository's market branch). */
interface Listing {
  id: string;
  name: string;
  description: string;
  /** The name the sharer gave; `author` is the GitHub account that posted it. */
  creator?: string;
  author: string;
  game: string;
  version: string;
  /** Session types it is set for (empty: every session). */
  sessions?: string[];
  kinds: string[];
  count: number;
  issue: string;
  published: string;
  updated: string;
  screen: Size | null;
  /** The screen resolution it was made for (pixels). */
  resolution?: Size | null;
  /** Preview image (previews/<id>-<n>.jpg), once rendered. */
  preview?: string | null;
  /** Where each overlay sits (thousandths of the screen) and its kind, for an outline. */
  boxes?: [number, number, number, number, string][];
}

/** What a share code holds (and a market layout file, plus its id, name and author). */
interface MarketPackage {
  apexOverlays: 'market';
  version: string;
  game: string;
  name: string;
  description: string;
  creator: string;
  sessions: string[];
  /** The screen in the units overlays are placed in, and its resolution in pixels. */
  screen: Size | null;
  resolution: Size | null;
  overlays: SharedOverlay[];
}

const SESSIONS: [string, string][] = [
  ['practice', 'Practice'],
  ['qualifying', 'Qualifying'],
  ['warmup', 'Warmup'],
  ['race', 'Race'],
];
const sessionText = (s: string[] | undefined) => (s?.length ? s.map((x) => SESSIONS.find((y) => y[0] === x)?.[1] ?? x).join(', ') : 'All sessions');

let listings: Listing[] | null = null;
let loadError = '';
let loading = false;
let gameFilter: string | null = null;
let resFilter = '';
let search = '';

/** This screen's resolution in pixels, and in the units overlays are placed in. */
const myScreen = (): Size => ({ w: window.screen.width, h: window.screen.height });
const myResolution = (): Size => ({ w: Math.round(window.screen.width * devicePixelRatio), h: Math.round(window.screen.height * devicePixelRatio) });
const resText = (r: Size | null | undefined) => (r ? `${r.w}×${r.h}` : '');

const ver = (v: string) => (v.split('.').map((n) => Number(n) || 0).concat(0, 0, 0).slice(0, 3) as [number, number, number]);
/** A layout made with a newer version than this one (by major.minor): its overlays or settings may be unknown here. */
function needsNewer(layoutVersion: string): boolean {
  const mine = state.status?.version;
  // Not known yet (the page is still connecting): do not hold anything back.
  if (!mine) return false;
  const [a, b] = ver(layoutVersion);
  const [x, y] = ver(mine);
  return a > x || (a === x && b > y);
}

/** Open one of the app's repository pages in the browser (the desktop app does it). */
function openPage(url: string) {
  client.send({ op: 'openUrl', url });
}

async function load(refresh = false) {
  loading = true;
  try {
    const r = await fetch(`/api/market${refresh ? '?refresh=true' : ''}`);
    if (!r.ok) throw new Error(await r.text());
    const d = (await r.json()) as { layouts?: Listing[] };
    listings = Array.isArray(d.layouts) ? d.layouts : [];
    loadError = '';
  } catch (e) {
    loadError = `The market could not be reached (${e instanceof Error ? e.message : e}). Check your internet connection and try again.`;
  } finally {
    loading = false;
  }
}

// ---- Sharing ----------------------------------------------------------------------------

async function gzipBase64(textIn: string): Promise<string> {
  const stream = new Blob([textIn]).stream().pipeThrough(new CompressionStream('gzip'));
  const bytes = new Uint8Array(await new Response(stream).arrayBuffer());
  let bin = '';
  for (let i = 0; i < bytes.length; i += 0x8000) bin += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(bin);
}

/** The share code for a set of overlays: "AOL1:" + gzip + base64 of the package. Custom CSS is left out. */
export async function shareCode(pkg: MarketPackage): Promise<string> {
  const overlays = pkg.overlays.map((o) => {
    const settings = { ...o.settings };
    delete settings.customCss;
    return { ...o, settings };
  });
  return `AOL1:${await gzipBase64(JSON.stringify({ ...pkg, overlays }))}`;
}

/** The name the user shares under, remembered for next time (this PC only). */
const NAME_KEY = 'apex.market.creator';
const rememberedName = () => {
  try {
    return localStorage.getItem(NAME_KEY) ?? '';
  } catch {
    return '';
  }
};

function shareDialog() {
  const cfg = state.config!;
  const back = h('div', 'modal-back');
  const dlg = h('div', 'modal market-share');
  const close = () => back.remove();
  back.onclick = (e) => {
    if (e.target === back) close();
  };
  const head = h('div', 'modal-head');
  head.append(h('h2', '', 'Share a layout'), button('✕', close, 'subtle'));

  const field = (label: string, input: HTMLElement, help = '') => {
    const l = h('label', 'mk-field');
    l.append(h('span', 'mk-label', label), input);
    if (help) l.append(h('small', 'dim', help));
    return l;
  };
  const name = h('input') as HTMLInputElement;
  name.maxLength = 60;
  name.placeholder = 'e.g. Clean race HUD';
  const creator = h('input') as HTMLInputElement;
  creator.maxLength = 40;
  creator.placeholder = 'e.g. Sam Racing';
  creator.value = rememberedName();
  const desc = h('textarea') as HTMLTextAreaElement;
  desc.maxLength = 400;
  desc.rows = 3;
  desc.placeholder = 'What it is for: the kind of racing, what is where…';
  // The game it is for: required (iRacing or Le Mans Ultimate).
  const game = h('select') as HTMLSelectElement;
  for (const [v, t] of [
    ['', 'Choose…'],
    ['iracing', 'iRacing'],
    ['lmu', 'Le Mans Ultimate'],
  ]) {
    const o = h('option', '', t) as HTMLOptionElement;
    o.value = v;
    game.append(o);
  }
  game.value = cfg.layoutGame === 'iracing' || cfg.layoutGame === 'lmu' ? cfg.layoutGame : '';
  const res = myResolution();
  const resLine = h('div', 'mk-static', `${resText(res)} (this screen)`);
  const which = h('select') as HTMLSelectElement;
  const optCurrent = h('option', '', 'The overlays I have now') as HTMLOptionElement;
  optCurrent.value = '';
  which.append(optCurrent);
  for (const l of cfg.layouts) {
    const o = h('option', '', `Saved layout: ${l.name}${l.sessions?.length ? ` (${sessionText(l.sessions)})` : ''}`) as HTMLOptionElement;
    o.value = l.id;
    which.append(o);
  }
  const sessionsLine = h('small', 'dim');
  const showSessions = () => {
    const l = cfg.layouts.find((x) => x.id === which.value);
    text(sessionsLine, `Sessions: ${sessionText(l?.sessions)}${l?.sessions?.length ? ' (as set for this layout under Overlays → Layouts)' : ''}`);
  };
  which.onchange = showSessions;
  showSessions();
  const onlyOn = h('input') as HTMLInputElement;
  onlyOn.type = 'checkbox';
  onlyOn.checked = true;
  const onlyLabel = h('label', 'mk-check');
  onlyLabel.append(onlyOn, ' Only the overlays that are switched on');

  const out = h('div', 'mk-out');
  const make = button(
    'Make the share code',
    async () => {
      const n = name.value.trim();
      const who = creator.value.trim();
      if (!n) return name.focus();
      if (!who) return creator.focus();
      if (!game.value) {
        alert('Choose the game this layout is for.');
        return game.focus();
      }
      try {
        localStorage.setItem(NAME_KEY, who);
      } catch {
        /* not remembered */
      }
      const saved = cfg.layouts.find((l) => l.id === which.value);
      const src = saved ? saved.overlays : cfg.overlays;
      const overlays = src.filter((o) => !hiddenKind(o.kind) && byKind(o.kind) && (!onlyOn.checked || o.enabled)).map(shareable);
      if (!overlays.length) {
        alert('There are no overlays to share: switch some on, or untick “Only the overlays that are switched on”.');
        return;
      }
      const pkg: MarketPackage = {
        apexOverlays: 'market',
        version: state.status?.version ?? '',
        game: game.value,
        name: n,
        description: desc.value.trim(),
        creator: who,
        sessions: saved?.sessions ?? [],
        screen: myScreen(),
        resolution: res,
        overlays,
      };
      const code = await shareCode(pkg);
      // The form opens with everything filled in when the link is short enough; otherwise
      // the code is copied, to paste into the form.
      const params = new URLSearchParams({ template: 'share-layout.yml', title: `Layout: ${n}`, name: n, description: desc.value.trim() });
      let url = `${REPO}/issues/new?${params}`;
      const withCode = `${url}&code=${encodeURIComponent(code)}`;
      const inLink = withCode.length <= 7500;
      if (inLink) url = withCode;
      else await navigator.clipboard?.writeText(code).catch(() => {});
      const box = h('textarea', 'mk-code') as HTMLTextAreaElement;
      box.readOnly = true;
      box.rows = 4;
      box.value = code;
      const copyBtn = button('Copy the code', () => {
        navigator.clipboard?.writeText(code);
        text(copyBtn, 'Copied');
      });
      const linkBox = h('input', 'mk-link') as HTMLInputElement;
      linkBox.readOnly = true;
      linkBox.value = url;
      const actions = h('div', 'toolbar mk-actions');
      actions.append(button('Open the share form', () => openPage(url), 'primary'), copyBtn);
      out.replaceChildren(
        h('p', '', `Share code ready: ${overlays.length} overlay${overlays.length === 1 ? '' : 's'} for ${gameName(game.value)} at ${resText(res)}, ${sessionText(pkg.sessions).toLowerCase()}.`),
        h(
          'p',
          'dim small',
          inLink
            ? 'The share form opens on GitHub with your layout filled in: press “Submit new issue”. It is checked and listed in a minute or two, and its preview image follows a few minutes later. You need a free GitHub account.'
            : 'The code has been copied. The share form opens on GitHub: paste it under “Share code” and press “Submit new issue”. It is checked and listed in a minute or two, and its preview image follows a few minutes later. You need a free GitHub account.',
        ),
        box,
        actions,
        h('small', 'dim', 'If the form did not open, copy this link into your browser:'),
        linkBox,
      );
      openPage(url);
    },
    'primary',
  );

  const makeBar = h('div', 'toolbar');
  makeBar.append(make);
  const row = (...els: HTMLElement[]) => {
    const r = h('div', 'mk-row');
    r.append(...els);
    return r;
  };
  dlg.append(
    head,
    h('p', 'dim', 'Share your overlays, their positions, sizes and settings with everyone using Apex Overlays. Never your theme, logins or tokens; custom CSS is left out.'),
    row(field('Layout name', name), field('Your name', creator, 'Shown as who made it')),
    field('What it is for', desc),
    row(field('Game', game), field('Screen resolution', resLine, 'Others’ screens are fitted to it when they add it')),
    field('Overlays to share', which),
    sessionsLine,
    onlyLabel,
    makeBar,
    out,
  );
  back.append(dlg);
  document.body.append(back);
  name.focus();
}

// ---- Previews ---------------------------------------------------------------------------

/** The preview image, or (until it is rendered) an outline of where each overlay sits. */
function thumb(l: Listing, big = false): HTMLElement {
  const r = l.resolution ?? l.screen ?? { w: 16, h: 9 };
  const box = h('div', `mk-thumb${big ? ' big' : ''}`);
  box.style.aspectRatio = `${r.w} / ${r.h}`;
  if (l.preview && /^previews\/[A-Za-z0-9-]+\.jpg$/.test(l.preview)) {
    const img = h('img') as HTMLImageElement;
    img.loading = 'lazy';
    img.alt = `Preview of ${l.name}`;
    img.src = `/api/market/preview/${l.preview.slice('previews/'.length)}`;
    // Not reachable: the outline instead.
    img.onerror = () => box.replaceChildren(outline(l));
    box.append(img);
  } else {
    box.append(outline(l));
  }
  return box;
}

function outline(l: Listing): HTMLElement {
  const o = h('div', 'mk-outline');
  for (const [x, y, w, hh, kind] of l.boxes ?? []) {
    const b = h('span', 'mk-box', byKind(kind)?.name ?? kind);
    b.style.cssText = `left:${x / 10}%;top:${y / 10}%;width:${w / 10}%;height:${hh / 10}%`;
    o.append(b);
  }
  o.append(h('span', 'mk-outline-note', l.preview ? 'Outline' : 'Preview image on its way'));
  return o;
}

function previewDialog(l: Listing) {
  const back = h('div', 'modal-back');
  const dlg = h('div', 'modal market-preview');
  back.onclick = (e) => {
    if (e.target === back) back.remove();
  };
  const head = h('div', 'modal-head');
  head.append(h('h2', '', l.name), button('✕', () => back.remove(), 'subtle'));
  dlg.append(head, thumb(l, true), h('p', 'dim small', `${facts(l)} · shown with sample data and the standard theme (yours is used when you add it)`));
  back.append(dlg);
  document.body.append(back);
}

const facts = (l: Listing) =>
  [`by ${l.creator || l.author}`, gameName(l.game), resText(l.resolution), sessionText(l.sessions), `${l.count} overlay${l.count === 1 ? '' : 's'}`].filter(Boolean).join(' · ');

// ---- Downloading ------------------------------------------------------------------------

/** Positions moved from the sharer's screen to this one (sizes kept), and kept on screen. */
function fitToScreen(overlays: SharedOverlay[], from: Size | null): SharedOverlay[] {
  const { w: W, h: H } = myScreen();
  const sx = from ? W / from.w : 1;
  const sy = from ? H / from.h : 1;
  return overlays.map((o) => {
    const x = Math.round(o.x * sx);
    const y = Math.round(o.y * sy);
    return { ...o, x: Math.max(0, Math.min(W - Math.min(o.width, W), x)), y: Math.max(0, Math.min(H - Math.min(o.height, H), y)) };
  });
}

async function addLayout(l: Listing, btn: HTMLButtonElement) {
  const cfg = state.config!;
  // Made for the other game, or another screen: say so first.
  const mine = cfg.layoutGame;
  if (l.game && (mine === 'iracing' || mine === 'lmu') && l.game !== mine) {
    if (!confirm(`This layout is made for ${gameName(l.game)}, and you are setting up ${gameName(mine)}’s overlays. Overlays only for ${gameName(l.game)} will not show in ${gameName(mine)}.\n\nAdd it anyway?`)) return;
  }
  text(btn, 'Downloading…');
  btn.disabled = true;
  try {
    const r = await fetch(`/api/market/${encodeURIComponent(l.id)}`);
    if (!r.ok) throw new Error(r.status === 404 ? 'it is no longer on the market' : await r.text());
    const pkg = (await r.json()) as MarketPackage;
    const known = (pkg.overlays ?? []).filter((o) => o && typeof o.kind === 'string' && byKind(o.kind) && !hiddenKind(o.kind));
    if (!known.length) throw new Error('none of its overlays exist in this version');
    const name = (prompt('Name for this layout', l.name) ?? '').trim();
    if (!name) return;
    const saved: SavedLayout = { id: uid('layout'), name, overlays: fitToScreen(known, pkg.screen).map(instanceFrom), sessions: [] };
    // Its session types, where none of your layouts already has them.
    const taken: string[] = [];
    for (const s of pkg.sessions ?? []) {
      if (!SESSIONS.some((x) => x[0] === s)) continue;
      if (cfg.layouts.some((o) => o.sessions?.includes(s))) taken.push(s);
      else saved.sessions.push(s);
    }
    cfg.layouts.push(saved);
    const notes: string[] = [];
    const skipped = (pkg.overlays?.length ?? 0) - known.length;
    if (skipped > 0) notes.push(`${skipped} overlay${skipped === 1 ? ' is' : 's are'} not in this version and ${skipped === 1 ? 'was' : 'were'} left out.`);
    const res = myResolution();
    if (l.resolution && (l.resolution.w !== res.w || l.resolution.h !== res.h)) notes.push(`It was made for ${resText(l.resolution)}: positions are fitted to your ${resText(res)}; you may want to nudge a few.`);
    if (saved.sessions.length) notes.push(`Switched to automatically for: ${sessionText(saved.sessions)}.`);
    if (taken.length) notes.push(`Made for ${sessionText(taken)}, which another of your layouts already has: set it under Overlays → Layouts if you want.`);
    if (confirm(`“${name}” is in your layouts (Overlays → Layouts).${notes.length ? `\n\n${notes.join('\n')}` : ''}\n\nUse it now? Your current overlays are kept in the layout they belong to.`)) applyLayout(saved.id);
    else save(true);
    text(btn, 'Added ✓');
  } catch (e) {
    alert(`This layout could not be added: ${e instanceof Error ? e.message : e}`);
    text(btn, 'Add to my layouts');
    btn.disabled = false;
  }
}

// ---- The page ---------------------------------------------------------------------------

export const marketPage: Page = {
  id: 'market',
  title: 'Layout Market',
  render(el) {
    const rerender = () => this.render(el);
    const cfg = state.config!;
    if (gameFilter == null) gameFilter = cfg.layoutGame === 'iracing' || cfg.layoutGame === 'lmu' ? cfg.layoutGame : '';
    if (listings == null && !loading && !loadError) load().then(rerender);

    const intro = card(
      'Layout Market',
      h('p', 'dim', 'Overlay layouts shared by other drivers: their overlays, positions, sizes and settings (your own theme stays). Add one to your layouts and switch to it under Overlays → Layouts.'),
    );
    const bar = h('div', 'toolbar');
    const select = (opts: [string, string][], value: string, onChange: (v: string) => void) => {
      const s = h('select') as HTMLSelectElement;
      for (const [v, t] of opts) {
        const o = h('option', '', t) as HTMLOptionElement;
        o.value = v;
        s.append(o);
      }
      s.value = value;
      s.onchange = () => onChange(s.value);
      return s;
    };
    const game = select(
      [
        ['', 'All games'],
        ['iracing', 'iRacing'],
        ['lmu', 'Le Mans Ultimate'],
      ],
      gameFilter,
      (v) => {
        gameFilter = v;
        rerender();
      },
    );
    // Resolutions: any, this screen's, and every one listed.
    const mine = resText(myResolution());
    const listed = [...new Set((listings ?? []).map((l) => resText(l.resolution)).filter((r) => r && r !== mine))].sort();
    const res = select(
      [['', 'Any resolution'], [mine, `${mine} (my screen)`], ...listed.map((r): [string, string] => [r, r])],
      resFilter,
      (v) => {
        resFilter = v;
        rerender();
      },
    );
    const find = h('input', 'mk-search') as HTMLInputElement;
    find.type = 'search';
    find.placeholder = 'Search names, descriptions, creators…';
    find.value = search;
    find.oninput = () => {
      search = find.value;
      drawList();
    };
    bar.append(
      game,
      res,
      find,
      h('span', 'spacer'),
      button('↻ Refresh', () => {
        listings = null;
        loadError = '';
        loading = true;
        rerender();
        load(true).then(rerender);
      }),
      button('Share my layout…', shareDialog, 'primary'),
    );
    intro.append(bar);
    const list = h('div', 'mk-list');
    intro.append(list);

    const drawList = () => {
      if (loadError) {
        list.replaceChildren(h('div', 'notice', loadError));
        return;
      }
      if (!listings) {
        list.replaceChildren(h('p', 'dim', 'Loading the market…'));
        return;
      }
      const q = search.trim().toLowerCase();
      const shown = listings.filter(
        (l) =>
          (!gameFilter || l.game === gameFilter) &&
          (!resFilter || resText(l.resolution) === resFilter) &&
          (!q || `${l.name} ${l.description} ${l.creator ?? ''} ${l.author}`.toLowerCase().includes(q)),
      );
      if (!shown.length) {
        list.replaceChildren(h('p', 'dim', listings.length ? 'No layouts match.' : 'No layouts have been shared yet. Be the first: Share my layout.'));
        return;
      }
      list.replaceChildren(
        ...shown.map((l) => {
          const item = h('div', 'mk-item');
          const t = thumb(l);
          t.title = 'Click for a bigger preview';
          t.onclick = () => previewDialog(l);
          const top = h('div', 'mk-top');
          top.append(h('span', 'mk-name', l.name));
          if (l.game) top.append(h('span', `mk-game g-${l.game}`, l.game === 'lmu' ? 'LMU' : 'iRacing'));
          const tags = h('div', 'mk-tags');
          if (l.resolution) tags.append(h('span', 'mk-tag', resText(l.resolution)));
          tags.append(h('span', `mk-tag${l.sessions?.length ? ' sessions' : ''}`, sessionText(l.sessions)));
          tags.append(h('span', 'mk-tag', `${l.count} overlay${l.count === 1 ? '' : 's'}`));
          const when = l.updated ? new Date(l.updated).toLocaleDateString() : '';
          const meta = h('div', 'mk-meta dim small', [`by ${l.creator || l.author}`, `v${l.version}`, when].filter(Boolean).join(' · '));
          if (l.creator && l.creator !== l.author) meta.title = `Shared from the GitHub account ${l.author}`;
          const kinds = h('div', 'mk-kinds');
          for (const k of l.kinds ?? []) kinds.append(h('span', `mk-kind${byKind(k) ? '' : ' unknown'}`, byKind(k)?.name ?? k));
          const actions = h('div', 'mk-actions');
          const newer = needsNewer(l.version);
          const add = button(newer ? `Needs Apex Overlays ${l.version}` : 'Add to my layouts', () => addLayout(l, add), newer ? '' : 'primary');
          add.disabled = newer;
          if (newer) add.title = 'Made with a newer version: update Apex Overlays (General → Updates) to use it';
          actions.append(add, button('Preview', () => previewDialog(l), 'subtle'));
          if (l.issue?.startsWith(`${REPO}/`)) actions.append(button('Page', () => openPage(l.issue), 'subtle'));
          item.append(t, top, tags, meta);
          if (l.description) item.append(h('p', 'mk-desc', l.description));
          item.append(kinds, actions);
          return item;
        }),
      );
    };
    drawList();
    el.replaceChildren(intro);
  },
};
