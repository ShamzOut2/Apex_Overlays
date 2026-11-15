// Fixed-row grid table with user-selectable, re-orderable columns. Rows are
// created once and updated in place.

import { h, show, text, toggle } from './dom';
import { onColor } from './theme';

export interface Col<R> {
  id: string;
  label: string;
  /** CSS grid track size, e.g. '2.4em' or 'minmax(0, 1fr)'. */
  width: string;
  align?: 'left' | 'right' | 'center';
  render(cell: HTMLElement, r: R): void;
}

interface RowEl {
  el: HTMLElement;
  cells: HTMLElement[];
  key: unknown;
}

export class Table<R> {
  readonly el = h('div', 'table');
  private header = h('div', 'row head');
  private headCells: HTMLElement[] = [];
  private body = h('div', 'body');
  private rows: RowEl[] = [];
  private cols: Col<R>[];

  constructor(all: Col<R>[], order: string[], showHeader: boolean) {
    this.cols = order.map((id) => all.find((c) => c.id === id)).filter((c): c is Col<R> => !!c);
    this.el.style.setProperty('--cols', this.cols.map((c) => c.width).join(' '));
    for (const c of this.cols) {
      const cell = h('div', `cell c-${c.id} ${c.align ?? ''}`, c.label);
      this.headCells.push(cell);
      this.header.append(cell);
    }
    show(this.header, showHeader);
    this.el.append(this.header, this.body);
  }

  private ensure(n: number) {
    while (this.rows.length < n) {
      const el = h('div', 'row');
      const cells = this.cols.map((c) => {
        const cell = h('div', `cell c-${c.id} ${c.align ?? ''}`);
        el.append(cell);
        return cell;
      });
      this.body.append(el);
      this.rows.push({ el, cells, key: undefined });
    }
    this.rows.forEach((r, i) => show(r.el, i < n));
  }

  /**
   * Auto-width: the stretchy columns (driver, team, car) get the width of their widest cell,
   * the same in every table, so the panel's natural width shows them in full.
   */
  static fitColumns<R>(tables: Table<R>[]) {
    if (!tables.length) return;
    const flex = tables[0].cols.map((c) => c.width.includes('fr'));
    if (!flex.some(Boolean)) return;
    for (const t of tables) t.el.style.setProperty('--cols', t.cols.map((c, i) => (flex[i] ? 'max-content' : c.width)).join(' '));
    const widths = flex.map(() => 0);
    for (const t of tables) {
      for (const cells of [t.headCells, ...t.rows.map((r) => r.cells)]) {
        flex.forEach((f, i) => {
          if (f && cells[i]) widths[i] = Math.max(widths[i], Math.ceil(cells[i].getBoundingClientRect().width));
        });
      }
    }
    const min = (w: string) => /minmax\(([^,]+),/.exec(w)?.[1] ?? '0px';
    for (const t of tables) t.el.style.setProperty('--cols', t.cols.map((c, i) => (flex[i] ? `minmax(${min(c.width)}, ${widths[i] + 2}px)` : c.width)).join(' '));
  }

  /** Render `data` into the table; `null` entries become empty placeholder rows. */
  render(data: (R | null)[], classes: (r: R) => Record<string, boolean>, before?: (row: HTMLElement, r: R) => void) {
    this.ensure(data.length);
    data.forEach((r, i) => {
      const row = this.rows[i];
      toggle(row.el, 'empty', r == null);
      if (r == null) {
        row.cells.forEach((c) => text(c, ''));
        return;
      }
      const cls = classes(r);
      for (const k in cls) toggle(row.el, k, cls[k]);
      before?.(row.el, r);
      this.cols.forEach((c, j) => c.render(row.cells[j], r));
    });
  }
}

/** Auto-width: re-fits the tables' stretchy columns at most once a second. */
export function columnFitter() {
  let last = 0;
  return <R>(tables: Table<R>[]) => {
    const now = performance.now();
    if (now - last < 1000) return;
    last = now;
    Table.fitColumns(tables);
  };
}

/** Renders a coloured chip (class number, licence) without recreating nodes. */
export function chip(cell: HTMLElement, label: string, bg: string, fg?: string) {
  let el = cell.firstElementChild as HTMLElement | null;
  if (!el) {
    el = h('span', 'chip');
    cell.append(el);
  }
  text(el, label);
  // Compared with what was last set: reading the style back builds a new string each time.
  const c = el as HTMLElement & { _bg?: string; _fg?: string };
  if (c._bg !== bg) {
    el.style.background = bg;
    c._bg = bg;
  }
  const color = fg ?? contrast(bg);
  if (c._fg !== color) {
    el.style.color = color;
    c._fg = color;
  }
}

export const contrast = (hex: string) => onColor(hex);
