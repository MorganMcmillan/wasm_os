use string_interner::symbol::SymbolU32;

use crate::kernel::Pid;

pub type EventData = [u8; 512];

pub struct Event {
    pub data: EventData,
    pub sent_by_pid: Pid,
    pub interned_name: SymbolU32,
}

impl Event {
    pub fn new(data: EventData, sent_by_pid: Pid, name: SymbolU32) -> Self {
        Self {
            data,
            sent_by_pid,
            interned_name: name,
        }
    }
}
