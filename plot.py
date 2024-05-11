import json
import matplotlib.pyplot as plt
import numpy as np

# name = "distance_func"
# name = "move_forces"
# name = "micro_optimizations"
name = "spatial"

with open(f"statistics-{name}.json") as f:
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
width = 0.15

for i, feature in enumerate(features):
    pos = x + width * i - (width / 2) * (len(features) - 1)
    rects = plt.bar(pos, data[feature], width)
    plt.bar_label(rects, fmt="%.2f", padding=3)

plt.margins(x=0.1, y=0.1)
plt.xticks(x, levels)
plt.xlabel("Level")
plt.ylabel("Mean time (ms)")

move_baseline = "spatial_array,distance_func2,new_movement,new_move_clamp"
spatial_baseline = (
    "distance_func2,branchless,floatneighbors,new_movement,new_move_clamp"
)
legend_names = {
    "distance_func": {
        "spatial_array": "Linear distance",
        "spatial_array,distance_func2": "Quadratic distance",
    },
    "move_forces": {
        "spatial_array,distance_func2": "Baseline",
        "spatial_array,distance_func2,new_movement": "Low naviagation force",
        "spatial_array,distance_func2,new_movement,new_move_clamp": "Low naviagation force & fixed clamping",
    },
    "micro_optimizations": {
        move_baseline: "Baseline",
        move_baseline + ",branchless": "Branchless",
        move_baseline + ",branchless,floatneighbors": "Branchless & float neighbors",
    },
    "spatial": {
        spatial_baseline: "Spatial array",
        spatial_baseline + ",spatial_hash": "Spatial hash (ahash)",
        spatial_baseline + ",spatial_hash_std": "Spatial hash (std hash)",
        spatial_baseline + ",spatial_kdtree": "KD-Tree",
        spatial_baseline + ",spatial_kdbush": "KD-Bush",
        spatial_baseline + ",spatial_rstar": "R-Star",
    },
}

legend = [legend_names[name][f] for f in features]

plt.legend(legend)

plt.tight_layout()
plt.show()
