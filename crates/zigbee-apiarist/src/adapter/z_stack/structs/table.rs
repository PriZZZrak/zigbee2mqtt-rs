// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/structs/table.ts

use serde_json::{json, Value};

use super::r#struct::{Alignment, Struct};

/// Upstream `Table<T>`.
#[derive(Debug, Clone)]
pub struct Table {
    entry_factory: fn(Option<&[u8]>) -> Struct,
    entries: Vec<Struct>,
    capacity: usize,
    inline_header: bool,
    padding_byte: u8,
}

impl Table {
    pub fn new(entry_factory: fn(Option<&[u8]>) -> Struct, capacity: usize) -> Self {
        Self {
            entry_factory,
            entries: (0..capacity).map(|_| entry_factory(None)).collect(),
            capacity,
            inline_header: true,
            padding_byte: 0xff,
        }
    }

    /// Upstream `Table.build(data, source, alignment)`.
    pub fn build(mut self, data: Option<&[u8]>, alignment: Alignment) -> Self {
        let entry_length = (self.entry_factory)(None).get_length(alignment, false, 0);
        if let Some(data) = data {
            let mut offset = 0;
            for entry in &mut self.entries {
                let end = (offset + entry_length).min(data.len());
                if offset >= end {
                    break;
                }
                let bytes = data[offset..end].to_vec();
                *entry = std::mem::take(entry).build(Some(&bytes), None);
                offset = end;
            }
        }
        self
    }

    pub fn get_next_free(&self) -> Option<usize> {
        self.entries
            .iter()
            .position(|entry| entry.get_u8("isSet").unwrap_or(0) == 0)
    }

    pub fn index_of(&self, predicate: impl Fn(&Struct) -> bool) -> Option<usize> {
        self.entries.iter().position(predicate)
    }

    pub fn entry(&self, index: usize) -> Option<&Struct> {
        self.entries.get(index)
    }

    pub fn entry_mut(&mut self, index: usize) -> Option<&mut Struct> {
        self.entries.get_mut(index)
    }

    pub fn set_inline_header(&mut self, inline: bool) {
        self.inline_header = inline;
    }

    /// Upstream `occupancy()`: `(used, free)`.
    pub fn occupancy(&self) -> (usize, usize) {
        let used = self
            .entries
            .iter()
            .filter(|entry| entry.get_u8("isSet").unwrap_or(0) != 0)
            .count();
        (used, self.capacity - used)
    }

    /// Upstream `serialize(alignment, padLength, includeHeader)`.
    pub fn serialize(
        &self,
        alignment: Alignment,
        pad_length: bool,
        include_header: bool,
    ) -> Vec<u8> {
        let mut output = Vec::new();
        if include_header && self.inline_header {
            output.push((self.entries.len() & 0xff) as u8);
        }
        for entry in &self.entries {
            output.extend(entry.serialize(alignment, pad_length));
        }
        while output.len() % 2 != 0 && pad_length {
            output.push(self.padding_byte);
        }
        output
    }

    pub fn to_json(&self) -> Value {
        Value::Array(self.entries.iter().map(Struct::to_json).collect())
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn as_structs(&self) -> &[Struct] {
        &self.entries
    }

    pub fn as_structs_mut(&mut self) -> &mut [Struct] {
        &mut self.entries
    }

    pub fn build_json(&self) -> Value {
        json!({"occupancy": {
            "used": self.occupancy().0,
            "free": self.occupancy().1,
        }, "entries": self.to_json()})
    }
}
