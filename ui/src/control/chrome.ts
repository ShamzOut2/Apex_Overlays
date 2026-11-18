// The page header (a small label over the page title, actions on the right) that every
// settings page sits under: pages put their main buttons in it with pageActions().

import { h, text } from '../lib/dom';

const eyebrow = h('span', 'page-eyebrow');
const title = h('h1', 'page-title');
const actions = h('div', 'page-actions');
const left = h('div', 'page-head-left');
// A kerb under the title: red and white (the accent and the text colour).
const kerb = h('span', 'page-kerb');
kerb.setAttribute('aria-hidden', 'true');
left.append(eyebrow, title, kerb);
export const pageHead = h('header', 'page-head');
pageHead.append(left, actions);
/** Where the current page's content goes, under the header. */
export const pageBody = h('div', 'page-body');

/** Sets the header for the page about to be drawn (its actions are cleared). */
export function setPageHead(label: string, name: string) {
  text(eyebrow, label);
  text(title, name);
  actions.replaceChildren();
}

/** The current page's main buttons, on the right of its title. */
export function pageActions(...nodes: Node[]) {
  actions.replaceChildren(...nodes);
}
