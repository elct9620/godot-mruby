#!/usr/bin/env bash
#
# Stop hook: before a turn ends, confirm the extension still compiles cleanly,
# its tests pass and the GDScript side lints. Any one of them failing hands the
# result back for correction rather than carrying a broken state into the next
# turn. Formatting is not gated here: the edit hook applies it as files are
# written, and it is settled before a commit rather than at every turn's end.
#
# stdin carries Claude Code's hook input JSON; decision:"block" hands the turn
# back to the model.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MANIFEST="$ROOT/rust/Cargo.toml"

input="$(cat)"

# The previous turn was already blocked by this hook; blocking again leaves no exit
if [ "$(printf '%s' "$input" | jq -r '.stop_hook_active // false')" = "true" ]; then
	exit 0
fi

cd "$ROOT" || exit 0

# A working tree matching HEAD has nothing to verify
if git rev-parse --verify HEAD >/dev/null 2>&1 && [ -z "$(git status --porcelain)" ]; then
	exit 0
fi

report=""

record() {
	report="${report}## ${1}"$'\n'"${2}"$'\n\n'
}

if command -v cargo >/dev/null 2>&1; then
	if ! out="$(cargo clippy --manifest-path "$MANIFEST" --all-targets --quiet -- -D warnings 2>&1)"; then
		record "Lint (cargo clippy)" "$out"
	fi
	if ! out="$(cargo test --manifest-path "$MANIFEST" --quiet 2>&1)"; then
		record "Tests (cargo test)" "$out"
	fi
fi

# The edit hook only sees files written through Edit/Write; this sweep also
# covers scripts that changed some other way
if command -v gdlint >/dev/null 2>&1; then
	if ! out="$(gdlint "$ROOT/godot" 2>&1)"; then
		record "Lint (gdlint godot/)" "$out"
	fi
fi

if [ -n "$report" ]; then
	jq -n --arg r "Quality gate failed before the turn ends; fix these first:"$'\n\n'"$report" \
		'{decision: "block", reason: $r}'
fi
