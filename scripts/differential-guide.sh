#!/usr/bin/env bash
# Assemble every assembly-guide fixture with GNU as/ld, run it natively, and
# compare its stdout bytes and shell status with `x86-63 exec`.
set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root_dir"

for tool in as ld cargo cmp; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "missing required tool: $tool" >&2
        exit 1
    fi
done

scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
cargo build --quiet -p x86-63-cli
engine=./target/debug/x86-63
helpers=(course-content/lecture6/readInt.s course-content/lecture6/writeInt.s)

objects_for_helpers=()
for helper in "${helpers[@]}"; do
    object="$scratch/$(basename "${helper%.s}").o"
    as "$helper" -o "$object"
    objects_for_helpers+=("$object")
done

# fixture  stdin  (empty stdin means the program does not read)
cases=(
    "localLabels.s|"
    "signOfNumber.s|"
    "sumOneToTen.s|"
    "maxOfArray.s|"
    "absoluteValue.s|"
    "localVariables.s|"
    "recursiveFactorial.s|"
    "sumToN.s|10"
    "signedDivide.s|"
)

failures=0
for case in "${cases[@]}"; do
    fixture="${case%%|*}"
    input="${case#*|}"
    source="course-content/guide/$fixture"
    name="${fixture%.s}"
    as "$source" -o "$scratch/$name.o"
    ld "$scratch/$name.o" "${objects_for_helpers[@]}" -o "$scratch/$name"

    set +e
    printf '%s' "${input:+$input$'\n'}" | "$scratch/$name" >"$scratch/$name.native"
    native_status=${PIPESTATUS[1]}
    printf '%s' "${input:+$input$'\n'}" | "$engine" exec "$source" "${helpers[@]}" >"$scratch/$name.engine"
    engine_status=${PIPESTATUS[1]}
    set -e

    if [[ "$native_status" != "$engine_status" ]]; then
        echo "FAIL $name: native status $native_status, engine status $engine_status" >&2
        failures=$((failures + 1))
    elif ! cmp -s "$scratch/$name.native" "$scratch/$name.engine"; then
        echo "FAIL $name: stdout bytes differ" >&2
        failures=$((failures + 1))
    else
        echo "ok  $name  status=$native_status  stdout=$(wc -c <"$scratch/$name.native") bytes"
    fi
done

if ((failures > 0)); then
    echo "$failures guide differential check(s) failed" >&2
    exit 1
fi
echo "All ${#cases[@]} assembly-guide fixtures match GNU as/ld."
