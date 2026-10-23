#!/usr/bin/env bash
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
. "$SCRIPT_DIR/hardware_profile.sh"

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

profile_output_returns_expected_keys
profile_output_returns_one_value_per_key
profile_output_omits_identifying_fields
profile_output_returns_numeric_or_sentinel_values

if [ "$FAILURES" -gt 0 ]; then
  echo "$FAILURES assertion(s) failed" >&2
  exit 1
fi

echo "all assertions passed"
