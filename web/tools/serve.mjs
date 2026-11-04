// A development server for the site: it sends every file with caching off, so a reload always gets the current files.
//
//   node web/tools/serve.mjs [port]
//
// Serves the folder web/ on 127.0.0.1 only (port 8780 unless given).  A static server that sends no cache headers lets
// a browser reuse an old script with new CSS after an edit; this one sends Cache-Control: no-store on every response.

import { createServer } from 'node:http';
import { readFile, stat } from 'node:fs/promises';
import { dirname, extname, join, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const webDir = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const port = Number(process.argv[2] ?? 8780);
const host = '127.0.0.1';

const types = {
  '.html': 'text/html; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.svg': 'image/svg+xml; charset=utf-8',
  '.txt': 'text/plain; charset=utf-8',
  '.md': 'text/markdown; charset=utf-8',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.woff2': 'font/woff2',
};

const send = (res, status, body, type = 'text/plain; charset=utf-8') => {
  res.writeHead(status, { 'Cache-Control': 'no-store', 'Content-Type': type, 'Content-Length': Buffer.byteLength(body) });
  return body;
};

// The file a request names, or null when it tries to leave web/.  The raw path is checked, because the URL parser
// would quietly fold a leading ".." away.
const resolvePath = (url) => {
  const pathname = decodeURIComponent(url.split(/[?#]/)[0]);
  if (pathname.split(/[/\\]/).includes('..')) return null;
  const file = resolve(join(webDir, pathname));
  return file === webDir || file.startsWith(webDir + sep) ? file : null;
};

const server = createServer(async (req, res) => {
  if (req.method !== 'GET' && req.method !== 'HEAD') {
    res.setHeader('Allow', 'GET, HEAD');
    return res.end(send(res, 405, 'Method Not Allowed'));
  }
  const head = req.method === 'HEAD';
  const reply = (status, text) => res.end(head ? (send(res, status, text), undefined) : send(res, status, text));
  let file;
  try {
    file = resolvePath(req.url);
  } catch {
    return reply(400, 'Bad Request');
  }
  if (file === null) return reply(403, 'Forbidden');
  try {
    if ((await stat(file)).isDirectory()) file = join(file, 'index.html');
    const body = await readFile(file);
    res.writeHead(200, {
      'Cache-Control': 'no-store',
      'Content-Type': types[extname(file).toLowerCase()] ?? 'application/octet-stream',
      'Content-Length': body.length,
    });
    res.end(head ? undefined : body);
  } catch {
    reply(404, 'Not Found');
  }
});

server.listen(port, host, () => console.log(`Serving ${webDir} at http://${host}:${port}/`));
