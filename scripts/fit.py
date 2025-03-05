#! /usr/bin/env python3

import sys
import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
from numpy.polynomial import *
from scipy.interpolate import CubicSpline


def approximation(x, y):
    base = CubicSpline(x, y)
    xanchor = [*np.linspace(0.0, 0.35, 5), *np.linspace(0.4, 0.6, 20), *np.linspace(0.65, 1.0, 5)]
    yanchor = [base(x) for x in xanchor]
    print(xanchor)
    print(yanchor)
    return CubicSpline(xanchor, yanchor)


def main():

    df = pd.read_csv(sys.argv[1], skipinitialspace=True)

    for mu in df["mu"].unique():
        plt.figure()
        for r in df["r"].unique():
            for v in df["v"].unique():
                subdf = df[(df.r == r) & (df.v == v) & (df.mu == mu)]
                if len(subdf) > 5:
                    t = (subdf.t / subdf.t.max()).to_numpy()
                    y = subdf.chi / mu
                    m = y.iloc[-1] / t[-1]
                    yline = t * m
                    ysmash = y - yline

                    fit = approximation(t, ysmash)

                    approx = fit(t)

                    error = approx - ysmash

                    # plt.plot(t, ysmash, label="smash")
                    # plt.plot(t, approx, label="approx")
                    plt.plot(t, error, label="error")

        plt.title(f"mu = {mu}")
        plt.grid()
        plt.legend()
    plt.show()


if __name__ == "__main__":
    main()
