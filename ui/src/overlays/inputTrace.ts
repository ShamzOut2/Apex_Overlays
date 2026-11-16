// The scrolling input trace (throttle, brake, clutch, steering, ABS), shared by the Driver
// inputs overlay and the stand-alone Input graph overlay.

import { fitCanvas } from '../lib/dom';
import type { SettingDef, Values } from '../lib/settings';
import type { Player } from '../lib/types';

/** Trace options, the same in both overlays. */
export const TRACE_SETTINGS: SettingDef[] = [
  { key: 'traceSeconds', when: (v) => v.showTrace !== false, label: 'Trace length', type: 'number', default: 6, min: 1, max: 30, unit: 's', group: 'Trace' },
  { key: 'traceWidth', when: (v) => v.showTrace !== false, label: 'Trace line width', type: 'number', default: 2, min: 0.5, max: 6, step: 0.5, unit: 'px', group: 'Trace' },
  { key: 'traceSteering', when: (v) => v.showTrace !== false, label: 'Show steering in trace', type: 'bool', default: false, group: 'Trace' },
  { key: 'traceFill', when: (v) => v.showTrace !== false, label: 'Fill under throttle/brake', type: 'bool', default: true, group: 'Trace' },
  { key: 'absHighlight', label: 'Colour the brake trace and bar while ABS is active', type: 'bool', default: true, group: 'Trace' },
];

/** A trace drawn on `canvas`: feed it every player update, it redraws itself. */
export function inputTrace(canvas: HTMLCanvasElement, s: Values, clutch: () => boolean) {
  const n = Math.round((s.traceSeconds as number) * 60);
  const thr = new Float32Array(n);
  const brk = new Float32Array(n);
  const clt = new Float32Array(n);
  const ste = new Float32Array(n);
  const abs = new Uint8Array(n);
  let head = 0;
  const g = canvas.getContext('2d')!;

  const series = (arr: Float32Array, color: string, fill: boolean, absArr?: Uint8Array) => {
    const w = canvas.width;
    const hh = canvas.height;
    const pad = 2 * (window.devicePixelRatio || 1);
    const y = (v: number) => hh - pad - v * (hh - pad * 2);
    const x = (i: number) => (i / (n - 1)) * w;
    g.beginPath();
    for (let i = 0; i < n; i++) {
      const v = arr[(head + i) % n];
      if (i === 0) g.moveTo(x(i), y(v));
      else g.lineTo(x(i), y(v));
    }
    g.strokeStyle = color;
    g.lineWidth = (s.traceWidth as number) * (window.devicePixelRatio || 1);
    g.stroke();
    if (fill) {
      g.lineTo(w, hh);
      g.lineTo(0, hh);
      g.closePath();
      g.globalAlpha = 0.15;
      g.fillStyle = color;
      g.fill();
      g.globalAlpha = 1;
    }
    if (absArr && s.absHighlight) {
      // Each stretch with ABS working: the line thicker and the area under it filled in the
      // ABS colour, from where it kicked in to where it stopped.
      const absColor = s.absColor as string;
      const width = (s.traceWidth as number) * (window.devicePixelRatio || 1) * 1.7;
      for (let i = 0; i < n; i++) {
        if (!absArr[(head + i) % n]) continue;
        const start = i;
        while (i + 1 < n && absArr[(head + i + 1) % n]) i++;
        const end = i;
        const from = Math.max(0, start - 1);
        g.beginPath();
        for (let k = from; k <= end; k++) {
          const v = arr[(head + k) % n];
          if (k === from) g.moveTo(x(k), y(v));
          else g.lineTo(x(k), y(v));
        }
        g.strokeStyle = absColor;
        g.lineWidth = width;
        g.lineJoin = 'round';
        g.stroke();
        g.lineTo(x(end), hh);
        g.lineTo(x(from), hh);
        g.closePath();
        g.globalAlpha = 0.4;
        g.fillStyle = absColor;
        g.fill();
        g.globalAlpha = 1;
      }
    }
  };

  const traces = () => {
    const list: [Float32Array, string, boolean, Uint8Array?][] = [];
    if (s.traceSteering) list.push([ste, s.steeringColor as string, false]);
    if (clutch()) list.push([clt, s.clutchColor as string, false]);
    list.push([thr, s.throttleColor as string, s.traceFill as boolean]);
    list.push([brk, s.brakeColor as string, s.traceFill as boolean, abs]);
    return list;
  };

  /** Where the newest point was drawn (canvas pixels), for drawing on from it. */
  let lastX = -1;
  /** Where the shading under the lines has been drawn to: whole pixels, so the slivers meet
   * exactly (soft edges side by side would show as stripes). */
  let fillX = -1;
  let filled = 0;

  /** Everything, from the stored history (first time, and when the canvas is resized). */
  const redraw = () => {
    g.clearRect(0, 0, canvas.width, canvas.height);
    for (const [arr, color, fill, absArr] of traces()) series(arr, color, fill, absArr);
    lastX = canvas.width;
    fillX = canvas.width;
  };

  /**
   * Only the newest stretch: the picture so far moves left by whole pixels (copied, no
   * redrawing) and the new bit of each line is drawn at the right. Points land exactly where a
   * full redraw would put them; drawing a few pixels instead of every line's whole length
   * each tick is several times lighter.
   */
  const step = () => {
    const w = canvas.width;
    const hh = canvas.height;
    const dpr = window.devicePixelRatio || 1;
    const pad = 2 * dpr;
    const dx = w / (n - 1);
    const y = (v: number) => hh - pad - v * (hh - pad * 2);
    let from = lastX;
    let to = lastX + dx;
    const shift = Math.ceil(to - w);
    if (shift > 0) {
      g.globalCompositeOperation = 'copy';
      g.drawImage(canvas, -shift, 0);
      g.globalCompositeOperation = 'source-over';
      g.clearRect(w - shift, 0, shift, hh);
      from -= shift;
      to -= shift;
      fillX -= shift;
    }
    lastX = to;
    // The shading goes on in whole pixel columns, from where it stopped.
    const fx0 = fillX;
    const fx1 = Math.min(w, Math.round(to));
    if (fx1 > fx0) fillX = fx1;
    const at = (x: number, y0: number, y1: number) => (to > from ? y0 + ((y1 - y0) * Math.min(1, Math.max(0, (x - from) / (to - from)))) : y1);
    const cur = (head + n - 1) % n;
    const prev = (head + n - 2) % n;
    const width = (s.traceWidth as number) * dpr;
    g.lineCap = 'round';
    for (const [arr, color, fill, absArr] of traces()) {
      const y0 = y(arr[prev]);
      const y1 = y(arr[cur]);
      const hot = !!absArr && !!s.absHighlight && absArr[cur] === 1;
      // As a full redraw: the line's own shading, then the ABS colour over it.
      for (const [on, alpha, c] of [[fill, 0.15, color], [hot, 0.4, s.absColor as string]] as [boolean, number, string][]) {
        if (!on || fx1 <= fx0) continue;
        g.beginPath();
        g.moveTo(fx0, at(fx0, y0, y1));
        g.lineTo(fx1, at(fx1, y0, y1));
        g.lineTo(fx1, hh);
        g.lineTo(fx0, hh);
        g.closePath();
        g.globalAlpha = alpha;
        g.fillStyle = c;
        g.fill();
        g.globalAlpha = 1;
      }
      g.beginPath();
      g.moveTo(from, y0);
      g.lineTo(to, y1);
      g.strokeStyle = hot ? (s.absColor as string) : color;
      g.lineWidth = hot ? width * 1.7 : width;
      g.stroke();
    }
    g.lineCap = 'butt';
  };

  return {
    /** Record one telemetry tick and draw it. */
    push(p: Player) {
      thr[head] = p.throttle;
      brk[head] = p.brake;
      clt[head] = p.clutch;
      ste[head] = p.steeringMax ? 0.5 - p.steering / p.steeringMax / 2 : 0.5;
      abs[head] = p.abs ? 1 : 0;
      head = (head + 1) % n;
      filled = Math.min(n, filled + 1);
      if (fitCanvas(canvas) || lastX < 0 || filled < 2) redraw();
      else step();
    },
    /** Draws the whole trace again from its history. */
    redraw,
  };
}
