// Original: .upstream/zigbee-herdsman/src/adapter/ember/uart/queues.ts
// TODO: port the full public API from the original file.

/// Upstream `EZSP_MAX_FRAME_LENGTH`.
pub const EZSP_MAX_FRAME_LENGTH: usize = 220;
/// Upstream `EZSP_HOST_RX_POOL_SIZE`.
pub const EZSP_HOST_RX_POOL_SIZE: usize = 32;
/// Upstream `TX_POOL_BUFFERS = EZSP_HOST_RX_POOL_SIZE + 5`.
pub const TX_POOL_BUFFERS: usize = EZSP_HOST_RX_POOL_SIZE + 5;

/// Upstream `EzspBuffer`.
#[derive(Debug, Clone, PartialEq)]
pub struct EzspBuffer {
    pub id: usize,
    /// Upstream `data: Buffer`.
    pub data: Vec<u8>,
    /// Upstream `link?: EzspBuffer`.
    pub link: Option<Box<EzspBuffer>>,
}

/// Upstream `EzspQueue`.
#[derive(Debug, Default)]
pub struct EzspQueue {
    pub length: usize,
    pub tail: Option<EzspBuffer>,
    items: Vec<EzspBuffer>,
}

impl EzspQueue {
    pub fn new() -> Self {
        Self::default()
    }

    /// Upstream `addTail(buffer)`.
    pub fn add_tail(&mut self, mut buffer: EzspBuffer) {
        buffer.link = None;
        self.tail = Some(buffer.clone());
        self.items.push(buffer);
        self.length += 1;
    }

    /// Upstream `removeHead()`.
    pub fn remove_head(&mut self) -> Option<EzspBuffer> {
        if self.items.is_empty() {
            return None;
        }
        let head = self.items.remove(0);
        self.length -= 1;
        self.tail = self.items.last().cloned();
        Some(head)
    }
}

/// Upstream `EzspFreeList`.
#[derive(Debug, Default)]
pub struct EzspFreeList {
    pub length: usize,
    /// Upstream `link?: EzspBuffer`.
    pub link: Option<EzspBuffer>,
}

impl EzspFreeList {
    pub fn new(count: usize, buffer_length: usize) -> Self {
        let mut free = Self::default();
        let mut next: Option<Box<EzspBuffer>> = None;
        for id in (0..count).rev() {
            next = Some(Box::new(EzspBuffer {
                id,
                data: vec![0; buffer_length],
                link: next,
            }));
        }
        if let Some(head) = next {
            free.link = Some(*head);
        }
        free.length = count;
        free
    }

    /// Upstream `allocBuffer()`.
    pub fn alloc_buffer(&mut self) -> Option<EzspBuffer> {
        let buffer = self.link.take()?;
        self.link = buffer.link.as_deref().cloned();
        self.length -= 1;
        Some(buffer)
    }

    /// Upstream `freeBuffer(buffer)`.
    pub fn free_buffer(&mut self, mut buffer: Option<EzspBuffer>) {
        if let Some(buffer) = buffer.as_mut() {
            buffer.link = self.link.take().map(Box::new);
            self.link = Some(buffer.clone());
            self.length += 1;
        }
    }
}

pub fn ezsp_buffer() {
    todo!()
}
pub fn ezsp_free_list() {
    todo!()
}
pub fn ezsp_queue() {
    todo!()
}
pub fn add_tail() {
    todo!()
}
pub fn alloc_buffer() {
    todo!()
}
pub fn free_buffer() {
    todo!()
}
pub fn get_nth_entry() {
    todo!()
}
pub fn get_preceding_entry() {
    todo!()
}
pub fn remove_entry() {
    todo!()
}
pub fn remove_head() {
    todo!()
}
