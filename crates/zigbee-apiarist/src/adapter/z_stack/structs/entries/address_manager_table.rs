// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/structs/entries/address-manager-table.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

use crate::adapter::z_stack::structs::r#struct::Alignment;
use crate::adapter::z_stack::structs::table::{Table, TableData};

use super::address_manager_entry::{address_manager_entry, is_set};

pub fn address_manager_table(data: TableData, alignment: Alignment) -> Table {
    Table::new(address_manager_entry)
        .occupancy(is_set)
        .build(data, alignment)
}
