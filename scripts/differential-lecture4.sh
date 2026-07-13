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

cases=(
    "addglobal:addGlobal.s:210"
    "addglobalbetter:addGlobalBetter.s:210"
    "addgloballea:addGlobalLea.s:210"
    "addarray1:addArray1.s:210"
    "addarray2:addArray2.s:210"
    "addarray3:addArray3.s:54"
    "addarray4:addArray4.s:160"
    "cmp1:cmp1.s:255"
    "sumloop:sumLoop.s:55"
    "sumloopb:sumLoopB.s:55"
    "hello:hello.s:0"
)

cargo build --quiet -p x86-63-cli

failures=0
for specification in "${cases[@]}"; do
    IFS=: read -r lesson file expected_status <<<"$specification"
    object="$scratch/$lesson.o"
    executable="$scratch/$lesson"
    native_output="$scratch/$lesson.out"

    as "course-content/lecture4/$file" -o "$object"
    ld "$object" -o "$executable"
    set +e
    "$executable" >"$native_output"
    native_status=$?
    set -e

    engine_output="$(./target/debug/x86-63 run --example "$lesson")"
    engine_status="$(sed -n 's/.*shell status = \([0-9][0-9]*\)\..*/\1/p' <<<"$engine_output")"

    if [[ "$native_status" != "$expected_status" || "$engine_status" != "$expected_status" ]]; then
        echo "$lesson: native=$native_status engine=${engine_status:-missing} expected=$expected_status" >&2
        failures=$((failures + 1))
        continue
    fi

    if [[ "$lesson" == "hello" ]]; then
        printf 'Hello world!\n\0' >"$scratch/hello.expected"
        if ! cmp -s "$scratch/hello.expected" "$native_output"; then
            echo "hello: native output bytes differ from Hello world!\\n\\0" >&2
            failures=$((failures + 1))
            continue
        fi
        if [[ "$engine_output" != *'stdout: `Hello world!\n\0`'* ]]; then
            echo "hello: teaching-machine output bytes differ" >&2
            failures=$((failures + 1))
            continue
        fi
    fi

    echo "ok  $lesson  shell=$expected_status"
done

if ((failures > 0)); then
    echo "$failures Lecture 4 differential check(s) failed" >&2
    exit 1
fi

echo "All ${#cases[@]} new Lecture 4 examples match GNU as/ld."
