// Layout Market: checks a layout shared through the "Share a layout" issue form and publishes
// it to the `market` branch (index.json + layouts/<issue number>.json), which the app reads.
// Run by .github/workflows/market.yml on issue events. The issue text is untrusted: it is only
// ever parsed as data here, never passed to a shell.

import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { gunzipSync } from 'node:zlib';

const MAX_CODE = 60_000; // characters of share code
const MAX_JSON = 400_000; // bytes once unpacked
const MAX_OVERLAYS = 40;
const MAX_PER_AUTHOR = 10;
const DIR = 'market';

const event = JSON.parse(readFileSync(process.env.GITHUB_EVENT_PATH, 'utf8'));
const repo = process.env.GITHUB_REPOSITORY;
const token = process.env.GITHUB_TOKEN;
const issue = event.issue;
const id = String(issue.number);

async function api(method, path, body) {
  // Testing locally: say what would be done.
  if (process.env.MARKET_DRY_RUN) {
    console.log(`[api] ${method} ${path} ${body ? JSON.stringify(body) : ''}`);
    return;
  }
  const r = await fetch(`https://api.github.com/repos/${repo}${path}`, {
    method,
    headers: { authorization: `Bearer ${token}`, accept: 'application/vnd.github+json', 'content-type': 'application/json', 'user-agent': 'apex-overlays-market' },
    body: body ? JSON.stringify(body) : undefined,
  });
  if (!r.ok) console.log(`${method} ${path}: ${r.status} ${await r.text()}`);
}
const comment = (text) => api('POST', `/issues/${id}/comments`, { body: text });
const label = (...names) => api('POST', `/issues/${id}/labels`, { labels: names });

const git = (...args) => execFileSync('git', args, { cwd: DIR, stdio: 'inherit' });
const remote = process.env.MARKET_REMOTE || `https://x-access-token:${token}@github.com/${repo}.git`;

/** The market branch, checked out (or started, the first time). */
function checkout() {
  rmSync(DIR, { recursive: true, force: true });
  try {
    execFileSync('git', ['clone', '--depth', '1', '--branch', 'market', remote, DIR], { stdio: 'inherit' });
  } catch {
    rmSync(DIR, { recursive: true, force: true });
    mkdirSync(DIR);
    git('init', '-b', 'market');
    git('remote', 'add', 'origin', remote);
  }
  git('config', 'user.name', 'Apex Overlays Market');
  git('config', 'user.email', '41898282+github-actions[bot]@users.noreply.github.com');
  mkdirSync(join(DIR, 'layouts'), { recursive: true });
}

function readIndex() {
  const p = join(DIR, 'index.json');
  if (!existsSync(p)) return { layouts: [] };
  try {
    const d = JSON.parse(readFileSync(p, 'utf8'));
    return Array.isArray(d.layouts) ? d : { layouts: [] };
  } catch {
    return { layouts: [] };
  }
}

function commit(message) {
  writeFileSync(join(DIR, 'README.md'), '# Apex Overlays Layout Market\n\nLayouts shared from the app, published by `.github/workflows/market.yml`. Browse them in the app: Layout Market.\n');
  git('add', '-A');
  try {
    execFileSync('git', ['diff', '--cached', '--quiet'], { cwd: DIR });
    return; // nothing changed
  } catch {
    /* changes to commit */
  }
  git('commit', '-q', '-m', message);
  git('push', '-q', 'origin', 'market');
}

/** The answers of an issue form: "### Heading\n\nanswer" blocks. */
function formFields(body) {
  const out = {};
  const parts = String(body ?? '').split(/^### /m).slice(1);
  for (const p of parts) {
    const nl = p.indexOf('\n');
    const head = p.slice(0, nl).trim().toLowerCase();
    let value = p.slice(nl + 1).trim();
    value = value.replace(/^```[a-z]*\n?/i, '').replace(/\n?```$/, '').trim();
    if (value === '_No response_') value = '';
    out[head] = value;
  }
  return out;
}

const plain = (s, max) =>
  String(s ?? '')
    .replace(/[\u0000-\u001f\u007f<>]/g, ' ')
    .replace(/\s+/g, ' ')
    .trim()
    .slice(0, max);

/** Settings copied as plain JSON only: limited depth, size and string length, and no custom CSS. */
function cleanSettings(v, depth = 0, budget = { keys: 0 }) {
  if (v === null || typeof v === 'boolean') return v;
  if (typeof v === 'number') return Number.isFinite(v) ? v : null;
  if (typeof v === 'string') return v.slice(0, 500);
  if (depth >= 6) return null;
  if (Array.isArray(v)) return v.slice(0, 100).map((x) => cleanSettings(x, depth + 1, budget));
  if (typeof v === 'object') {
    const out = {};
    for (const [k, x] of Object.entries(v)) {
      if (k === 'customCss' || k === '__proto__' || k === 'constructor' || !/^[A-Za-z0-9_-]{1,40}$/.test(k)) continue;
      if (++budget.keys > 2000) break;
      out[k] = cleanSettings(x, depth + 1, budget);
    }
    return out;
  }
  return null;
}

const num = (v, lo, hi) => (typeof v === 'number' && Number.isFinite(v) ? Math.min(hi, Math.max(lo, Math.round(v))) : null);

/** The layout in a share code, checked; or a reason it cannot be published. */
function decode(code) {
  const c = code.replace(/\s+/g, '');
  if (!c.startsWith('AOL1:')) return { error: 'The share code should start with `AOL1:`. Make it in the app: Layout Market → Share my layout.' };
  if (c.length > MAX_CODE) return { error: 'This layout is too big for the market.' };
  let data;
  try {
    const json = gunzipSync(Buffer.from(c.slice(5), 'base64'), { maxOutputLength: MAX_JSON });
    data = JSON.parse(json.toString('utf8'));
  } catch {
    return { error: 'The share code could not be read: it may be cut short. Copy it again from the app and edit this issue.' };
  }
  if (!data || data.apexOverlays !== 'market' || !Array.isArray(data.overlays)) return { error: 'That is not an Apex Overlays market layout.' };
  if (!data.overlays.length || data.overlays.length > MAX_OVERLAYS) return { error: `A layout has 1 to ${MAX_OVERLAYS} overlays.` };
  const overlays = [];
  for (const o of data.overlays) {
    if (!o || typeof o.kind !== 'string' || !/^[A-Za-z0-9]{1,32}$/.test(o.kind)) return { error: 'One of the overlays is not valid.' };
    const x = num(o.x, -20000, 20000);
    const y = num(o.y, -20000, 20000);
    const width = num(o.width, 20, 10000);
    const height = num(o.height, 20, 10000);
    if (x == null || y == null || width == null || height == null) return { error: 'One of the overlays has no position or size.' };
    overlays.push({ kind: o.kind, name: plain(o.name, 60), enabled: o.enabled !== false, x, y, width, height, settings: cleanSettings(o.settings && typeof o.settings === 'object' ? o.settings : {}) });
  }
  const version = /^\d{1,3}\.\d{1,3}\.\d{1,3}$/.test(String(data.version)) ? String(data.version) : '';
  if (!version) return { error: 'The share code has no app version: make it again with the app.' };
  const game = data.game === 'iracing' || data.game === 'lmu' ? data.game : '';
  const sw = num(data.screen?.w, 320, 16000);
  const sh = num(data.screen?.h, 200, 16000);
  return { layout: { version, game, screen: sw && sh ? { w: sw, h: sh } : null, overlays } };
}

async function main() {
  checkout();
  const index = readIndex();
  const others = index.layouts.filter((l) => l.id !== id);

  // Taken off the market by a maintainer.
  if (event.action === 'labeled') {
    if (event.label?.name !== 'remove') return;
    rmSync(join(DIR, 'layouts', `${id}.json`), { force: true });
    if (others.length !== index.layouts.length) {
      writeFileSync(join(DIR, 'index.json'), JSON.stringify({ layouts: others }, null, 1));
      commit(`Remove layout ${id}`);
      await comment('This layout has been taken off the Layout Market.');
    }
    return;
  }
  if (issue.labels?.some((l) => l.name === 'remove')) return;

  const f = formFields(issue.body);
  const code = f['share code'] ?? '';
  if (!code) {
    await comment('No share code found. Make one in the app (Layout Market → Share my layout), paste it under **Share code** and edit this issue.');
    await label('layout', 'invalid');
    return;
  }
  const author = String(issue.user?.login ?? '');
  if (others.filter((l) => l.author === author).length >= MAX_PER_AUTHOR) {
    await comment(`You already have ${MAX_PER_AUTHOR} layouts on the market. Edit one of your earlier issues to update it instead.`);
    await label('layout', 'invalid');
    return;
  }
  const r = decode(code);
  if (r.error) {
    await comment(`This layout could not be published: ${r.error}`);
    await label('layout', 'invalid');
    return;
  }
  const name = plain(f['layout name'] || issue.title.replace(/^Layout:/i, ''), 60) || 'Untitled layout';
  const description = plain(f['what it is for'], 400);
  const now = new Date().toISOString();
  const prev = index.layouts.find((l) => l.id === id);
  const entry = {
    id,
    name,
    description,
    author,
    game: r.layout.game,
    version: r.layout.version,
    screen: r.layout.screen,
    kinds: [...new Set(r.layout.overlays.filter((o) => o.enabled).map((o) => o.kind))],
    count: r.layout.overlays.length,
    issue: issue.html_url,
    published: prev?.published ?? now,
    updated: now,
  };
  writeFileSync(join(DIR, 'layouts', `${id}.json`), JSON.stringify({ apexOverlays: 'market', id, name, description, author, ...r.layout }, null, 1));
  const layouts = [entry, ...others].sort((a, b) => String(b.updated).localeCompare(String(a.updated)));
  writeFileSync(join(DIR, 'index.json'), JSON.stringify({ layouts }, null, 1));
  commit(`${prev ? 'Update' : 'Publish'} layout ${id}: ${name}`);
  await api('DELETE', `/issues/${id}/labels/invalid`);
  await label('layout', 'published');
  await comment(
    `${prev ? 'Updated' : 'Published'} on the Layout Market: **${name}** (${r.layout.overlays.length} overlays, made with Apex Overlays ${r.layout.version}). ` +
      'It shows in the app under Layout Market within a few minutes. Edit this issue to update it.',
  );
  if (issue.state === 'open') await api('PATCH', `/issues/${id}`, { state: 'closed', state_reason: 'completed' });
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
