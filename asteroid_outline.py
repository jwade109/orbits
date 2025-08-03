from math import pi
from random import random, randint
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.pyplot as plt
from perlin_noise import PerlinNoise
from dataclasses import dataclass
from typing import List, Tuple


@dataclass
class Asteroid:
    base_radius: float
    parameters: List[Tuple[float, float, float]]


def generate_ast_params() -> Asteroid:
    params = []
    terms = randint(5, 12)
    frequency = 2
    for i in range(terms):
        amplitude = random() * 0.5 * 1 / frequency
        phase = random() * 2.0 * pi
        params.append((amplitude, frequency, phase))
        frequency += randint(2, 5)
    radius = random() * 200 + 20
    return Asteroid(radius, params)


def radius_func(radius, params, theta):
    r = 1
    for (a, f, p) in params:
        r += a * np.cos(f * theta + p)
    return r * radius


def main():

    while True:

        noise = PerlinNoise(octaves=2)

        noise_scale = 100

        ast = generate_ast_params()
        theta = np.linspace(0, 2 * pi, 400)
        radius = radius_func(ast.base_radius, ast.parameters, theta)

        max_radius = np.max(radius)

        pic = []
        for x in np.linspace(-max_radius, max_radius, int(max_radius) * 4):
            row = []
            for y in np.linspace(-max_radius, max_radius, int(max_radius) * 4):
                r = np.sqrt(x*x + y*y)
                t = np.arctan2(x, y)
                r_ast = radius_func(ast.base_radius, ast.parameters, t)
                if r < r_ast:
                    row.append(noise([x / noise_scale, y / noise_scale]))
                else:
                    row.append(-1)
            pic.append(row)

        plt.imshow(pic, cmap='copper')
        # plt.polar(theta, radius)
        plt.show()


if __name__ == "__main__":
    main()
