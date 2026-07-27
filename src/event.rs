use string_interner::symbol::SymbolU32;

use crate::kernel::Pid;

pub struct Event {
    pub data: Box<[u8]>,
    pub sent_by_pid: Pid,
    pub interned_name: SymbolU32,
}

impl Event {
    pub fn new(data: Box<[u8]>, sent_by_pid: Pid, interned_name: SymbolU32) -> Self {
        Self {
            data,
            sent_by_pid,
            interned_name,
        }
    }
}
