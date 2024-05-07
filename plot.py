import json
import matplotlib
import matplotlib.pyplot as plt
import matplotlib.ticker as ticker
import numpy as np


with open("statistics-optimize.json") as f:
    stats = json.load(f)

data = {}

features = list(stats.keys())
levels = sorted(list(stats[features[0]]["flocking"].keys()))

for feature in stats:
    data[feature] = []
    for level in levels:
        x = np.array(stats[feature]["flocking"][level]) * 1000
        data[feature].append(np.mean(x))

# Sample data
x = np.arange(len(levels))
width = 0.3


for i, feature in enumerate(features):
    pos = x + width * i - 0.15 * (len(features) - 1)
    rects = plt.bar(pos, data[feature], width)
    plt.bar_label(rects, fmt="%.2f", padding=3)

plt.margins(x=0.1, y=0.1)
plt.xticks(x, levels)
plt.xlabel("Level")
plt.ylabel("Mean time (ms)")
plt.legend(features)

plt.tight_layout()
plt.show()
