// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/structs/entries/aps-tc-link-key-table.ts

use crate::adapter::z_stack::structs::r#struct::Alignment;
use crate::adapter::z_stack::structs::table::Table;

use super::aps_tc_link_key_entry::aps_tc_link_key_entry;

pub fn aps_tc_link_key_table(data: Option<&[u8]>, capacity: usize, alignment: Alignment) -> Table {
    let mut table = Table::new(aps_tc_link_key_entry, capacity);
    if let Some(data) = data {
        table = table.build(Some(data), alignment);
    }
    table
}
