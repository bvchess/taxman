// The map of the site, drawn as a route: the story is one line of stops, and the findings along the way are pins
// beside their stops.  Drawn from js/site.js, so the map, the contents and the pages agree.  A stop is a raised coin,
// since it is a link; one not yet written lies flat and dashed, like a source not played.

import { ALONG, STORY, hrefOf } from '../js/site.js';
import { numberSVG } from '../js/vocab.js';

const PIN = `<svg viewBox="0 0 22 22" width="20" height="20" aria-hidden="true"><path d="M11 20.5s-6-6.2-6-10.6a6 6 0 0 1 12 0c0 4.4-6 10.6-6 10.6z" fill="#fff" stroke="currentColor" stroke-width="2"/><circle cx="11" cy="9.8" r="2.2" fill="currentColor"/></svg>`;

export function mountSiteMap(host) {
  const coin = (c) => numberSVG(c.n, {
    rendering: 'raised', size: 40, pad: 3, looks: hrefOf(c) ? ['kept'] : ['lying', 'rejected'],
  }).outerHTML;
  const items = STORY.map((c) => {
    const href = hrefOf(c);
    const title = href ? `<a href="${href}">${c.title}</a>` : `<span>${c.title}</span>`;
    const pins = ALONG.filter((a) => a.at === c.n).map((a) => {
      const t = a.href ? `<a href="${a.href}">${a.title}</a>` : `<span>${a.title}</span>`;
      return `<div class="pin${a.href ? '' : ' is-unwritten'}">${PIN}<div>${t}<small>Along the way: ${a.where}${a.href ? '' : '; not yet written'}</small></div></div>`;
    }).join('');
    return `<li class="stop${href ? '' : ' is-unwritten'}"><span class="stop-coin">${coin(c)}</span><div class="stop-text">${title}<p>${c.line}${href ? '' : ' Not yet written.'}</p>${pins}</div></li>`;
  }).join('');
  host.innerHTML = `<ol class="route" aria-label="The story and the findings along the way">${items}</ol>`;
}
