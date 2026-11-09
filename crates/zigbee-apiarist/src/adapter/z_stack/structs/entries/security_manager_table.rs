// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/structs/entries/security-manager-table.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

use crate::adapter::z_stack::structs::r#struct::Alignment;
use crate::adapter::z_stack::structs::table::{Table, TableData};

use super::security_manager_entry::{is_used, security_manager_entry};

pub fn security_manager_table(data: TableData, alignment: Alignment) -> Table {
    Table::new(security_manager_entry)
        .occupancy(is_used)
        .inline_header(true)
        .build(data, alignment)
}
