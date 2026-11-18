import { h, show, style, text, toggle } from '../lib/dom';
import * as f from '../lib/format';
import type { OverlayDef } from '../lib/overlay';
import type { CarInfo, Player } from '../lib/types';
import { cornerSpeeds } from './cornerSpeeds';
import { inputTrace, TRACE_SETTINGS } from './inputTrace';
import { shiftLightSettings, shiftLights } from './shiftLights';

const WHEEL_SVG = `<svg viewBox="-50 -50 100 100"><circle r="42" fill="none" stroke="currentColor" stroke-width="9"/><path d="M-40 4 H-12 L-6 14 H6 L12 4 H40" fill="none" stroke="currentColor" stroke-width="8" stroke-linejoin="round"/><path d="M0 14 V40" stroke="currentColor" stroke-width="8"/><rect x="-3" y="-50" width="6" height="12" fill="var(--accent)"/></svg>`;

export const inputs: OverlayDef = {
  kind: 'inputs',
  fill: true,
  name: 'Driver inputs',
  description: 'Throttle, brake and clutch bars with a scrolling input trace, steering wheel, gear, speed and RPM. Updates every telemetry tick.',
  topics: ['player', 'session', 'car'],
  size: [420, 140],
  settings: [
    { key: 'showShiftLights', label: 'Shift lights', type: 'bool', default: true, group: 'Shift lights', help: 'Per-gear shift points from Lovely Car Data when the car is listed, otherwise iRacing’s own' },
    ...shiftLightSettings('Shift lights', (v) => v.showShiftLights !== false),
    {
      key: 'showTrace',
      label: 'Input trace graph',
      type: 'bool',
      default: true,
      group: 'Elements',
      help: 'To place the graph on its own, switch this off and turn on the Input graph overlay',
    },
    { key: 'showBars', label: 'Pedal bars', type: 'bool', default: true, group: 'Elements' },
    { key: 'showClutch', label: 'Clutch', type: 'bool', default: true, group: 'Elements' },
    { key: 'showWheel', label: 'Steering wheel', type: 'bool', default: true, group: 'Elements' },
    { key: 'showGear', label: 'Gear', type: 'bool', default: true, group: 'Elements' },
    { key: 'showSpeed', label: 'Speed', type: 'bool', default: true, group: 'Elements' },
    {
      key: 'showCornerSpeeds',
      label: 'Corner speeds (max in, min through)',
      type: 'bool',
      default: false,
      group: 'Corner speeds',
      help: 'Above your speed: the top speed going into the last corner, stored when you brake. Below it: the lowest speed through it, stored when you are back on the throttle',
    },
    { key: 'cornerBrakeAt', when: (v) => !!v.showCornerSpeeds, label: 'A corner starts at brake', type: 'number', default: 10, min: 1, max: 80, unit: '%', group: 'Corner speeds', help: 'Lighter touches of the brake are not counted' },
    { key: 'cornerThrottleAt', when: (v) => !!v.showCornerSpeeds, label: 'It ends at throttle', type: 'number', default: 20, min: 1, max: 100, unit: '%', group: 'Corner speeds', help: 'With the brake released' },
    { key: 'showRpm', label: 'RPM', type: 'bool', default: false, group: 'Elements', help: 'Engine RPM under the shift lights. It turns the “shift now” colour at the car’s shift point' },
    { key: 'showSteerAngle', when: (v) => v.showWheel !== false, label: 'Steering angle', type: 'bool', default: false, group: 'Elements', help: 'The wheel’s angle in degrees above the steering wheel, with L or R for the direction' },
    {
      key: 'valuePosition', when: (v) => v.showBars !== false,
      label: 'Percent values',
      type: 'select',
      default: 'top',
      options: [['top', 'Above each bar'], ['inside', 'Inside the bar'], ['off', 'Off']],
      group: 'Elements',
    },
    { key: 'valueColored', when: (v) => v.showBars !== false && v.valuePosition === 'top', label: 'Colour values like their bar', type: 'bool', default: true, group: 'Elements' },
    ...TRACE_SETTINGS,
    { key: 'throttleColor', label: 'Throttle', type: 'color', default: '#3ddc84', group: 'Colours' },
    { key: 'brakeColor', label: 'Brake', type: 'color', default: '#ff4d4d', group: 'Colours' },
    { key: 'clutchColor', when: (v) => v.showClutch !== false, label: 'Clutch', type: 'color', default: '#4da3ff', group: 'Colours' },
    { key: 'absColor', when: (v) => v.absHighlight !== false, label: 'ABS', type: 'color', default: '#ffd84d', group: 'Colours' },
    { key: 'steeringColor', when: (v) => v.traceSteering === true && v.showTrace !== false, label: 'Steering', type: 'color', default: '#e6e9ef', group: 'Colours' },
    { key: 'wheelColor', when: (v) => v.showWheel !== false, label: 'Wheel', type: 'color', default: '#e6e9ef', group: 'Colours' },
  ],
  create(root, ctx) {
    const s = ctx.settings;
    root.classList.add('inputs');
    root.style.setProperty('--abs-color', s.absColor as string);
    // The shift lights and RPM readout (shared with the Shift lights overlay).
    const sl = shiftLights(s, () => !!s.showShiftLights, () => !!s.showRpm);
    const lights = sl.lights;
    const rpm = sl.rpm;
    const main = h('div', 'inputs-main');
    const canvas = h('canvas', 'trace');
    const bars = h('div', 'bars');
    const valuePos = s.valuePosition as string;
    const mk = (cls: string, color: string) => {
      const wrap = h('div', `barcol ${cls}`);
      const bar = h('div', 'bar');
      const fill = h('div', 'fill');
      const val = h('div', `val ${valuePos}`);
      fill.style.background = color;
      if (s.valueColored && valuePos === 'top') val.style.color = color;
      bar.append(fill);
      if (valuePos === 'top') wrap.append(val, bar);
      else if (valuePos === 'inside') {
        bar.append(val);
        wrap.append(bar);
      } else wrap.append(bar);
      bars.append(wrap);
      return { wrap, fill, val };
    };
    const clutch = mk('clutch', s.clutchColor as string);
    const brake = mk('brake', s.brakeColor as string);
    const throttle = mk('throttle', s.throttleColor as string);
    show(clutch.wrap, s.showClutch as boolean);
    const side = h('div', 'side');
    const wheel = h('div', 'wheel');
    wheel.innerHTML = WHEEL_SVG;
    wheel.style.color = s.wheelColor as string;
    const gear = h('div', 'gear');
    const spd = h('div', 'speed');
    const angle = h('div', 'steer-angle');
    // Corner speeds: the top speed into the last corner above the speed, the lowest through it below.
    const cornerOn = !!s.showCornerSpeeds;
    const cMax = h('div', 'corner-speed c-max');
    const cMin = h('div', 'corner-speed c-min');
    cMax.title = 'Top speed going into the last corner (stored when you braked)';
    cMin.title = 'Lowest speed through the last corner (stored when you were back on the throttle)';
    cMax.style.color = s.brakeColor as string;
    cMin.style.color = s.throttleColor as string;
    side.append(angle, wheel, gear, cMax, spd, cMin);
    show(cMax, cornerOn);
    show(cMin, cornerOn);
    const corners = cornerSpeeds({ brakeOn: (s.cornerBrakeAt as number) / 100, throttleOn: (s.cornerThrottleAt as number) / 100 });
    show(canvas, s.showTrace as boolean);
    show(bars, s.showBars as boolean);
    show(wheel, s.showWheel as boolean);
    show(gear, s.showGear as boolean);
    show(spd, s.showSpeed as boolean);
    show(rpm, !!s.showRpm);
    const showAngle = !!s.showSteerAngle && s.showWheel !== false;
    show(angle, showAngle);
    show(side, (s.showWheel || s.showGear || s.showSpeed || cornerOn) as boolean);
    main.append(canvas, bars, side);
    root.append(lights, rpm, main);

    const trace = inputTrace(canvas, s, () => !!s.showClutch);

    return {
      update(topic, data) {
        if (topic === 'car') {
          sl.setCar(data as CarInfo);
          return;
        }
        if (topic !== 'player') return;
        const p = data as Player;
        sl.draw(p);

        const bar = (b: { fill: HTMLElement; val: HTMLElement }, v: number) => {
          style(b.fill, 'transform', `scaleY(${v.toFixed(3)})`);
          if (valuePos !== 'off') text(b.val, String(Math.round(v * 100)));
        };
        bar(throttle, p.throttle);
        bar(brake, p.brake);
        bar(clutch, p.clutch);
        toggle(brake.wrap, 'abs', !!s.absHighlight && p.abs);
        style(wheel, 'transform', `rotate(${(-p.steering * 180) / Math.PI}deg)`);
        if (showAngle) {
          // iRacing's angle is positive to the left.
          const deg = Math.round((p.steering * 180) / Math.PI);
          text(angle, deg === 0 ? '0°' : `${Math.abs(deg)}° ${deg > 0 ? 'L' : 'R'}`);
        }
        text(gear, p.gear === -1 ? 'R' : p.gear === 0 ? 'N' : String(p.gear));
        text(spd, `${Math.round(f.speed(p.speed, ctx.units))}`);
        spd.dataset.unit = f.speedUnit(ctx.units);
        if (cornerOn) {
          const c = corners.push(p.speed, p.throttle, p.brake, p.onTrack);
          const v = (x: number | null) => (x == null ? '–' : String(Math.round(f.speed(x, ctx.units))));
          text(cMax, `▲ ${v(c.max)}`);
          text(cMin, `▼ ${v(c.min)}`);
          // While in a corner the last minimum is the previous corner's: shown dimmer.
          toggle(cMin, 'stale', c.inCorner);
        }
        if (s.showTrace) trace.push(p);
      },
    };
  },
};
