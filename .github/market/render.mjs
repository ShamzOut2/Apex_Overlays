// Layout Market: renders a preview image of a published layout and adds it to the market
// branch. The app's server runs with simulated race data, and the layout's overlays are laid
// out in headless Chromium at the screen size they were made for, as they would be in the game.
//
// Run by .github/workflows/market.yml after publish.mjs (which leaves the market branch
// checked out in ./market). Env: MARKET_ID (the layout), APP_DIR (the built app: ui/dist and
// target/release/apex-overlays-server), PLAYWRIGHT_DIR (where playwright is installed).

import { execFileSync, spawn } from 'node:child_process';
import { mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const DIR = resolve(process.env.MARKET_DIR || 'market');
const APP = resolve(process.env.APP_DIR || 'app');
const id = String(process.env.MARKET_ID || '');
if (!/^[A-Za-z0-9-]{1,40}$/.test(id)) throw new Error(`bad layout id: ${id}`);
const { chromium } = createRequire(join(resolve(process.env.PLAYWRIGHT_DIR || '.'), 'node_modules') + '/')('playwright');

const PORT = 8399;
const BASE = `http://127.0.0.1:${PORT}`;
/** Preview images are this wide (JPEG). */
const OUT_WIDTH = 1280;

const layout = JSON.parse(readFileSync(join(DIR, 'layouts', `${id}.json`), 'utf8'));
const screen = layout.screen ?? { w: 1920, h: 1080 };

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function waitFor(url, tries = 60) {
  for (let i = 0; i < tries; i++) {
    try {
      if ((await fetch(url)).ok) return;
    } catch {
      /* not up yet */
    }
    await sleep(500);
  }
  throw new Error(`${url} did not answer`);
}

/** The page: a dark backdrop the size of the screen, and each overlay where it sits. */
function page(instances) {
  const frames = instances
    .map((o) => {
      // Overlays that fit themselves to their content get room to grow; the others (drawn to
      // fill their window) keep their size.
      const fill = ['trackmap', 'radar', 'inputs', 'inputGraph', 'flags', 'spotter'].includes(o.kind) || o.settings?.autoHeight === false;
      const h = fill ? o.height : Math.max(o.height, screen.h - o.y);
      return `<iframe src="/o/${o.id}" style="left:${o.x}px;top:${o.y}px;width:${o.width}px;height:${h}px"></iframe>`;
    })
    .join('');
  return `<!doctype html><html><head><meta charset="utf-8"><style>
    :root{color-scheme:dark}
    html,body{margin:0;width:${screen.w}px;height:${screen.h}px;overflow:hidden}
    body{background:radial-gradient(120% 90% at 50% 38%,#2a3340 0%,#161b23 55%,#0b0e13 100%)}
    .horizon{position:absolute;left:0;right:0;top:46%;height:54%;background:linear-gradient(180deg,#1d232c,#0e1116 80%)}
    .road{position:absolute;left:30%;right:30%;bottom:0;height:54%;background:linear-gradient(180deg,#30353d,#202328);clip-path:polygon(46% 0,54% 0,100% 100%,0 100%);opacity:.7}
    iframe{position:absolute;border:0;background:transparent;color-scheme:dark}
    .tag{position:absolute;right:${Math.round(screen.w * 0.012)}px;bottom:${Math.round(screen.h * 0.015)}px;font:600 ${Math.max(12, Math.round(screen.h / 70))}px system-ui,sans-serif;color:rgba(255,255,255,.35);letter-spacing:.06em}
  </style></head><body><div class="horizon"></div><div class="road"></div>${frames}<div class="tag">PREVIEW · SAMPLE DATA</div></body></html>`;
}

async function main() {
  const data = mkdtempSync(join(tmpdir(), 'apex-market-'));
  const server = spawn(join(APP, 'target/release/apex-overlays-server'), ['--mock', '--port', String(PORT), '--data-dir', data, '--ui-dir', join(APP, 'ui/dist')], { stdio: 'ignore' });
  try {
    await waitFor(`${BASE}/api/config`);
    // The layout's overlays, switched on as shared, with the standard theme.
    const cfg = await (await fetch(`${BASE}/api/config`)).json();
    const instances = layout.overlays
      .filter((o) => o.enabled !== false)
      .map((o, i) => ({ id: `m${i + 1}`, kind: o.kind, name: o.name ?? '', enabled: true, x: o.x, y: o.y, width: o.width, height: o.height, settings: o.settings ?? {} }));
    cfg.overlays = instances;
    cfg.layouts = [];
    cfg.activeLayout = '';
    const put = await fetch(`${BASE}/api/config`, { method: 'PUT', headers: { 'content-type': 'application/json' }, body: JSON.stringify(cfg) });
    if (!put.ok) throw new Error(`config: ${put.status}`);
    // Let the simulated race get going (laps, gaps, fuel use).
    await sleep(8000);

    const browser = await chromium.launch();
    const ctx = await browser.newContext({ viewport: { width: screen.w, height: screen.h }, deviceScaleFactor: OUT_WIDTH / screen.w });
    const p = await ctx.newPage();
    // Only the app's own server: nothing else is fetched while rendering.
    await p.route('**/*', (r) => (r.request().url().startsWith(BASE) ? r.continue() : r.abort()));
    await p.route(`${BASE}/market-preview`, (r) => r.fulfill({ contentType: 'text/html', body: page(instances) }));
    await p.goto(`${BASE}/market-preview`);
    await sleep(6000);
    const jpg = await p.screenshot({ type: 'jpeg', quality: 82 });
    await browser.close();

    // A new name each time, so the app never shows an old copy from a cache.
    mkdirSync(join(DIR, 'previews'), { recursive: true });
    for (const f of readdirSync(join(DIR, 'previews'))) if (f.startsWith(`${id}-`)) rmSync(join(DIR, 'previews', f));
    const file = `previews/${id}-${Date.now().toString(36)}.jpg`;
    writeFileSync(join(DIR, file), jpg);
    const index = JSON.parse(readFileSync(join(DIR, 'index.json'), 'utf8'));
    const entry = index.layouts.find((l) => l.id === id);
    if (!entry) return; // taken off meanwhile
    entry.preview = file;
    writeFileSync(join(DIR, 'index.json'), JSON.stringify(index, null, 1));
    const git = (...a) => execFileSync('git', a, { cwd: DIR, stdio: 'inherit' });
    git('add', '-A');
    git('commit', '-q', '-m', `Preview for layout ${id}`);
    git('push', '-q', 'origin', 'market');
    console.log(`preview: ${file} (${jpg.length} bytes)`);
  } finally {
    server.kill();
    rmSync(data, { recursive: true, force: true });
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
