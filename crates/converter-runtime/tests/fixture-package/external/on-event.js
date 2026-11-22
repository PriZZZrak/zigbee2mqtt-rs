"use strict";
// Synthetic definition used by the automatic onEvent lifecycle tests. It is
// loaded through the external-converter path so no upstream catalog is needed.
exports.definitions = [
  {
    model: "synthetic-on-event",
    vendor: "test",
    description: "synthetic onEvent fixture",
    exposes: [],
    fromZigbee: [],
    toZigbee: [],
    configure: async function () {},
    onEvent: async function (event) {
      if (event.type === "start") {
        setTimeout(function () {
          globalThis.__onEventTimerFired = true;
        }, 120);
        return;
      }
      if (event.type === "deviceJoined") {
        await new Promise(function (resolve) {
          setTimeout(function () {
            globalThis.__onEventWaitTimerFired = true;
            resolve();
          }, 10000);
        });
      }
    },
  },
];
