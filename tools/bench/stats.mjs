// Shared statistics helpers for the converter benchmarks.

export function percentile(values, fraction) {
  if (values.length === 0) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  const index = Math.min(sorted.length - 1, Math.max(0, Math.ceil(fraction * sorted.length) - 1));
  return sorted[index];
}

export function stats(values) {
  if (values.length === 0) return { count: 0 };
  const total = values.reduce((sum, value) => sum + value, 0);
  return {
    count: values.length,
    meanMs: total / values.length,
    p50Ms: percentile(values, 0.5),
    p95Ms: percentile(values, 0.95),
    p99Ms: percentile(values, 0.99),
    minMs: Math.min(...values),
    maxMs: Math.max(...values),
    totalMs: total,
    opsPerSecond: total > 0 ? (values.length * 1000) / total : 0,
  };
}

// Stable JSON used to compare golden entries regardless of key insertion order.
export function canonicalJson(value) {
  if (value === null || typeof value !== "object") return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  const keys = Object.keys(value).sort();
  return `{${keys.map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`).join(",")}}`;
}
