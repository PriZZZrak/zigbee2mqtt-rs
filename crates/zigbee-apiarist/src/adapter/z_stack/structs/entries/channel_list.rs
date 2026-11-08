// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/structs/entries/channel-list.ts

use crate::adapter::z_stack::structs::r#struct::{MemberType, Struct};

pub fn channel_list(data: Option<&[u8]>) -> Struct {
    Struct::new()
        .member(MemberType::Uint32, "channelList")
        .build(data, None)
}
