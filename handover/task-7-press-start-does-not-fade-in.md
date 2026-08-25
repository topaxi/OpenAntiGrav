# Task #7: PRESS START does not fade in or throb

`BOOT_PRESS_START` carries `pulse="true"` and `delay="1"`; ours appears at once, at full opacity, and stands still. Neither attribute is decoded. The two numbers needed are the period between peaks and the min/max alpha, both readable off a real display: `just launch-pulse-psp data/images/pulse-psp-eu.chd`, reach PRESS START, record, step frames over ~4 seconds. **Do not measure it off our own build** - ours is static, so a capture would confirm nothing. Full context on [frontend-boot.md](../docs/architecture/frontend-boot.md).

## Open

- `pulse` and `delay` attributes on `BOOT_PRESS_START` are not decoded; ours renders static at full opacity
- The period between pulse peaks and the min/max alpha are both still unmeasured

## Next Steps

- Capture off the real PSP via `just launch-pulse-psp data/images/pulse-psp-eu.chd`: reach PRESS START, record, step frames over ~4 seconds to read the period and min/max alpha
