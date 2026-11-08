#!/bin/bash
# Copy the website to the web server, where it appears at https://vantuyl.com/taxman/.
# The site has no build step.  Everything in web/ goes out, taken from the last commit, except the working files listed
# in NOT_SITE below (the tools, the notes, the pictures of the repository's README, and the internal style guide that no
# page links).  A new working file in web/ must be added to that list.  Files on the server that are no longer in the
# site are removed (only inside the taxman folder).
#
#   web/tools/publish.sh             test, then publish the last commit
#   web/tools/publish.sh --dry-run   test, then list what would change on the server without changing it
#   web/tools/publish.sh --dirty     publish even though web/ has uncommitted changes (what goes out is still HEAD)
#
# It stops if the site's test fails, or if web/ has uncommitted changes (unless --dirty).

set -euo pipefail
cd "$(dirname "$0")/../.."

DEST="bchess@vantuyl.com:/home/bchess/vantuyl.com/taxman/"
# web/'s working files, which are not part of the site.  A new working file goes here.
NOT_SITE=(web/tools web/OUTLINE.md web/README.md web/img web/style.html web/css/style-page.css)

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
git archive HEAD -- web | tar -x -C "$STAGE"
rm -rf "${NOT_SITE[@]/#/$STAGE/}"
find "$STAGE" \( -name '.DS_Store' -o -name '*~' -o -name '*.swp' \) -delete

echo "Publishing commit $(git log -1 --format='%h %s' HEAD | cut -c1-100)"
# --checksum compares contents, not timestamps: a fresh checkout rewrites every file, but only real changes are sent.
rsync --archive --compress --checksum --delete --itemize-changes --exclude '.DS_Store' ${ARGS[@]+"${ARGS[@]}"} \
  "$STAGE/web/" "$DEST"
