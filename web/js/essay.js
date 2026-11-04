// The essay's machinery: the contents, which mark the reader's place; the notes of the technical layer, which fold
// into the text on a narrow window; the words that move a figure; and the numbers in the prose that put a halo on
// their mark in a figure.

import { AFTER, ALONG, SAMPLE, STORY, hrefOf } from './site.js';
import { numberSVG } from './vocab.js';

const figures = new Map();
const waiting = new Map();

/** Make a figure reachable from the prose: name is what data-fig says; api has goState(state), highlight(n, on) and,
 *  optionally, stopName(state), the name of the slider's stop for a state. */
export function registerFigure(name, api) {
  figures.set(name, api);
  for (const fn of waiting.get(name) ?? []) fn(api);
  waiting.delete(name);
}

function withFigure(name, fn) {
  if (figures.has(name)) fn(figures.get(name));
  else waiting.set(name, [...(waiting.get(name) ?? []), fn]);
}

// ---- the contents ------------------------------------------------------------------------------------------------

export const ALONG_ICON = '<svg viewBox="0 0 22 22" width="22" height="22" aria-hidden="true"><path d="M11 20.5s-6-6.2-6-10.6a6 6 0 0 1 12 0c0 4.4-6 10.6-6 10.6z" fill="var(--paper)" stroke="currentColor" stroke-width="2"/><circle cx="11" cy="9.8" r="2.2" fill="currentColor"/></svg>';

// A chapter's badge in the contents: a raised token, since it is a link; the reader's own chapter is kept, and one
// not yet written lies flat and dashed, since there is nothing to press.
function badge(entry, here) {
  const written = Boolean(hrefOf(entry));
  const looks = written ? (here ? ['kept'] : []) : ['lying', 'rejected'];
  return numberSVG(entry.n, { rendering: 'raised', looks, size: 24, pad: 2 }).outerHTML;
}

/** The contents of the site: the main story's chapters as the outline lists them, the findings along the way and the
 *  about page, each shown as not yet written, and the sample chapter. */
export function contentsHTML({ onStoryPage }) {
  const unwritten = (badgeHTML, title) =>
    `<li><span class="unwritten"><span class="badge">${badgeHTML}</span><span>${title}<small>not yet written</small></span></span></li>`;
  const story = STORY.map((c) => unwritten(badge(c, false), c.title)).join('');
  const after = AFTER.map((a) => unwritten(ALONG_ICON, a.title)).join('');
  const sample = `<li><a href="${SAMPLE.href}"${onStoryPage ? ' aria-current="page" class="is-here"' : ''}><span class="badge">${ALONG_ICON}</span><span>${SAMPLE.title}</span></a></li>`;
  return `<p class="toc-group">The main story</p><ul class="toc">${story}</ul><ul class="toc">${after}</ul><p class="toc-group">Prototype</p><ul class="toc">${sample}</ul>`;
}

export const homeMark = () => numberSVG('', { rendering: 'raised', looks: ['player'], size: 24, pad: 2 }).outerHTML;

function mountContents() {
  const rail = document.querySelector('.rail');
  const top = document.querySelector('.topbar');
  const onStoryPage = document.body.dataset.page === 'story';
  const foot = '<p class="rail-foot"><a href="style.html">The style of the site</a></p>';
  if (rail && !rail.dataset.own) {
    rail.innerHTML = `<a class="home" href="index.html">${homeMark()}Taxman</a><nav aria-label="Contents">${contentsHTML({ onStoryPage })}</nav><div class="progress" aria-hidden="true"><span></span></div>${foot}`;
  }
  if (top) {
    const own = rail?.dataset.own ? rail.querySelector('nav').outerHTML : `<nav aria-label="Contents">${contentsHTML({ onStoryPage })}</nav>`;
    top.innerHTML = `<a class="home" href="index.html">${homeMark()}Taxman</a><span class="where"></span><button class="btn contents-btn" type="button" aria-expanded="false" aria-controls="toc-panel">Contents</button><span class="bar" aria-hidden="true"></span>`;
    const panel = document.createElement('div');
    panel.id = 'toc-panel';
    panel.className = 'toc-panel';
    panel.hidden = true;
    panel.innerHTML = own;
    top.after(panel);
    const btn = top.querySelector('.contents-btn');
    btn.addEventListener('click', () => {
      panel.hidden = !panel.hidden;
      btn.setAttribute('aria-expanded', String(!panel.hidden));
    });
    panel.addEventListener('click', (e) => {
      if (e.target.closest('a')) {
        panel.hidden = true;
        btn.setAttribute('aria-expanded', 'false');
      }
    });
  }
  // Where the reader is: the last section whose top has passed the upper third of the window.
  const sections = [...document.querySelectorAll('[data-toc]')];
  const titleOf = (id) => {
    if (!id) return '';
    return [...STORY, ...ALONG].find((c) => c.id === id)?.title ?? document.querySelector(`[data-toc="${id}"]`)?.dataset.title ?? '';
  };
  const bars = [...document.querySelectorAll('.rail .progress span, .topbar .bar')];
  const where = top?.querySelector('.where');
  const update = () => {
    const line = innerHeight * 0.33;
    let current = null;
    for (const c of sections) if (c.getBoundingClientRect().top < line) current = c;
    for (const a of document.querySelectorAll('[data-chapter]')) {
      const on = Boolean(current && a.dataset.chapter === current.dataset.toc);
      a.classList.toggle('is-here', on);
      const b = a.querySelector('.badge');
      const entry = STORY.find((c) => c.id === a.dataset.chapter);
      if (entry && b && b.dataset.on !== String(on)) {
        b.dataset.on = String(on);
        b.innerHTML = badge(entry, on);
      }
      if (on) a.setAttribute('aria-current', 'location');
      else a.removeAttribute('aria-current');
    }
    if (where) where.textContent = titleOf(current?.dataset.toc) ?? '';
    const max = document.documentElement.scrollHeight - innerHeight;
    for (const b of bars) b.style.width = `${max > 0 ? (100 * scrollY) / max : 0}%`;
  };
  addEventListener('scroll', update, { passive: true });
  addEventListener('resize', update);
  update();
}

// ---- the technical layer: each note gets the line it folds into on a narrow window -------------------------------

const KIND = [['card', 'Precisely'], ['tech', 'Technical note'], ['links', 'Further reading']];

/** Give each note of the right column the button that opens it when the margin folds into the text. */
export function mountNotes(root = document) {
  let k = 0;
  for (const note of root.querySelectorAll('.note')) {
    if (note.querySelector('.note-fold')) continue;
    if (!note.id) note.id = `note-${++k}`;
    const kind = KIND.find(([c]) => note.classList.contains(c))?.[1] ?? 'Technical note';
    const name = note.querySelector('.note-name')?.firstChild?.textContent?.trim() ?? kind;
    const standing = note.querySelector('.note-head .standing');
    const fold = document.createElement('button');
    fold.type = 'button';
    fold.className = 'note-fold';
    fold.setAttribute('aria-expanded', 'false');
    fold.setAttribute('aria-controls', note.id);
    fold.innerHTML = `${name === kind ? '' : `<span class="fold-kind">${kind}</span>`}<span class="fold-name">${name}</span>${standing ? standing.outerHTML : ''}<span class="fold-sign" aria-hidden="true">+</span>`;
    fold.addEventListener('click', () => {
      const open = !note.classList.contains('is-open');
      note.classList.toggle('is-open', open);
      fold.setAttribute('aria-expanded', String(open));
    });
    note.prepend(fold);
  }
}

// ---- words that move a figure, and numbers that wear a halo ------------------------------------------------------

function mountFigureLinks() {
  for (const b of document.querySelectorAll('.go[data-fig]')) {
    withFigure(b.dataset.fig, (api) => {
      const name = api.stopName ? api.stopName(b.dataset.state) : b.dataset.state;
      b.dataset.name = name;
      b.setAttribute('aria-label', `${b.textContent.trim()}: show the figure at “${name}”`);
    });
    b.addEventListener('click', (e) => {
      e.preventDefault();
      withFigure(b.dataset.fig, (api) => {
        // The figure stays where it is in the text; if it is out of sight, the page brings it into view first.
        const fig = document.getElementById(b.dataset.fig);
        if (fig) {
          const r = fig.getBoundingClientRect();
          const topLimit = getComputedStyle(document.querySelector('.topbar') ?? document.body).display === 'none' ? 0 : 52;
          if (r.top < topLimit || r.bottom > innerHeight) {
            const reduce = matchMedia('(prefers-reduced-motion: reduce)').matches;
            fig.scrollIntoView({ block: r.height < innerHeight - topLimit ? 'center' : 'start', behavior: reduce ? 'auto' : 'smooth' });
          }
        }
        api.goState(b.dataset.state);
      });
    });
  }
  const figOf = (node) => node.dataset.fig ?? node.closest('[data-fig]')?.dataset.fig;
  for (const s of document.querySelectorAll('.n[data-n]')) {
    const name = figOf(s);
    if (!name) continue;
    s.tabIndex = 0;
    const on = () => withFigure(name, (api) => api.highlight(Number(s.dataset.n), true));
    const off = () => withFigure(name, (api) => api.highlight(Number(s.dataset.n), false));
    s.addEventListener('mouseenter', on);
    s.addEventListener('mouseleave', off);
    s.addEventListener('focus', on);
    s.addEventListener('blur', off);
  }
}

export function initEssay() {
  mountContents();
  mountNotes();
  mountFigureLinks();
}
