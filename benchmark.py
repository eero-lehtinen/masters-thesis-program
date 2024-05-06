import subprocess
from pathlib import Path
import json
import sys

LEVELS = ["1-Empty", "2-Labyrinth", "3-Cathedral", "4-Centipedetown"]

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
        # ["distance_func2"],
        # ["distance_func2", "branchless"],
        # ["distance_func2", "branchless", "floatneighbors"],
        ["distance_func2", "branchless", "floatneighbors", "no_id_check"],
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
    for feature in FEATURES[feature_key]:
        statistics = {
            "spatial_reset": [],
            "spatial_insert": [],
            "avoidance": [],
        }

        for level in LEVELS:
            print(f"Running {level} with features {feature}")

            if isinstance(feature, list):
                feature = ",".join(feature)

            subprocess.run(
                [
                    "cargo",
                    "run",
                    "--release",
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
                    statistics[key].extend(stats[key])

        means = {}

        for key in statistics:
            means[key] = sum(statistics[key]) / len(statistics[key]) * 1000

        print(f"Feature {feature}, means:")
        for key in means:
            print(f"{key}: {means[key]} ms")


if __name__ == "__main__":
    if len(sys.argv) < 2:
        arg = "test"
    else:
        arg = sys.argv[1]

    main(arg)
