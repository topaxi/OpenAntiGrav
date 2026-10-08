#!/usr/bin/env python3
"""Fire one weapon from Pure's player craft in a running Time Trial, and photograph the frames after it.

The Pure half of `psp-weapon-pair.py`, measured 2026-10-08 on PPSSPP 1.20.4 (Pure USA, UCUS98612):
walk the front end by hand to a Time Trial (Venom, Alpha league, Vineta K, Feisar), pause, RESTART,
then run this while the countdown is still on. It holds cross, counts `Ship_UpdateWeapons`
(`0x0892935c`) hits (one per game update, about 30 a second here), and at hit `--skip` writes the
weapon id into the holder's `+0x1d8` (`*(ship+0xd0)`) and sets the fire byte at
`*(*(ship+200)+0x48)+0x15`. Weapon ids: 1 Rocket, 2 Missile, 3 Quake, 4 Disruptor, 5 Turbo,
6 Shield, 7 Autopilot, 8 Plasma, 9 Mine, 10 Bomb (`docs/ghidra/functions/psp-pure-usa/weapons.md`).
Screenshots are taken of the Xvfb root with ImageMagick `import`, the CPU stopped on each hit.

    uv run --with websocket-client scripts/psp-pure-weapon-fire.py 9 data/scratch/<lane>/emu/mine \
        --port 45498 --display :98 --skip 150 --frames 36 --every 1
"""
import sys,time,subprocess,argparse
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger
SHIP_UPDATE_WEAPONS=0x0892935c
ap=argparse.ArgumentParser()
ap.add_argument('weapon',type=int); ap.add_argument('out')
ap.add_argument('--lead',type=float,default=0.0)
ap.add_argument('--skip',type=int,default=0)
ap.add_argument('--frames',type=int,default=40)
ap.add_argument('--every',type=int,default=2)
ap.add_argument('--port',type=int,default=45498)
ap.add_argument('--display',default=':98')
a=ap.parse_args()
d=Debugger(a.port)
def gpr():
    r=d.call('cpu.getAllRegs'); c=next(c for c in r['categories'] if c['name']=='GPR')
    return dict(zip(c['registerNames'],c['uintValues']))
d.hold(cross=True)
time.sleep(a.lead)
for j,_ in d.each_hit(SHIP_UPDATE_WEAPONS, a.skip+a.frames+1):
    i=j-a.skip
    if i<0: continue
    if i==0:
        ship=gpr()['a0']
        holder=d.read_u32(ship+0xd0); rec=d.read_u32(d.read_u32(ship+200)+0x48)
        print('ship',hex(ship),'holder',hex(holder),'rec',hex(rec),'held',d.read_u32(holder+0x1d8),flush=True)
        d.write_u32(holder+0x1d8,a.weapon)
        d.write(rec+0x15,bytes([1]))
    if i % a.every==0:
        subprocess.run(['import','-window','root',f'{a.out}-{i:03d}.png'],env={'DISPLAY':a.display,'PATH':'/usr/bin'})
d.hold(cross=False)
d.resume()
