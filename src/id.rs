#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Id(u16, u16);

impl Id {
    pub fn new(foo: u16) -> Self {
        Self(foo, 0)
    }

    pub fn number(&self) -> u16 {
        self.0
    }

    pub fn version(&self) -> u16 {
        self.1
    }

    pub fn increment_version(&mut self) {
        self.1 += 1
    }
}

/// An IdStore is used to associate data with versioned Ids.
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

    // TODO: needs testing
    pub fn new_id(&mut self, data: T) -> Id {
        if self.live_count == self.ids.len() {
            self.make_new_id(data)
        } else {
            // Recycle id
            let id = &mut self.ids[self.live_count];
            id.increment_version();
            self.dense_array[id.number() as usize] = (self.live_count, Some(data));
            self.live_count += 1;
            *id
        }
    }

    fn make_new_id(&mut self, data: T) -> Id {
        let id = Id::new(self.next_id);
        self.next_id += 1;
        self.ids.push(id);
        self.dense_array.push((self.ids.len() - 1, Some(data)));
        self.live_count += 1;
        id
    }

    pub fn data(&self, id: Id) -> Option<&T> {
        self.dense_array
            .get(id.number() as usize)
            .and_then(|i| i.1.as_ref())
    }

    pub fn data_mut(&mut self, id: Id) -> Option<&mut T> {
        self.dense_array
            .get_mut(id.number() as usize)
            .and_then(|i| i.1.as_mut())
    }

    fn get_dense_index(&self, id: Id) -> Option<usize> {
        self.dense_array.get(id.number() as usize).map(|i| i.0)
    }

    /// Tests for if the id is still valid.
    /// An id is invalid if it is deleted, not yet created, or has a different version number.
    pub fn id_is_valid(&self, id: Id) -> bool {
        let Some(index) = self.get_dense_index(id) else {
            return false;
        };

        if index < self.live_count {
            return false;
        }

        id == self.ids[index]
    }

    fn delete_data(&mut self, id: Id) {
        if let Some(pair) = self.dense_array.get_mut(id.number() as usize) {
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
