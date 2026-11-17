// Layout Market: overlay layouts people share. They are kept in the app's GitHub repository
// (see crates/core/src/market.rs and .github/market/publish.mjs): browsing and downloading
// work for everyone; sharing makes a share code here and opens the repository's "Share a
// layout" form (a free GitHub account), where it is checked and listed automatically.

import { h, text } from '../lib/dom';
import type { SavedLayout } from '../lib/types';
import { byKind, hiddenKind } from '../overlays/registry';
import { applyLayout, button, card, gameName, instanceFrom, type Page, shareable, type SharedOverlay } from './pages';
import { client, save, state, uid } from './state';

const REPO = 'https://github.com/ShamzOut2/Apex_Overlays';

/** A layout as listed (index.json on the repository's market branch). */
interface Listing {
  id: string;
  name: string;
  description: string;
  author: string;
  game: string;
  version: string;
  kinds: string[];
  count: number;
  issue: string;
  published: string;
  updated: string;
  screen: { w: number; h: number } | null;
}

/** What a share code holds (and a market layout file, plus its id, name and author). */
interface MarketPackage {
  apexOverlays: 'market';
  version: string;
  game: string;
  name: string;
  description: string;
  screen: { w: number; h: number } | null;
  overlays: SharedOverlay[];
}

let listings: Listing[] | null = null;
let loadError = '';
let loading = false;
let gameFilter: string | null = null;
let search = '';

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
  const desc = h('textarea') as HTMLTextAreaElement;
  desc.maxLength = 400;
  desc.rows = 3;
  desc.placeholder = 'What it is for: the kind of racing, what is where…';
  const which = h('select') as HTMLSelectElement;
  const optCurrent = h('option', '', 'The overlays I have now') as HTMLOptionElement;
  optCurrent.value = '';
  which.append(optCurrent);
  for (const l of cfg.layouts) {
    const o = h('option', '', `Saved layout: ${l.name}`) as HTMLOptionElement;
    o.value = l.id;
    which.append(o);
  }
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
      if (!n) {
        name.focus();
        return;
      }
      const src = which.value ? (cfg.layouts.find((l) => l.id === which.value)?.overlays ?? []) : cfg.overlays;
      const overlays = src.filter((o) => !hiddenKind(o.kind) && byKind(o.kind) && (!onlyOn.checked || o.enabled)).map(shareable);
      if (!overlays.length) {
        alert('There are no overlays to share: switch some on, or untick “Only the overlays that are switched on”.');
        return;
      }
      const pkg: MarketPackage = {
        apexOverlays: 'market',
        version: state.status?.version ?? '',
        game: cfg.layoutGame ?? '',
        name: n,
        description: desc.value.trim(),
        screen: { w: window.screen.width, h: window.screen.height },
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
      out.replaceChildren(
        h('p', '', `Share code ready: ${overlays.length} overlay${overlays.length === 1 ? '' : 's'}.`),
        h(
          'p',
          'dim small',
          inLink
            ? 'The share form opens on GitHub with your layout filled in: press “Submit new issue”. It is checked and listed in a minute or two. You need a free GitHub account.'
            : 'The code has been copied. The share form opens on GitHub: paste it under “Share code” and press “Submit new issue”. It is checked and listed in a minute or two. You need a free GitHub account.',
        ),
        box,
        h('div', 'toolbar mk-actions'),
        h('small', 'dim', 'If the form did not open, copy this link into your browser:'),
        linkBox,
      );
      out.querySelector('.mk-actions')!.append(button('Open the share form', () => openPage(url), 'primary'), copyBtn);
      openPage(url);
    },
    'primary',
  );

  const makeBar = h('div', 'toolbar');
  makeBar.append(make);
  dlg.append(
    head,
    h('p', 'dim', 'Share your overlays, their positions, sizes and settings with everyone using Apex Overlays. Never your theme, logins or tokens; custom CSS is left out.'),
    field('Name', name),
    field('What it is for', desc),
    field('Overlays to share', which),
    onlyLabel,
    makeBar,
    out,
  );
  back.append(dlg);
  document.body.append(back);
  name.focus();
}

// ---- Downloading ------------------------------------------------------------------------

/** Positions moved from the sharer's screen to this one (sizes kept), and kept on screen. */
function fitToScreen(overlays: SharedOverlay[], from: { w: number; h: number } | null): SharedOverlay[] {
  const W = window.screen.width;
  const H = window.screen.height;
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
    cfg.layouts.push(saved);
    const skipped = (pkg.overlays?.length ?? 0) - known.length;
    const note = skipped > 0 ? `\n\n${skipped} overlay${skipped === 1 ? ' is' : 's are'} not in this version and ${skipped === 1 ? 'was' : 'were'} left out.` : '';
    if (confirm(`“${name}” is in your layouts (Overlays → Layouts).${note}\n\nUse it now? Your current overlays are kept in the layout they belong to.`)) applyLayout(saved.id);
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
    if (gameFilter == null) gameFilter = cfg.layoutGame || '';
    if (listings == null && !loading && !loadError) load().then(rerender);

    const intro = card(
      'Layout Market',
      h('p', 'dim', 'Overlay layouts shared by other drivers: their overlays, positions, sizes and settings (your own theme stays). Add one to your layouts and switch to it under Overlays → Layouts.'),
    );
    const bar = h('div', 'toolbar');
    const game = h('select') as HTMLSelectElement;
    for (const [v, t] of [
      ['', 'All games'],
      ['iracing', 'iRacing'],
      ['lmu', 'Le Mans Ultimate'],
    ]) {
      const o = h('option', '', t) as HTMLOptionElement;
      o.value = v;
      game.append(o);
    }
    game.value = gameFilter;
    game.onchange = () => {
      gameFilter = game.value;
      rerender();
    };
    const find = h('input', 'mk-search') as HTMLInputElement;
    find.type = 'search';
    find.placeholder = 'Search names, descriptions, authors…';
    find.value = search;
    find.oninput = () => {
      search = find.value;
      drawList();
    };
    bar.append(
      game,
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
        (l) => (!gameFilter || !l.game || l.game === gameFilter) && (!q || `${l.name} ${l.description} ${l.author}`.toLowerCase().includes(q)),
      );
      if (!shown.length) {
        list.replaceChildren(
          h('p', 'dim', listings.length ? 'No layouts match.' : 'No layouts have been shared yet. Be the first: Share my layout.'),
        );
        return;
      }
      list.replaceChildren(
        ...shown.map((l) => {
          const item = h('div', 'mk-item');
          const top = h('div', 'mk-top');
          top.append(h('span', 'mk-name', l.name));
          if (l.game) top.append(h('span', `mk-game g-${l.game}`, gameName(l.game)));
          const when = l.updated ? new Date(l.updated).toLocaleDateString() : '';
          const meta = h('div', 'mk-meta dim small', [`by ${l.author}`, `${l.count} overlay${l.count === 1 ? '' : 's'}`, `made with v${l.version}`, when].filter(Boolean).join(' · '));
          const kinds = h('div', 'mk-kinds');
          for (const k of l.kinds ?? []) kinds.append(h('span', `mk-kind${byKind(k) ? '' : ' unknown'}`, byKind(k)?.name ?? k));
          const actions = h('div', 'mk-actions');
          const newer = needsNewer(l.version);
          const add = button(newer ? `Needs Apex Overlays ${l.version}` : 'Add to my layouts', () => addLayout(l, add), newer ? '' : 'primary');
          add.disabled = newer;
          if (newer) add.title = 'Made with a newer version: update Apex Overlays (General → Updates) to use it';
          actions.append(add);
          if (l.issue?.startsWith(`${REPO}/`)) actions.append(button('Page', () => openPage(l.issue), 'subtle'));
          item.append(top, meta);
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
