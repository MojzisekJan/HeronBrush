#!/usr/bin/env python3
"""Types into the game's console through XTest.

Usage: console.py DISPLAY [COMMAND]

Gives the game window the keyboard, then presses the console key. With a
COMMAND it types it and presses Enter, then closes the console again, so the command
runs with the game, not the console, taking the keys. Needs python-xlib.
"""

import sys
import time

from Xlib import X, XK, display
from Xlib.ext import xtest


def press(d, keysym, shift=False):
    code = d.keysym_to_keycode(keysym)
    shift_code = d.keysym_to_keycode(XK.XK_Shift_L)
    if shift:
        xtest.fake_input(d, X.KeyPress, shift_code)
    xtest.fake_input(d, X.KeyPress, code)
    xtest.fake_input(d, X.KeyRelease, code)
    if shift:
        xtest.fake_input(d, X.KeyRelease, shift_code)
    d.sync()
    time.sleep(0.05)


def type_text(d, text):
    for char in text:
        keysym = XK.string_to_keysym(char) if char.isalnum() else XK.string_to_keysym(
            {" ": "space", "_": "underscore", ".": "period", "-": "minus", '"': "quotedbl",
             ";": "semicolon", "/": "slash"}[char]
        )
        shifted = char.isupper() or char in '_"'
        press(d, keysym, shifted)


def main():
    d = display.Display(sys.argv[1])
    screen = d.screen()
    # The game's window is the biggest child of the root.
    windows = screen.root.query_tree().children
    game = max(windows, key=lambda w: w.get_geometry().width * w.get_geometry().height)
    # No click: the game grabs the mouse, and the pointer moving onto the window would
    # turn the player.
    game.set_input_focus(X.RevertToParent, X.CurrentTime)
    d.sync()
    time.sleep(0.3)
    if len(sys.argv) > 2:
        press(d, XK.XK_grave)
        time.sleep(0.3)
        type_text(d, sys.argv[2])
        press(d, XK.XK_Return)
        time.sleep(0.3)
    press(d, XK.XK_grave)


if __name__ == "__main__":
    main()
