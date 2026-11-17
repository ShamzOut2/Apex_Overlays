// Settings / control panel (served at /). Works in the desktop app and in any browser.

import { h, text } from './lib/dom';
import { leaguesPage } from './control/leagues';
import { teamsPage } from './control/teams';
import { raceControlPage } from './control/racecontrol';
import { marketPage } from './control/market';
import { renderUpdatePrompt } from './control/updatePrompt';
import { renderClosePrompt } from './control/closePrompt';
import { gameName, PAGES, type Page } from './control/pages';
import { listen, pageTopics, state } from './control/state';
import { applyUiTheme } from './lib/theme';
import { labsLogo } from './control/brand';

// General first, then the overlays and everything else.
const byId = (id: string) => PAGES.find((p) => p.id === id)!;
const pages: Page[] = [byId('general'), byId('overlays'), marketPage, byId('theme'), byId('strategy'), leaguesPage, raceControlPage, byId('streaming')];

const nav = h('nav');
const brand = h('div', 'brand');
const logo = h('span', 'logo');
logo.append(h('b', '', 'Apex'), ' Overlays');
const mark = h('img', 'brand-mark') as HTMLImageElement;
mark.src = '/assets/brand/apex-overlays-icon-64.png';
mark.alt = '';
brand.append(mark, logo);
const conn = h('div', 'conn');
const main = h('main');
nav.append(brand);
const links = new Map<string, HTMLAnchorElement>();
// Line icons for the sidebar (24×24, drawn with the current text colour).
const ICONS: Record<string, string> = {
  general: '<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1Z"/>',
  overlays: '<rect x="3" y="3" width="13" height="13" rx="2"/><path d="M8 21h11a2 2 0 0 0 2-2V8"/>',
  theme: '<path d="M12 3a9 9 0 1 0 0 18c1 0 1.6-.8 1.6-1.6 0-.4-.2-.8-.4-1.1-.3-.3-.4-.7-.4-1.1 0-.9.7-1.6 1.6-1.6H16a5 5 0 0 0 5-5C21 6.6 17 3 12 3Z"/><circle cx="7.5" cy="10.5" r="1"/><circle cx="10.5" cy="7" r="1"/><circle cx="15" cy="7.5" r="1"/>',
  strategy: '<path d="M10 2h4"/><path d="M12 14l3-3"/><circle cx="12" cy="14" r="8"/>',
  leagues: '<path d="M8 21h8M12 17v4M7 4h10v5a5 5 0 0 1-10 0V4Z"/><path d="M17 5h3v2a3 3 0 0 1-3 3M7 5H4v2a3 3 0 0 0 3 3"/>',
  racecontrol: '<path d="M4 22V4"/><path d="M4 4h14l-2 4 2 4H4"/>',
  garage61: '<path d="M3 21V9l9-6 9 6v12"/><path d="M7 21v-8h10v8M7 17h10"/>',
  market: '<path d="M4 7h16l-1.5 11a2 2 0 0 1-2 1.7h-9a2 2 0 0 1-2-1.7L4 7Z"/><path d="M9 7V5a3 3 0 0 1 6 0v2"/><path d="M12 11v5m-2.5-2.5L12 16l2.5-2.5"/>',
  streaming: '<rect x="2" y="5" width="15" height="14" rx="2"/><path d="m17 10 5-3v10l-5-3"/>',
};
for (const p of pages) {
  const a = h('a');
  if (ICONS[p.id]) a.insertAdjacentHTML('beforeend', `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${ICONS[p.id]}</svg>`);
  a.append(h('span', '', p.title));
  a.href = `#${p.id}`;
  nav.append(a);
  links.set(p.id, a);
}
nav.append(labsLogo('nav-labs'), conn);
document.body.append(nav, main);
// The desktop app opens this page as a small "update available" or "close?" window.
const promptKind = new URLSearchParams(location.search).get('prompt');
const promptMode = promptKind === 'update' || promptKind === 'close';
if (promptMode) {
  nav.hidden = true;
  document.body.classList.add('prompt');
}

// Pages opened from another page rather than the sidebar (and the sidebar entry they belong to).
const subPages: [Page, string][] = [[teamsPage, 'leagues']];
const current = () => pages.find((p) => `#${p.id}` === location.hash) ?? subPages.find(([p]) => `#${p.id}` === location.hash)?.[0] ?? pages[0];

function render() {
  if (!state.config) {
    main.replaceChildren(h('p', 'dim', 'Connecting to Apex Overlays…'));
    return;
  }
  applyUiTheme(state.config.theme, document.documentElement);
  if (promptKind === 'close') return renderClosePrompt(main);
  if (promptMode) return renderUpdatePrompt(main);
  const p = current();
  pageTopics(p.id);
  const parent = subPages.find(([s]) => s === p)?.[1];
  links.forEach((a, id) => a.classList.toggle('sel', id === p.id || id === parent));
  p.render(main);
}

function renderStatus() {
  const s = state.status;
  const sess = state.session;
  conn.className = `conn ${s?.connected ? 'up' : 'down'}`;
  const game = gameName(s?.source ?? '');
  text(conn, s ? (s.connected ? `${game} connected${sess?.trackName ? ` · ${sess.trackName}` : ''}` : `Waiting for ${s.source ? game : 'iRacing or Le Mans Ultimate'}…`) + ` · v${s.version}` : 'Not connected to server');
}

let lastLive = 0;
window.addEventListener('hashchange', render);
// Drop-down menus open towards the side with room (a narrow window wraps the toolbar).
document.addEventListener(
  'toggle',
  (e) => {
    const d = e.target as HTMLElement;
    if (!(d instanceof HTMLDetailsElement) || !d.open) return;
    const pop = d.querySelector<HTMLElement>(':scope > .menu-pop');
    if (!pop) return;
    pop.style.left = '';
    pop.style.right = '';
    const r = pop.getBoundingClientRect();
    const mainLeft = main.getBoundingClientRect().left;
    if (r.left < mainLeft + 8) {
      pop.style.left = '0';
      pop.style.right = 'auto';
    }
  },
  true,
);
// Escape closes the dialog on top (Add overlay, Share a layout, previews…).
window.addEventListener('keydown', (e) => {
  if (e.key !== 'Escape') return;
  const dialogs = document.querySelectorAll('.modal-back');
  if (dialogs.length) {
    e.preventDefault();
    dialogs[dialogs.length - 1].remove();
  }
});
let leagueKey = '';
let wasInVr = false;
let lastGame = '';
listen((t) => {
  if (t === 'config') render();
  if (t === 'status') {
    renderStatus();
    // The Game card on General says which game is being read.
    const game = `${state.status?.source}|${state.status?.connected}`;
    if (game !== lastGame && current().id === 'general' && !(document.activeElement instanceof HTMLInputElement)) render();
    lastGame = game;
    // Edit-mode / visibility buttons live on the overlays page.
    if (current().id === 'overlays') render();
  }
  if (t === 'session') {
    renderStatus();
    // The league builder says which league applies to the session you are in.
    const s = state.session;
    const k = `${s?.connected}|${s?.leagueId}|${s?.league}`;
    if (k !== leagueKey) {
      leagueKey = k;
      if (current().id === 'leagues' && !(document.activeElement instanceof HTMLInputElement)) render();
    }
  }
  if ((t === 'trackmap' || t === 'vr') && current().id === 'general') render();
  // Entering or leaving VR: the overlay editor's position boxes switch between the screen and VR positions.
  if (t === 'vr') {
    const inVr = state.vr?.status === 'on';
    if (inVr !== wasInVr && current().id === 'overlays' && !(document.activeElement instanceof HTMLInputElement)) render();
    wasInVr = inVr;
  }
  if (t === 'update' && (promptKind === 'update' || (!promptMode && current().id === 'general'))) render();
  // Race control lists live flags: refresh every couple of seconds unless the user is typing.
  if (t === 'standings' && current().id === 'racecontrol' && Date.now() - lastLive > 2000 && !(document.activeElement instanceof HTMLInputElement)) {
    lastLive = Date.now();
    render();
  }
});
render();
