#!/system/bin/sh
# On-device sampler for a phone race: one line a second to OUT, for SECONDS seconds.
# Pushed and run by hand (docs/tools/android.md, "Performance on the Galaxy S21"):
#
#   adb push scripts/android-perf-poll.sh /data/local/tmp/
#   adb shell "nohup sh /data/local/tmp/android-perf-poll.sh /data/local/tmp/poll.txt 600 >/dev/null 2>&1 &"
#
# Columns: seconds, GPU busy %, GPU clock and its current cap (kHz), then the current
# and maximum clock of CPU policies 0 (A55), 4 (A78) and 7 (X1) as an Exynos 2100 numbers
# them, then every fifth second the thermal status and the AP, SKIN, BAT and PA
# temperatures from the thermal HAL. OUT.mem holds `dumpsys meminfo` every ten seconds and
# OUT.stat the per-core /proc/stat counters every second. A phone whose GPU does not
# expose /sys/kernel/gpu prints errors in those columns and nothing else breaks.
out=$1
secs=$2
pkg=org.openantigrav.game
i=0
echo "# t_s gpu_busy gpu_clock gpu_max cpu0cur cpu0max cpu4cur cpu4max cpu7cur cpu7max thermal_status ap skin bat pa" > "$out"
while [ "$i" -lt "$secs" ]; do
  gb=$(cat /sys/kernel/gpu/gpu_busy)
  gc=$(cat /sys/kernel/gpu/gpu_clock)
  gm=$(cat /sys/kernel/gpu/gpu_max_clock)
  c=""
  for p in 0 4 7; do
    d=/sys/devices/system/cpu/cpufreq/policy$p
    c="$c $(cat $d/scaling_cur_freq) $(cat $d/scaling_max_freq)"
  done
  th=""
  if [ $((i % 5)) -eq 0 ]; then
    t=$(dumpsys thermalservice)
    st=$(echo "$t" | grep -m1 'Thermal Status' | sed 's/.*: //')
    v() { echo "$t" | sed -n '/Current temperatures from HAL/,/Current cooling/p' | grep "mName=$1," | sed 's/.*mValue=\([0-9.]*\),.*/\1/'; }
    th="$st $(v AP) $(v SKIN) $(v BAT) $(v PA)"
  fi
  echo "$i ${gb%\%} $gc $gm$c $th" >> "$out"
  if [ $((i % 10)) -eq 0 ]; then
    echo "== $i" >> "$out.mem"
    dumpsys meminfo $pkg 2>/dev/null | sed -n '/App Summary/,/TOTAL SWAP/p' >> "$out.mem"
    dumpsys meminfo $pkg 2>/dev/null | grep -E '^ +(TOTAL|Graphics|GL mtrack|EGL mtrack|Native Heap|Private Other|Unknown)' | head -12 >> "$out.mem"
  fi
  echo "== $i" >> "$out.stat"
  grep '^cpu' /proc/stat >> "$out.stat"
  i=$((i + 1))
  sleep 1
done
