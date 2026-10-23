#!/usr/bin/env python3
"""Plays heron_lift in Neverball through to the goal, on an X display of its own.

Usage: playtest.py DISPLAY SHOTS_DIR

Neverball must be showing the level's start, in a 1024x768 window, with a top-down
camera: view_fov 50, view_dp 900, view_dc 0 and view_dz 250 in its neverballrc. The
script clicks past the intro, picks the manual camera (key 3) and drives the ball up
the middle: the start pad's coins, the ramp, the switch, the red coin, onto the lift
when it comes in, across, and into the goal. It saves a screenshot at each step.

The ball stays at the centre of the screen and the floor slides under it. How far it
slid is measured by phase correlation between consecutive frames; the sum is the ball's
position from the known start. Tilt is the arrow keys, held for part of each tick. On
the lift the ball is kept over the lift's middle, which is found by its colour.

Needs numpy, Pillow and python-xlib (XTest).
"""
import sys
import time

import numpy as np
from PIL import Image
from Xlib import X, XK, display
from Xlib.ext import xtest

SCALE = 1.25  # pixels per map unit, measured from runs: the camera looks down at an angle
TICK = 0.08
SHOTS = sys.argv[2]

d = display.Display(sys.argv[1])
root = d.screen().root
codes = {name: d.keysym_to_keycode(XK.string_to_keysym(name)) for name in ("Up", "Down", "Left", "Right")}
held = set()


def press(name, down):
    if down and name not in held:
        xtest.fake_input(d, X.KeyPress, codes[name])
        held.add(name)
    elif not down and name in held:
        xtest.fake_input(d, X.KeyRelease, codes[name])
        held.discard(name)


def release_all():
    for name in list(held):
        press(name, False)
    d.sync()


def grab():
    raw = root.get_image(0, 0, 1024, 768, X.ZPixmap, 0xFFFFFFFF)
    rgb = np.frombuffer(raw.data, dtype=np.uint8).reshape(768, 1024, 4)[:, :, 2::-1]
    return rgb


def gray(rgb):
    g = rgb[64:704:2, 128:896:2].astype(np.float32).mean(axis=2)  # 320 x 384
    g -= g.mean()
    # The ball sits still at the centre; leave it out.
    cy, cx = 384 // 2 - 32, 384 // 2
    g[cy - 25:cy + 25, cx - 25:cx + 25] = 0
    return g * WINDOW


WINDOW = np.outer(np.hanning(320), np.hanning(384)).astype(np.float32)


def shift(a, b):
    """How far b's content moved from a's, in (down, right) half-resolution pixels."""
    fa = np.fft.fft2(a)
    fb = np.fft.fft2(b)
    r = fb * np.conj(fa)
    r /= np.abs(r) + 1e-6
    c = np.fft.ifft2(r).real
    y, x = np.unravel_index(np.argmax(c), c.shape)
    if y > c.shape[0] // 2:
        y -= c.shape[0]
    if x > c.shape[1] // 2:
        x -= c.shape[1]
    # sub-pixel by parabola
    def sub(m, p, n):
        den = m - 2 * p + n
        return 0.0 if den == 0 else 0.5 * (m - n) / den
    yy, xx = y % c.shape[0], x % c.shape[1]
    dy = sub(c[(yy - 1) % c.shape[0], xx], c[yy, xx], c[(yy + 1) % c.shape[0], xx])
    dx = sub(c[yy, (xx - 1) % c.shape[1]], c[yy, xx], c[yy, (xx + 1) % c.shape[1]])
    return y + dy, x + dx


class Tracker:
    def __init__(self, x, y):
        self.x, self.y = x, y
        self.vx = self.vy = 0.0
        # The game pulls the camera back as the ball rolls away from it, by
        # 1 + (speed away in m/s) / 10, eased in at a rate of one per second
        # (ball/game_server.c, game_update_view). The floor looks smaller by as much.
        self.k = 1.0
        self.prev = gray(grab())
        self.t = time.time()

    def update(self):
        rgb = grab()
        g = gray(rgb)
        now = time.time()
        dt = max(now - self.t, 1e-3)
        self.k += (1.0 + self.vy / 640.0 - self.k) * min(dt, 1.0)
        self.k = max(self.k, 0.5)
        down, right = shift(self.prev, g)
        # The floor moving down the screen means the ball went up it (+y).
        dx = -right * 2 * self.k / SCALE
        dy = down * 2 * self.k / SCALE
        self.x += dx
        self.y += dy
        self.vx = 0.5 * self.vx + 0.5 * dx / dt
        self.vy = 0.5 * self.vy + 0.5 * dy / dt
        self.prev, self.t = g, now
        return rgb


def lift_at(rgb, world_y, tracker):
    """Whether the floor at (0, world_y) is there (not the background green)."""
    v = int(384 - (world_y - tracker.y) * SCALE)
    u = int(512 - tracker.x * SCALE)
    if not (0 <= v < 768 and 0 <= u < 1024):
        return None
    patch = rgb[max(v - 6, 0):v + 6, max(u - 6, 0):u + 6].reshape(-1, 3).astype(int)
    r, g, b = patch.mean(axis=0)
    background = g > 150 and r < 60 and b < 60
    return not background


def lift_offset(rgb, tracker):
    """Where the lift's centre is from the ball, in map units, or None if not in view."""
    r, g, b = (rgb[:, :, i].astype(int) for i in range(3))
    mask = (r >= 245) & (g >= 200) & (b >= 120) & (b <= 190)
    ys, xs = np.nonzero(mask)
    if len(xs) < 400:
        return None
    k = tracker.k
    return (xs.mean() - 512) * k / SCALE, (384 - ys.mean()) * k / SCALE


def keys_for(ax, ay):
    """Holds the arrow keys for the parts of a tick that ax and ay (-1 to 1) ask for."""
    press("Right", ax > 0.1)
    press("Left", ax < -0.1)
    press("Up", ay > 0.1)
    press("Down", ay < -0.1)
    d.sync()
    start = time.time()
    for release_at, keys in sorted([(abs(ax) * TICK, ("Left", "Right")), (abs(ay) * TICK, ("Up", "Down"))]):
        time.sleep(max(0.0, release_at - (time.time() - start)))
        for key in keys:
            press(key, False)
        d.sync()
    time.sleep(max(0.0, TICK - (time.time() - start)))


def servo_lift(tr, seconds, log):
    """Keeps the ball over the lift's centre for `seconds`, from where the lift is seen."""
    end = time.time() + seconds
    last = None
    n = 0
    while time.time() < end:
        rgb = tr.update()
        offset = lift_offset(rgb, tr)
        if offset is None:
            keys_for(0.0, 0.0)
            continue
        ox, oy = offset
        # The offset's change is the ball's speed relative to the lift, negated.
        vx, vy = (0.0, 0.0) if last is None else ((last[0] - ox) / TICK, (last[1] - oy) / TICK)
        last = offset
        ax = max(-1.0, min(1.0, (0.8 * ox - vx) / 60.0))
        ay = max(-1.0, min(1.0, (0.8 * oy - vy) / 60.0))
        if n % 5 == 0:
            print(f"{log} lift at {ox:6.1f} {oy:6.1f} rel vel {vx:6.1f} {vy:6.1f}", flush=True)
        n += 1
        keys_for(ax, ay)


def drive(tr, tx, ty, until, slope=0.0, gain=0.8, vmax=70.0, log="", seconds=60.0):
    """Steers towards (tx, ty) until `until(tr, rgb)` says so, or `seconds` pass."""
    n = 0
    end = time.time() + seconds
    while True:
        rgb = tr.update()
        if until(tr, rgb) or time.time() > end:
            release_all()
            return rgb
        ex, ey = tx - tr.x, ty - tr.y
        # Sideways only when well off the line: a tilt skews the floor in the picture,
        # which the measurement takes for motion.
        vdx = max(-40.0, min(40.0, 0.5 * ex)) if abs(ex) > 40 else 0.0
        vdy = max(-vmax, min(vmax, gain * ey))
        ax = max(-1.0, min(1.0, (vdx - tr.vx) / 120.0))
        ay = max(-1.0, min(1.0, (vdy - tr.vy) / 80.0 + slope))
        # Each key is held for the part of the tick its tilt asks for.
        press("Right", ax > 0.1)
        press("Left", ax < -0.1)
        press("Up", ay > 0.1)
        press("Down", ay < -0.1)
        d.sync()
        if n % 10 == 0:
            print(f"{log} pos {tr.x:7.1f} {tr.y:7.1f} vel {tr.vx:6.1f} {tr.vy:6.1f} keys {sorted(held)}", flush=True)
        n += 1
        long_x, long_y = abs(ax) * TICK, abs(ay) * TICK
        start = time.time()
        for release_at, keys in sorted([(long_x, ("Left", "Right")), (long_y, ("Up", "Down"))]):
            time.sleep(max(0.0, release_at - (time.time() - start)))
            for key in keys:
                press(key, False)
            d.sync()
        time.sleep(max(0.0, TICK - (time.time() - start)))


def start():
    """Clicks past the level's intro and picks the manual camera."""
    xtest.fake_input(d, X.MotionNotify, x=512, y=384)
    xtest.fake_input(d, X.ButtonPress, 1)
    d.sync()
    time.sleep(0.05)
    xtest.fake_input(d, X.ButtonRelease, 1)
    d.sync()
    time.sleep(0.5)
    three = d.keysym_to_keycode(XK.string_to_keysym("3"))
    xtest.fake_input(d, X.KeyPress, three)
    d.sync()
    time.sleep(0.1)
    xtest.fake_input(d, X.KeyRelease, three)
    d.sync()
    time.sleep(2.5)  # "Ready?", "Set?", "GO!"


def main():
    start()
    tr = Tracker(0.0, -64.0)
    shot = lambda name: Image.fromarray(grab()).save(f"{SHOTS}/{name}.png")
    near = lambda x, y, r: (lambda t, rgb: (t.x - x) ** 2 + (t.y - y) ** 2 < r * r and abs(t.vx) < 40 and abs(t.vy) < 40)

    # Up the middle: the start pad's coins, the ramp, the deck's first coin, the switch.
    drive(tr, 0, 300, lambda t, rgb: t.y > 140, log="pad")
    drive(tr, 0, 600, lambda t, rgb: t.y > 540, slope=0.5, log="ramp")
    shot("d1-deck")
    drive(tr, 0, 700, near(0, 700, 30), log="switch")
    shot("d2-switch")
    drive(tr, 0, 840, near(0, 840, 20), log="red coin")
    shot("d3-wait")
    # Hold still by the deck's edge until the lift comes in: the floor at y = 1000 turns
    # from the background's green to the lift's wood.
    seen = {"gone": False}

    def lift_arrived(t, rgb):
        here = lift_at(rgb, 1000, t)
        seen["gone"] = seen["gone"] or here is False
        return seen["gone"] and here

    drive(tr, 0, 840, lift_arrived, log="wait")
    arrived = time.time()
    print("lift is in", flush=True)
    shot("d4-lift-in")
    # Onto the lift and over its middle, and kept there while it crosses: it waits
    # three seconds at each end and takes three to cross.
    servo_lift(tr, 3.0, "board")
    shot("d5-aboard")
    servo_lift(tr, 6.5 - (time.time() - arrived), "ride")
    # The lift moved the ball and the floor it stands on together, which the frames
    # barely show: the ball is where the lift stopped.
    print(f"after ride pos {tr.x:.1f} {tr.y:.1f}", flush=True)
    tr.x, tr.y = 0.0, 1184.0
    shot("d6-across")
    # Into the goal: past its middle, so the ball is well inside the ring.
    # The level ends as the ball drops into it, and the keys then work the menu.
    drive(tr, 0, 1580, near(0, 1580, 30), log="goal", seconds=6.0)
    time.sleep(2.0)
    shot("d7-goal")
    release_all()


if __name__ == "__main__":
    try:
        main()
    finally:
        release_all()
