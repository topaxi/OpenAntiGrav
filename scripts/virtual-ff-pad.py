"""A virtual gamepad with rumble (needs /dev/uinput writable): prints every force-feedback request it receives.

Usage: uv run --with evdev python3 -I scripts/virtual-ff-pad.py SECONDS
"""
import select
import sys
import time
from evdev import UInput, ecodes as e, AbsInfo

duration = float(sys.argv[1])
caps = {
    e.EV_KEY: [e.BTN_SOUTH, e.BTN_EAST, e.BTN_NORTH, e.BTN_WEST, e.BTN_TL, e.BTN_TR, e.BTN_SELECT, e.BTN_START, e.BTN_MODE, e.BTN_THUMBL, e.BTN_THUMBR],
    e.EV_ABS: [(e.ABS_X, AbsInfo(0, -32768, 32767, 16, 128, 0)), (e.ABS_Y, AbsInfo(0, -32768, 32767, 16, 128, 0)),
               (e.ABS_RX, AbsInfo(0, -32768, 32767, 16, 128, 0)), (e.ABS_RY, AbsInfo(0, -32768, 32767, 16, 128, 0)),
               (e.ABS_Z, AbsInfo(0, 0, 255, 0, 0, 0)), (e.ABS_RZ, AbsInfo(0, 0, 255, 0, 0, 0)),
               (e.ABS_HAT0X, AbsInfo(0, -1, 1, 0, 0, 0)), (e.ABS_HAT0Y, AbsInfo(0, -1, 1, 0, 0, 0))],
    e.EV_FF: [e.FF_RUMBLE, e.FF_PERIODIC, e.FF_SQUARE, e.FF_TRIANGLE, e.FF_SINE, e.FF_GAIN],
}
ui = UInput(caps, name="OAG test virtual pad", vendor=0x045e, product=0x028e, version=0x0110, bustype=e.BUS_USB)
print("READY", ui.device.path, flush=True)
t0 = time.monotonic()
log = []
while time.monotonic() - t0 < duration:
    r, _, _ = select.select([ui.fd], [], [], 0.05)
    if not r:
        continue
    for ev in ui.read():
        now = time.monotonic() - t0
        if ev.type == e.EV_UINPUT and ev.code == e.UI_FF_UPLOAD:
            up = ui.begin_upload(ev.value)
            rum = up.effect.u.ff_rumble_effect
            print(f"{now:8.3f} UPLOAD id={up.effect.id} type={up.effect.type} strong={rum.strong_magnitude} weak={rum.weak_magnitude} len={up.effect.ff_replay.length}", flush=True)
            up.retval = 0
            ui.end_upload(up)
        elif ev.type == e.EV_UINPUT and ev.code == e.UI_FF_ERASE:
            er = ui.begin_erase(ev.value); er.retval = 0; ui.end_erase(er)
            print(f"{now:8.3f} ERASE id={er.effect_id}", flush=True)
        elif ev.type == e.EV_FF:
            print(f"{now:8.3f} EV_FF code={ev.code} value={ev.value}", flush=True)
ui.close()
