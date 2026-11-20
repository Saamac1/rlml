#!/bin/sh
# Packs a game for the browser: game.json holds
#   { "main": "game.rlml", "game": {path: text}, "engine": {path: text} }
# with the game folder's .rlml, .md, .rlml.md and .rogue files (M 2.1),
# without its testit/ folder or hidden files, and the repository's lib/.
# Usage: bundle.sh GAMEDIR [OUT]   (OUT defaults to www/game.json here)
set -e
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
GAME=${1:?usage: bundle.sh GAMEDIR [OUT]}
OUT=${2:-$HERE/www/game.json}
exec python3 -I - "$GAME" "$REPO" "$OUT" <<'PY'
import json, os, sys

game, repo, out = sys.argv[1:4]
EXT = (".rlml", ".md", ".rogue")

def tree(root, sub, skip=()):
    files = {}
    for d, dirs, names in os.walk(os.path.join(root, sub)):
        rel_dir = os.path.relpath(d, root)
        dirs[:] = sorted(x for x in dirs if not x.startswith(".") and os.path.normpath(os.path.join(rel_dir, x)) not in skip)
        for n in sorted(names):
            if n.startswith(".") or not n.endswith(EXT):
                continue
            rel = os.path.normpath(os.path.join(rel_dir, n)).replace(os.sep, "/")
            with open(os.path.join(d, n), encoding="utf-8") as f:
                files[rel] = f.read()
    return files

main = next((m for m in ("game.rlml", "game.rlml.md", "game.md") if os.path.isfile(os.path.join(game, m))), None)
if main is None:
    sys.exit(f"{game}: no game.rlml, game.rlml.md or game.md")
bundle = {"main": main, "game": tree(game, ".", skip={"testit"}), "engine": tree(repo, "lib")}
os.makedirs(os.path.dirname(os.path.abspath(out)), exist_ok=True)
with open(out, "w", encoding="utf-8") as f:
    json.dump(bundle, f, ensure_ascii=False)
print(f"{out}: {len(bundle['game'])} game files, {len(bundle['engine'])} engine files")
PY
