#!/usr/bin/env bash
#
# PostToolUse hook: bring a file just written in line with the project's
# formatting and lint rules, so every later step builds on a consistent file
# rather than discovering the problem at the end of the turn.
#
# Formatting is applied silently because it is not a decision; what the linter
# still reports is, so exit 2 hands stderr back to the model for correction.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

file="$(jq -r '.tool_response.filePath // .tool_input.file_path // empty')"
[ -n "$file" ] || exit 0
[ -f "$file" ] || exit 0

# A file outside the project does not answer to the project's rules
case "$file" in
"$ROOT"/*) ;;
*) exit 0 ;;
esac

case "$file" in
*.rs)
	# rustfmt reads the crate's edition from Cargo.toml only through cargo;
	# a failure here is a file that no longer parses
	command -v cargo >/dev/null 2>&1 || exit 0
	if ! out="$(cargo fmt --manifest-path "$ROOT/rust/Cargo.toml" 2>&1)"; then
		printf 'rustfmt could not format the crate:\n%s\n' "$out" >&2
		exit 2
	fi
	;;
*.gd)
	command -v gdformat >/dev/null 2>&1 && gdformat "$file" >/dev/null 2>&1
	command -v gdlint >/dev/null 2>&1 || exit 0
	if ! out="$(gdlint "$file" 2>&1)"; then
		printf 'gdlint still reports problems in %s:\n%s\n' "${file#"$ROOT"/}" "$out" >&2
		exit 2
	fi
	;;
esac

exit 0
