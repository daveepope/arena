#!/usr/bin/env bash
set -euo pipefail

# --- begin runfiles.bash initialization v3 ---
set -uo pipefail; set +e; f=bazel_tools/tools/bash/runfiles/runfiles.bash
source "${RUNFILES_DIR:-/dev/null}/$f" 2>/dev/null || \
  source "$(grep -sm1 "^$f " "${RUNFILES_MANIFEST_FILE:-/dev/null}" | cut -f2- -d' ')" 2>/dev/null || \
  source "$0.runfiles/$f" 2>/dev/null || \
  source "$(grep -sm1 "^$f " "$0.runfiles_manifest" | cut -f2- -d' ')" 2>/dev/null || \
  source "$(grep -sm1 "^$f " "$0.exe.runfiles_manifest" | cut -f2- -d' ')" 2>/dev/null || \
  { echo >&2 "ERROR: cannot find $f"; exit 1; }; f=; set -e
# --- end runfiles.bash initialization v3 ---
runfiles_export_envvars

BINARY="$(rlocation TEMPLATED_binary)"

if [[ -z "${COVERAGE_DIR:-}" ]]; then
  exec "$BINARY" "$@"
fi

COVERAGE_TOOL="$(rlocation TEMPLATED_coverage_tool)"
LCOV_CONVERTER="$(rlocation TEMPLATED_lcov_converter)"
COBERTURA_FILE="$COVERAGE_DIR/dotnet_coverage.cobertura.xml"

"$COVERAGE_TOOL" collect -f cobertura -if "**/*.dll" -o "$COBERTURA_FILE" -- "$BINARY" "$@"
"$LCOV_CONVERTER" "$COBERTURA_FILE" "$COVERAGE_OUTPUT_FILE"
