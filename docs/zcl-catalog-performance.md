# ZCL catalog runtime cost (Routerich AX3000)

Measured on the real R1 router:

```text
Linux RouteRich 6.6.144 aarch64
OpenWrt 24.10.8 (mediatek/filogix)
```

Release build, `aarch64-unknown-linux-musl`, LTO. The benchmark binary was
built outside this repository and copied to `/tmp` on the router; it exercises
`Utils::get_cluster` and `Frame::from_buffer(report)`.

## Before the Arc/COW change

```text
first catalog lookup (JSON parse + maps): 47–65 ms
catalog RSS:                              +1764 KB

get_cluster, µs/call:
  seMetering (202 attributes)             720–808
  haElectricalMeasurement (129)           291–456
  lightingColorCtrl (52)                  137–158
  greenPower (19)                         116–140
  hvacThermostat (50)                      55–65
  genBasic (21)                            20–23
  genOnOff (6)                             11–16

Frame::from_buffer(report), µs/frame:
  seMetering                              955–991
  lightingColorCtrl                       409–421
  genOnOff                                 19–24
```

The deep `Cluster` clone dominated frame parsing (about 85% for `seMetering`).

## After Arc/COW + startup warmup

```text
first catalog lookup (JSON parse + maps): 46–47 ms
catalog RSS:                              +1824 KB

get_cluster, µs/call:
  seMetering                              0.44–0.63
  haElectricalMeasurement                 0.54–1.04
  lightingColorCtrl                       0.48–0.49
  genBasic                                0.42
  genOnOff                                0.42
  hvacThermostat                          0.46
  greenPower                              0.43

Frame::from_buffer(report), µs/frame:
  seMetering                              3.33–3.81
  lightingColorCtrl                       3.33–3.58
  genOnOff                                3.30–3.70
```

`Cluster` is now an `Arc<ClusterInner>`; catalog lookups are a cheap handle
clone, and custom clusters use copy-on-write. `Utils::warm_cluster_catalog()`
is called from `Controller.start` and `ZStackAdapter.start` so the one-time
JSON parse is paid during startup rather than by the first ZCL frame.

The JSON itself is not a runtime issue: it is embedded with `include_str!`,
parsed once under `OnceLock`, and immutable afterwards.
