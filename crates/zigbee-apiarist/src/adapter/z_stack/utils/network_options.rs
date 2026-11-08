// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/utils/network-options.ts

use crate::models::network_options::NetworkOptions;

use super::channel_list::compare_channel_lists;

/// Upstream `compareNetworkOptions(opts1, opts2, lenientExtendedPanIdMatching)`.
pub fn compare_network_options(
    opts1: &NetworkOptions,
    opts2: &NetworkOptions,
    lenient_extended_pan_id_matching: bool,
) -> bool {
    let extended_pan_id_matches = opts1.extended_pan_id == opts2.extended_pan_id
        || (lenient_extended_pan_id_matching
            && (opts1.has_default_extended_pan_id || opts2.has_default_extended_pan_id))
        || (lenient_extended_pan_id_matching
            && opts1
                .extended_pan_id
                .iter()
                .rev()
                .eq(opts2.extended_pan_id.iter()));

    opts1.pan_id == opts2.pan_id
        && extended_pan_id_matches
        && opts1.network_key == opts2.network_key
        && compare_channel_lists(&opts1.channel_list, &opts2.channel_list)
        && opts1.network_key_distribute == opts2.network_key_distribute
}
