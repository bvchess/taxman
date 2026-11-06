// Works out the frames of a large game for figures/frames-row.js, off the page's own thread: descending greedy on a
// board of thousands takes seconds.  Posts back each frame, in the model's order, with its sources and which of them
// greedy keeps, its count of factors, its freedoms and its number, and the sum of U*.

import { frameModel } from '../js/model.js';

self.onmessage = (event) => {
  const frames = frameModel(event.data.n).map((f) => ({
    layer: f.layer, number: f.number, sources: f.sources, kept: f.kept, factors: f.factors.length, freedoms: f.freedoms,
    closed: f.closed,
  }));
  const inTheBag = frames.reduce((total, f) => total + f.kept.reduce((a, b) => a + b, 0), 0);
  self.postMessage({ n: event.data.n, frames, inTheBag });
};
