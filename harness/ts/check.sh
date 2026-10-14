#!/bin/sh
# Installs the pinned TypeScript from the npm cache, prints its version,
# then type-checks, builds and runs hello.ts. Needs no network once the cache is filled.
set -eu
dir=$(cd "$(dirname "$0")" && pwd)
[ -d "$dir/node_modules" ] || npm ci --offline --ignore-scripts --prefix "$dir"
tsc="$dir/node_modules/.bin/tsc"
"$tsc" --version
out=$(mktemp -d)
trap 'rm -rf "$out"' EXIT
"$tsc" --strict --target es2022 --module commonjs --types node \
  --typeRoots "$dir/node_modules/@types" --noEmitOnError --skipLibCheck \
  --rootDir "$dir" --outDir "$out" "$dir/hello.ts"
echo world | node "$out/hello.js"
