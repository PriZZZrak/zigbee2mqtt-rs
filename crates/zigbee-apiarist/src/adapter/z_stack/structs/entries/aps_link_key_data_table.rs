// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/structs/entries/aps-link-key-data-table.ts

use crate::adapter::z_stack::structs::r#struct::Alignment;
use crate::adapter::z_stack::structs::table::Table;

use super::aps_link_key_data_entry::aps_link_key_data_entry;

pub fn aps_link_key_data_table(
    data: Option<&[u8]>,
    capacity: usize,
    alignment: Alignment,
) -> Table {
    let mut table = Table::new(aps_link_key_data_entry, capacity);
    if let Some(data) = data {
        table = table.build(Some(data), alignment);
    }
    table
}
