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

  return {
    /** Record one telemetry tick and redraw. */
    push(p: Player) {
      thr[head] = p.throttle;
      brk[head] = p.brake;
      clt[head] = p.clutch;
      ste[head] = p.steeringMax ? 0.5 - p.steering / p.steeringMax / 2 : 0.5;
      abs[head] = p.abs ? 1 : 0;
      head = (head + 1) % n;
      fitCanvas(canvas);
      g.clearRect(0, 0, canvas.width, canvas.height);
      if (s.traceSteering) series(ste, s.steeringColor as string, false);
      if (clutch()) series(clt, s.clutchColor as string, false);
      series(thr, s.throttleColor as string, s.traceFill as boolean);
      series(brk, s.brakeColor as string, s.traceFill as boolean, abs);
    },
  };
}
