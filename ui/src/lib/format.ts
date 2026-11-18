// Formatting and unit conversion helpers shared by all overlays.

export type Units = 'metric' | 'imperial';

export function lapTime(s: number | null | undefined): string {
  if (s == null || !(s > 0)) return '--:--.---';
  const m = Math.floor(s / 60);
  const rest = s - m * 60;
  return m > 0 ? `${m}:${rest.toFixed(3).padStart(6, '0')}` : rest.toFixed(3);
}

export function duration(s: number | null | undefined): string {
  if (s == null || s < 0) return '--:--';
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = Math.floor(s % 60);
  const mm = String(m).padStart(2, '0');
  const ss = String(sec).padStart(2, '0');
  return h > 0 ? `${h}:${mm}:${ss}` : `${mm}:${ss}`;
}

export function clock(secondsOfDay: number, h24 = true): string {
  const h = Math.floor(secondsOfDay / 3600) % 24;
  const m = Math.floor((secondsOfDay % 3600) / 60);
  if (h24) return `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}`;
  return `${h % 12 || 12}:${String(m).padStart(2, '0')} ${h < 12 ? 'AM' : 'PM'}`;
}

export function signed(v: number | null | undefined, digits = 1): string {
  if (v == null || Number.isNaN(v)) return '';
  const s = Math.abs(v).toFixed(digits);
  return v > 0 ? `+${s}` : v < 0 ? `-${s}` : s;
}

export function gap(v: number | null | undefined, laps = 0, digits = 1): string {
  if (laps > 0) return `+${laps}L`;
  if (v == null) return '';
  return v.toFixed(digits);
}

export const temp = (c: number, u: Units) => (u === 'imperial' ? `${(c * 9) / 5 + 32 | 0}°F` : `${c.toFixed(1)}°C`);
export const speed = (ms: number, u: Units) => (u === 'imperial' ? ms * 2.23694 : ms * 3.6);
export const speedUnit = (u: Units) => (u === 'imperial' ? 'mph' : 'km/h');
export const fuel = (l: number, u: Units) => (u === 'imperial' ? l / 3.78541 : l);
export const fuelUnit = (u: Units) => (u === 'imperial' ? 'gal' : 'L');

export function compass(deg: number): string {
  const dirs = ['N', 'NE', 'E', 'SE', 'S', 'SW', 'W', 'NW'];
  return dirs[Math.round(deg / 45) % 8];
}

/** Driver name in the requested style. */
export function name(full: string, style: string): string {
  const parts = full.trim().split(/\s+/);
  if (parts.length < 2) return full;
  const first = parts[0];
  const last = parts[parts.length - 1];
  switch (style) {
    case 'last':
      return last;
    case 'initialLast':
      return `${first[0]}. ${last}`;
    case 'lastUpper':
      return `${first[0]}. ${last.toUpperCase()}`;
    case 'first':
      return first;
    default:
      return full;
  }
}

export function iRating(ir: number): string {
  return ir >= 1000 ? `${(ir / 1000).toFixed(1)}k` : String(ir);
}

/** Strength of field: in full up to 9999 (1539, 8320), then 10.0k, 12.3k… */
export function sof(v: number): string {
  const n = Math.round(v);
  return n >= 10_000 ? `${(n / 1000).toFixed(1)}k` : String(n);
}
