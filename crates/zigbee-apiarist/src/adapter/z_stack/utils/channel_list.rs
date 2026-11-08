// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/utils/channel-list.ts

/// Upstream `unpackChannelList(packedList)`.
pub fn unpack_channel_list(packed_list: u32) -> Vec<u8> {
    (11u8..=26)
        .filter(|channel| (1u32 << channel) & packed_list > 0)
        .collect()
}

/// Upstream `packChannelList(channelList)`.
pub fn pack_channel_list(channel_list: &[u8]) -> Result<u32, String> {
    if let Some(channel) = channel_list
        .iter()
        .find(|channel| !(11u8..=26).contains(*channel))
    {
        return Err(format!(
            "Cannot pack channel list - unsupported channel {channel}"
        ));
    }
    Ok(channel_list
        .iter()
        .fold(0u32, |mask, channel| mask | (1u32 << *channel)))
}

/// Upstream `compareChannelLists(list1, list2)` for channel arrays.
pub fn compare_channel_lists(list1: &[u8], list2: &[u8]) -> bool {
    pack_channel_list(list1).ok() == pack_channel_list(list2).ok()
}

/// Upstream overload accepting packed `uint32` values.
pub fn compare_packed_channel_lists(list1: u32, list2: u32) -> bool {
    list1 == list2
}
