"use strict";
// Minimal package used by the autonomous runtime tests that only need the
// loader/event loop (timers, long JS, admission). Real converter parity tests
// use the pinned upstream package.
exports.prepareDefinition = function (definition) { return definition; };
exports.postProcessConvertedFromZigbeeMessage = function () {};
exports.clearGlobalStore = function () {};
