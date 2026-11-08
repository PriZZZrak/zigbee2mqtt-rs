// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/structs/entries/security-manager-table.ts

use crate::adapter::z_stack::structs::r#struct::Alignment;
use crate::adapter::z_stack::structs::table::Table;

use super::security_manager_entry::security_manager_entry;

pub fn security_manager_table(data: Option<&[u8]>, capacity: usize, alignment: Alignment) -> Table {
    let mut table = Table::new(security_manager_entry, capacity);
    if let Some(data) = data {
        table = table.build(Some(data), alignment);
    }
    table
}
