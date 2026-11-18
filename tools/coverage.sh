#!/usr/bin/env bash
# Reproducible line/function coverage for the R1 software gate (CP-11).
#
# Instruments the workspace test build with the stable LLVM tools, runs every
# test binary, then writes:
#   target/coverage/summary.txt   llvm-cov report (library code only)
#   target/coverage/lcov.info     llvm-cov export (for other consumers)
#
# HIL is not part of this gate; see docs/coverage.md for the split.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${OUT:-$ROOT/target/coverage}"

# TOOLCHAIN=nightly COVERAGE_OPTIONS="-Zcoverage-options=branch" records branch
# coverage (stable rustc has no branch instrumentation).
if [ -n "${TOOLCHAIN:-}" ]; then
  export RUSTUP_TOOLCHAIN="$TOOLCHAIN"
fi
COVERAGE_OPTIONS="${COVERAGE_OPTIONS:-}"
TOOLCHAIN_BIN="$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | awk '/host:/ {print $2}')/bin"
LLVM_PROFDATA="$TOOLCHAIN_BIN/llvm-profdata"
LLVM_COV="$TOOLCHAIN_BIN/llvm-cov"

rm -rf "$OUT"
mkdir -p "$OUT/profile"

cd "$ROOT"
export RUSTFLAGS="${RUSTFLAGS:-} -Cinstrument-coverage $COVERAGE_OPTIONS"
export LLVM_PROFILE_FILE="$OUT/profile/%p-%m.profraw"

echo "== building instrumented tests =="
cargo test --workspace --no-run --message-format=json > "$OUT/build.json"

mapfile -t BINARIES < <(jq -r 'select(.profile.test == true) | .executable // empty' "$OUT/build.json" | sort -u)
if [ "${#BINARIES[@]}" -eq 0 ]; then
  echo "no test binaries found" >&2
  exit 1
fi
mapfile -t Z2M_BINARIES < <(jq -r 'select(.profile.test == true and (.package_id | contains("crates/z2m-rs#"))) | .executable // empty' "$OUT/build.json" | sort -u)
mapfile -t DEP_BINARIES < <(jq -r 'select(.profile.test == true and (.package_id | contains("crates/z2m-rs#") | not)) | .executable // empty' "$OUT/build.json" | sort -u)
echo "== running ${#BINARIES[@]} test binaries =="
for binary in "${BINARIES[@]}"; do
  "$binary" --test-threads=4 >/dev/null
done

echo "== merging profiles =="
"$LLVM_PROFDATA" merge -sparse "$OUT"/profile/*.profraw -o "$OUT/merged.profdata"

echo "== report =="
IGNORE='/(registry|rustlib|tests|examples|benches|target)/'
"$LLVM_COV" report "${DEP_BINARIES[@]}" \
  -instr-profile="$OUT/merged.profdata" -ignore-filename-regex="$IGNORE" \
  > "$OUT/summary_deps.txt"
"$LLVM_COV" report "${Z2M_BINARIES[@]}" \
  -instr-profile="$OUT/merged.profdata" -ignore-filename-regex="$IGNORE" \
  > "$OUT/summary_z2m.txt"
python3 - "$OUT" <<'PY'
import sys
from pathlib import Path
out = Path(sys.argv[1])
rows = []
header = None
total = None
deps_lines = (out / "summary_deps.txt").read_text().splitlines()
deps_paths = set()
for line in deps_lines[2:]:
    stripped = line.strip()
    if stripped and not stripped.startswith("---") and not stripped.startswith("TOTAL"):
        deps_paths.add(line.split()[0])
header = deps_lines[:2]
for line in deps_lines[2:]:
    stripped = line.strip()
    if stripped and not stripped.startswith("---") and not stripped.startswith("TOTAL"):
        rows.append(line)
for name in ("summary_z2m.txt",):
    for line in (out / name).read_text().splitlines()[2:]:
        stripped = line.strip()
        if not stripped or stripped.startswith("---") or stripped.startswith("TOTAL"):
            continue
        path = line.split()[0]
        # z2m binaries also link zigbee-apiarist/converters; add only rows not
        # already covered by the dedicated dependency report.
        if path not in deps_paths:
            rows.append(line)
# Totals: sum of the deduplicated rows. Nightly with
# `-Zcoverage-options=branch` adds Branches/Missed Branches/Cover columns.
branch_mode = "Branches" in header[0]
total = [0, 0, 0, 0, 0, 0] + ([0, 0] if branch_mode else [])
for line in rows:
    parts = line.split()
    values = [int(parts[1]), int(parts[2]), int(parts[4]), int(parts[5]), int(parts[7]), int(parts[8])]
    if branch_mode:
        values += [int(parts[10]), int(parts[11])]
    total = [a + b for a, b in zip(total, values)]
rows.sort(key=lambda row: row.split()[0])
number = lambda a, b: f"{100 * (a - b) / a:6.2f}%" if a else "  0.00%"
r, mr, f, mf, l, ml = total[:6]
total_line = (
    f"TOTAL {r:>10} {mr:>10} {number(r, mr):>10} {f:>10} {mf:>10} {number(f, mf):>10} "
    f"{l:>10} {ml:>10} {number(l, ml):>10}"
)
if branch_mode:
    b, mb = total[6], total[7]
    total_line += f" {b:>10} {mb:>10} {number(b, mb):>10}"
else:
    total_line += f" {'0':>10} {'0':>10} {'-':>10}"
(out / "summary.txt").write_text("\n".join(header + rows + ["-" * 260, total_line]) + "\n")
PY
"$LLVM_COV" export "${DEP_BINARIES[@]}" \
  -instr-profile="$OUT/merged.profdata" -ignore-filename-regex="$IGNORE" \
  -format=lcov > "$OUT/lcov_deps.info"
"$LLVM_COV" export "${Z2M_BINARIES[@]}" \
  -instr-profile="$OUT/merged.profdata" -ignore-filename-regex="$IGNORE" \
  -format=lcov > "$OUT/lcov_z2m.info"
python3 - "$OUT" <<'PY'
import sys
from pathlib import Path
out = Path(sys.argv[1])
records = []
seen = set()
for name in ("lcov_deps.info", "lcov_z2m.info"):
    current = []
    sf = None
    for line in (out / name).read_text().splitlines():
        if line.startswith("SF:"):
            sf = line
            current = [line]
        elif current:
            current.append(line)
            if line == "end_of_record":
                path = (sf or "")[3:]
                if path not in seen:
                    seen.add(path)
                    records.append("\n".join(current))
                current = []
(out / "lcov.info").write_text("\n".join(records) + "\n")
PY

cat "$OUT/summary.txt"
echo "artifacts: $OUT/summary.txt, $OUT/lcov.info"
