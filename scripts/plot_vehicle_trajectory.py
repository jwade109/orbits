#! /usr/bin/env python3

import pandas
import matplotlib.pyplot as plt
import sys
import os
import yaml
import numpy as np

simdir = sys.argv[1]

fig, axs = plt.subplots(3, 2)
plt.suptitle(f"Tracking Performance: {simdir}")
# fig.tight_layout(rect=[0, 0.03, 1, 0.95])

target_pos = None

for path in os.listdir(simdir):

    path = os.path.join(simdir, path)
    data = yaml.load(open(path, "r"))
    data["t"] = np.array(data["t"]) / 1E9

    if target_pos is None:
        target_pos = data["target_pos"]

    print((path, data["convergence"]))

    color = None
    if data["convergence"] is None:
        color = (0.1, 0.1, 0.1, 0.1)

    axs[0, 0].set_title("Position (XY)")
    axs[1, 0].set_title("X Coordinate")
    axs[2, 0].set_title("Y Coordinate")
    axs[0, 1].set_title("Angle")
    axs[1, 1].set_title("Acceleration")
    axs[2, 1].set_title("Convergence Time")

    # df = pandas.read_csv(path)
    axs[0, 0].plot(data["x"], data["y"], color=color)
    axs[1, 0].plot(data["t"], data["x"], color=color)
    axs[2, 0].plot(data["t"], data["y"], color=color)
    axs[0, 1].plot(data["t"], data["a"], color=color)
    axs[1, 1].plot(data["t"], data["accel"], color=color)

    if "convergence" in data and data["convergence"] is not None:
        axs[2, 1].plot(data["convergence"] / 1E9, 1, "*")

axs[0, 0].plot(target_pos[0], target_pos[1], "r*")
axs[1, 0].axhline(target_pos[0], color="r")
axs[2, 0].axhline(target_pos[1], color="r")

for ax in axs:
    for ax in ax:
        ax.grid()

plt.show()
    # plt.draw()
    # plt.waitforbuttonpress()

    # for ax in axs:
    #     ax.cla()
