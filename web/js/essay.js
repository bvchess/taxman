// The essay's machinery: the contents, which mark the reader's place; the notes of the technical layer, which fold
// into the text on a narrow window; the words that move a figure; and the numbers in the prose that put a halo on
// their mark in a figure.

import { ALONG, INTRO, REFERENCE, STORY, hrefOf } from './site.js';
import { iconSVG } from './icon.js';
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

/** The page the reader is on, as site.js names pages: its file name, index.html for the folder itself. */
export const herePage = () => decodeURIComponent(location.pathname.split('/').pop() || 'index.html');

/** The contents of the site: the story's chapters as the outline lists them, each a link once it is written, and the
 *  findings along the way; below them, the reference pages.  part is 'story' or 'reference' for one of the two, else
 *  both, with a rule between.  here is the page the reader is on. */
export function contentsHTML({ here = herePage(), part = 'both' } = {}) {
  const unwritten = (badgeHTML, title) =>
    `<li><span class="unwritten"><span class="badge">${badgeHTML}</span><span>${title}<small>not yet written</small></span></span></li>`;
  const link = (badgeHTML, title, href) =>
    `<li><a href="${href}"${href === here ? ' aria-current="page" class="is-here"' : ''}><span class="badge">${badgeHTML}</span><span>${title}</span></a></li>`;
  const intro = link(`<span class="glyph">${INTRO.mark}</span>`, INTRO.title, INTRO.href);
  const story = intro + STORY.map((c) => {
    const href = hrefOf(c);
    return href ? link(badge(c, href === here), c.title, href) : unwritten(badge(c, false), c.title);
  }).join('');
  const entries = (list) => list.map((a) => (a.href ? link(a.mark ? `<span class="glyph">${a.mark}</span>` : ALONG_ICON, a.title, a.href) : unwritten(ALONG_ICON, a.title))).join('');
  const first = `<p class="toc-group">The story</p><ul class="toc">${story}</ul>`;
  const reference = `<ul class="toc">${entries(REFERENCE)}</ul>`;
  if (part === 'story') return first;
  if (part === 'reference') return reference;
  return `${first}<hr class="toc-rule">${reference}`;
}

/** The site's icon beside its name, in the page's colors. */
export const homeMark = () => iconSVG({ size: 26 });

function mountContents() {
  const rail = document.querySelector('.rail');
  const top = document.querySelector('.topbar');
  if (rail && !rail.dataset.own) {
    rail.innerHTML = `<a class="home" href="index.html">${homeMark()}Taxman</a><nav aria-label="Contents">${contentsHTML({ part: 'story' })}</nav><div class="progress" aria-hidden="true"><span></span></div><nav aria-label="Reference pages">${contentsHTML({ part: 'reference' })}</nav>`;
  }
  if (top) {
    const own = rail?.dataset.own ? rail.querySelector('nav').outerHTML : `<nav aria-label="Contents">${contentsHTML()}</nav>`;
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
  // Where the reader is: the last section whose top has passed the upper third of the window.  A section's name is
  // the site's for a chapter or a finding, else its own: data-title, its heading, or its label.  Above the first
  // section, a chapter page names its chapter.
  const sections = [...document.querySelectorAll('[data-toc]')];
  const titleOf = (section) => {
    const id = section.dataset.toc;
    const named = [...STORY, ...ALONG].find((c) => c.id === id)?.title;
    return named ?? section.dataset.title ?? section.querySelector('h2, h3')?.textContent ?? section.getAttribute('aria-label') ?? '';
  };
  const chapter = STORY.find((c) => String(c.n) === document.body.dataset.chapter);
  const pageTitle = chapter ? `${chapter.n}. ${chapter.title}` : document.body.dataset.page === 'front' ? INTRO.title : '';
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
    if (where) where.textContent = current ? titleOf(current) : pageTitle;
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
    fold.innerHTML = `${name === kind ? '' : `<span class="fold-kind">${kind}</span>`}<span class="fold-name">${name}</span>${standing ? `<span class="${standing.className}">${standing.firstChild.textContent}</span>` : ''}<span class="fold-sign" aria-hidden="true">+</span>`;
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
  // A number names its mark in the figure of its paragraph's data-fig, or else in the figure nearest its paragraph
  // that holds a mark for it, measured when the reader points, so that the numbers of one paragraph agree.
  const figOf = (node) => node.dataset.fig ?? node.closest('[data-fig]')?.dataset.fig;
  const nearest = (s) => {
    const n = s.dataset.n;
    const at = (s.closest('p, li') ?? s).getBoundingClientRect();
    let best = null;
    let bestGap = Infinity;
    for (const name of figures.keys()) {
      const fig = document.getElementById(name);
      if (!fig || !fig.querySelector(`svg [data-n="${n}"]`) || fig.offsetParent === null) continue;
      const r = fig.getBoundingClientRect();
      const gap = Math.max(0, r.top - at.bottom, at.top - r.bottom);
      if (gap < bestGap) [best, bestGap] = [name, gap];
    }
    return best;
  };
  for (const s of document.querySelectorAll('.n[data-n]')) {
    const named = figOf(s);
    s.tabIndex = 0;
    let lit = null;
    const on = () => {
      lit = named ?? nearest(s);
      if (lit) withFigure(lit, (api) => api.highlight(Number(s.dataset.n), true));
    };
    const off = () => {
      if (lit) withFigure(lit, (api) => api.highlight(Number(s.dataset.n), false));
      lit = null;
    };
    s.addEventListener('mouseenter', on);
    s.addEventListener('mouseleave', off);
    s.addEventListener('focus', on);
    s.addEventListener('blur', off);
  }
}

// ---- asides: each wears its pin, and a link to one opens it ----------------------------------------------------

/** The cards of findings, as the HTML of the items of a ul.finding-cards: each links to its chapter, with its title and
 *  line, and at the foot the pin and the chapter's name. */
export function findingCardsHTML(findings) {
  const pin = ALONG_ICON.replace('width="22" height="22"', 'width="18" height="18"');
  return findings.map((f) => {
    const chapter = STORY.find((c) => c.href === f.href.split('#')[0]);
    return `<li><a href="${f.href}"><span class="title">${f.title}</span><span class="line">${f.line}</span>`
      + `<span class="to">${pin}Chapter ${chapter.n}, ${chapter.title}</span></a></li>`;
  }).join('');
}

function mountAsides() {
  for (const title of document.querySelectorAll('details.aside[id] > summary .title')) {
    const finding = ALONG.find((f) => f.id === title.closest('details').id);
    if (finding) title.textContent = finding.title;
  }
  for (const pin of document.querySelectorAll('.aside-pin:empty')) {
    pin.innerHTML = ALONG_ICON.replace('width="22" height="22"', 'width="30" height="30"');
  }
  const openTarget = () => {
    const id = decodeURIComponent(location.hash.slice(1));
    const target = id ? document.getElementById(id) : null;
    if (!(target instanceof HTMLDetailsElement) || target.open) return;
    target.open = true;
    target.scrollIntoView({ block: 'start' });
    // The figures above it may still be drawing, and the page grows as they do: keep the aside in view while the
    // page settles, until the reader scrolls.
    const hold = new ResizeObserver(() => target.scrollIntoView({ block: 'start' }));
    const release = () => hold.disconnect();
    hold.observe(document.body);
    for (const type of ['wheel', 'touchstart', 'keydown', 'pointerdown']) addEventListener(type, release, { once: true, passive: true });
    setTimeout(release, 4000);
  };
  addEventListener('hashchange', openTarget);
  openTarget();
}

// The way on at the foot of a page, drawn from site.js: the introduction and the story in order, each page linking to
// the page before it and the page after.  A reference page or any page not in the sequence has no way on.  The link before wears the site's icon when
// it is the front page, as in the contents; a page not yet written is a dashed span, not a link.
function mountChapterNav() {
  const nav = document.querySelector('nav.chapter-nav');
  if (!nav) return;
  const pages = [INTRO, ...STORY];
  const at = pages.findIndex((p) => p.href === herePage());
  if (at < 0) return;
  const name = (p) => (p.n ? `${p.n}. ${p.title}` : p.title);
  const parts = [];
  const before = pages[at - 1];
  if (before) {
    const written = Boolean(hrefOf(before));
    const mark = before.href === INTRO.href ? homeMark() : '';
    const body = `<span class="nav-kicker">${written ? 'Previous' : 'Previous, not yet written:'}</span> `
      + `<span class="nav-title">${mark}${name(before)}</span>`;
    parts.push(written ? `<a class="prev" href="${before.href}">${body}</a>` : `<span class="prev">${body}</span>`);
  }
  const after = pages[at + 1];
  if (after) {
    const written = Boolean(hrefOf(after));
    const body = `<span class="nav-kicker">${written ? 'Next' : 'Next, not yet written:'}</span> `
      + `<span class="nav-title">${name(after)}</span>  ${after.line}`;
    parts.push(written ? `<a class="next" href="${after.href}">${body}</a>` : `<span class="next">${body}</span>`);
  }
  nav.innerHTML = parts.join('\n');
}

// A button at the corner of each command that copies the command's text.  It says so for two seconds, or says that the
// clipboard refused.
function mountCopyButtons() {
  for (const dd of document.querySelectorAll('dl.commands dd')) {
    const code = dd.querySelector('code');
    if (!code) continue;
    const button = document.createElement('button');
    button.type = 'button';
    button.className = 'btn copy';
    button.textContent = 'Copy';
    button.setAttribute('aria-label', `Copy the command: ${dd.previousElementSibling?.textContent.trim() ?? 'command'}`);
    let timer;
    button.addEventListener('click', async () => {
      let said = 'Copied';
      try {
        await navigator.clipboard.writeText(code.textContent);
      } catch {
        said = 'Copy failed';
      }
      button.textContent = said;
      clearTimeout(timer);
      timer = setTimeout(() => { button.textContent = 'Copy'; }, said === 'Copied' ? 2000 : 1500);
    });
    dd.append(button);
  }
}

export function initEssay() {
  mountContents();
  mountNotes();
  mountAsides();
  mountFigureLinks();
  mountChapterNav();
  mountCopyButtons();
}
