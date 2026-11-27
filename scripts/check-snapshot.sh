#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if ! command -v zellij-plugin-snapshot >/dev/null 2>&1; then
    printf '%s\n' \
        "zellij-plugin-snapshot is not installed. Run: cargo install zellij-plugin-snapshot --version 0.2.3 --locked" >&2
    exit 1
fi

if [[ ! -s dist/pane-switcher.wasm ]]; then
    printf '%s\n' "dist/pane-switcher.wasm is missing. Run ./scripts/build-wasm.sh first." >&2
    exit 1
fi

output_dir="$(mktemp -d /tmp/pane-switcher-snapshot.XXXXXX)"
snapshot_log="$output_dir/plugin-commands.txt"
for fixture in smoke template mouse ime hint_narrow; do
    zellij-plugin-snapshot "tests/snapshot/${fixture}.yaml" --out "$output_dir" |
        tee -a "$snapshot_log"
done

smoke="$output_dir/smoke.ansi.txt"
template="$output_dir/template.ansi.txt"
mouse="$output_dir/mouse.ansi.txt"
ime="$output_dir/ime.ansi.txt"
hint_narrow="$output_dir/hint_narrow.ansi.txt"
smoke_plain="$output_dir/smoke.txt"
template_plain="$output_dir/template.txt"
mouse_plain="$output_dir/mouse.txt"
ime_plain="$output_dir/ime.txt"
hint_narrow_plain="$output_dir/hint_narrow.txt"

test -s "$smoke"
test -s "$template"
test -s "$mouse"
test -s "$ime"
test -s "$hint_narrow"
perl -pe 's/\e\[[0-9;]*m//g' "$smoke" >"$smoke_plain"
perl -pe 's/\e\[[0-9;]*m//g' "$template" >"$template_plain"
perl -pe 's/\e\[[0-9;]*m//g' "$mouse" >"$mouse_plain"
perl -pe 's/\e\[[0-9;]*m//g' "$ime" >"$ime_plain"
perl -pe 's/\e\[[0-9;]*m//g' "$hint_narrow" >"$hint_narrow_plain"

grep -a -q "FIND" "$smoke"
grep -q "src color renderer" "$smoke_plain"
grep -q "workspace/notes" "$smoke_plain"
grep -q "work/application" "$smoke_plain"
grep -q "codex --full-auto --model gpt-5 --sandbox danger" "$smoke_plain"
grep -q "0-1" "$smoke_plain"
grep -a -F -q $'\033[38;5;226;48;5;238m0-3' "$smoke"
grep -a -F -q $'\033[38;5;154m◐' "$smoke"
grep -a -F -q $'\033[38;5;226m✓' "$smoke"
grep -a -F -q $'\033[38;5;124;48;5;238m✕' "$smoke"
grep -q "↑↓/Tab move  Shift+Tab back" "$smoke_plain"
grep -q "Enter/click jump  Ctrl+Enter new tab  wheel scroll" "$smoke_plain"
grep -a -E -q "FIND  › o.*Ctrl\+Enter new tab  Esc" "$smoke_plain"
grep -a -E -q "38;(2|5);" "$smoke"
grep -a -E -q "1;38;(2|5);" "$smoke"
grep -a -E -q "48;(2|5);" "$smoke"
grep -q "RenamePluginPane" "$snapshot_log"
grep -q "Mouse" "$snapshot_log"
grep -q "ChangeFloatingPanesCoordinates" "$snapshot_log"
grep -q "width: Some(Percent(72))" "$snapshot_log"
grep -q "height: Some(Percent(68))" "$snapshot_log"

if grep -q "selected " "$smoke_plain"; then
    printf '%s\n' "Unexpected detail panel heading in smoke snapshot." >&2
    exit 1
fi

grep -q "C demo/code release tasks" "$template_plain"
grep -q "codex --full-auto" "$template_plain"
grep -a -E -q "1;38;(2|5);" "$template"
grep -a -F -q $'\033[38;5;45;48;5;238mE' "$template"

if grep -q "selected " "$template_plain"; then
    printf '%s\n' "Unexpected detail panel heading in template snapshot." >&2
    exit 1
fi

grep -q "first row  zsh" "$mouse_plain"
grep -q "second row  nvim notes.md" "$mouse_plain"
grep -q "third row  bash" "$mouse_plain"
grep -a -F -q $'\033[38;5;51;48;5;238msecond row' "$mouse"
grep -a -F -q $'\033[38;5;51;48;2;22;22;22mthird row' "$mouse"

grep -q "FIND  › 的输入" "$ime_plain"
grep -q "的输入法测试" "$ime_plain"
if grep -q "其他窗口" "$ime_plain"; then
    printf '%s\n' "IME query did not filter non-matching rows." >&2
    exit 1
fi

grep -q "FIND" "$hint_narrow_plain"
grep -q "Ctrl+Enter  Esc" "$hint_narrow_plain"
if grep -q "Ctrl+Enter new tab" "$hint_narrow_plain"; then
    printf '%s\n' "Narrow input hint did not shorten to Ctrl+Enter." >&2
    exit 1
fi

printf 'Snapshots: %s\n' "$output_dir"
