#! /usr/bin/env python3

import pandas as pd
import sys
import matplotlib.pyplot as plt


def main():
    infile = sys.argv[1]
    data = pd.read_csv(infile)

    print(data)

    _, axs = plt.subplots(4, 2)

    axs[0, 0].plot(data.ticks, data.x, label="x")
    axs[0, 0].plot(data.ticks, data.tx, label="tx")
    axs[0, 0].grid()
    axs[0, 0].legend()

    axs[1, 0].plot(data.ticks, data.y, label="y")
    axs[1, 0].plot(data.ticks, data.ty, label="ty")
    axs[1, 0].grid()
    axs[1, 0].legend()

    axs[0, 1].plot(data.ticks, data.a, label="a")
    axs[0, 1].plot(data.ticks, data.ta, label="ta")
    axs[0, 1].grid()
    axs[0, 1].legend()

    axs[1, 1].plot(data.x, data.y, label="Ground Track")
    axs[1, 1].scatter(data.x, data.y, s=data.thrusters, c="black")
    axs[1, 1].plot(data.tx, data.ty, "*", label="Target")
    axs[1, 1].grid()
    axs[1, 1].legend()

    axs[2, 0].plot(data.ticks, data.updates, label="Acceleration Updates")
    axs[2, 0].grid()
    axs[2, 0].legend()

    axs[2, 1].plot(data.ticks, data.thrusters, label="# Thrusters Firing")
    axs[2, 1].grid()
    axs[2, 1].legend()

    axs[3, 0].plot(data.ticks, data.vx, label="X Velocity")
    axs[3, 0].plot(data.ticks, data.vy, label="Y Velocity")
    axs[3, 0].grid()
    axs[3, 0].legend()

    axs[3, 1].plot(data.ticks, data.va, label="Angular Velocity")
    axs[3, 1].grid()
    axs[3, 1].legend()

    plt.show()


if __name__ == "__main__":
    main()
