// Minimal DOM helpers. Overlays update text/attributes only when values
// change, so a 60 Hz feed costs almost nothing in layout and paint.

export function h<K extends keyof HTMLElementTagNameMap>(tag: K, cls = '', text = ''): HTMLElementTagNameMap[K] {
  const el = document.createElement(tag);
  if (cls) el.className = cls;
  if (text) el.textContent = text;
  return el;
}

export function text(el: HTMLElement, value: string) {
  // An element holding just text: compare and change that text node. Reading textContent
  // builds a new string every time (for every cell of every table, many times a second), and
  // setting it replaces the node.
  const t = el.firstChild;
  if (t && t === el.lastChild && t.nodeType === Node.TEXT_NODE) {
    if ((t as Text).data !== value) (t as Text).data = value;
    return;
  }
  if (el.textContent !== value) el.textContent = value;
}

export function toggle(el: Element, cls: string, on: boolean) {
  if (el.classList.contains(cls) !== on) el.classList.toggle(cls, on);
}

export function style(el: HTMLElement, prop: string, value: string) {
  if (el.style.getPropertyValue(prop) !== value) el.style.setProperty(prop, value);
}

export function show(el: HTMLElement, visible: boolean) {
  const d = visible ? '' : 'none';
  if (el.style.display !== d) el.style.display = d;
}

/** Canvas sized to its CSS box at device pixel ratio. Returns true if resized. */
/** Canvas sizes kept up to date by a ResizeObserver: reading clientWidth every frame would
 *  force a layout in the middle of drawing. */
const sizes = new WeakMap<Element, { w: number; h: number }>();
const sizer = new ResizeObserver((entries) => {
  for (const e of entries) sizes.set(e.target, { w: e.contentRect.width, h: e.contentRect.height });
});

export function fitCanvas(c: HTMLCanvasElement): boolean {
  const dpr = window.devicePixelRatio || 1;
  let size = sizes.get(c);
  if (!size) {
    size = { w: c.clientWidth, h: c.clientHeight };
    sizes.set(c, size);
    sizer.observe(c);
  }
  const w = Math.max(1, Math.round(size.w * dpr));
  const hgt = Math.max(1, Math.round(size.h * dpr));
  if (c.width !== w || c.height !== hgt) {
    c.width = w;
    c.height = hgt;
    return true;
  }
  return false;
}
