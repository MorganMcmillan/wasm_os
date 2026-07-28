use std::sync::Arc;

use string_interner::symbol::SymbolU32;

use crate::kernel::Pid;

pub type EventName = SymbolU32;

pub struct Event {
    pub info: Arc<EventInfo>,
    pub sent_by_pid: Pid,
}

pub struct EventInfo {
    pub data: Box<[u8]>,
    pub interned_name: EventName,
}

impl Event {
    pub fn new(data: Box<[u8]>, sent_by_pid: Pid, interned_name: EventName) -> Self {
        Self {
            info: Arc::new(EventInfo {
                data,
                interned_name,
            }),
            sent_by_pid,
        }
    }

    pub fn from_resent(event: &Event, sender: Pid) -> Self {
        Self {
            info: event.info.clone(),
            sent_by_pid: sender,
        }
    }

    pub fn data(&self) -> &[u8] {
        &self.info.data
    }

    pub fn interned_name(&self) -> EventName {
        self.info.interned_name
    }
}
