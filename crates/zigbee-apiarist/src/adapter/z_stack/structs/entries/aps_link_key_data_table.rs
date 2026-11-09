// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/structs/entries/aps-link-key-data-table.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

use crate::adapter::z_stack::structs::r#struct::{Alignment, Struct};
use crate::adapter::z_stack::structs::table::{Table, TableData};

use super::aps_link_key_data_entry::aps_link_key_data_entry;

pub fn is_used(entry: &Struct) -> bool {
    entry.get_bytes("key").unwrap_or_default() != [0u8; 16]
}

pub fn aps_link_key_data_table(data: TableData, alignment: Alignment) -> Table {
    Table::new(aps_link_key_data_entry)
        .occupancy(is_used)
        .build(data, alignment)
}
