#!/usr/bin/env bash
set -uo pipefail

if [ -n "${BUILD_WORKSPACE_DIRECTORY:-}" ]; then
  PROJECT_DIR="$BUILD_WORKSPACE_DIRECTORY/tools/benches"
  REPO_ROOT="$BUILD_WORKSPACE_DIRECTORY"
else
  PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  REPO_ROOT="$(cd "$PROJECT_DIR/../.." && pwd)"
fi

RUNFILES="${RUNFILES_DIR:-${BASH_SOURCE[0]}.runfiles}"
BENCH_PYPI="$RUNFILES/_main/tools/benches/pypi/bench_pypi"
BENCH_MAVEN="$RUNFILES/_main/tools/benches/maven/bench_maven"
BENCH_DOTNET="$RUNFILES/_main/tools/benches/dotnet/bench_dotnet"
for bench in "$BENCH_PYPI" "$BENCH_MAVEN" "$BENCH_DOTNET"; do
  if [ ! -x "$bench" ]; then
    echo "run_all.sh: $(basename "$bench") not found in runfiles; run via 'bazel run //tools/benches:bench_all'" >&2
    exit 1
  fi
done

. "$PROJECT_DIR/hardware_profile.sh"

USAGE="usage: run_all.sh --python <version> --java <version> --dotnet <version> [--python-pre-release]
                   [--baseline-python <version>] [--baseline-java <version>] [--baseline-dotnet <version>]
                   [--baseline-python-pre-release] [--iterations N]"
PYPI_VERSION=""
MAVEN_VERSION=""
DOTNET_VERSION=""
PYPI_PRE_RELEASE=false
BASE_PYPI_VERSION=""
BASE_MAVEN_VERSION=""
BASE_DOTNET_VERSION=""
BASE_PYPI_PRE_RELEASE=false
ITERATIONS=50

while [ $# -gt 0 ]; do
  case "$1" in
    --python) PYPI_VERSION="$2"; shift 2 ;;
    --java) MAVEN_VERSION="$2"; shift 2 ;;
    --dotnet) DOTNET_VERSION="$2"; shift 2 ;;
    --python-pre-release) PYPI_PRE_RELEASE=true; shift ;;
    --baseline-python) BASE_PYPI_VERSION="$2"; shift 2 ;;
    --baseline-java) BASE_MAVEN_VERSION="$2"; shift 2 ;;
    --baseline-dotnet) BASE_DOTNET_VERSION="$2"; shift 2 ;;
    --baseline-python-pre-release) BASE_PYPI_PRE_RELEASE=true; shift ;;
    --iterations) ITERATIONS="$2"; shift 2 ;;
    *) echo "run_all.sh: unknown argument '$1'" >&2; echo "$USAGE" >&2; exit 1 ;;
  esac
done

if [ -z "$PYPI_VERSION" ] || [ -z "$MAVEN_VERSION" ] || [ -z "$DOTNET_VERSION" ]; then
  echo "$USAGE" >&2
  exit 1
fi

HAS_BASELINE=false
if [ -n "$BASE_PYPI_VERSION" ] || [ -n "$BASE_MAVEN_VERSION" ] || [ -n "$BASE_DOTNET_VERSION" ]; then
  if [ -z "$BASE_PYPI_VERSION" ] || [ -z "$BASE_MAVEN_VERSION" ] || [ -z "$BASE_DOTNET_VERSION" ]; then
    echo "run_all.sh: a baseline needs all three of --baseline-python, --baseline-java, --baseline-dotnet" >&2
    exit 1
  fi
  HAS_BASELINE=true
fi

declare -A RESULTS
declare -A REQUESTED_VERSION
FAILED_LANGS=()

run_suite() {
  local label="$1" pypi_version="$2" maven_version="$3" dotnet_version="$4" pre_release="$5"
  local pre_release_args=()
  [ "$pre_release" = true ] && pre_release_args=(--pre-release)

  REQUESTED_VERSION[$label.pypi]="$pypi_version"
  REQUESTED_VERSION[$label.maven]="$maven_version"
  REQUESTED_VERSION[$label.dotnet]="$dotnet_version"

  if ! RESULTS[$label.pypi]="$("$BENCH_PYPI" --version "$pypi_version" --iterations "$ITERATIONS" "${pre_release_args[@]}")"; then
    RESULTS[$label.pypi]="FAILED"
    FAILED_LANGS+=("$label/pypi")
  fi
  if ! RESULTS[$label.maven]="$("$BENCH_MAVEN" "$maven_version" "$ITERATIONS")"; then
    RESULTS[$label.maven]="FAILED"
    FAILED_LANGS+=("$label/maven")
  fi
  if ! RESULTS[$label.dotnet]="$("$BENCH_DOTNET" "$dotnet_version" "$ITERATIONS")"; then
    RESULTS[$label.dotnet]="FAILED"
    FAILED_LANGS+=("$label/dotnet")
  fi
}

if [ "$HAS_BASELINE" = true ]; then
  run_suite baseline "$BASE_PYPI_VERSION" "$BASE_MAVEN_VERSION" "$BASE_DOTNET_VERSION" "$BASE_PYPI_PRE_RELEASE"
fi
run_suite candidate "$PYPI_VERSION" "$MAVEN_VERSION" "$DOTNET_VERSION" "$PYPI_PRE_RELEASE"

COL_WIDTHS=(8 24 7 10 13 13 13 10 10)
COL_TITLES=("lang" "version" "iters" "open_ms" "interact_min" "interact_med" "interact_p95" "close_ms" "e2e_ms")

ROW_FORMAT="|"
for width in "${COL_WIDTHS[@]}"; do
  ROW_FORMAT+=" %-${width}s |"
done
ROW_FORMAT+="\n"

print_separator() {
  local line="+"
  for width in "${COL_WIDTHS[@]}"; do
    line+="$(printf -- '-%.0s' $(seq 1 $((width + 2))))+"
  done
  echo "$line"
}

field() {
  grep -oE "$2=[0-9.]+" <<<"$1" | cut -d= -f2
}

print_table() {
  local label="$1" lang line version iterations
  print_separator
  printf "$ROW_FORMAT" "${COL_TITLES[@]}"
  print_separator
  for lang in pypi maven dotnet; do
    line="${RESULTS[$label.$lang]}"
    if [ "$line" = "FAILED" ]; then
      printf "$ROW_FORMAT" "$lang" "${REQUESTED_VERSION[$label.$lang]}" "FAILED" "-" "-" "-" "-" "-" "-"
      continue
    fi
    version="$(grep -oE 'version=[^ ]+' <<<"$line" | cut -d= -f2)"
    iterations="$(field "$line" iterations)"
    printf "$ROW_FORMAT" \
      "$lang" "$version" "$iterations" \
      "$(field "$line" open_ms)" "$(field "$line" interact_min_ms)" "$(field "$line" interact_ms)" \
      "$(field "$line" interact_p95_ms)" "$(field "$line" close_ms)" "$(field "$line" e2e_ms)"
  done
  print_separator
}

DELTA_WIDTHS=(8 24 24 12 12)
DELTA_TITLES=("lang" "baseline" "candidate" "e2e_delta" "e2e_pct")
DELTA_FORMAT="|"
for width in "${DELTA_WIDTHS[@]}"; do
  DELTA_FORMAT+=" %-${width}s |"
done
DELTA_FORMAT+="\n"

print_delta_separator() {
  local line="+"
  for width in "${DELTA_WIDTHS[@]}"; do
    line+="$(printf -- '-%.0s' $(seq 1 $((width + 2))))+"
  done
  echo "$line"
}

print_delta_table() {
  local lang base_line cand_line base_e2e cand_e2e delta pct
  print_delta_separator
  printf "$DELTA_FORMAT" "${DELTA_TITLES[@]}"
  print_delta_separator
  for lang in pypi maven dotnet; do
    base_line="${RESULTS[baseline.$lang]}"
    cand_line="${RESULTS[candidate.$lang]}"
    base_e2e="$(field "$base_line" e2e_ms)"
    cand_e2e="$(field "$cand_line" e2e_ms)"
    if [ -z "$base_e2e" ] || [ -z "$cand_e2e" ]; then
      printf "$DELTA_FORMAT" "$lang" "${REQUESTED_VERSION[baseline.$lang]}" "${REQUESTED_VERSION[candidate.$lang]}" "-" "-"
      continue
    fi
    delta="$(awk -v b="$base_e2e" -v c="$cand_e2e" 'BEGIN { printf "%+.2f", c - b }')"
    pct="$(awk -v b="$base_e2e" -v c="$cand_e2e" 'BEGIN { if (b == 0) print "-"; else printf "%+.1f%%", (c - b) * 100 / b }')"
    printf "$DELTA_FORMAT" "$lang" "${REQUESTED_VERSION[baseline.$lang]}" "${REQUESTED_VERSION[candidate.$lang]}" "$delta" "$pct"
  done
  print_delta_separator
}

ARENA_VERSION="$(tr -d '[:space:]' < "$REPO_ROOT/VERSION" 2>/dev/null || echo unknown)"
COMMIT="$(git -C "$REPO_ROOT" rev-parse --short HEAD 2>/dev/null || echo unknown)"
RUN_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
RUN_AT_SLUG="$(date -u +%Y-%m-%dT%H-%M-%SZ)"
RESULTS_DIR="$PROJECT_DIR/results"
RESULTS_FILE="$RESULTS_DIR/${ARENA_VERSION}_${COMMIT}_${RUN_AT_SLUG}.txt"
mkdir -p "$RESULTS_DIR"

render_report() {
  echo "arena_version=$ARENA_VERSION"
  echo "commit=$COMMIT"
  echo "run_at=$RUN_AT"
  echo "iterations=$ITERATIONS"
  print_hardware_profile
  if [ "$HAS_BASELINE" = true ]; then
    echo
    echo "[baseline]"
    print_table baseline
  fi
  echo
  echo "[candidate]"
  print_table candidate
  if [ "$HAS_BASELINE" = true ]; then
    echo
    echo "[delta]"
    print_delta_table
  fi
}

render_report > "$RESULTS_FILE"
cat "$RESULTS_FILE"
echo "recorded $RESULTS_FILE"

if [ "${#FAILED_LANGS[@]}" -gt 0 ]; then
  echo "failed: ${FAILED_LANGS[*]} (see output above for each failure's error)" >&2
  exit 1
fi
