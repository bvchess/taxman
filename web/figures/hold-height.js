// Holds a stepped figure's caption at the height of its longest words, so that stepping the figure never moves the
// page: the caption is measured with each of its texts at its own width, and measured again when its width changes.

/** Give box, which shows one of texts at a time, the min-height of the tallest of them. */
export function holdHeight(box, texts) {
  let width = -1;
  const fit = () => {
    const shown = box.textContent;
    box.style.minHeight = '';
    let tallest = 0;
    for (const text of texts) {
      box.textContent = text;
      tallest = Math.max(tallest, box.offsetHeight);
    }
    box.textContent = shown;
    box.style.minHeight = `${tallest}px`;
  };
  new ResizeObserver(() => {
    if (box.clientWidth === width) return;
    width = box.clientWidth;
    fit();
  }).observe(box);
}
