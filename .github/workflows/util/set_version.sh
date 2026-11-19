#!/usr/bin/env bash
# Point the pre-commit examples of the docs and the benchmark at a release of unref, given as `1.2.3`.
set -euo pipefail

version=$1
cd "$(dirname "$0")/../../.."

# perl edits in place alike on every runner, unlike sed. A Windows checkout can end its lines with CRLF.
for file in README.md docs/usage.md; do
  perl -0pi -e "s{(repo: https://github\.com/Apakottur/unref\r?\n\s+rev: v)[0-9.]+}{\${1}$version}g" "$file"
  grep -q "rev: v$version " "$file" || { echo "No unref rev in $file" >&2; exit 1; }
done
perl -pi -e "s{\"unref==[0-9.]+\"}{\"unref==$version\"}" scripts/benchmark.py
grep -q "\"unref==$version\"" scripts/benchmark.py || { echo "No unref pin in scripts/benchmark.py" >&2; exit 1; }
