// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/adapter/adapter-nv-memory.ts

use serde_json::{json, Value};

use crate::adapter::z_stack::structs::r#struct::Alignment;
use crate::adapter::z_stack::unpi::constants::Subsystem;
use crate::adapter::z_stack::znp::znp::Znp;

const NWKKEY: u16 = 130;
const RETRIES: usize = 3;
const MAX_WRITE_SIZE: usize = 240;

/// Upstream `AdapterNvMemory`.
pub struct AdapterNvMemory {
    pub znp: Znp,
    pub memory_alignment: Option<Alignment>,
}

impl AdapterNvMemory {
    pub fn new(znp: Znp) -> Self {
        Self {
            znp,
            memory_alignment: None,
        }
    }

    async fn request(
        &mut self,
        command: &str,
        payload: Value,
    ) -> Result<crate::adapter::z_stack::znp::zpi_object::ZpiObject, String> {
        let mut last = String::new();
        for _ in 0..RETRIES {
            match self
                .znp
                .request_with_reply(Subsystem::SYS, command, payload.clone())
                .await
            {
                Ok(object) => return Ok(object),
                Err(error) => last = error,
            }
        }
        Err(last)
    }

    /// Upstream `init()`: determine memory alignment from NWKKEY length.
    pub async fn init(&mut self) -> Result<(), String> {
        let key = self.read_item(NWKKEY, 0).await?;
        self.memory_alignment = Some(match key.as_ref().map(Vec::len) {
            Some(21) => Alignment::Unaligned,
            _ => Alignment::Aligned,
        });
        Ok(())
    }

    /// Upstream `readItem(id, offset)`.
    pub async fn read_item(&mut self, id: u16, offset: u16) -> Result<Option<Vec<u8>>, String> {
        let length_response = self.request("osalNvLength", json!({"id": id})).await?;
        let length = length_response
            .payload
            .get("length")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize;
        if length == 0 {
            return Ok(None);
        }
        let mut buffer = Vec::with_capacity(length);
        let mut offset = offset as usize;
        while offset < length {
            let response = self
                .request("osalNvReadExt", json!({"id": id, "offset": offset as u64}))
                .await?;
            if response.payload.get("status").and_then(Value::as_u64) != Some(0) {
                return Err(format!(
                    "Received non-success status while reading NV (id={id}, offset={offset})"
                ));
            }
            let value = response
                .payload
                .get("value")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_u64)
                        .map(|value| value as u8)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if value.is_empty() {
                break;
            }
            offset += value.len();
            buffer.extend_from_slice(&value);
        }
        buffer.truncate(length);
        Ok(Some(buffer))
    }

    /// Upstream `writeItem(id, data, offset, autoInit)`.
    pub async fn write_item(
        &mut self,
        id: u16,
        data: &[u8],
        offset: u16,
        auto_init: bool,
    ) -> Result<(), String> {
        let length_response = self.request("osalNvLength", json!({"id": id})).await?;
        let exists = length_response
            .payload
            .get("length")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            > 0;
        if !exists {
            if !auto_init {
                return Err(format!(
                    "Cannot write NV memory item which does not exist (id={id})"
                ));
            }
            let init_length = data.len().min(MAX_WRITE_SIZE);
            self.request(
                "osalNvItemInit",
                json!({
                    "id": id,
                    "len": data.len(),
                    "initlen": init_length,
                    "initvalue": &data[..init_length],
                }),
            )
            .await?;
        }

        let mut remaining = data.len();
        let mut position = offset as usize;
        while remaining > 0 {
            let write_length = remaining.min(MAX_WRITE_SIZE);
            let start = position - offset as usize;
            self.request(
                "osalNvWriteExt",
                json!({
                    "id": id,
                    "offset": position,
                    "len": write_length,
                    "value": &data[start..start + write_length],
                }),
            )
            .await?;
            position += write_length;
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
        let current = self.read_item(id, 0).await?;
        if current.as_deref() != Some(data) {
            self.write_item(id, data, 0, auto_init).await?;
        }
        Ok(())
    }

    /// Upstream `deleteItem(id)`.
    pub async fn delete_item(&mut self, id: u16) -> Result<(), String> {
        let length_response = self.request("osalNvLength", json!({"id": id})).await?;
        let length = length_response
            .payload
            .get("length")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        if length > 0 {
            self.request("osalNvDelete", json!({"id": id, "len": length}))
                .await?;
        }
        Ok(())
    }

    /// Upstream `readExtendedTableEntry(sysId, id, subId, offset)`.
    pub async fn read_extended_table_entry(
        &mut self,
        sys_id: u8,
        id: u16,
        sub_id: u16,
        offset: u16,
    ) -> Result<Option<Vec<u8>>, String> {
        let length_response = self
            .request(
                "nvLength",
                json!({"sysid": sys_id, "itemid": id, "subid": sub_id}),
            )
            .await?;
        let length = length_response
            .payload
            .get("len")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u8;
        if length == 0 {
            return Ok(None);
        }
        let response = self
            .request(
                "nvRead",
                json!({
                    "sysid": sys_id,
                    "itemid": id,
                    "subid": sub_id,
                    "offset": offset,
                    "len": length,
                }),
            )
            .await?;
        if response.payload.get("status").and_then(Value::as_u64) != Some(0) {
            return Err(format!(
                "Received non-success status while reading NV extended table entry (sysId={sys_id}, id={id}, subId={sub_id})"
            ));
        }
        Ok(response
            .payload
            .get("value")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_u64)
                    .map(|value| value as u8)
                    .collect()
            }))
    }

    /// Upstream `writeExtendedTableEntry(...)`.
    pub async fn write_extended_table_entry(
        &mut self,
        sys_id: u8,
        id: u16,
        sub_id: u16,
        data: &[u8],
        offset: u16,
        _auto_init: bool,
    ) -> Result<(), String> {
        self.request(
            "nvWrite",
            json!({
                "sysid": sys_id,
                "itemid": id,
                "subid": sub_id,
                "offset": offset,
                "len": data.len(),
                "value": data,
            }),
        )
        .await?;
        Ok(())
    }
}
