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
        // Required for Shelly Wi-Fi and RPC clusters
        // (`ZSpec.CUSTOM_SHELLY_PROFILE_ID`).
        defaults(239, 0xc001),
        defaults(242, 0xa1e0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Values taken from upstream `adapter/z-stack/adapter/endpoints.ts`.
    #[test]
    fn matches_upstream_endpoint_table() {
        let list = endpoints();
        let ids = list
            .iter()
            .map(|endpoint| endpoint.endpoint)
            .collect::<Vec<_>>();
        assert_eq!(
            ids,
            vec![1, 2, 3, 4, 5, 6, 8, 10, 11, 0x6e, 12, 13, 47, 239, 242]
        );

        let eleven = list.iter().find(|e| e.endpoint == 11).unwrap();
        assert_eq!(eleven.appprofid, 0x0104);
        assert_eq!(eleven.appdeviceid, 0x0400);
        assert_eq!(eleven.appnuminclusters, 2);
        assert_eq!(eleven.appinclusterlist, vec![0x0501, 0x000a]);
        assert_eq!(eleven.appnumoutclusters, 2);
        assert_eq!(eleven.appoutclusterlist, vec![0x0500, 0x0502]);

        let thirteen = list.iter().find(|e| e.endpoint == 13).unwrap();
        assert_eq!(thirteen.appinclusterlist, vec![0x0019]);

        assert_eq!(
            list.iter().find(|e| e.endpoint == 239).unwrap().appprofid,
            0xc001
        );
        assert_eq!(
            list.iter().find(|e| e.endpoint == 242).unwrap().appprofid,
            0xa1e0
        );
        for endpoint in &list {
            assert_eq!(endpoint.latencyreq, 0);
            assert_eq!(endpoint.appdevver, 0);
        }
    }
}
