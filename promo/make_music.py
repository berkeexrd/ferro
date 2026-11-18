"""Generates the soundtrack for the Ferro promo (original, synthesised; no samples)."""
import numpy as np, sys
from scipy import signal
from scipy.io import wavfile

SR = 44100
DUR = 91.0
BPM = 108
BEAT = 60 / BPM
BAR = BEAT * 4
N = int(SR * DUR)
rng = np.random.default_rng(7)
L = np.zeros(N)
R = np.zeros(N)


def midi(n):
    return 440.0 * 2 ** ((n - 69) / 12)


def add(buf_l, buf_r, t0, x, pan=0.0, gain=1.0):
    i = int(t0 * SR)
    if i >= N:
        return
    j = min(N, i + len(x))
    x = x[: j - i]
    gl = gain * np.cos((pan + 1) * np.pi / 4)
    gr = gain * np.sin((pan + 1) * np.pi / 4)
    buf_l[i:j] += x * gl
    buf_r[i:j] += x * gr


def lp(x, fc, order=2):
    b, a = signal.butter(order, fc / (SR / 2), "low")
    return signal.lfilter(b, a, x)


def hp(x, fc, order=2):
    b, a = signal.butter(order, fc / (SR / 2), "high")
    return signal.lfilter(b, a, x)


def env_ad(n, a, d):
    t = np.arange(n) / SR
    return np.minimum(t / max(a, 1e-4), 1.0) * np.exp(-t / d)


def pad_note(freq, dur, bright=1400):
    n = int(dur * SR)
    t = np.arange(n) / SR
    x = np.zeros(n)
    for det in (-0.012, -0.004, 0.0, 0.005, 0.011):
        f = freq * (1 + det)
        x += signal.sawtooth(2 * np.pi * f * t + rng.uniform(0, 6.28)) * 0.2
    x = lp(x, bright)
    a = np.minimum(t / 0.9, 1.0)
    r = np.minimum((dur - t) / 1.2, 1.0).clip(0, 1)
    return x * a * r


def pluck(freq, dur=1.2):
    n = int(dur * SR)
    t = np.arange(n) / SR
    x = np.sin(2 * np.pi * freq * t) + 0.35 * np.sin(2 * np.pi * 2 * freq * t) * np.exp(-t * 14) + 0.15 * signal.sawtooth(2 * np.pi * freq * t) * np.exp(-t * 20)
    return x * np.exp(-t * 7.5) * np.minimum(t / 0.004, 1.0)


def kick(vol=1.0):
    n = int(0.42 * SR)
    t = np.arange(n) / SR
    f = 46 + 110 * np.exp(-t * 28)
    ph = 2 * np.pi * np.cumsum(f) / SR
    return np.sin(ph) * np.exp(-t * 9) * vol + 0.15 * hp(rng.standard_normal(n), 3000) * np.exp(-t * 90)


def hat(vol=0.5, open_=False):
    n = int((0.25 if open_ else 0.07) * SR)
    t = np.arange(n) / SR
    return hp(rng.standard_normal(n), 7000) * np.exp(-t * (18 if open_ else 70)) * vol


def clap(vol=0.6):
    n = int(0.3 * SR)
    t = np.arange(n) / SR
    x = hp(lp(rng.standard_normal(n), 6000), 900)
    e = np.exp(-t * 22) * (1 + 0.8 * np.exp(-((t % 0.012) * 400)))
    return x * e * vol


def bass_note(freq, dur):
    n = int(dur * SR)
    t = np.arange(n) / SR
    x = np.sin(2 * np.pi * freq * t) + 0.3 * lp(signal.sawtooth(2 * np.pi * freq * t), 400)
    return x * np.minimum(t / 0.01, 1) * np.minimum((dur - t) / 0.08, 1).clip(0, 1) * 0.9


def riser(dur, vol=0.35):
    n = int(dur * SR)
    t = np.arange(n) / SR
    x = rng.standard_normal(n)
    # sweep a bandpass up using a time-varying one-pole
    fc = 300 + 7000 * (t / dur) ** 2.2
    a = np.exp(-2 * np.pi * fc / SR)
    y = np.zeros(n)
    z = 0.0
    for i in range(n):
        z = (1 - a[i]) * x[i] + a[i] * z
        y[i] = x[i] - z
    return y * (t / dur) ** 1.6 * vol


def whoosh(dur=0.9, vol=0.22):
    n = int(dur * SR)
    t = np.arange(n) / SR
    x = rng.standard_normal(n)
    x = lp(hp(x, 400), 5200)
    return x * np.sin(np.pi * t / dur) ** 2 * vol


def impact(vol=1.0):
    n = int(3.0 * SR)
    t = np.arange(n) / SR
    sub = np.sin(2 * np.pi * (38 + 60 * np.exp(-t * 6)) * t) * np.exp(-t * 1.6)
    crash = hp(rng.standard_normal(n), 2500) * np.exp(-t * 2.2) * 0.35
    return (sub + crash) * vol


CHORDS = [  # (bass, pad notes, arp notes)
    (45, [57, 60, 64, 67], [69, 72, 76, 81]),   # Am
    (41, [53, 57, 60, 65], [65, 69, 72, 77]),   # F
    (48, [55, 60, 64, 67], [67, 72, 76, 79]),   # C
    (43, [55, 59, 62, 67], [67, 71, 74, 79]),   # G
]


def chord_at(t):
    return CHORDS[int(t // BAR) % 4]


# ---- section map (seconds) ----
T_LOGO = 5.0
T_HIT = 5.45
T_BEAT = 11.0
T_BREAK = 65.0
T_BUILD = 77.0
T_SETUP = 78.0
T_OUT = 84.0

# pads: continuous, one bar per chord
t = 0.0
while t < DUR:
    b, pads, arp = chord_at(t)
    vol = 0.5 if t < T_HIT else 0.8
    if t >= T_OUT:
        vol = 0.9
    for k, nn in enumerate(pads):
        x = pad_note(midi(nn), BAR + 1.4, 1100 if t < T_BEAT else 1700)
        add(L, R, t, x, pan=(-0.5 + k * 0.33), gain=0.17 * vol)
    t += BAR

# bass
t = T_BEAT
while t < DUR - 2:
    if T_BREAK <= t < T_BUILD:
        t += BAR
        continue
    b, _, _ = chord_at(t)
    for step, dur in ((0, BEAT * 1.5), (1.5 * BEAT, BEAT * 0.5), (2 * BEAT, BEAT * 1.5), (3.5 * BEAT, BEAT * 0.5)):
        add(L, R, t + step, bass_note(midi(b), dur), 0, 0.42)
    t += BAR

# melody / arp
t = T_HIT
while t < DUR - 3:
    _, _, arp = chord_at(t)
    sparse = t < T_BEAT
    order = [0, 2, 1, 3, 2, 1, 3, 2]
    for s in range(8):
        if sparse and s % 2 == 1:
            continue
        nn = arp[order[s]]
        x = pluck(midi(nn))
        add(L, R, t + s * BEAT / 2, x, pan=(-0.6 if s % 2 else 0.6), gain=0.16 if not sparse else 0.2)
    t += BAR

# drums
t = T_BEAT
while t < DUR - 3:
    in_break = T_BREAK <= t < T_BUILD
    for beat in range(4):
        bt = t + beat * BEAT
        if not in_break:
            add(L, R, bt, kick(0.9), 0, 0.6)
            if beat in (1, 3):
                add(L, R, bt, clap(0.5), 0.1, 0.5)
            add(L, R, bt + BEAT / 2, hat(0.35), 0.3, 0.5)
            if beat == 3:
                add(L, R, bt + BEAT / 2, hat(0.4, True), -0.3, 0.5)
        else:
            if beat % 2 == 0:
                add(L, R, bt, hat(0.25), 0.2, 0.4)
    t += BAR

# transitions
add(L, R, T_HIT - 2.0, riser(2.0, 0.4), 0, 1.0)
add(L, R, T_HIT, impact(0.9), 0, 0.9)
add(L, R, T_BUILD - 3.0, riser(3.0, 0.4), 0, 1.0)
add(L, R, T_OUT, impact(0.8), 0, 0.8)
for tt in (11.0, 19.0, 34.0, 45.0, 53.0, 59.0, 65.0, 72.0, 78.0, 84.0):
    add(L, R, tt - 0.45, whoosh(0.9, 0.22), 0, 1.0)

# gentle sidechain-style pump on pads/bass by ducking on kicks (cheap: shape the sum envelope)
duck = np.ones(N)
t = T_BEAT
while t < DUR - 3:
    if not (T_BREAK <= t < T_BUILD):
        for beat in range(4):
            i = int((t + beat * BEAT) * SR)
            n = int(0.22 * SR)
            if i + n < N:
                duck[i : i + n] *= 1 - 0.28 * np.exp(-np.arange(n) / SR * 14)
    t += BAR
L *= duck
R *= duck

# ending: let the last chord ring, then fade
fade_in = np.minimum(np.arange(N) / (SR * 1.5), 1)
fade_out = np.minimum((N - np.arange(N)) / (SR * 3.5), 1)
L *= fade_in * fade_out
R *= fade_in * fade_out

# simple stereo reverb: a few decorrelated delays
def reverb(x, wet=0.22):
    y = np.zeros_like(x)
    for d, g in ((0.037, 0.5), (0.061, 0.42), (0.089, 0.35), (0.131, 0.28), (0.187, 0.2)):
        k = int(d * SR)
        y[k:] += x[:-k] * g
    return x + lp(y, 4500) * wet


L = reverb(L)
R = reverb(np.roll(R, 11))
peak = max(np.abs(L).max(), np.abs(R).max())
g = 0.89 / peak
out = np.stack([L * g, R * g], axis=1)
wavfile.write(sys.argv[1], SR, (out * 32767).astype(np.int16))
print("ok", DUR, "s, peak gain", round(g, 3))
