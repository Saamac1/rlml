#!/bin/sh
# The browser version's wasm and glue without a browser (macOS): each
# play-through's keys go through the worker's calls in JavaScriptCore,
# and the log must be the same as `rlml run` gives for the same keys.
# Run build.sh first.
# Usage: jsc/smoke.sh GAMEDIR KEYSFILE...   (e.g. testit/*.keys)
set -e
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../../.." && pwd)
JSC=${JSC:-/System/Library/Frameworks/JavaScriptCore.framework/Versions/A/Helpers/jsc}
GAME=${1:?usage: smoke.sh GAMEDIR KEYSFILE...}; shift
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
(cd "$REPO" && cargo build -q -p rlml)
fail=0
for keys in "$@"; do
    name=$(basename "$keys" .keys)
    "$REPO/target/debug/rlml" run "$GAME" --keys "$(cat "$keys")" | sed -n 's/^  · //p' > "$TMP/run"
    "$JSC" -m "$HERE/smoke.mjs" -- "$HERE/../www" "$keys" "$TMP/web" | grep -E '^(draw|input|tick) ' | sed "s/^/$name: /"
    if cmp -s "$TMP/run" "$TMP/web"; then
        echo "ok   $name: the log is the same as rlml run ($(wc -l < "$TMP/run" | tr -d ' ') lines)"
    else
        echo "FAIL $name"; diff "$TMP/run" "$TMP/web" | head -20; fail=1
    fi
done
exit $fail
