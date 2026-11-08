"""Converte as capas em tons de cinza para duotone. Uso: python duotone.py <pasta_cinza> <pasta_saida>"""
import sys
import pathlib
import numpy as np
from PIL import Image

SRC, OUT = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
OUT.mkdir(parents=True, exist_ok=True)
PAL = {
    'silk': ('#050a1a', '#8cc8ff'), 'flow': ('#03110f', '#5ff0c8'), 'contour': ('#0a1206', '#b6f07a'),
    'halftone': ('#160604', '#ff7a3d'), 'glow': ('#120c02', '#ffcf6e'), 'sphere': ('#050c14', '#9fd0ff'),
    'moire': ('#14040a', '#ff5c86'), 'dither': ('#0b0b0b', '#f0f0f0'), 'bars': ('#120a02', '#ffb547'),
    'warpgrid': ('#03101a', '#4fc3ff'), 'scan': ('#160303', '#ff4747'), 'orbit': ('#04110d', '#7cf5c4'),
}


def rgb(h):
    return np.array([int(h[i:i + 2], 16) for i in (1, 3, 5)], np.float32)


for name, (dark, light) in PAL.items():
    g = np.asarray(Image.open(SRC / f'{name}.jpg').convert('L'), np.float32) / 255.0
    t = np.clip(g, 0, 1)[..., None] ** 0.9
    img = rgb(dark) * (1 - t) + rgb(light) * t
    Image.fromarray(np.clip(img, 0, 255).astype(np.uint8), 'RGB').save(OUT / f'{name}.jpg', quality=86)
    print('ok', name)
