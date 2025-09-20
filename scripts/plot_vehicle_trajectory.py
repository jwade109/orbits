#! /usr/bin/env python3

import pandas
import matplotlib.pyplot as plt
import sys
import os
import yaml

simdir = sys.argv[1]

fig, axs = plt.subplots(6, 1)
# plt.suptitle(f"Tracking Performance: {simdir}")
# fig.tight_layout(rect=[0, 0.03, 1, 0.95])

all_data = []

for path in os.listdir(simdir):

    path = os.path.join(simdir, path)
    df = yaml.load(open(path, "r"))
    all_data.append((path, df))

for (i, (path, data)) in enumerate(sorted(all_data, key=lambda d: d[1]["convergence"] or 100000000)):

    print((path, data["convergence"]))

    axs[0].set_title("Position (XY)")
    axs[1].set_title("Angle")
    axs[2].set_title("X Coordinate")
    axs[3].set_title("Y Coordinate")
    axs[4].set_title("Acceleration")
    axs[5].set_title("Convergence Time")

    for ax in axs:
        ax.grid()

    # df = pandas.read_csv(path)
    axs[0].plot(data["x"], data["y"])
    axs[1].plot(data["t"], data["a"])
    axs[2].plot(data["t"], data["x"])
    axs[3].plot(data["t"], data["y"])
    axs[4].plot(data["t"], data["accel"])

    if "convergence" in data and data["convergence"] is not None:
        axs[5].plot(data["convergence"], 1, "*")

    plt.show(block=False)
    plt.draw()
    plt.waitforbuttonpress()

    # for ax in axs:
    #     ax.cla()
