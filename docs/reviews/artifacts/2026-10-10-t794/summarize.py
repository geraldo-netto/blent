"""Summarize T794 paired microbench JSON; all output timing units are us."""
import hashlib
import json
from pathlib import Path
import statistics
import sys


def summarize(path):
    frames = json.loads(path.read_text())["frames"]
    result = {key.removesuffix("_ns") + "_us": statistics.median(row[key] for row in frames) / 1000 for key in frames[0]}
    times = sorted(row["total_ns"] for row in frames)
    for quantile in (95, 99):
        result[f"p{quantile}_us"] = times[int((len(times)-1)*quantile/100)] / 1000
    return result


def node_summary(root, node):
    pairs = []
    for pair in range(1, 6):
        variants = [summarize(root / f"n{node}-p{pair}-m{mode}.json") for mode in (0, 1)]
        changes = {key: 100 * (variants[1][key] / variants[0][key] - 1) for key in variants[0]}
        pairs.append({"pair": pair, "upload": variants[0], "mapped": variants[1], "change_percent": changes})
    median = [{key: statistics.median(pair[label][key] for pair in pairs) for key in pairs[0][label]} for label in ("upload", "mapped")]
    hashes = {path.name: hashlib.sha256(path.read_bytes()).hexdigest() for path in sorted(root.glob(f"n{node}*.h264"))}
    return {"pairs": pairs, "median_of_run_medians": {"upload": median[0], "mapped": median[1]}, "bitstream_sha256": hashes, "identical_bitstreams": len(set(hashes.values())) == 1}


def main():
    root = Path(sys.argv[1])
    result = {str(node): node_summary(root, node) for node in (128, 129)}
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
