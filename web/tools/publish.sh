#!/bin/bash
# Copy the website to the web server, where it appears at https://vantuyl.com/taxman/.
# The site has no build step, but web/ also holds files that are not part of it (tools/, OUTLINE.md, README.md, img/,
# style.html and css/style-page.css, the internal style guide that no page links), so only the files named in SITE
# below go out, taken from the last commit.  Files on the server that are no longer in the site are removed (only
# inside the taxman folder).
#
#   web/tools/publish.sh             test, then publish the last commit
#   web/tools/publish.sh --dry-run   test, then list what would change on the server without changing it
#   web/tools/publish.sh --dirty     publish even though web/ has uncommitted changes (what goes out is still HEAD)
#
# It stops if the site's test fails, or if web/ has uncommitted changes (unless --dirty).

set -euo pipefail
cd "$(dirname "$0")/../.."

DEST="bchess@vantuyl.com:/home/bchess/vantuyl.com/taxman/"
# What the site serves: its pages (every page that site.js lists, and none other), and what they load.
SITE=(web/*.html web/css web/js web/figures web/data web/fonts web/icon.svg)
# Pages and styles that no page links.
NOT_SITE=(web/style.html web/css/style-page.css)

ARGS=()
DIRTY=0
for arg in "$@"; do
  if [ "$arg" = "--dirty" ]; then DIRTY=1; else ARGS+=("$arg"); fi
done

if [ "$DIRTY" = 0 ] && [ -n "$(git status --porcelain -- web)" ]; then
  echo "web/ has uncommitted changes; commit them first (or use --dirty):" >&2
  git status --short -- web >&2
  exit 1
fi

node web/tools/test.mjs

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
# git archive takes the files of the commit, so nothing untracked or edited since can go out.
git archive HEAD -- "${SITE[@]}" | tar -x -C "$STAGE"
rm -f "${NOT_SITE[@]/#/$STAGE/}"
find "$STAGE" \( -name '.DS_Store' -o -name '*~' -o -name '*.swp' \) -delete

echo "Publishing commit $(git log -1 --format='%h %s' HEAD | cut -c1-100)"
# --checksum compares contents, not timestamps: a fresh checkout rewrites every file, but only real changes are sent.
rsync --archive --compress --checksum --delete --itemize-changes --exclude '.DS_Store' ${ARGS[@]+"${ARGS[@]}"} \
  "$STAGE/web/" "$DEST"
