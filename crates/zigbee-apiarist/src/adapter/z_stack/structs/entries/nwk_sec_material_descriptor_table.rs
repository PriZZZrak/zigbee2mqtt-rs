// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/structs/entries/nwk-sec-material-descriptor-table.ts

use crate::adapter::z_stack::structs::r#struct::Alignment;
use crate::adapter::z_stack::structs::table::Table;

use super::nwk_sec_material_descriptor_entry::nwk_sec_material_descriptor_entry;

pub fn nwk_sec_material_descriptor_table(
    data: Option<&[u8]>,
    capacity: usize,
    alignment: Alignment,
) -> Table {
    let mut table = Table::new(nwk_sec_material_descriptor_entry, capacity);
    if let Some(data) = data {
        table = table.build(Some(data), alignment);
    }
    table
}
