#!/usr/bin/env bash

print_hardware_profile() {
  local threads cores max_mhz mem_kib mem_gib mem_mts

  threads="$(getconf _NPROCESSORS_ONLN 2>/dev/null)"
  [ -n "$threads" ] || threads=unknown

  cores="$(awk -F: '/^core id/ { gsub(/ /, "", $2); print $2 }' /proc/cpuinfo 2>/dev/null | sort -u | wc -l | tr -d ' ')"
  [ "$cores" = "0" ] && cores=unknown
  [ -n "$cores" ] || cores=unknown

  max_mhz="$(lscpu 2>/dev/null | awk -F: '/^CPU max MHz/ { gsub(/ /, "", $2); printf "%.0f", $2 }')"
  if [ -z "$max_mhz" ]; then
    max_mhz="$(awk -F: '/^cpu MHz/ { gsub(/ /, "", $2); printf "%.0f", $2; exit }' /proc/cpuinfo 2>/dev/null)"
  fi
  [ -n "$max_mhz" ] || max_mhz=unknown

  mem_kib="$(awk '/^MemTotal:/ { print $2 }' /proc/meminfo 2>/dev/null)"
  if [ -n "$mem_kib" ]; then
    mem_gib="$(awk -v kib="$mem_kib" 'BEGIN { printf "%.1f", kib / 1048576 }')"
  else
    mem_gib=unknown
  fi

  mem_mts="$(dmidecode -t memory 2>/dev/null | awk -F: '/Configured Memory Speed/ { gsub(/[^0-9]/, "", $2); if ($2 != "") { print $2; exit } }')"
  [ -n "$mem_mts" ] || mem_mts=unavailable

  echo "cpu_cores=$cores"
  echo "cpu_threads=$threads"
  echo "cpu_max_mhz=$max_mhz"
  echo "mem_total_gib=$mem_gib"
  echo "mem_speed_mts=$mem_mts"
}
