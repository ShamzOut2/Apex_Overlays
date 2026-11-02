// Settings / control panel (served at /). Works in the desktop app and in any browser.

import { h, text } from './lib/dom';
import { leaguesPage } from './control/leagues';
import { raceControlPage } from './control/racecontrol';
import { renderUpdatePrompt } from './control/updatePrompt';
import { renderClosePrompt } from './control/closePrompt';
import { PAGES, type Page } from './control/pages';
import { listen, pageTopics, state } from './control/state';
import { applyUiTheme } from './lib/theme';

// General first, then the overlays and everything else.
const byId = (id: string) => PAGES.find((p) => p.id === id)!;
const pages: Page[] = [byId('general'), byId('overlays'), byId('theme'), byId('strategy'), leaguesPage, raceControlPage, byId('garage61'), byId('streaming')];

const nav = h('nav');
const brand = h('div', 'brand');
brand.append(h('span', 'logo', 'Apex Overlays'));
const conn = h('div', 'conn');
const main = h('main');
nav.append(brand);
const links = new Map<string, HTMLAnchorElement>();
for (const p of pages) {
  const a = h('a', '', p.title);
  a.href = `#${p.id}`;
  nav.append(a);
  links.set(p.id, a);
}
nav.append(conn);
document.body.append(nav, main);
// The desktop app opens this page as a small "update available" or "close?" window.
const promptKind = new URLSearchParams(location.search).get('prompt');
const promptMode = promptKind === 'update' || promptKind === 'close';
if (promptMode) {
  nav.hidden = true;
  document.body.classList.add('prompt');
}

const current = () => pages.find((p) => `#${p.id}` === location.hash) ?? pages[0];

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
  links.forEach((a, id) => a.classList.toggle('sel', id === p.id));
  p.render(main);
}

function renderStatus() {
  const s = state.status;
  const sess = state.session;
  conn.className = `conn ${s?.connected ? 'up' : 'down'}`;
  text(conn, s ? (s.connected ? `${s.source === 'mock' ? 'Mock sim' : 'iRacing'} connected${sess?.trackName ? ` · ${sess.trackName}` : ''}` : `Waiting for iRacing…`) + ` · v${s.version}` : 'Not connected to server');
}

let lastLive = 0;
window.addEventListener('hashchange', render);
let leagueKey = '';
listen((t) => {
  if (t === 'config') render();
  if (t === 'status') {
    renderStatus();
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
  if (t === 'garage61' && current().id === 'garage61') render();
  if (t === 'leagueImport' && current().id === 'leagues') render();
  if ((t === 'trackmap' || t === 'vr') && current().id === 'general') render();
  if (t === 'update' && (promptKind === 'update' || (!promptMode && current().id === 'general'))) render();
  // Race control lists live flags: refresh every couple of seconds unless the user is typing.
  if (t === 'standings' && current().id === 'racecontrol' && Date.now() - lastLive > 2000 && !(document.activeElement instanceof HTMLInputElement)) {
    lastLive = Date.now();
    render();
  }
});
render();
