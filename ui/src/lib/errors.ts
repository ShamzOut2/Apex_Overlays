// Sends a page's script errors to the app's log file (apex.log), so a problem in an overlay
// or the settings can be looked into from the log alone.

import type { Client } from './client';

/** Text for anything thrown or logged: the stack when there is one. */
function describe(v: unknown): string {
  if (v instanceof Error) return v.stack && v.stack.includes(v.message) ? v.stack : `${v.name}: ${v.message}\n${v.stack ?? ''}`;
  if (typeof v === 'string') return v;
  try {
    return JSON.stringify(v);
  } catch {
    return String(v);
  }
}

/**
 * Reports uncaught errors, failed promises and console.error calls of this page. The same
 * message is sent at most 3 times, and no more than 30 reports a minute, so an error that
 * repeats every frame cannot flood the log.
 */
export function reportErrors(client: Client, source: () => string) {
  const seen = new Map<string, number>();
  let budget = 30;
  setInterval(() => (budget = 30), 60_000);
  const send = (level: 'error' | 'warn', message: string) => {
    const n = (seen.get(message) ?? 0) + 1;
    seen.set(message, n);
    if (n > 3 || budget <= 0) return;
    budget--;
    client.report(level, source(), n === 3 ? `${message} (repeated; not reported again)` : message);
  };
  window.addEventListener('error', (e) => {
    const where = e.filename ? ` (${e.filename.split('/').pop()}:${e.lineno}:${e.colno})` : '';
    send('error', e.error ? describe(e.error) : `${e.message}${where}`);
  });
  window.addEventListener('unhandledrejection', (e) => send('error', `unhandled: ${describe(e.reason)}`));
  const original = console.error.bind(console);
  console.error = (...args: unknown[]) => {
    original(...args);
    send('error', args.map(describe).join(' '));
  };
  const warn = console.warn.bind(console);
  console.warn = (...args: unknown[]) => {
    warn(...args);
    send('warn', args.map(describe).join(' '));
  };
}
