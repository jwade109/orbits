#! /usr/bin/env python3

import pandas
import matplotlib.pyplot as plt
import sys
import os

simdir = sys.argv[1]

fig, axs = plt.subplots(5, 1)
fig.tight_layout(rect=[0, 0.03, 1, 0.95])

axs[0].set_title("Position (XY)")
axs[1].set_title("Angular Error")
axs[2].set_title("X Coordinate")
axs[3].set_title("Y Coordinate")
axs[4].set_title("Convergence Time")

for path in os.listdir(simdir):
    path = os.path.join(simdir, path)
    df = pandas.read_csv(path)
    axs[0].plot(df.x, df.y)
    axs[1].plot(df.time, df.angular_error)
    axs[2].plot(df.time, df.x)
    axs[3].plot(df.time, df.y)

    mask = df.converged == 1
    axs[4].plot(df.time[mask], df.converged[mask], "*")

    # df.plot(x=df.columns[0], grid=True, subplots=True, title=path)

for ax in axs:
    ax.grid()

plt.suptitle(f"Tracking Performance: {simdir}")
plt.show()
