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
    "echo:echo.s:0"
    "helloret:helloRet.s:14"
    "routine:routine.s:40"
    "fun1:fun1.s:80"
    "fun2:fun2.s:40"
    "funstack:funStack.s:40"
)

cargo build --quiet -p x86-63-cli

failures=0
for specification in "${cases[@]}"; do
    IFS=: read -r lesson file expected_status <<<"$specification"
    object="$scratch/$lesson.o"
    executable="$scratch/$lesson"
    native_output="$scratch/$lesson.out"

    as "course-content/lecture5/$file" -o "$object"
    ld "$object" -o "$executable"
    set +e
    if [[ "$lesson" == "echo" ]]; then
        printf 'CS201\n' | "$executable" >"$native_output"
        native_status=${PIPESTATUS[1]}
    else
        "$executable" >"$native_output"
        native_status=$?
    fi
    set -e

    if [[ "$lesson" == "echo" ]]; then
        engine_output="$(./target/debug/x86-63 run --example "$lesson" --stdin CS201)"
    else
        engine_output="$(./target/debug/x86-63 run --example "$lesson")"
    fi
    engine_status="$(sed -n 's/.*shell status = \([0-9][0-9]*\)\..*/\1/p' <<<"$engine_output")"

    if [[ "$native_status" != "$expected_status" || "$engine_status" != "$expected_status" ]]; then
        echo "$lesson: native=$native_status engine=${engine_status:-missing} expected=$expected_status" >&2
        failures=$((failures + 1))
        continue
    fi

    if [[ "$lesson" == "echo" ]]; then
        printf 'CS201\n' >"$scratch/echo.expected"
        if ! cmp -s "$scratch/echo.expected" "$native_output"; then
            echo "echo: native output bytes differ from submitted input" >&2
            failures=$((failures + 1))
            continue
        fi
        if [[ "$engine_output" != *'stdout: `CS201\n`'* ]]; then
            echo "echo: teaching-machine output bytes differ" >&2
            failures=$((failures + 1))
            continue
        fi
    fi

    if [[ "$lesson" == "helloret" ]]; then
        printf 'Hello world!\n\0' >"$scratch/helloret.expected"
        if ! cmp -s "$scratch/helloret.expected" "$native_output"; then
            echo "helloret: native output bytes differ from Hello world!\\n\\0" >&2
            failures=$((failures + 1))
            continue
        fi
    fi

    echo "ok  $lesson  shell=$expected_status"
done

if ((failures > 0)); then
    echo "$failures Lecture 5 differential check(s) failed" >&2
    exit 1
fi

echo "All ${#cases[@]} Lecture 5 examples match GNU as/ld."
