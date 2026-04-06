#!/usr/bin/env bash

set -euo pipefail

REPORT_ONLY=false
MIN_LINES="${IZEL_MIN_COVERAGE:-100}"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --report-only)
            REPORT_ONLY=true
            shift
            ;;
        --min-lines)
            MIN_LINES="${2:-}"
            shift 2
            ;;
        *)
            echo "Unknown argument: $1"
            echo "Usage: $0 [--report-only] [--min-lines <percent>]"
            exit 2
            ;;
    esac
done

if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
    echo "[missing] cargo-llvm-cov is not installed."
    echo "Install with: cargo install cargo-llvm-cov --locked"
    exit 1
fi

echo "== Izel Coverage Check =="
echo "Target minimum line coverage: ${MIN_LINES}%"

mkdir -p target/coverage
lcov_file="target/coverage/check_coverage.lcov"
output="$(cargo llvm-cov --workspace --all-features --lcov --output-path "$lcov_file" 2>&1)"
echo "$output"

if [[ ! -f "$lcov_file" ]]; then
    echo "[error] Expected LCOV output file was not generated: $lcov_file"
    exit 1
fi

line_percent="$(awk -F'[:,]' '
    /^DA:/ {
        total += 1;
        if ($3 + 0 > 0) {
            covered += 1;
        }
    }
    END {
        if (total == 0) {
            print "0";
        } else {
            printf "%.2f", (covered * 100.0) / total;
        }
    }
' "$lcov_file")"

if [[ -z "$line_percent" ]]; then
    echo "[error] Could not compute line coverage percentage from LCOV data."
    exit 1
fi

if [[ "$REPORT_ONLY" == true ]]; then
    echo "Report-only mode: measured line coverage is ${line_percent}%"
    exit 0
fi

if awk -v got="$line_percent" -v want="$MIN_LINES" 'BEGIN { exit !(got + 0 >= want + 0) }'; then
    echo "[ok] line coverage ${line_percent}% >= ${MIN_LINES}%"
    exit 0
fi

echo "[low] line coverage ${line_percent}% < ${MIN_LINES}%"
exit 1
