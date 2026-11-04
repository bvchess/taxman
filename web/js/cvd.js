// How far apart two colors stay for a reader with color-blind vision, worked out in the page from the stylesheet's
// own tokens, so that the style page's numbers cannot drift from the colors.  The same method as the dataviz palette
// validator: the protan and deutan simulations of Machado, Oliveira and Fernandes (2009) at severity 1, in linear
// sRGB, and the distance in OKLab times 100.  A distance of 8 or more is the target; 6 to 8 needs a second cue.

const MACHADO = {
  protan: [[0.152286, 1.052583, -0.204868], [0.114503, 0.786281, 0.099216], [-0.003882, -0.048116, 1.051998]],
  deutan: [[0.367322, 0.860646, -0.227968], [0.280085, 0.672501, 0.047413], [-0.011820, 0.042940, 0.968881]],
};

function linear(hex) {
  const h = hex.trim().replace('#', '');
  return [0, 2, 4].map((i) => {
    const c = parseInt(h.slice(i, i + 2), 16) / 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
}

function oklab([r, g, b]) {
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  return [
    0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
    1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
    0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
  ];
}

function simulate(rgb, kind) {
  const M = MACHADO[kind];
  return M.map((row) => Math.min(1, Math.max(0, row[0] * rgb[0] + row[1] * rgb[1] + row[2] * rgb[2])));
}

/** The distance between two hex colors in OKLab, times 100, as seen with kind ('protan' or 'deutan') or, with no
 *  kind, with full color vision. */
export function separation(a, b, kind) {
  const [x, y] = [a, b].map((c) => oklab(kind ? simulate(linear(c), kind) : linear(c)));
  return 100 * Math.hypot(x[0] - y[0], x[1] - y[1], x[2] - y[2]);
}

/** The WCAG contrast ratio of two hex colors. */
export function contrast(a, b) {
  const lum = (c) => {
    const [r, g, bl] = linear(c);
    return 0.2126 * r + 0.7152 * g + 0.0722 * bl;
  };
  const [hi, lo] = [lum(a), lum(b)].sort((p, q) => q - p);
  return (hi + 0.05) / (lo + 0.05);
}
