#!/bin/sh
# SPDX-License-Identifier: GPL-2.0-or-later

set -eu

cpu_count=$(getconf _NPROCESSORS_ONLN 2>/dev/null || nproc)
memory_kib=$(awk '/^MemTotal:/ { print $2 }' /proc/meminfo)
available_kib=$(awk '/^MemAvailable:/ { print $2 }' /proc/meminfo)

if [ "$cpu_count" -le 4 ] || [ "$memory_kib" -lt 8388608 ]; then
    mode=responsive
    jobs=1
elif [ "$cpu_count" -ge 8 ] && [ "$memory_kib" -ge 16777216 ]; then
    mode=unrestricted
    jobs=default
else
    mode=balanced
    jobs=$(( (cpu_count + 1) / 2 ))
fi

# A temporarily memory-starved machine should remain usable even when its
# installed hardware would normally select a faster mode.
if [ "$available_kib" -lt 4194304 ] && [ "$mode" != responsive ]; then
    mode=responsive
    jobs=1
fi

printf 'mode=%s\n' "$mode"
printf 'jobs=%s\n' "$jobs"
printf 'cpus=%s\n' "$cpu_count"
printf 'memory_mib=%s\n' "$((memory_kib / 1024))"
printf 'available_mib=%s\n' "$((available_kib / 1024))"
