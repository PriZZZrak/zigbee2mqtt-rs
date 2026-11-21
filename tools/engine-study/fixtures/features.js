// Language/feature probe for the converter engine study.
// The script records pass/fail flags and leaves the report on globalThis.

var report = {};
function test(name, fn) {
  try {
    var value = fn();
    report[name] = value === undefined ? true : value;
  } catch (error) {
    report[name] = "ERROR: " + (error && error.message ? error.message : String(error));
  }
}
async function testAsync(name, fn) {
  try {
    var value = await fn();
    report[name] = value === undefined ? true : value;
  } catch (error) {
    report[name] = "ERROR: " + (error && error.message ? error.message : String(error));
  }
}

test("class", function () {
  class A { constructor(x) { this.x = x; } get y() { return this.x + 1; } }
  class B extends A { method() { return super.y; } }
  return new B(1).method() === 2;
});
test("optional-chaining", function () {
  var o = { a: { b: 1 } };
  return o?.a?.b === 1 && o?.missing?.b === undefined;
});
test("nullish", function () { return (null ?? 2) === 2 && (0 ?? 3) === 0; });
test("spread-rest", function () {
  var a = [1, 2];
  var b = { x: 1 };
  return [...a, 3].length === 3 && { ...b, y: 2 }.y === 2;
});
test("destructuring", function () {
  var { a, b = 5 } = { a: 1 };
  var [c, ...rest] = [1, 2, 3];
  return a === 1 && b === 5 && c === 1 && rest.length === 2;
});
test("template-literals", function () { var x = 2; return `a${x + 1}b` === "a3b"; });
test("for-of", function () { var sum = 0; for (var v of [1, 2, 3]) sum += v; return sum === 6; });
test("generators", function () {
  function* gen() { yield 1; yield 2; }
  return Array.from(gen()).join(",") === "1,2";
});
test("async-generators", function () { return typeof (async function* () {}) === "function"; });
test("symbol-iterator", function () {
  var o = {};
  o[Symbol.iterator] = function* () { yield 1; };
  return [...o][0] === 1;
});
test("map-set", function () {
  var m = new Map([["a", 1]]);
  var s = new Set([1, 1, 2]);
  return m.get("a") === 1 && s.size === 2;
});
test("weakmap", function () {
  var k = {};
  var w = new WeakMap([[k, 2]]);
  return w.get(k) === 2;
});
test("proxy-reflect", function () {
  var target = { a: 1 };
  var p = new Proxy(target, { get: function (t, k) { return k === "a" ? 2 : t[k]; } });
  return p.a === 2 && Reflect.get(target, "a") === 1;
});
test("typed-arrays", function () {
  var u = new Uint8Array([1, 2, 3]);
  var b = new ArrayBuffer(4);
  var d = new DataView(b);
  d.setUint16(0, 258);
  return u.length === 3 && u[2] === 3 && d.getUint16(0) === 258;
});
test("buffer-like", function () {
  // Node Buffer shim, if the host provides one.
  if (typeof Buffer === "undefined") return "MISSING";
  var b = Buffer.from([1, 2, 3]);
  return b.length === 3 && b[1] === 2 && typeof b.toString === "function";
});
test("bigint", function () { return (2n ** 64n).toString() === "18446744073709551616"; });
test("regex-named-groups", function () {
  var m = /(?<year>\d{4})-(?<month>\d{2})/.exec("2026-11");
  return m.groups.year === "2026" && m.groups.month === "11";
});
test("regex-lookbehind", function () { return /(?<=\$)\d+/.exec("$42")[0] === "42"; });
test("json", function () {
  var s = JSON.stringify({ a: [1, "x", null] });
  return JSON.parse(s).a[1] === "x";
});
test("object-from-entries", function () { return Object.fromEntries([["a", 1]]).a === 1; });
test("array-flat-at", function () { return [1, [2]].flat()[1] === 2 && [1, 2].at(-1) === 2; });
test("promise-combinators", function () {
  return typeof Promise.allSettled === "function" && typeof Promise.any === "function";
});
test("new-function", function () {
  var f = new Function("a", "b", "return a + b");
  return f(2, 3) === 5;
});
test("function-constructor-scope", function () {
  var x = 7;
  var f = new Function("return typeof x");
  return f() === "undefined";
});
test("error-stack", function () {
  try { throw new Error("boom"); } catch (e) { return typeof e.stack === "string" && e.message === "boom"; }
});

var order = [];
Promise.resolve().then(function () { order.push("microtask"); });
order.push("sync");
globalThis.__order = order;

testAsync("async-await", async function () {
  var v = await Promise.resolve(41);
  return v + 1 === 42;
});
testAsync("async-host-style", async function () {
  async function api() { return { ok: true }; }
  var r = await api();
  return r.ok === true;
});

globalThis.__report = function () {
  report["microtask-order"] = __order.join(",");
  return JSON.stringify(report);
};
