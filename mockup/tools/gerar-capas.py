"""Gera capas abstratas monocromáticas (512 px) para o mockup. Uso: python covers.py <pasta_saida>"""
import sys
import pathlib
import numpy as np
from PIL import Image

OUT = pathlib.Path(sys.argv[1])
OUT.mkdir(parents=True, exist_ok=True)
S = 512
yy, xx = np.mgrid[0:S, 0:S].astype(np.float32) / S


def field(seed, k=10, freq=(1.5, 6.0)):
    r = np.random.default_rng(seed)
    terms = [(r.uniform(*freq) * np.cos(a), r.uniform(*freq) * np.sin(a), r.uniform(0, 6.283), r.uniform(0.4, 1.0))
             for a in r.uniform(0, 6.283, k)]

    def f(x, y):
        v = np.zeros_like(x, dtype=np.float32)
        tot = 0.0
        for fx, fy, ph, amp in terms:
            v += amp * np.sin(fx * x * 6.283 + fy * y * 6.283 + ph)
            tot += amp
        return v / tot
    return f


def blur(a, n=2):
    for _ in range(n):
        a = (a + np.roll(a, 1, 0) + np.roll(a, -1, 0) + np.roll(a, 1, 1) + np.roll(a, -1, 1)) / 5.0
    return a


def finish(a, name, seed, grain=9.0, vignette=0.35):
    r = np.random.default_rng(seed + 99)
    a = np.clip(a, 0, 1)
    d = np.sqrt((xx - 0.5) ** 2 + (yy - 0.5) ** 2)
    a = a * (1 - vignette * d ** 2 * 2)
    img = a * 255 + r.normal(0, grain, a.shape)
    Image.fromarray(np.clip(img, 0, 255).astype(np.uint8), 'L').convert('RGB').save(OUT / f'{name}.jpg', quality=86)


def accum(px, py, w=None):
    h = np.zeros((S, S), np.float32)
    ix = np.clip((px * S).astype(int), 0, S - 1)
    iy = np.clip((py * S).astype(int), 0, S - 1)
    m = (px >= 0) & (px < 1) & (py >= 0) & (py < 1)
    np.add.at(h, (iy[m], ix[m]), 1 if w is None else w[m])
    return h


def silk(seed):
    r = np.random.default_rng(seed)
    n = 260000
    u, v = r.random(n).astype(np.float32), r.random(n).astype(np.float32)
    a, b, c, d = field(seed, 8, (0.8, 2.4)), field(seed + 1, 8, (0.8, 2.4)), field(seed + 2, 9, (0.6, 2.0)), field(seed + 3, 9, (0.6, 2.0))
    wx, wy = a(u, v) * 0.55, b(u, v) * 0.55
    px = 0.5 + c(wx * 1.3 + u * 0.3, wy * 1.3 + v * 0.3) * 1.35
    py = 0.5 + d(wx * 1.3 + u * 0.3 + 3.1, wy * 1.3 + v * 0.3 + 1.7) * 1.35
    h = accum(px, py)
    img = 1 - np.exp(-h * 0.5)
    return np.clip(img * 0.9 + blur(img, 3) * 0.6, 0, 1)


def flow(seed):
    r = np.random.default_rng(seed)
    f = field(seed, 7, (0.6, 1.8))
    p = r.random((1800, 2)).astype(np.float32)
    h = np.zeros((S, S), np.float32)
    for _ in range(180):
        ang = f(p[:, 0], p[:, 1]) * 6.283 * 1.2
        p[:, 0] += np.cos(ang) * 0.0024
        p[:, 1] += np.sin(ang) * 0.0024
        h += accum(p[:, 0], p[:, 1])
    img = 1 - np.exp(-h * 0.5)
    return img * 0.95


def contour(seed):
    f = field(seed, 9, (0.7, 2.2))(xx, yy)
    k = 14
    g = f * k
    gy, gx = np.gradient(g)
    gm = np.sqrt(gx ** 2 + gy ** 2) + 1e-6
    dist = np.abs(g - np.round(g)) / gm
    line = np.clip(1.2 - dist / 0.9, 0, 1)
    fade = 0.35 + 0.65 * np.clip((f + 0.6), 0, 1)
    return line * fade


def halftone(seed):
    cell = 14
    cx = (np.floor(xx * S / cell) + 0.5) * cell / S
    cy = (np.floor(yy * S / cell) + 0.5) * cell / S
    lx, ly = 0.3 + 0.1 * np.sin(seed), 0.35
    val = np.clip(1.05 - np.sqrt((cx - lx) ** 2 + (cy - ly) ** 2) * 1.6, 0, 1) ** 1.4
    rad = val * cell * 0.55 / S
    d = np.sqrt((xx - cx) ** 2 + (yy - cy) ** 2)
    return np.clip((rad - d) * S * 0.9 + 0.5, 0, 1)


def glow(seed):
    r = np.random.default_rng(seed)
    a = np.zeros((S, S), np.float32)
    for _ in range(3):
        cx, cy, s = r.uniform(0.25, 0.75), r.uniform(0.25, 0.75), r.uniform(0.12, 0.3)
        a += r.uniform(0.5, 1.0) * np.exp(-((xx - cx) ** 2 + (yy - cy) ** 2) / (2 * s * s))
    return np.clip(a, 0, 1) ** 1.3


def sphere(seed):
    cx, cy, rr = 0.5, 0.46, 0.27
    dx, dy = (xx - cx) / rr, (yy - cy) / rr
    d2 = dx ** 2 + dy ** 2
    inside = d2 < 1
    dz = np.sqrt(np.clip(1 - d2, 0, 1))
    lx, ly, lz = -0.55, -0.6, 0.58
    lam = np.clip(dx * lx + dy * ly + dz * lz, 0, 1)
    rim = np.clip(1 - dz, 0, 1) ** 3 * 0.35
    sph = np.where(inside, 0.04 + 0.9 * lam ** 1.4 + rim, 0)
    edge = np.clip((1 - np.sqrt(d2)) * rr * S * 1.5, 0, 1)
    sh = np.exp(-(((xx - 0.53) / 0.24) ** 2 + ((yy - 0.83) / 0.035) ** 2)) * 0.18
    return np.maximum(sph * edge, sh)


def moire(seed):
    r1 = np.sqrt((xx - 0.42) ** 2 + (yy - 0.5) ** 2)
    r2 = np.sqrt((xx - 0.58) ** 2 + (yy - 0.5) ** 2)
    v = np.sin(r1 * 150) * np.sin(r2 * 150)
    m = np.clip(v * 3, 0, 1)
    return m * np.clip(1.1 - np.sqrt((xx - 0.5) ** 2 + (yy - 0.5) ** 2) * 2.2, 0, 1)


def dither(seed):
    bayer = np.array([[0, 32, 8, 40, 2, 34, 10, 42], [48, 16, 56, 24, 50, 18, 58, 26], [12, 44, 4, 36, 14, 46, 6, 38],
                      [60, 28, 52, 20, 62, 30, 54, 22], [3, 35, 11, 43, 1, 33, 9, 41], [51, 19, 59, 27, 49, 17, 57, 25],
                      [15, 47, 7, 39, 13, 45, 5, 37], [63, 31, 55, 23, 61, 29, 53, 21]]) / 64.0
    px = 3
    gx, gy = (xx * S / px).astype(int), (yy * S / px).astype(int)
    th = bayer[gy % 8, gx % 8]
    base = np.clip(1 - np.sqrt((xx - 0.62) ** 2 + (yy - 0.4) ** 2) * 2.1, 0, 1) ** 1.2
    base += 0.25 * np.clip(field(seed, 6, (0.8, 2))(xx, yy), 0, 1)
    return (base > th).astype(np.float32) * 0.92


def bars(seed):
    f = field(seed, 6, (0.8, 3.0))
    n = 46
    w = 1.0 / n
    i = np.floor(xx / w)
    hgt = 0.18 + 0.5 * (0.5 + 0.5 * f((i + 0.5) * w, np.zeros_like(xx) + 0.3))
    inbar = ((xx / w - i) > 0.22) & ((xx / w - i) < 0.78)
    top = 0.86 - hgt
    v = (inbar & (yy > top) & (yy < 0.86)).astype(np.float32)
    v += ((yy > 0.875) & (yy < 0.879)).astype(np.float32) * 0.6
    return v * 0.92


def warpgrid(seed):
    f, g = field(seed, 6, (0.6, 1.6)), field(seed + 5, 6, (0.6, 1.6))
    x2 = xx + 0.06 * f(xx, yy)
    y2 = yy + 0.06 * g(xx, yy)
    s = 1 / 18
    gxx, gyy = x2 / s, y2 / s
    dx = np.abs(gxx - np.round(gxx))
    dy = np.abs(gyy - np.round(gyy))
    line = np.clip(1.4 - np.minimum(dx, dy) / 0.045, 0, 1)
    return line * (0.35 + 0.65 * np.clip(1.1 - np.sqrt((xx - 0.5) ** 2 + (yy - 0.5) ** 2) * 1.6, 0, 1))


def scan(seed):
    shape = np.clip(1 - np.sqrt((xx - 0.5) ** 2 + (yy - 0.52) ** 2) * 2.6, 0, 1)
    disp = 0.02 * np.sin(yy * 40 + seed)
    shape2 = np.clip(1 - np.sqrt((xx - 0.5 - disp) ** 2 + (yy - 0.52) ** 2) * 2.6, 0, 1)
    period = 6 / S
    ph = (yy / period) % 1
    thick = 0.12 + 0.75 * shape2
    return (np.abs(ph - 0.5) < thick / 2).astype(np.float32) * (0.25 + 0.7 * (shape2 > 0.02))


def orbit(seed):
    r = np.random.default_rng(seed)
    a = np.zeros((S, S), np.float32)
    for k in range(9):
        rx, ry, rot = r.uniform(0.12, 0.42), r.uniform(0.05, 0.3), r.uniform(0, 3.14)
        cx, cy = 0.5 + r.uniform(-0.05, 0.05), 0.5 + r.uniform(-0.05, 0.05)
        X, Y = xx - cx, yy - cy
        xr = X * np.cos(rot) + Y * np.sin(rot)
        yr = -X * np.sin(rot) + Y * np.cos(rot)
        e = np.sqrt((xr / rx) ** 2 + (yr / ry) ** 2)
        a = np.maximum(a, np.clip(1 - np.abs(e - 1) * min(rx, ry) * S / 1.1, 0, 1) * r.uniform(0.45, 1))
    dot = np.clip((0.035 - np.sqrt((xx - 0.5) ** 2 + (yy - 0.5) ** 2)) * S * 0.5, 0, 1)
    return np.maximum(a, dot)


GEN = {'silk': silk, 'flow': flow, 'contour': contour, 'halftone': halftone, 'glow': glow, 'sphere': sphere,
       'moire': moire, 'dither': dither, 'bars': bars, 'warpgrid': warpgrid, 'scan': scan, 'orbit': orbit}
for i, (name, fn) in enumerate(GEN.items()):
    finish(fn(11 + i * 7), name, i)
    print('ok', name)
