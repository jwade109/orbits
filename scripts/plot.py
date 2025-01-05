#! /usr/bin/env python3

import pandas
import matplotlib.pyplot as plt
import sys

path = sys.argv[1]

df = pandas.read_csv(path)

print(df)

df.plot(x=df.columns[0])

plt.show()
