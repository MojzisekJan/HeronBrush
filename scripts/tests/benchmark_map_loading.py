#!/usr/bin/env python3
"""Alternate two prebuilt map_load_probe binaries; never build/download."""
import argparse
import hashlib
import json
from pathlib import Path
import statistics
import subprocess
import sys


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("before", type=Path)
    parser.add_argument("after", type=Path)
    parser.add_argument("fixtures", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--cpu", type=int, help="Pin to this CPU on Linux; omit on macOS")
    parser.add_argument("--runs", type=int, default=7, choices=range(1, 32))
    args = parser.parse_args()
    if args.cpu is not None and sys.platform != "linux":
        parser.error("--cpu requires Linux taskset; omit it on this platform")
    result = {"platform": sys.platform, "cpu": args.cpu, "order": ["before", "after", "after", "before"],
              "runs_per_process": args.runs, "cases": []}
    for fixture in sorted(args.fixtures.glob("*.map")):
        for mode in ["parse", "load"]:
            case = {"fixture": fixture.name, "sha256": hashlib.sha256(fixture.read_bytes()).hexdigest(),
                    "mode": mode, "processes": []}
            for variant in result["order"]:
                binary = getattr(args, variant).resolve()
                prefix = ["nice", "-n", "10"]
                if args.cpu is not None:
                    prefix += ["taskset", "-c", str(args.cpu)]
                completed = subprocess.run(prefix + [
                    str(binary), str(fixture.resolve()), mode, str(args.runs)],
                    check=True, capture_output=True, text=True, timeout=60)
                sample = json.loads(completed.stdout)
                sample["variant"] = variant
                case["processes"].append(sample)
            for variant in ["before", "after"]:
                processes = [p for p in case["processes"] if p["variant"] == variant]
                samples = [value for p in processes for value in p["samples_ms"]]
                case[variant] = {"median_ms": statistics.median(samples),
                    "min_ms": min(samples), "max_ms": max(samples),
                    "peak_rss_kib": [p["peak_rss_kib"] for p in processes],
                    "brushes": processes[0]["brushes"], "source_bytes": processes[0]["source_bytes"]}
            assert case["before"]["brushes"] == case["after"]["brushes"], "brush counts differ"
            case["median_change_percent"] = (case["after"]["median_ms"] / case["before"]["median_ms"] - 1) * 100
            result["cases"].append(case)
            args.output.write_text(json.dumps(result, indent=2) + "\n")
            print(f'{fixture.name:24} {mode:5} {case["before"]["median_ms"]:8.2f} -> '
                  f'{case["after"]["median_ms"]:8.2f} ms ({case["median_change_percent"]:+.1f}%)', flush=True)


if __name__ == "__main__":
    main()
