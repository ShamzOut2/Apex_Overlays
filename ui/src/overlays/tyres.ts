import { h, show, text } from '../lib/dom';
import * as f from '../lib/format';
import type { OverlayDef } from '../lib/overlay';
import type { TyreRecommendation, Tyres } from '../lib/types';

const MESSAGES: Record<TyreRecommendation, [string, string]> = {
  stayOut: ['STAY OUT', 'ok'],
  considerWets: ['WETS SOON', 'warn'],
  boxForWets: ['BOX FOR WETS', 'box'],
  considerSlicks: ['SLICKS SOON', 'warn'],
  boxForSlicks: ['BOX FOR SLICKS', 'box'],
  noWets: ['NO WET TYRES', 'ok'],
};

export const tyres: OverlayDef = {
  kind: 'tyres',
  name: 'Tyre advisor',
  description: 'Tells you when to switch between slicks and wets: track wetness, rain trend, what the rest of your class is running and the wet/dry lap time crossover.',
  topics: ['tyres'],
  size: [300, 150],
  settings: [
    { key: 'showGauge', label: 'Wetness gauge', type: 'bool', default: true, group: 'Elements' },
    { key: 'showReasons', label: 'Explain why', type: 'bool', default: true, group: 'Elements' },
    { key: 'maxReasons', when: (v) => v.showReasons !== false, label: 'Reasons shown', type: 'number', default: 3, min: 1, max: 8, group: 'Elements' },
    { key: 'showField', label: 'Field tyre split', type: 'bool', default: true, group: 'Elements' },
    { key: 'showPace', label: 'Wet vs dry pace', type: 'bool', default: true, group: 'Elements' },
    { key: 'flash', label: 'Flash when it is time to box', type: 'bool', default: true, group: 'Alerts' },
    { key: 'onlyWhenRelevant', label: 'Hide while the track is fully dry and you are on slicks', type: 'bool', default: false, group: 'Alerts' },
  ],
  create(root, ctx) {
    const s = ctx.settings;
    root.classList.add('tyres');
    const banner = h('div', 'banner');
    const bannerText = h('span', 'msg');
    const current = h('span', 'current');
    banner.append(bannerText, current);
    const gauge = h('div', 'wet-gauge');
    const segs = Array.from({ length: 7 }, (_, i) => {
      const seg = h('div', `seg s${i + 1}`);
      gauge.append(seg);
      return seg;
    });
    const gaugeLabel = h('div', 'wet-label');
    const stats = h('div', 'stats dim');
    const reasons = h('ul', 'reasons');
    show(gauge, s.showGauge as boolean);
    show(gaugeLabel, s.showGauge as boolean);
    show(reasons, s.showReasons as boolean);
    root.append(banner, gauge, gaugeLabel, stats, reasons);

    return {
      update(topic, data) {
        if (topic !== 'tyres') return;
        const d = data as Tyres;
        const [msg, level] = MESSAGES[d.recommendation];
        text(bannerText, msg);
        banner.className = `banner ${level}${s.flash && level === 'box' ? ' flash' : ''}`;
        text(current, d.current === 'unknown' ? '' : `on ${d.current === 'wet' ? 'WETS' : 'SLICKS'}`);
        segs.forEach((seg, i) => seg.classList.toggle('on', i < d.wetness));
        const arrow = d.trend === 'wetting' ? ' ↑ wetter' : d.trend === 'drying' ? ' ↓ drying' : '';
        text(gaugeLabel, `${d.wetnessLabel}${d.precipitation > 0 ? ` · rain ${Math.round(d.precipitation * 100)}%` : ''}${arrow}`);
        const parts: string[] = [];
        if (s.showField && d.fieldTotal) parts.push(`${d.fieldOnWets}/${d.fieldTotal} on wets`);
        if (s.showPace && d.wetPace != null && d.dryPace != null) parts.push(`wet ${f.lapTime(d.wetPace)} · dry ${f.lapTime(d.dryPace)}`);
        text(stats, parts.join('  ·  '));
        show(stats, parts.length > 0);
        if (s.showReasons) {
          const list = d.reasons.slice(0, s.maxReasons as number);
          while (reasons.children.length < list.length) reasons.append(h('li'));
          [...reasons.children].forEach((li, i) => {
            show(li as HTMLElement, i < list.length);
            if (i < list.length) text(li as HTMLElement, list[i]);
          });
        }
        const hidden = !!s.onlyWhenRelevant && d.wetness <= 1 && d.current !== 'wet' && d.recommendation === 'stayOut';
        root.classList.toggle('auto-hidden', hidden);
      },
    };
  },
};
