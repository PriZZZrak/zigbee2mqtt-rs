// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/structs/entries/address-manager-table.ts

use crate::adapter::z_stack::structs::r#struct::Alignment;
use crate::adapter::z_stack::structs::table::Table;

use super::address_manager_entry::address_manager_entry;

pub fn address_manager_table(data: Option<&[u8]>, capacity: usize, alignment: Alignment) -> Table {
    let mut table = Table::new(address_manager_entry, capacity);
    if let Some(data) = data {
        table = table.build(Some(data), alignment);
    }
    table
}
