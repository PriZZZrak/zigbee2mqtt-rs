// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/structs/entries/nwk-sec-material-descriptor-table.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

use crate::adapter::z_stack::structs::r#struct::Alignment;
use crate::adapter::z_stack::structs::table::{Table, TableData};

use super::nwk_sec_material_descriptor_entry::{is_set, nwk_sec_material_descriptor_entry};

pub fn nwk_sec_material_descriptor_table(data: TableData, alignment: Alignment) -> Table {
    Table::new(nwk_sec_material_descriptor_entry)
        .occupancy(is_set)
        .build(data, alignment)
}
