import { h, show, style, text, toggle } from '../lib/dom';
import * as f from '../lib/format';
import type { OverlayDef } from '../lib/overlay';
import type { CarInfo, Player } from '../lib/types';
import { cornerSpeeds } from './cornerSpeeds';
import { inputTrace, TRACE_SETTINGS } from './inputTrace';
import { shiftLightSettings, shiftLights } from './shiftLights';

/** The original outline wheel (Wheel style: Simple). */
const WHEEL_SIMPLE = `<svg viewBox="-50 -50 100 100"><circle r="42" fill="none" stroke="currentColor" stroke-width="9"/><path d="M-40 4 H-12 L-6 14 H6 L12 4 H40" fill="none" stroke="currentColor" stroke-width="8" stroke-linejoin="round"/><path d="M0 14 V40" stroke="currentColor" stroke-width="8"/><rect x="-3" y="-50" width="6" height="12" fill="var(--wheel-mark)"/></svg>`;

/** A sim racing wheel (Wheel style: Detailed): rim with textured grips, three spokes, a hub
 *  with a small display and buttons, and the top-centre marker. The rim and spokes take the
 *  Wheel colour, the marker its own. */
const WHEEL_DETAILED = `<svg viewBox="-50 -50 100 100">
<circle r="41" fill="none" stroke="currentColor" stroke-width="8"/>
<path d="M-38.6-14A41 41 0 0 0-38.6 14M38.6-14A41 41 0 0 1 38.6 14" fill="none" stroke="#000" stroke-opacity=".38" stroke-width="8.6" stroke-dasharray="1 1.3"/>
<circle r="36.6" fill="none" stroke="#000" stroke-opacity=".28" stroke-width="1"/>
<circle r="45.2" fill="none" stroke="#000" stroke-opacity=".22" stroke-width="0.8"/>
<path d="M-37-1-15-5-13 9-36 9ZM37-1 15-5 13 9 36 9ZM-7 15H7L5 37.5H-5Z" fill="currentColor" fill-opacity=".9"/>
<rect x="-17" y="-13" width="34" height="30" rx="9" fill="#14171d" stroke="currentColor" stroke-width="2.4"/>
<rect x="-7" y="-9" width="14" height="6" rx="1.5" fill="#2a2f38"/>
<circle cx="-10" cy="1" r="2.4" fill="#ff4d4d"/><circle cx="10" cy="1" r="2.4" fill="#4da3ff"/><circle cx="-10" cy="9" r="2.4" fill="#3ddc84"/><circle cx="10" cy="9" r="2.4" fill="#ffd400"/>
<circle cx="0" cy="7" r="3.2" fill="none" stroke="currentColor" stroke-opacity=".7" stroke-width="1.4"/>
<rect x="-2.8" y="-49.6" width="5.6" height="9.5" rx="1" fill="var(--wheel-mark)"/>
</svg>`;

/** Orders of the wheel, the speeds and the gear (Layout → Order). */
const ORDERS: [string, string][] = [
  ['wheel-speeds-gear', 'Wheel · speeds · gear'],
  ['wheel-gear-speeds', 'Wheel · gear · speeds'],
  ['gear-wheel-speeds', 'Gear · wheel · speeds'],
  ['speeds-wheel-gear', 'Speeds · wheel · gear'],
  ['gear-speeds-wheel', 'Gear · speeds · wheel'],
  ['speeds-gear-wheel', 'Speeds · gear · wheel'],
];

export const inputs: OverlayDef = {
  kind: 'inputs',
  fill: true,
  name: 'Driver inputs',
  description: 'Throttle, brake and clutch bars with a scrolling input trace, steering wheel, gear, speed and RPM. Updates every telemetry tick.',
  topics: ['player', 'session', 'car'],
  size: [420, 140],
  regions: [['.shift-lights', 'showShiftLights'], ['.rpm', 'showRpm'], ['.trace', 'traceSeconds'], ['.bars', 'showBars'], ['.wheel-block', 'wheelStyle'], ['.speeds', 'showCornerSpeeds'], ['.gear', 'gearColor']],
  settings: [
    {
      key: 'sideLayout',
      label: 'Wheel, speeds and gear',
      type: 'select',
      default: 'row',
      options: [
        ['row', 'Side by side'],
        ['column', 'Stacked (one above the other)'],
      ],
      group: 'Layout',
    },
    { key: 'sideOrder', label: 'Order', type: 'select', default: 'wheel-speeds-gear', options: ORDERS, group: 'Layout', help: 'Left to right side by side, top to bottom when stacked' },
    {
      key: 'sidePlacement',
      label: 'They go',
      type: 'select',
      default: 'right',
      options: [
        ['right', 'Right of the pedals'],
        ['left', 'Left of the pedals'],
      ],
      group: 'Layout',
    },
    {
      key: 'wheelStyle', when: (v) => v.showWheel !== false,
      label: 'Wheel style',
      type: 'select',
      default: 'photo',
      options: [
        ['photo', 'Formula-style wheel (photo)'],
        ['detailed', 'Drawn round wheel (rim, spokes, hub)'],
        ['simple', 'Simple outline'],
      ],
      group: 'Layout',
      help: 'The drawn wheels take the Wheel colours',
    },
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
      default: true,
      group: 'Corner speeds',
      help: 'In boxes with your speed: above it the top speed going into the last corner (stored when you brake), below it the lowest speed through it (stored when you are back on the throttle)',
    },
    { key: 'cornerBrakeAt', when: (v) => !!v.showCornerSpeeds, label: 'A corner starts at brake', type: 'number', default: 10, min: 1, max: 80, unit: '%', group: 'Corner speeds', help: 'Lighter touches of the brake are not counted' },
    { key: 'cornerThrottleAt', when: (v) => !!v.showCornerSpeeds, label: 'It ends at throttle', type: 'number', default: 20, min: 1, max: 100, unit: '%', group: 'Corner speeds', help: 'With the brake released' },
    { key: 'showRpm', label: 'RPM', type: 'bool', default: false, group: 'Elements', help: 'Engine RPM under the shift lights. It turns the “shift now” colour at the car’s shift point' },
    { key: 'showSteerAngle', when: (v) => v.showWheel !== false, label: 'Steering angle', type: 'bool', default: true, group: 'Elements', help: 'The wheel’s angle in degrees above the steering wheel, with L or R for the direction' },
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
    { key: 'wheelColor', when: (v) => v.showWheel !== false && v.wheelStyle !== 'photo', label: 'Wheel', type: 'color', default: '#e6e9ef', group: 'Colours' },
    { key: 'wheelMarkColor', when: (v) => v.showWheel !== false && v.wheelStyle !== 'photo', label: 'Wheel top marker', type: 'color', default: '#ffd400', group: 'Colours' },
    { key: 'gearColor', when: (v) => v.showGear !== false, label: 'Gear', type: 'color', default: '#ffd400', group: 'Colours' },
    { key: 'speedColor', when: (v) => v.showSpeed !== false, label: 'Speed box', type: 'color', default: '#e6e9ef', group: 'Colours' },
    { key: 'cornerMaxColor', when: (v) => !!v.showCornerSpeeds, label: 'Top speed into the corner', type: 'color', default: '#ff8a00', group: 'Colours' },
    { key: 'cornerMinColor', when: (v) => !!v.showCornerSpeeds, label: 'Lowest speed through it', type: 'color', default: '#3ddc84', group: 'Colours' },
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
    // The wheel (with its angle above it), the three speeds stacked (top speed into the last
    // corner, your speed, the lowest through it) and the gear: side by side or stacked, in the
    // order chosen, right or left of the pedals.
    const side = h('div', `side ${s.sideLayout === 'column' ? 'col' : 'row'}`);
    const wheelBlock = h('div', 'wheel-block');
    const wheel = h('div', 'wheel');
    // The photo wheel is wider than tall: it sits in the middle of the square it turns in,
    // as wide as fits that square at any angle (the square's side over its diagonal).
    const photo = (s.wheelStyle ?? 'photo') === 'photo';
    if (photo) {
      const img = h('img', 'wheel-photo');
      img.src = '/assets/wheels/wheel.png';
      img.alt = '';
      img.draggable = false;
      img.style.width = `${100 / Math.hypot(1, 271 / 520)}%`;
      wheel.append(img);
      wheel.classList.add('photo');
    } else wheel.innerHTML = s.wheelStyle === 'simple' ? WHEEL_SIMPLE : WHEEL_DETAILED;
    wheel.style.color = s.wheelColor as string;
    root.style.setProperty('--wheel-mark', s.wheelMarkColor as string);
    const angle = h('div', 'steer-angle', '0°');
    wheelBlock.append(angle, wheel);
    const gear = h('div', 'gear');
    gear.style.color = s.gearColor as string;
    const cornerOn = !!s.showCornerSpeeds;
    const speeds = h('div', 'speeds');
    const box = (cls: string, color: string, title: string) => {
      const b = h('div', `spd-box ${cls}`);
      b.style.setProperty('--box', color);
      b.title = title;
      return b;
    };
    const cMax = box('c-max', s.cornerMaxColor as string, 'Top speed going into the last corner (stored when you braked)');
    const spd = box('speed', s.speedColor as string, 'Your speed');
    const cMin = box('c-min', s.cornerMinColor as string, 'Lowest speed through the last corner (stored when you were back on the throttle)');
    speeds.append(cMax, spd, cMin);
    show(cMax, cornerOn);
    show(cMin, cornerOn);
    const parts: Record<string, HTMLElement> = { wheel: wheelBlock, speeds, gear };
    const order = String(s.sideOrder ?? 'wheel-speeds-gear').split('-').filter((k) => parts[k]);
    side.append(...(order.length === 3 ? order : ['wheel', 'speeds', 'gear']).map((k) => parts[k]));
    const corners = cornerSpeeds({ brakeOn: (s.cornerBrakeAt as number) / 100, throttleOn: (s.cornerThrottleAt as number) / 100 });
    show(canvas, s.showTrace as boolean);
    show(bars, s.showBars as boolean);
    show(wheelBlock, s.showWheel as boolean);
    show(gear, s.showGear as boolean);
    show(spd, s.showSpeed as boolean);
    show(speeds, !!s.showSpeed || cornerOn);
    show(rpm, !!s.showRpm);
    const showAngle = !!s.showSteerAngle && s.showWheel !== false;
    show(angle, showAngle);
    show(side, (s.showWheel || s.showGear || s.showSpeed || cornerOn) as boolean);
    if (s.sidePlacement === 'left') main.append(side, canvas, bars);
    else main.append(canvas, bars, side);
    root.append(lights, rpm, main);

    // The wheel takes the room the overlay leaves it, at any steering angle: it turns inside a
    // square, so a wheel wider than tall (the photo) is sized for its turning circle (its
    // diagonal) and never runs off the overlay or over its angle line when turned. Side by
    // side: as tall as the room (less the angle line above it), as wide as the speeds, gear
    // and pedals leave. Stacked: as wide as the column, as tall as the speeds and gear leave.
    // Sized when the overlay is, not on every update.
    if (s.showWheel !== false) {
      const stacked = s.sideLayout === 'column';
      const fit = () => {
        const cs = getComputedStyle(side);
        const gap = parseFloat(stacked ? cs.rowGap : cs.columnGap) || 0;
        // The angle line above it and the gap between them (the angle is given its line
        // from the start, so it does not push the wheel down when the first reading comes).
        const above = showAngle ? angle.offsetHeight + (parseFloat(getComputedStyle(wheelBlock).rowGap) || 0) : 0;
        const others = [...side.children].filter((c) => c !== wheelBlock && (c as HTMLElement).offsetParent) as HTMLElement[];
        let tall: number;
        let wide: number;
        if (stacked) {
          tall = side.clientHeight - above - others.reduce((t, c) => t + c.offsetHeight + gap, 0) - 2;
          wide = side.clientWidth;
        } else {
          tall = side.clientHeight - above - 2;
          const beside = others.reduce((w, c) => w + c.offsetWidth + gap, 0);
          const rest = [...main.children].filter((c) => c !== side && (c as HTMLElement).offsetParent && c !== canvas).reduce((w, c) => w + (c as HTMLElement).offsetWidth + 8, 0);
          // With the trace shown it keeps at least 40% of the width.
          wide = main.clientWidth * (s.showTrace ? 0.6 : 1) - beside - rest - 8;
        }
        const size = Math.max(8, Math.floor(Math.min(tall, wide)));
        if (wheel.style.width !== `${size}px`) {
          wheel.style.width = `${size}px`;
          wheel.style.height = `${size}px`;
        }
      };
      // Fitted again when the overlay, or the angle line, changes size, and once the fonts
      // are in (they change how tall the angle line is).
      const ro = new ResizeObserver(fit);
      ro.observe(main);
      if (showAngle) ro.observe(angle);
      requestAnimationFrame(fit);
      void document.fonts?.ready.then(fit);
    }

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
          text(cMax, v(c.max));
          text(cMin, v(c.min));
          // While in a corner the last minimum is the previous corner's: shown dimmer.
          toggle(cMin, 'stale', c.inCorner);
        }
        if (s.showTrace) trace.push(p);
      },
    };
  },
};
