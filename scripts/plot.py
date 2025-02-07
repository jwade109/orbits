#! /usr/bin/env python3

import pandas
import matplotlib.pyplot as plt
import sys

path = sys.argv[1]

df = pandas.read_csv(path)

print(df)

df.plot(x=df.columns[0], grid=True, subplots=True, title=path)

if "x" in df.columns and "y" in df.columns:
    ax = df.plot(x="x", y="y", title="Cartesian Coordinates", grid=True)
    ax.scatter([0.0], [0.0], marker='*')
    ax.axis("equal")

plt.show()
