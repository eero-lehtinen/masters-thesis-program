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

FEATURES = {
    "spatial": [
        ["spatial_array", "branchless", "floatneighbors", "no_id_check"],
        "spatial_hash",
        "spatial_kdtree",
        # "spatial_kdtree_kiddo",
        "spatial_kdbush",
        "spatial_rstar",
    ],
    "optimize": [
        ["distance_func2"],
        ["distance_func2", "branchless"],
        # ["distance_func2", "branchless", "floatneighbors"],
        # ["distance_func2", "branchless", "floatneighbors", "no_id_check"],
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
