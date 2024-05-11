import subprocess
import numpy as np
from pathlib import Path
import json
import sys

LEVELS = [
    "1-Empty",
    "2-Labyrinth",
    "3-Cathedral",
    "4-Centipedetown",
]

move_baseline = ["spatial_array", "distance_func2", "new_movement", "new_move_clamp"]
spatial_baseline = [
    "distance_func2",
    "branchless",
    "floatneighbors",
    "new_movement",
    "new_move_clamp",
]

FEATURES = {
    "distance_func": ["spatial_array", ["spatial_array", "distance_func2"]],
    "move_forces": [
        ["spatial_array", "distance_func2"],
        ["spatial_array", "distance_func2", "new_movement"],
        ["spatial_array", "distance_func2", "new_movement", "new_move_clamp"],
    ],
    "micro_optimizations": [
        move_baseline,
        move_baseline + ["branchless"],
        move_baseline + ["branchless", "floatneighbors"],
    ],
    "spatial": [
        spatial_baseline,
        spatial_baseline + ["spatial_hash"],
        spatial_baseline + ["spatial_hash_std"],
        spatial_baseline + ["spatial_kdbush"],
        spatial_baseline + ["spatial_kdtree"],
        # spatial_baseline + ["spatial_kdtree_kiddo"],
        spatial_baseline + ["spatial_rstar"],
    ],
    "test": [],
}

# FEATURES = [
#     "distance_func2",
#     ["distance_func2", "branchless"],
# ]
#
# FEATURES = [
#     ["distance_func2", "branchless"],
#     ["distance_func2", "branchless", "floatneighbors"],
# ]


def main(feature_key="test"):
    total_statistics = {}
    for feature in FEATURES[feature_key]:

        statistics = {"flocking": {}}

        for level in LEVELS:
            print(f"Running {level} with features {feature}")

            if isinstance(feature, list):
                feature = ",".join(feature)

            subprocess.run(
                [
                    "cargo",
                    "run",
                    "--release",
                    "--no-default-features",
                    "--features",
                    feature,
                    "--",
                    "--level",
                    level,
                    "bench",
                ],
                check=True,
            )
            stats_file = Path(f"statistics.json")
            with stats_file.open() as f:
                stats = json.load(f)
                for key in statistics:
                    statistics[key][level] = stats[key]
                    data = np.array(stats[key])
                    mean = np.mean(data)
                    std = np.std(data)

                    # Pring mean and std
                    print(f"{key} {level}: {mean} +- {std} ms")

        total_statistics[feature] = statistics

        # Write to file
    with Path(f"statistics-{feature_key}.json").open("w") as f:
        json.dump(total_statistics, f)


if __name__ == "__main__":
    if len(sys.argv) < 2:
        arg = "test"
    else:
        arg = sys.argv[1]

    main(arg)
