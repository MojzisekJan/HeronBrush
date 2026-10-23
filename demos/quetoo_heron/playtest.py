#!/usr/bin/env python3
"""Tries heron_yard_playtest's jump pad and teleporter by respawning on them.

Usage: playtest.py DISPLAY [SECONDS_TO_WAIT] [ROUNDS]

Start the game on DISPLAY first with `+exec playtest.cfg +map heron_yard_playtest`; the
script waits for it to load (90 seconds by default, which a software renderer needs),
then presses keys through XTest. The playtest map has two spawns, one on the jump pad
and one on the teleporter, and Quetoo picks one at random. Each round the player is
killed and respawned, and a screenshot is taken a few seconds later with the
diagnostics showing: a player who spawned on the pad stands on the ledge at x 288,
one who spawned on the teleporter stands at its destination, x 96 on the ledge. Either
way the origin's z is 216, the ledge's top plus 24.

Walking with the movement keys was tried first and dropped: in the nested display the
player sometimes walked the opposite way to the key while the view stayed put.
"""

import sys
import time

from Xlib import X
from Xlib import XK
from Xlib.display import Display
from Xlib.ext import xtest


def main():
    display = Display(sys.argv[1])
    wait = float(sys.argv[2]) if len(sys.argv) > 2 else 90.0
    rounds = int(sys.argv[3]) if len(sys.argv) > 3 else 6

    def key(name, down):
        code = display.keysym_to_keycode(XK.string_to_keysym(name))
        xtest.fake_input(display, X.KeyPress if down else X.KeyRelease, code)
        display.sync()

    def tap(name, seconds=0.2):
        key(name, True)
        time.sleep(seconds)
        key(name, False)
        time.sleep(0.5)

    def shot(step):
        print(f"PLAYTEST {step}", flush=True)
        tap("o")
        tap("j")
        time.sleep(1.5)
        tap("k")
        time.sleep(1.5)
        tap("l")

    time.sleep(wait)
    shot("first spawn")
    for number in range(1, rounds + 1):
        tap("u")
        time.sleep(3)
        tap("i", 0.5)
        time.sleep(4)
        shot(f"respawn {number}")


if __name__ == "__main__":
    main()
