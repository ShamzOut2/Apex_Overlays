// Host page for a single overlay: /o/<id>. Loads the instance's settings from
// the config topic, mounts the overlay and feeds it topic data.

import { Client } from './lib/client';
import { h, show, text } from './lib/dom';
import { reportErrors } from './lib/errors';
import type { Units } from './lib/format';
import { type Ctx, type OverlayDef, schemaOf, type View } from './lib/overlay';
import { merge, type Values } from './lib/settings';
import { applyTheme } from './lib/theme';
import type { Config, OverlayInstance, Player, Session, Status, TopicName } from './lib/types';
import { byKind } from './overlays/registry';

const q = new URLSearchParams(location.search);
const id = decodeURIComponent(location.pathname.split('/o/')[1] ?? '') || q.get('id') || '';
const preview = q.has('preview');
document.body.classList.toggle('preview', preview);

const panel = h('div', 'panel');
const edit = h('div', 'edit-frame');
const editLabel = h('div', 'edit-label');
const grip = h('div', 'edit-grip');
edit.append(editLabel, grip);
const custom = h('style');
document.head.append(custom);
document.body.append(panel, edit);

let config: Config | null = null;
let inst: OverlayInstance | undefined;
let def: OverlayDef | undefined;
let view: View | null = null;
let settings: Values = {};
let key = '';
let status: Status | null = null;
let session: Session | null = null;
let player: Player | null = null;
let lastHidden: boolean | null = null;

const units = (): Units => (config?.units === 'metric' || config?.units === 'imperial' ? config.units : (session?.units ?? 'metric'));

const client = new Client(onData, (up) => {
  // The connection to the app: lost and found again (written once it is back).
  if (!up && wasConnected) lostAt = Date.now();
  if (up && lostAt) {
    logLine(`connection to the app lost for ${((Date.now() - lostAt) / 1000).toFixed(1)}s (hidden meanwhile)`);
    lostAt = 0;
  }
  wasConnected = up;
  updateVisibility();
  // The desktop app forgets auto sizes when it restarts, and messages sent while the
  // connection was opening are lost: tell it again.
  if (up) resendAutoSize();
});
client.subscribe('config', 'status', 'session');
const source = () => `overlay ${inst?.kind ?? '?'}${inst && inst.id !== inst.kind ? ` (${inst.id})` : ''}`;
reportErrors(client, source);

/** A line in the app's log about this overlay (not for the settings' preview); at most 40 a
 *  minute, so something that flips back and forth cannot flood it. */
let logBudget = 40;
setInterval(() => (logBudget = 40), 60_000);
function logLine(message: string) {
  if (preview || logBudget <= 0) return;
  logBudget--;
  client.report('info', source(), logBudget === 0 ? `${message} (more lines this minute left out)` : message);
}

let wasConnected = false;
let lostAt = 0;

edit.addEventListener('mousedown', (e) => {
  if (e.button !== 0) return;
  e.preventDefault();
  client.send(e.target === grip ? { op: 'resize', id, dir: 'SouthEast' } : { op: 'drag', id });
});

function onData(t: TopicName, d: unknown) {
  if (t === 'config') applyConfig(d as Config);
  else if (t === 'status') status = d as Status;
  else if (t === 'session') session = d as Session;
  else if (t === 'player') player = d as Player;
  if (t === 'status' || t === 'session' || t === 'player') updateVisibility();
  if (t === 'session' && config?.units === 'auto' && key && !key.endsWith(`|${units()}`)) rebuild();
  if (view && def?.topics.includes(t)) {
    // A hidden overlay skips drawing; it catches up with the newest data once shown.
    if (document.body.classList.contains('hidden')) stale.add(t);
    else deliver(t, d);
  }
}

/** Topics that arrived while the overlay was hidden. */
const stale = new Set<TopicName>();

function deliver(t: TopicName, d: unknown) {
  try {
    view?.update(t, d);
  } catch (err) {
    console.error(err);
  }
  measureSoon();
}

function applyConfig(c: Config) {
  config = c;
  inst = c.overlays.find((o) => o.id === id);
  def = inst ? byKind(inst.kind) : undefined;
  if (!inst || !def) {
    panel.replaceChildren(h('div', 'missing', id ? `No overlay with id "${id}"` : 'Missing overlay id'));
    view = null;
    return;
  }
  settings = merge(schemaOf(def), inst.settings);
  const k = `${JSON.stringify([settings, c.theme])}|${units()}`;
  text(editLabel, `${inst.name || def.name} — drag to move, corner to resize`);
  if (k !== key) rebuild();
  updateVisibility();
}

function rebuild() {
  if (!config || !inst || !def) return;
  key = `${JSON.stringify([settings, config.theme])}|${units()}`;
  const theme = applyTheme(config.theme, document.documentElement, { scale: settings.scale as number, bgOpacity: settings.bgOpacity as number, opacity: settings.opacity as number, fontSize: settings.textSize as number });
  custom.textContent = settings.customCss as string;
  panel.className = `panel kind-${def.kind}${settings.autoHeight && !def.fill ? ' auto-height' : ''}${settings.autoWidth && def.fitsWidth && !def.fill && !preview ? ' auto-width' : ''}`;
  setupAutoSize();
  panel.replaceChildren();
  const ctx: Ctx = {
    id,
    settings,
    theme,
    get units() {
      return units();
    },
    latest: client.latest,
    send: (m) => client.send(m),
    limit: (t, hz) => client.limit(t, hz),
    log: logLine,
  };
  view = def.create(panel, ctx);
  stale.clear();
  // The 60 Hz player topic only when the overlay shows it or hides itself by it: just to
  // know when to hide, a few times a second is plenty.
  if (settings.hideWhenNotDriving || settings.hideInPits || def.topics.includes('player')) client.subscribe('player');
  else {
    client.unsubscribe('player');
    player = null;
  }
  for (const t of ['player', ...def.topics] as TopicName[]) client.limit(t, def.rates?.[t] ?? (t === 'player' && !def.topics.includes('player') ? 10 : 0));
  client.subscribe(...def.topics);
  for (const t of def.topics) {
    const d = client.latest[t];
    if (d !== undefined) view.update(t, d);
  }
}

/** Auto-size: the panel hugs its content and asks the desktop app to fit the window to it (0 = off). */
let sizer: ResizeObserver | null = null;
let tinyLogged = false;
let everBig = false;
let lastSize = { width: -1, height: -1 };
let measure: (() => void) | null = null;
function resendAutoSize() {
  lastSize = { width: -1, height: -1 };
  measure?.();
}
function setupAutoSize() {
  sizer?.disconnect();
  sizer = null;
  const autoH = !!settings.autoHeight && !preview && !def?.fill;
  const autoW = !!settings.autoWidth && !preview && !!def?.fitsWidth && !def?.fill;
  // Fitting the content, without fitting the width to columns: widen the window when
  // something would not fit, and back to the width set once it does.
  const growW = autoH && !autoW;
  let grown = false;
  const send = (width: number, height: number) => {
    if (Math.abs(width - lastSize.width) <= 1 && Math.abs(height - lastSize.height) <= 1) return;
    // Only remember a size the app has actually been told.
    if (!client.connected) return;
    lastSize = { width, height };
    client.send({ op: 'autoSize', id, width, height });
    // A window fitted to almost nothing looks as if the overlay was switched off (not worth a
    // line while it starts, before its data has come).
    if (height >= 40) everBig = true;
    if (height > 0 && height < 40 && !tinyLogged && everBig) {
      tinyLogged = true;
      logLine(`window fitted to its content: ${Math.round(width)} x ${Math.round(height)} px (nothing much to show)`);
    } else if (height >= 40) tinyLogged = false;
  };
  if (!autoH && !autoW) {
    measure = null;
    if (lastSize.width > 0 || lastSize.height > 0) send(0, 0);
    return;
  }
  /** The width the content needs, at least `set`: measured at the width set. */
  const neededWidth = (set: number) => {
    const before = panel.style.cssText;
    panel.style.right = 'auto';
    panel.style.width = `${set}px`;
    const need = Math.max(set, Math.ceil(panel.scrollWidth));
    panel.style.cssText = before;
    return need;
  };
  measure = () => {
    const r = panel.getBoundingClientRect();
    if (r.width <= 0 || r.height <= 0) return;
    let width = 0;
    if (autoW) width = Math.ceil(r.width);
    else if (growW && inst) {
      const need = neededWidth(inst.width);
      if (need > inst.width + 1) {
        width = need;
        grown = true;
      } else if (grown) {
        // Fits again: back to the width set (then the width is the user's again).
        width = inst.width;
        grown = false;
      }
    }
    send(width, autoH ? Math.ceil(panel.getBoundingClientRect().height) : 0);
  };
  sizer = new ResizeObserver(() => measure?.());
  sizer.observe(panel);
}

/** Content can get wider without the panel changing size (a longer name, a new column):
 *  check now and then while the window is fitted to the content. */
let measureTimer = 0;
function measureSoon() {
  if (!measure || measureTimer) return;
  measureTimer = window.setTimeout(() => {
    measureTimer = 0;
    measure?.();
  }, 800);
}

function updateVisibility() {
  const editing = !!status?.editMode && !preview;
  document.body.classList.toggle('editing', editing);
  show(edit, editing);
  let visible = true;
  if (!preview && status && !status.visible) visible = false;
  if (settings.hideWhenDisconnected && !(session?.connected ?? false)) visible = false;
  if (settings.hideWhenNotDriving && !player?.onTrack) visible = false;
  if (settings.hideInPits && player?.onPitRoad) visible = false;
  if (settings.raceOnly && !session?.isRace) visible = false;
  if (!client.connected) visible = false;
  // Only for the other game: hidden, even while moving overlays.
  const src = status?.source;
  const wrongGame = !preview && !!def?.game && (src === 'iracing' || src === 'lmu') && src !== def.game;
  const hidden = wrongGame || !(visible || editing || preview);
  // Why it is hidden, for the log when that changes.
  if (status && def && hidden !== lastHidden) {
    const why = [
      !status.visible && 'all overlays hidden (show / hide)',
      settings.hideWhenDisconnected && !(session?.connected ?? false) && 'no game connected',
      settings.hideWhenNotDriving && !player?.onTrack && 'not in the car (Hide unless I am in the car)',
      settings.hideInPits && player?.onPitRoad && 'on pit road (Hide while on pit road)',
      settings.raceOnly && !session?.isRace && 'not a race (races only)',
      !client.connected && 'no connection to the app',
      wrongGame && `for the other game (${def.game})`,
    ].filter(Boolean);
    // Hidden before anything is known (starting up) is not worth a line.
    if (lastHidden !== null) logLine(hidden ? `hidden: ${why.join(', ') || 'unknown reason'}` : 'shown');
    lastHidden = hidden;
  }
  document.body.classList.toggle('hidden', hidden);
  if (!document.body.classList.contains('hidden') && stale.size) {
    for (const t of stale) if (client.latest[t] !== undefined) deliver(t, client.latest[t]);
    stale.clear();
  }
}
