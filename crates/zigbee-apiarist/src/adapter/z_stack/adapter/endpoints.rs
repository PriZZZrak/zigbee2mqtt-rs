// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/adapter/endpoints.ts

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointConfig {
    pub endpoint: u8,
    pub appprofid: u16,
    pub appdeviceid: u16,
    pub appdevver: u8,
    pub appnuminclusters: u8,
    pub appinclusterlist: Vec<u16>,
    pub appnumoutclusters: u8,
    pub appoutclusterlist: Vec<u16>,
    pub latencyreq: u8,
}

const fn defaults(endpoint: u8, appprofid: u16) -> EndpointConfig {
    EndpointConfig {
        endpoint,
        appprofid,
        appdeviceid: 0x0005,
        appdevver: 0,
        appnuminclusters: 0,
        appinclusterlist: Vec::new(),
        appnumoutclusters: 0,
        appoutclusterlist: Vec::new(),
        latencyreq: 0,
    }
}

/// Upstream `Endpoints` registered on startup.
pub fn endpoints() -> Vec<EndpointConfig> {
    vec![
        defaults(1, 0x0104),
        defaults(2, 0x0101),
        defaults(3, 0x0104),
        defaults(4, 0x0107),
        defaults(5, 0x0108),
        defaults(6, 0x0109),
        defaults(8, 0x0104),
        defaults(10, 0x0104),
        EndpointConfig {
            endpoint: 11,
            appprofid: 0x0104,
            appdeviceid: 0x0400,
            appdevver: 0,
            appnuminclusters: 2,
            appinclusterlist: vec![0x0501, 0x000a],
            appnumoutclusters: 2,
            appoutclusterlist: vec![0x0500, 0x0502],
            latencyreq: 0,
        },
        defaults(0x6e, 0x0104),
        defaults(12, 0xc05e),
        EndpointConfig {
            endpoint: 13,
            appprofid: 0x0104,
            appdeviceid: 0x0005,
            appdevver: 0,
            appnuminclusters: 1,
            appinclusterlist: vec![0x0019],
            appnumoutclusters: 0,
            appoutclusterlist: Vec::new(),
            latencyreq: 0,
        },
        defaults(47, 0x0104),
        defaults(239, 0xc05e),
        defaults(242, 0xa1e0),
    ]
}
