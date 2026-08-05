/// A versioned Id.
/// This is used to identify resources used by processes and the kernel, such as processes
/// themselves, open files, Etc.
/// Versioning is implemented so resources can be efficiently indexed.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Id(u16, u16);

impl Id {
    pub const fn new(number: u16) -> Self {
        Self(number, 0)
    }

    pub fn number(&self) -> u16 {
        self.0
    }

    pub fn version(&self) -> u16 {
        self.1
    }

    pub fn increment_version(&mut self) {
        // Wraps around to prevent panicing
        self.1 = self.1.wrapping_add(1)
    }

    pub fn as_i32(&self) -> i32 {
        self.number() as i32 | (self.version() as i32) << 16
    }

    pub fn from_i32(number: i32) -> Self {
        Self(number as u16, (number >> 16) as u16)
    }
}

/// An IdStore is used to associate data with versioned Ids.
/// Ids always start at 1, so that 0 can represent a universally invalid id.
pub struct IdStore<T> {
    /// Dense Mapping of ids back to the array of live ids
    dense_array: Vec<(usize, Option<T>)>,
    /// The array of live and dead ids
    ids: Vec<Id>,
    /// How many live ids there are.
    /// This is also the index of the first dead id.
    live_count: usize,
    /// The next id number to be created when there are no more ids to recycle
    next_id: u16,
}

impl<T> IdStore<T> {
    pub fn new() -> Self {
        Self {
            dense_array: Vec::with_capacity(32),
            ids: Vec::with_capacity(32),
            live_count: 0,
            next_id: 0,
        }
    }

    /// Associates an item with a new id.
    pub fn new_id(&mut self, item: T) -> Id {
        if self.live_count == self.ids.len() {
            self.make_new_id(item)
        } else {
            // Recycle id
            let id = &mut self.ids[self.live_count];
            id.increment_version();
            self.dense_array[id.number() as usize - 1] = (self.live_count, Some(item));
            self.live_count += 1;
            *id
        }
    }

    fn make_new_id(&mut self, item: T) -> Id {
        self.next_id += 1;
        let id = Id::new(self.next_id);
        self.ids.push(id);
        self.dense_array.push((self.ids.len() - 1, Some(item)));
        self.live_count += 1;
        id
    }

    /// Gets the data associated with this Id.
    pub fn data(&self, id: Id) -> Option<&T> {
        self.dense_array
            .get(id.number() as usize)
            .and_then(|i| i.1.as_ref())
    }

    /// Mutably gets the data associated with this Id.
    pub fn data_mut(&mut self, id: Id) -> Option<&mut T> {
        self.dense_array
            .get_mut(id.number() as usize - 1)
            .and_then(|i| i.1.as_mut())
    }

    fn get_dense_index(&self, id: Id) -> Option<usize> {
        self.dense_array.get(id.number() as usize - 1).map(|i| i.0)
    }

    /// Checks if the id is still valid.
    /// An id is invalid if it is deleted, not yet created, or has a different version number.
    pub fn id_is_valid(&self, id: Id) -> bool {
        let Some(index) = self.get_dense_index(id) else {
            return false;
        };

        if self.live_count < index {
            return false;
        }

        id == self.ids[index]
    }

    fn delete_data(&mut self, id: Id) {
        if let Some(pair) = self.dense_array.get_mut(id.number() as usize - 1) {
            pair.1 = None;
        }
    }

    /// Attempts to delete the given id.
    /// Returns false if the id is not valid.
    pub fn delete_id(&mut self, id: Id) -> bool {
        if !self.id_is_valid(id) {
            return false;
        }

        self.delete_data(id);
        let index = self.get_dense_index(id).unwrap();
        let last_live_index = self.live_count - 1;

        self.ids.swap(index, last_live_index);
        self.live_count -= 1;

        true
    }
}
