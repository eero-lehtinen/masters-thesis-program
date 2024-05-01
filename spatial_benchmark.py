import subprocess
from pathlib import Path
import json

LEVELS = ["1-Empty", "2-Labyrinth", "3-Cathedral", "4-Centipedetown"]
FEATURES = [
    "spatial_hash",
    "spatial_kdtree",
    # "spatial_kdtree_kiddo",
    "spatial_kdbush",
    "spatial_rstar",
]

def main():

    for feature in FEATURES:
        statistics = {
            "spatial_reset": [],
            "spatial_insert": [],
            "avoidance": [],
        }


        for level in LEVELS:
            print(f"Running level {level}")
            subprocess.run(["cargo", "run", "--release", "--features", feature, "--", "--level", level, "bench"], check=True)
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
    main()
