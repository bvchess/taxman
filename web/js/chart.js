// What the site's charts share: drawing at the container's own width, a tooltip that follows the pointer or the
// keyboard, and a table of the numbers behind each chart.  The marks follow the data-viz rules: thin marks,
// hairline axes, text in text colors, and a hover layer that only enhances what the table also gives.

import { el } from './vocab.js';

/** Draw with draw(width) now and whenever the container's width changes. */
export function responsive(host, draw) {
  let last = 0;
  const run = () => {
    const w = Math.round(host.clientWidth);
    if (w > 0 && w !== last) {
      last = w;
      draw(w);
    }
  };
  new ResizeObserver(run).observe(host);
  run();
}

/** An <svg> of the given size, replacing any the host holds. */
export function freshSVG(host, width, height, label) {
  host.querySelector('svg')?.remove();
  const svg = el('svg', { viewBox: `0 0 ${width} ${height}`, width, height, role: 'img', 'aria-label': label });
  host.prepend(svg);
  return svg;
}

/** A tooltip inside host.  show(rows, x, y) places it near (x, y) in the host's coordinates: a head, then rows
 *  [key, value] and, for a string after the head, a subhead. */
export function tooltip(host) {
  const tip = document.createElement('div');
  tip.className = 'chart-tip';
  tip.setAttribute('aria-hidden', 'true');
  host.appendChild(tip);
  return {
    show(rows, x, y) {
      tip.replaceChildren();
      const [head, ...rest] = rows;
      const b = document.createElement('b');
      b.textContent = head;
      tip.appendChild(b);
      for (const item of rest) {
        if (typeof item === 'string') {
          const sub = document.createElement('b');
          sub.className = 'tip-sub';
          sub.textContent = item;
          tip.appendChild(sub);
          continue;
        }
        const [k, v] = item;
        const row = document.createElement('div');
        row.className = 'row';
        const a = document.createElement('span');
        a.textContent = k;
        const c = document.createElement('span');
        c.textContent = v;
        row.append(a, c);
        tip.appendChild(row);
      }
      tip.classList.add('is-on');
      const w = tip.offsetWidth;
      const h = tip.offsetHeight;
      const W = host.clientWidth;
      let left = x + 16;
      if (left + w > W) left = x - w - 16;
      tip.style.left = `${Math.max(0, left)}px`;
      tip.style.top = `${Math.max(0, y - h - 12)}px`;
    },
    hide() {
      tip.classList.remove('is-on');
    },
  };
}

/** The table view of a chart, closed by default. */
export function tableView(host, caption, head, rows) {
  const d = document.createElement('details');
  d.className = 'chart-table';
  const s = document.createElement('summary');
  s.textContent = caption;
  const wrap = document.createElement('div');
  wrap.className = 'table-scroll';
  const t = document.createElement('table');
  const tr = document.createElement('tr');
  for (const h of head) {
    const th = document.createElement('th');
    th.textContent = h;
    tr.appendChild(th);
  }
  const thead = document.createElement('thead');
  thead.appendChild(tr);
  const tbody = document.createElement('tbody');
  for (const r of rows) {
    const row = document.createElement('tr');
    for (const v of r) {
      const td = document.createElement('td');
      td.textContent = v;
      row.appendChild(td);
    }
    tbody.appendChild(row);
  }
  t.append(thead, tbody);
  wrap.appendChild(t);
  d.append(s, wrap);
  host.appendChild(d);
}

/** Keyboard for a chart's hover layer: arrows step through the points, shift steps further. */
export function keyboard(target, { count, onIndex, onLeave }) {
  let i = -1;
  target.setAttribute('tabindex', '0');
  target.addEventListener('keydown', (e) => {
    const step = e.shiftKey ? Math.max(1, Math.round(count / 50)) : 1;
    const to = { ArrowRight: i + step, ArrowLeft: i - step, Home: 0, End: count - 1 }[e.key];
    if (to === undefined) return;
    e.preventDefault();
    i = Math.min(count - 1, Math.max(0, to < 0 && i < 0 ? count - 1 : to));
    onIndex(i);
  });
  target.addEventListener('blur', () => onLeave());
}

export const pct = (x, digits = 1) => `${(100 * x).toFixed(digits)}%`;
