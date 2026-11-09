// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/structs/entries/aps-tc-link-key-table.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

use crate::adapter::z_stack::structs::r#struct::{Alignment, Struct};
use crate::adapter::z_stack::structs::table::{Table, TableData};

use super::aps_tc_link_key_entry::aps_tc_link_key_entry;

pub fn is_used(entry: &Struct) -> bool {
    entry.get_bytes("extAddr").unwrap_or_default() != [0u8; 8]
}

pub fn aps_tc_link_key_table(data: TableData, alignment: Alignment) -> Table {
    Table::new(aps_tc_link_key_entry)
        .occupancy(is_used)
        .build(data, alignment)
}
