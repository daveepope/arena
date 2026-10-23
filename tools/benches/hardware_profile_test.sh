#!/usr/bin/env bash
# --- begin runfiles.bash initialization v3 ---
set -uo pipefail; set +e; f=bazel_tools/tools/bash/runfiles/runfiles.bash
source "${RUNFILES_DIR:-/dev/null}/$f" 2>/dev/null || \
  source "$(grep -sm1 "^$f " "${RUNFILES_MANIFEST_FILE:-/dev/null}" | cut -f2- -d' ')" 2>/dev/null || \
  source "$0.runfiles/$f" 2>/dev/null || \
  source "$(grep -sm1 "^$f " "$0.runfiles_manifest" | cut -f2- -d' ')" 2>/dev/null || \
  source "$(grep -sm1 "^$f " "$0.exe.runfiles_manifest" | cut -f2- -d' ')" 2>/dev/null || \
  { echo>&2 "ERROR: cannot find $f"; exit 1; }; f=; set -e
runfiles_export_envvars
# --- end runfiles.bash initialization v3 ---

set +e
set -uo pipefail

PROFILE_SH="$(rlocation _main/tools/benches/hardware_profile.sh)"
if [ -z "$PROFILE_SH" ] || [ ! -f "$PROFILE_SH" ]; then
  echo "hardware_profile_test.sh: could not locate hardware_profile.sh in runfiles" >&2
  exit 1
fi
. "$PROFILE_SH"

FAILURES=0

fail() {
  echo "FAIL: $1" >&2
  FAILURES=$((FAILURES + 1))
}

profile_output_returns_expected_keys() {
  local output expected actual
  output="$(print_hardware_profile)"
  expected="cpu_cores
cpu_threads
cpu_max_mhz
mem_total_gib
mem_speed_mts"
  actual="$(cut -d= -f1 <<<"$output")"
  if [ "$actual" != "$expected" ]; then
    fail "profile_output_returns_expected_keys: got '$actual'"
  fi
}

profile_output_returns_one_value_per_key() {
  local line key value
  while IFS= read -r line; do
    key="${line%%=*}"
    value="${line#*=}"
    if [ "$key" = "$line" ]; then
      fail "profile_output_returns_one_value_per_key: '$line' has no '='"
    elif [ -z "$value" ]; then
      fail "profile_output_returns_one_value_per_key: '$key' is empty"
    fi
  done <<<"$(print_hardware_profile)"
}

profile_output_omits_identifying_fields() {
  local output
  output="$(print_hardware_profile | tr '[:upper:]' '[:lower:]')"
  for banned in intel amd ryzen core\ i apple serial uuid hostname vendor model; do
    if grep -qF "$banned" <<<"$output"; then
      fail "profile_output_omits_identifying_fields: leaked '$banned'"
    fi
  done
}

profile_output_returns_numeric_or_sentinel_values() {
  local line key value
  while IFS= read -r line; do
    key="${line%%=*}"
    value="${line#*=}"
    case "$value" in
      unknown | unavailable) ;;
      *[!0-9.]* | "") fail "profile_output_returns_numeric_or_sentinel_values: '$key=$value'" ;;
    esac
  done <<<"$(print_hardware_profile)"
}

profile_output_strips_carriage_returns() {
  local output
  output="$(print_hardware_profile)"
  if [[ "$output" == *$'\r'* ]]; then
    fail "profile_output_strips_carriage_returns: output contains a carriage return"
  fi
}

profile_output_crlf_tool_output_returns_stripped_value() {
  local output value
  getconf() { printf '16\r\n'; }
  output="$(print_hardware_profile)"
  unset -f getconf
  value="$(grep '^cpu_threads=' <<<"$output" | cut -d= -f2)"
  if [ "$value" != "16" ]; then
    fail "profile_output_crlf_tool_output_returns_stripped_value: got '$(printf '%q' "$value")'"
  fi
}

profile_output_returns_expected_keys
profile_output_returns_one_value_per_key
profile_output_omits_identifying_fields
profile_output_returns_numeric_or_sentinel_values
profile_output_strips_carriage_returns
profile_output_crlf_tool_output_returns_stripped_value

if [ "$FAILURES" -gt 0 ]; then
  echo "$FAILURES assertion(s) failed" >&2
  exit 1
fi

echo "all assertions passed"
