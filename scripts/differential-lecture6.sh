#!/usr/bin/env bash
set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root_dir"

for tool in as ld cargo; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "missing required tool: $tool" >&2
        exit 1
    fi
done

scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
cargo build --quiet -p x86-63-cli

assemble() {
    local source="$1"
    local object="$scratch/$(basename "${source%.s}").o"
    as "course-content/lecture6/$source" -o "$object"
    printf '%s\n' "$object"
}

read_int_object="$(assemble readInt.s)"
write_int_object="$(assemble writeInt.s)"

failures=0
for lesson in readwrite fact sumlooparray; do
    case "$lesson" in
        readwrite)
            main_object="$(assemble readWriteTest.s)"
            objects=("$main_object" "$read_int_object" "$write_int_object")
            input="123"
            expected_output="123"
            ;;
        fact)
            main_object="$(assemble fact.s)"
            objects=("$main_object" "$read_int_object" "$write_int_object")
            input="5"
            expected_output="Enter a number: 120"
            ;;
        sumlooparray)
            main_object="$(assemble sumLoopArray.s)"
            objects=("$main_object" "$write_int_object")
            input=""
            expected_output="10"
            ;;
    esac

    executable="$scratch/$lesson"
    native_output="$scratch/$lesson.out"
    ld "${objects[@]}" -o "$executable"
    set +e
    if [[ -n "$input" ]]; then
        printf '%s\n' "$input" | "$executable" >"$native_output"
        native_status=${PIPESTATUS[1]}
        engine_output="$(./target/debug/x86-63 run --example "$lesson" --stdin "$input")"
    else
        "$executable" >"$native_output"
        native_status=$?
        engine_output="$(./target/debug/x86-63 run --example "$lesson")"
    fi
    set -e

    printf '%s' "$expected_output" >"$scratch/$lesson.expected"
    engine_status="$(sed -n 's/.*shell status = \([0-9][0-9]*\)\..*/\1/p' <<<"$engine_output")"
    if [[ "$native_status" != 0 || "$engine_status" != 0 ]]; then
        echo "$lesson: native=$native_status engine=${engine_status:-missing} expected=0" >&2
        failures=$((failures + 1))
        continue
    fi
    if ! cmp -s "$scratch/$lesson.expected" "$native_output"; then
        echo "$lesson: GNU output bytes differ from the expected course result" >&2
        failures=$((failures + 1))
        continue
    fi
    if [[ "$engine_output" != *"stdout: \`$expected_output\`"* ]]; then
        echo "$lesson: teaching-machine output bytes differ" >&2
        failures=$((failures + 1))
        continue
    fi
    echo "ok  $lesson  stdout=\`$expected_output\`"
done

if ((failures > 0)); then
    echo "$failures Lecture 6 differential check(s) failed" >&2
    exit 1
fi

echo "All 3 executable Lecture 6 compositions match GNU as/ld."
