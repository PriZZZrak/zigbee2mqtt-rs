#!/usr/bin/env python3
import csv, json, os, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
INV = os.path.join(ROOT, "docs", "parity", "inventory.csv")

def main():
    rows = list(csv.DictReader(open(INV)))
    stage1 = [r for r in rows if r["stage"] == "1"]
    global_rows = [r for r in rows if r["status"] != "OUT_OF_SCOPE_APPROVED"]

    def metrics(subset):
        total = len(subset)
        verified = [r for r in subset if r["status"] == "VERIFIED"]
        w_total = sum(int(r["weight"]) for r in subset)
        w_ver = sum(int(r["weight"]) for r in verified)
        sc_total = sum(int(r["total_cases"]) for r in subset if r["total_cases"].isdigit())
        sc_ver = sum(int(r["verified_cases"]) for r in subset if r["verified_cases"].isdigit())
        return total, len(verified), w_total, w_ver, sc_total, sc_ver

    s1 = metrics(stage1)
    gl = metrics(global_rows)

    print("Parity report")
    print(f"Rows: {len(rows)}")
    print(f"Stage1: verified {s1[1]}/{s1[0]} = {100*s1[1]/s1[0]:.2f}% (weighted {100*s1[3]/s1[2]:.2f}%, scenarios {s1[5]}/{s1[4]})")
    print(f"Global: verified {gl[1]}/{gl[0]} = {100*gl[1]/gl[0]:.2f}% (weighted {100*gl[3]/gl[2]:.2f}%, scenarios {gl[5]}/{gl[4]})")

if __name__ == "__main__":
    main()