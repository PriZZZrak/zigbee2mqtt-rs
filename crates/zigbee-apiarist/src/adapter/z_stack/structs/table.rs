// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/structs/table.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

use super::r#struct::{Alignment, Struct};

/// Upstream `build(data | data[] | capacity)` input.
#[derive(Debug, Clone)]
pub enum TableData {
    Capacity(usize),
    Buffer(Vec<u8>),
    Entries(Vec<Vec<u8>>),
}

/// Upstream `Table<R>`.
#[derive(Debug, Clone)]
pub struct Table {
    entry_factory: fn(Option<&[u8]>) -> Struct,
    empty_entry: Struct,
    entries: Vec<Struct>,
    has_inline_length_header: bool,
    entry_occupancy_function: Option<fn(&Struct) -> bool>,
}

impl Table {
    /// Upstream `Table.new()`.
    pub fn new(entry_factory: fn(Option<&[u8]>) -> Struct) -> Self {
        Self {
            entry_factory,
            empty_entry: entry_factory(None),
            entries: Vec::new(),
            has_inline_length_header: false,
            entry_occupancy_function: None,
        }
    }

    /// Upstream `occupancy(fn)`.
    pub fn occupancy(mut self, function: fn(&Struct) -> bool) -> Self {
        self.entry_occupancy_function = Some(function);
        self
    }

    /// Upstream `inlineHeader(hasInlineHeader = true)`.
    pub fn inline_header(mut self, has_inline_header: bool) -> Self {
        self.has_inline_length_header = has_inline_header;
        self
    }

    /// Upstream `build(dataOrCapacity, alignment)`.
    pub fn build(mut self, data: TableData, alignment: Alignment) -> Self {
        match data {
            TableData::Entries(entries) => {
                if let Some(first) = entries.first() {
                    if !entries.iter().all(|entry| entry.len() == first.len()) {
                        panic!("All table entries need to be the same length");
                    }
                }
                self.entries = entries
                    .iter()
                    .map(|entry| (self.entry_factory)(Some(entry)))
                    .collect();
            }
            TableData::Buffer(data) => {
                let entry_length = self.empty_entry.get_length(alignment, true, 0);
                let data_length = if self.has_inline_length_header {
                    data.len().saturating_sub(2)
                } else {
                    data.len()
                };
                if data_length % entry_length != 0 {
                    panic!(
                        "Table length not divisible by entry length (alignment={}, data_length={}, entry_length={entry_length})",
                        alignment.as_str(),
                        data.len()
                    );
                }
                let capacity = data_length / entry_length;
                let entries_start = if self.has_inline_length_header {
                    data.get(2..).unwrap_or_default()
                } else {
                    data.as_slice()
                };
                self.entries = (0..capacity)
                    .map(|index| {
                        let start = index * entry_length;
                        (self.entry_factory)(Some(&entries_start[start..start + entry_length]))
                    })
                    .collect();
            }
            TableData::Capacity(capacity) => {
                self.entries = (0..capacity).map(|_| (self.entry_factory)(None)).collect();
            }
        }
        self
    }

    /// Upstream `capacity`.
    pub fn capacity(&self) -> usize {
        self.entries.len()
    }

    /// Upstream `entries`.
    pub fn entries(&self) -> &[Struct] {
        &self.entries
    }

    pub fn entry(&self, index: usize) -> Option<&Struct> {
        self.entries.get(index)
    }

    pub fn entry_mut(&mut self, index: usize) -> Option<&mut Struct> {
        self.entries.get_mut(index)
    }

    /// Upstream `used`.
    pub fn used(&self) -> Vec<&Struct> {
        let function = self
            .entry_occupancy_function
            .expect("Table usage cannot be determined without occupancy function when header is not present.");
        self.entries
            .iter()
            .filter(|entry| function(entry))
            .collect()
    }

    /// Upstream `free`.
    pub fn free(&self) -> Vec<&Struct> {
        let function = self
            .entry_occupancy_function
            .expect("Table usage cannot be determined without occupancy function when header is not present.");
        self.entries
            .iter()
            .filter(|entry| !function(entry))
            .collect()
    }

    /// Upstream `freeCount`.
    pub fn free_count(&self) -> usize {
        self.free().len()
    }

    /// Upstream `usedCount`.
    pub fn used_count(&self) -> usize {
        self.used().len()
    }

    /// Upstream `getNextFree()`.
    pub fn next_free(&self) -> Option<&Struct> {
        self.free().into_iter().next()
    }

    /// Index variant used by mutable restore procedures.
    pub fn next_free_index(&self) -> Option<usize> {
        let function = self
            .entry_occupancy_function
            .expect("Table usage cannot be determined without occupancy function when header is not present.");
        self.entries.iter().position(|entry| !function(entry))
    }

    /// Upstream `indexOf(entry)`.
    pub fn index_of(&self, entry: &Struct) -> Option<usize> {
        let target = entry.serialize(Alignment::Unaligned, true, 0);
        self.entries
            .iter()
            .position(|candidate| candidate.serialize(Alignment::Unaligned, true, 0) == target)
    }

    /// Upstream `serialize(alignment)`.
    pub fn serialize(&self, alignment: Alignment) -> Vec<u8> {
        let entry_length = self.empty_entry.get_length(alignment, true, 0);
        let header = if self.has_inline_length_header { 2 } else { 0 };
        let mut output = vec![0u8; header + self.entries.len() * entry_length];
        let mut offset = 0usize;
        if self.has_inline_length_header {
            output[0..2].copy_from_slice(&(self.used_count() as u16).to_le_bytes());
            offset += 2;
        }
        for entry in &self.entries {
            let data = entry.serialize(alignment, true, 0);
            output[offset..offset + data.len()].copy_from_slice(&data);
            offset += entry.get_length(alignment, true, 0);
        }
        output
    }
}
