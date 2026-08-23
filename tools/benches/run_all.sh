#!/usr/bin/env bash
set -uo pipefail

if [ -n "${BUILD_WORKSPACE_DIRECTORY:-}" ]; then
  PROJECT_DIR="$BUILD_WORKSPACE_DIRECTORY/tools/benches"
else
  PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fi

USAGE="usage: run_all.sh --python <version> --java <version> --dotnet <version> [--python-pre-release] [--iterations N]"
PYPI_VERSION=""
MAVEN_VERSION=""
DOTNET_VERSION=""
PYPI_PRE_RELEASE=false
ITERATIONS=10

while [ $# -gt 0 ]; do
  case "$1" in
    --python) PYPI_VERSION="$2"; shift 2 ;;
    --java) MAVEN_VERSION="$2"; shift 2 ;;
    --dotnet) DOTNET_VERSION="$2"; shift 2 ;;
    --python-pre-release) PYPI_PRE_RELEASE=true; shift ;;
    --iterations) ITERATIONS="$2"; shift 2 ;;
    *) echo "run_all.sh: unknown argument '$1'" >&2; echo "$USAGE" >&2; exit 1 ;;
  esac
done

if [ -z "$PYPI_VERSION" ] || [ -z "$MAVEN_VERSION" ] || [ -z "$DOTNET_VERSION" ]; then
  echo "$USAGE" >&2
  exit 1
fi

declare -A RESULTS
declare -A REQUESTED_VERSION
REQUESTED_VERSION[pypi]="$PYPI_VERSION"
REQUESTED_VERSION[maven]="$MAVEN_VERSION"
REQUESTED_VERSION[dotnet]="$DOTNET_VERSION"

PYPI_PRE_RELEASE_ARGS=()
if [ "$PYPI_PRE_RELEASE" = true ]; then
  PYPI_PRE_RELEASE_ARGS=(--pre-release)
fi
if RESULTS[pypi]="$(python3 "$PROJECT_DIR/pypi/bench_pypi.py" --version "$PYPI_VERSION" --iterations "$ITERATIONS" "${PYPI_PRE_RELEASE_ARGS[@]}")"; then
  :
else
  RESULTS[pypi]="FAILED"
fi

if RESULTS[maven]="$("$PROJECT_DIR/maven/run.sh" "$MAVEN_VERSION" "$ITERATIONS")"; then
  :
else
  RESULTS[maven]="FAILED"
fi

if RESULTS[dotnet]="$("$PROJECT_DIR/dotnet/run.sh" "$DOTNET_VERSION" "$ITERATIONS")"; then
  :
else
  RESULTS[dotnet]="FAILED"
fi

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

print_separator
printf "$ROW_FORMAT" "${COL_TITLES[@]}"
print_separator
FAILED_LANGS=()
for lang in pypi maven dotnet; do
  line="${RESULTS[$lang]}"
  if [ "$line" = "FAILED" ]; then
    printf "$ROW_FORMAT" "$lang" "${REQUESTED_VERSION[$lang]}" "FAILED" "-" "-" "-" "-" "-" "-"
    FAILED_LANGS+=("$lang")
    continue
  fi
  version="$(grep -oE 'version=[^ ]+' <<<"$line" | cut -d= -f2)"
  iterations="$(grep -oE 'iterations=[0-9]+' <<<"$line" | cut -d= -f2)"
  open_ms="$(grep -oE 'open_ms=[0-9.]+' <<<"$line" | cut -d= -f2)"
  interact_min_ms="$(grep -oE 'interact_min_ms=[0-9.]+' <<<"$line" | cut -d= -f2)"
  interact_ms="$(grep -oE 'interact_ms=[0-9.]+' <<<"$line" | cut -d= -f2)"
  interact_p95_ms="$(grep -oE 'interact_p95_ms=[0-9.]+' <<<"$line" | cut -d= -f2)"
  close_ms="$(grep -oE 'close_ms=[0-9.]+' <<<"$line" | cut -d= -f2)"
  e2e_ms="$(grep -oE 'e2e_ms=[0-9.]+' <<<"$line" | cut -d= -f2)"
  printf "$ROW_FORMAT" \
    "$lang" "$version" "$iterations" "$open_ms" "$interact_min_ms" "$interact_ms" "$interact_p95_ms" "$close_ms" "$e2e_ms"
done
print_separator

if [ "${#FAILED_LANGS[@]}" -gt 0 ]; then
  echo "failed: ${FAILED_LANGS[*]} (see output above for each failure's error)" >&2
  exit 1
fi
