// Deterministic clock shared by the converter benchmarks.
//
// Some modernExtend configure paths embed the current time (genTime via
// customTimeResponse/writeTimeDaily), which would otherwise make the golden
// corpus change on every run.

export const FIXED_NOW_MS = Date.UTC(2026, 10, 20, 12, 0, 0);

export function installFixedClock() {
  const RealDate = Date;
  class FixedDate extends RealDate {
    constructor(...args) {
      if (args.length === 0) {
        super(FIXED_NOW_MS);
      } else {
        super(...args);
      }
    }
    static now() {
      return FIXED_NOW_MS;
    }
  }
  globalThis.Date = FixedDate;
}
