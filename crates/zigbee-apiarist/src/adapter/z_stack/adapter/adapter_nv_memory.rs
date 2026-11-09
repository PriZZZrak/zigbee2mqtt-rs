// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/adapter/adapter-nv-memory.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

use serde_json::json;

use crate::adapter::z_stack::constants::common::{NvItemsIds, NvSystemIds, ZnpCommandStatus};
use crate::adapter::z_stack::structs::r#struct::{Alignment, Struct};
use crate::adapter::z_stack::structs::serializable_memory_object::SerializableMemoryObject;
use crate::adapter::z_stack::structs::table::{Table, TableData};
use crate::adapter::z_stack::unpi::constants::Subsystem;
use crate::adapter::z_stack::znp::znp::Znp;
use crate::adapter::z_stack::znp::zpi_object::ZpiObject;

const RETRIES: usize = 3;
const MAX_WRITE_SIZE: usize = 240;

/// Upstream `MemoryObjectFactory<T>` for structs.
pub type StructFactory = fn(Option<&[u8]>) -> Struct;
/// Upstream `MemoryObjectFactory<T>` for tables.
pub type TableFactory = fn(TableData, Alignment) -> Table;

/// Upstream `readTable` mode.
#[derive(Debug, Clone)]
pub enum TableMode {
    Legacy {
        id: u16,
        max_length: usize,
    },
    Extended {
        sys_id: NvSystemIds,
        id: u16,
        max_length: Option<usize>,
    },
}

/// Upstream `AdapterNvMemory`.
pub struct AdapterNvMemory {
    pub znp: Znp,
    pub memory_alignment: Option<Alignment>,
}

impl AdapterNvMemory {
    /// Upstream `constructor(znp)`.
    pub fn new(znp: Znp) -> Self {
        Self {
            znp,
            memory_alignment: None,
        }
    }

    /// Upstream `retry(fn, retries = 3)` for requests with a reply.
    async fn request_with_reply_retry(
        &mut self,
        subsystem: Subsystem,
        command: &str,
        payload: serde_json::Value,
        expected_statuses: &[u8],
    ) -> Result<ZpiObject, String> {
        let mut last_error = String::new();
        for _ in 0..RETRIES {
            match self
                .znp
                .request(
                    subsystem,
                    command,
                    payload.clone(),
                    None,
                    None,
                    expected_statuses,
                )
                .await
            {
                Ok(Some(reply)) => return Ok(reply),
                Ok(None) => last_error = format!("Command {command} has no reply"),
                Err(error) => last_error = error,
            }
        }
        Err(last_error)
    }

    /// Upstream `retry(fn, retries = 3)` for requests without a required reply.
    async fn request_retry(
        &mut self,
        subsystem: Subsystem,
        command: &str,
        payload: serde_json::Value,
        expected_statuses: &[u8],
    ) -> Result<Option<ZpiObject>, String> {
        let mut last_error = String::new();
        for _ in 0..RETRIES {
            match self
                .znp
                .request(
                    subsystem,
                    command,
                    payload.clone(),
                    None,
                    None,
                    expected_statuses,
                )
                .await
            {
                Ok(reply) => return Ok(reply),
                Err(error) => last_error = error,
            }
        }
        Err(last_error)
    }

    /// Upstream `init()`. `ZCD_NV_NWKKEY` length determines alignment
    /// (21 = unaligned, otherwise aligned).
    pub async fn init(&mut self) -> Result<(), String> {
        let raw_nwk_key = self.read_item(NvItemsIds::NWKKEY, 0).await?;
        self.memory_alignment = Some(if raw_nwk_key.as_ref().map(Vec::len) == Some(21) {
            Alignment::Unaligned
        } else {
            Alignment::Aligned
        });
        Ok(())
    }

    /// Upstream `readItem(id, offset)`.
    pub async fn read_item(&mut self, id: u16, offset: usize) -> Result<Option<Vec<u8>>, String> {
        let length_response = self
            .request_with_reply_retry(
                Subsystem::SYS,
                "osalNvLength",
                json!({"id": id}),
                &[ZnpCommandStatus::SUCCESS as u8],
            )
            .await?;
        let length = length_response
            .payload
            .get("length")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0) as usize;
        if length == 0 {
            return Ok(None);
        }

        let mut buffer = vec![0u8; length];
        let mut offset = offset;
        while offset < length {
            let read_response = self
                .request_retry(
                    Subsystem::SYS,
                    "osalNvReadExt",
                    json!({"id": id, "offset": offset}),
                    &[ZnpCommandStatus::SUCCESS as u8],
                )
                .await?;
            let Some(read_response) = read_response else {
                return Ok(None);
            };
            let status = read_response
                .payload
                .get("status")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            if status != 0 {
                return Err(format!(
                    "Received non-success status while reading NV (id={id}, offset={offset}, status={status})"
                ));
            }
            let value = read_response
                .payload
                .get("value")
                .map(bytes)
                .unwrap_or_default();
            buffer[offset..offset + value.len()].copy_from_slice(&value);
            offset += value.len();
        }

        Ok(Some(buffer))
    }

    /// Upstream `readItem(id, offset, useStruct)`.
    pub async fn read_item_struct(
        &mut self,
        id: u16,
        offset: usize,
        use_struct: StructFactory,
    ) -> Result<Option<Struct>, String> {
        self.check_memory_alignment_setup()?;
        Ok(self
            .read_item(id, offset)
            .await?
            .map(|buffer| use_struct(Some(&buffer))))
    }

    /// Upstream `writeItem(id, data, offset, autoInit)`.
    pub async fn write_item(
        &mut self,
        id: u16,
        data: &[u8],
        offset: usize,
        auto_init: bool,
    ) -> Result<(), String> {
        self.check_memory_alignment_setup()?;
        self.write_item_buffer(id, data, offset, auto_init).await
    }

    /// Upstream `writeItem(id, serializableObject, offset, autoInit)`.
    pub async fn write_item_object(
        &mut self,
        id: u16,
        data: &dyn SerializableMemoryObject,
        offset: usize,
        auto_init: bool,
    ) -> Result<(), String> {
        self.check_memory_alignment_setup()?;
        let alignment = self.memory_alignment.expect("memory alignment");
        let buffer = data.serialize_memory(alignment);
        self.write_item_buffer(id, &buffer, offset, auto_init).await
    }

    async fn write_item_buffer(
        &mut self,
        id: u16,
        buffer: &[u8],
        offset: usize,
        auto_init: bool,
    ) -> Result<(), String> {
        let length_response = self
            .request_with_reply_retry(
                Subsystem::SYS,
                "osalNvLength",
                json!({"id": id}),
                &[ZnpCommandStatus::SUCCESS as u8],
            )
            .await?;
        let exists = length_response
            .payload
            .get("length")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            > 0;

        if !exists {
            if !auto_init {
                return Err(format!(
                    "Cannot write NV memory item which does not exist (id={id})"
                ));
            }
            let init_length = buffer.len().min(MAX_WRITE_SIZE);
            let init_response = self
                .request_with_reply_retry(
                    Subsystem::SYS,
                    "osalNvItemInit",
                    json!({
                        "id": id,
                        "len": buffer.len(),
                        "initlen": init_length,
                        "initvalue": &buffer[..init_length],
                    }),
                    &[
                        ZnpCommandStatus::SUCCESS as u8,
                        ZnpCommandStatus::NV_ITEM_INITIALIZED as u8,
                    ],
                )
                .await?;
            let status = init_response
                .payload
                .get("status")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            if status != ZnpCommandStatus::NV_ITEM_INITIALIZED as u64 {
                return Err(format!(
                    "Failed to initialize NV memory item (id={id}, name={}, len={}, status={status})",
                    NvItemsIds::name(id),
                    buffer.len()
                ));
            }
        }

        let mut remaining = buffer.len();
        while remaining > 0 {
            let write_length = remaining.min(MAX_WRITE_SIZE);
            let data_offset = buffer.len() - remaining;
            let write_data = &buffer[data_offset..data_offset + write_length];
            let write_response = self
                .request_with_reply_retry(
                    Subsystem::SYS,
                    "osalNvWriteExt",
                    json!({"id": id, "offset": data_offset, "len": write_length, "value": write_data}),
                    &[ZnpCommandStatus::SUCCESS as u8],
                )
                .await?;
            let status = write_response
                .payload
                .get("status")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            if status != 0 {
                return Err(format!(
                    "Received non-success status while writing NV (id={id}, offset={offset}, status={status})"
                ));
            }
            remaining -= write_length;
        }
        Ok(())
    }

    /// Upstream `updateItem(id, data, autoInit)`.
    pub async fn update_item(
        &mut self,
        id: u16,
        data: &[u8],
        auto_init: bool,
    ) -> Result<(), String> {
        self.check_memory_alignment_setup()?;
        let current = self.read_item(id, 0).await?;
        if current.as_deref() != Some(data) {
            self.write_item(id, data, 0, auto_init).await?;
        }
        Ok(())
    }

    /// Upstream `deleteItem(id)`.
    pub async fn delete_item(&mut self, id: u16) -> Result<(), String> {
        self.check_memory_alignment_setup()?;
        let length_response = self
            .request_with_reply_retry(
                Subsystem::SYS,
                "osalNvLength",
                json!({"id": id}),
                &[ZnpCommandStatus::SUCCESS as u8],
            )
            .await?;
        let length = length_response
            .payload
            .get("length")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        if length > 0 {
            let delete_response = self
                .request_with_reply_retry(
                    Subsystem::SYS,
                    "osalNvDelete",
                    json!({"id": id, "len": length}),
                    &[
                        ZnpCommandStatus::SUCCESS as u8,
                        ZnpCommandStatus::NV_ITEM_INITIALIZED as u8,
                    ],
                )
                .await?;
            let status = delete_response
                .payload
                .get("status")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            if status != 0 && status != ZnpCommandStatus::NV_ITEM_INITIALIZED as u64 {
                return Err(format!(
                    "Received non-success status while deleting NV (id={id}, status={status})"
                ));
            }
        }
        Ok(())
    }

    /// Upstream `readExtendedTableEntry(sysId, id, subId, offset)`.
    pub async fn read_extended_table_entry(
        &mut self,
        sys_id: NvSystemIds,
        id: u16,
        sub_id: u16,
        offset: usize,
    ) -> Result<Option<Vec<u8>>, String> {
        self.check_memory_alignment_setup()?;
        let length_response = self
            .request_with_reply_retry(
                Subsystem::SYS,
                "nvLength",
                json!({"sysid": sys_id as u8, "itemid": id, "subid": sub_id}),
                &[ZnpCommandStatus::SUCCESS as u8],
            )
            .await?;
        let length = length_response
            .payload
            .get("len")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        if length == 0 {
            return Ok(None);
        }

        let read_response = self
            .request_with_reply_retry(
                Subsystem::SYS,
                "nvRead",
                json!({
                    "sysid": sys_id as u8,
                    "itemid": id,
                    "subid": sub_id,
                    "offset": offset,
                    "len": length,
                }),
                &[ZnpCommandStatus::SUCCESS as u8],
            )
            .await?;
        let status = read_response
            .payload
            .get("status")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        if status != 0 {
            return Err(format!(
                "Received non-success status while reading NV extended table entry (sysId={}, id={id}, subId={sub_id}, offset={offset}, status={status})",
                sys_id as u8
            ));
        }
        Ok(read_response
            .payload
            .get("value")
            .map(bytes)
            .filter(|value| !value.is_empty()))
    }

    /// Upstream `writeExtendedTableEntry(...)`.
    pub async fn write_extended_table_entry(
        &mut self,
        sys_id: NvSystemIds,
        id: u16,
        sub_id: u16,
        data: &[u8],
        offset: usize,
        auto_init: bool,
    ) -> Result<(), String> {
        self.check_memory_alignment_setup()?;
        let length_response = self
            .request_with_reply_retry(
                Subsystem::SYS,
                "nvLength",
                json!({"sysid": sys_id as u8, "itemid": id, "subid": sub_id}),
                &[ZnpCommandStatus::SUCCESS as u8],
            )
            .await?;
        let exists = length_response
            .payload
            .get("len")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            > 0;

        if !exists {
            if !auto_init {
                return Err(format!(
                    "Cannot write NV memory extended table item which does not exist (sudId={}, id={id}, subId={sub_id})",
                    sys_id as u8
                ));
            }
            let create_response = self
                .request_retry(
                    Subsystem::SYS,
                    "nvCreate",
                    json!({"sysid": sys_id as u8, "itemid": id, "subid": sub_id, "len": data.len()}),
                    &[ZnpCommandStatus::SUCCESS as u8],
                )
                .await?;
            match create_response {
                Some(create_response)
                    if create_response
                        .payload
                        .get("status")
                        .and_then(serde_json::Value::as_u64)
                        == Some(ZnpCommandStatus::SUCCESS as u64) => {}
                _ => {
                    return Err(format!(
                        "Failed to crate NV memory extended table item with status (sudId={}, id={id}, subId={sub_id})",
                        sys_id as u8
                    ))
                }
            }
        }

        let write_response = self
            .request_with_reply_retry(
                Subsystem::SYS,
                "nvWrite",
                json!({
                    "sysid": sys_id as u8,
                    "itemid": id,
                    "subid": sub_id,
                    "offset": offset,
                    "len": data.len(),
                    "value": data,
                }),
                &[ZnpCommandStatus::SUCCESS as u8],
            )
            .await?;
        let status = write_response
            .payload
            .get("status")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        if status != 0 {
            return Err(format!(
                "Received non-success status while writing NV extended table idem (sudId={}, id={id}, subId={sub_id}, offset={offset}, status={status})",
                sys_id as u8
            ));
        }
        Ok(())
    }

    /// Upstream `readTable(mode, ...)`.
    pub async fn read_table(&mut self, mode: TableMode) -> Result<Vec<Vec<u8>>, String> {
        self.check_memory_alignment_setup()?;
        let mut raw_entries = Vec::new();
        match mode {
            TableMode::Legacy { id, max_length } => {
                let mut entry_offset = 0u16;
                loop {
                    let raw_entry = self.read_item(id + entry_offset, 0).await?;
                    entry_offset += 1;
                    match raw_entry {
                        Some(raw_entry) => raw_entries.push(raw_entry),
                        None => break,
                    }
                    if entry_offset as usize >= max_length {
                        break;
                    }
                }
            }
            TableMode::Extended {
                sys_id,
                id,
                max_length,
            } => {
                let mut entry_offset = 0u16;
                loop {
                    let raw_entry = self
                        .read_extended_table_entry(sys_id, id, entry_offset, 0)
                        .await?;
                    entry_offset += 1;
                    match raw_entry {
                        Some(raw_entry) => raw_entries.push(raw_entry),
                        None => break,
                    }
                    if max_length.is_some_and(|max| entry_offset as usize >= max) {
                        break;
                    }
                }
            }
        }
        Ok(raw_entries)
    }

    /// Upstream `readTable(..., useTable)`.
    pub async fn read_table_as(
        &mut self,
        mode: TableMode,
        use_table: TableFactory,
    ) -> Result<Table, String> {
        let entries = self.read_table(mode).await?;
        let alignment = self.memory_alignment.expect("memory alignment");
        Ok(use_table(TableData::Entries(entries), alignment))
    }

    /// Upstream `writeTable(mode, ...)`.
    pub async fn write_table(&mut self, mode: TableMode, table: &Table) -> Result<(), String> {
        self.check_memory_alignment_setup()?;
        let alignment = self.memory_alignment.expect("memory alignment");
        match mode {
            TableMode::Legacy { id, .. } => {
                for (index, entry) in table.entries().iter().enumerate() {
                    self.write_item(
                        id + index as u16,
                        &entry.serialize_memory(alignment),
                        0,
                        true,
                    )
                    .await?;
                }
            }
            TableMode::Extended { sys_id, id, .. } => {
                for (index, entry) in table.entries().iter().enumerate() {
                    self.write_extended_table_entry(
                        sys_id,
                        id,
                        index as u16,
                        &entry.serialize_memory(alignment),
                        0,
                        true,
                    )
                    .await?;
                }
            }
        }
        Ok(())
    }

    fn check_memory_alignment_setup(&self) -> Result<(), String> {
        if self.memory_alignment.is_none() {
            return Err(
                "adapter memory alignment unknown - has nv memory driver been initialized?"
                    .to_string(),
            );
        }
        Ok(())
    }
}

fn bytes(payload: &serde_json::Value) -> Vec<u8> {
    payload
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(serde_json::Value::as_u64)
                .map(|value| value as u8)
                .collect()
        })
        .unwrap_or_default()
}
