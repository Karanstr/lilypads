#![warn(missing_docs)]
//! Fun little virtual memory allocator.
//! 
//! This is a learning experience for me and should be used with a mountain of salt.
//! At some point I need to rename it, but I don't have a good one yet.
//! 
//! This crate is intended for the creation of graphs and similar data structures, with a focus on storing data contiguously in memory while allowing it to have multiple owners. Internally the data is stored in a [Vec].
//! 
//! This crate does not yet support Weak or Atomic references to data, that's on the todo list (maybe).
//! 
//! Errors which are caused by external factors are handled, canceling the request and returning an [AccessError].
//! Errors which are caused by internal factors, such as a failure in the allocation process, will be automatically repaired. If the repair fails, we will panic!().
//! 
//! # Example
//! ```
//! use vec_mem_heap::prelude::*;
//! 
//! fn main() {
//! 
//!     let mut storage : NodeField<u32> = NodeField::new();
//! 
//!     // When you push data into the structure, it returns the index that data was stored at and sets the reference count to 1.
//!     let data1 = storage.push(15); // data1 == Index(0)
//!
//!     {
//!         let data2 = storage.push(72); // data2 == Index(1)
//! 
//!         // Now that a second reference to the data in Index(0) exists, we have to manually add to the reference count.
//!         let data3 = data1;
//!         storage.add_ref(data3);
//!     
//!         // data2 and data3 are about to go out of scope, so we have to manually remove their references.
//!         // returns Ok( Some(72) ) -> The data at Index(1) only had one reference, so it was freed.
//!         storage.remove_ref(data2);
//! 
//!         // returns Ok( None ) -> The data at Index(0) had two references, now one.
//!         storage.remove_ref(data3); 
//!     }
//! 
//!     // returns Ok( &15 ) -> The data at Index(0) still has one reference (data1).
//!     dbg!( storage.data( Index(0) ) );
//!     // Err( AccessError::FreeMemory(Index(1)) ) -> The data at Index(1) was freed when its last reference was removed.
//!     dbg!( storage.data( Index(1) ) );
//! 
//! }
//! ```

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::num::NonZeroUsize;

/// Common types and traits exported for convenience.
/// 
/// This module re-exports the most commonly used types from this crate.
/// Import everything from this module with `use vec_mem_heap::prelude::*`.
pub mod prelude {
    pub use super::{
        NodeField, 
        Index,
        enums::AccessError
    };
}

/// Internal type(s) exported for situations where you need to implement traits on the NodeField which require bounds or methods which aren't avaliable.
pub mod internals { pub use super::containers::MemorySlot; }

/// A trait which allows you to customize how indexes are stored on your side of the api
pub trait Indexable {
    ///Allows the library to convert your type to its internal [Index] representation
    fn to_index(&self) -> Index;
}
/// A newtype wrapper to represent indexes, the default implementation if you don't want to create your own [Indexable]
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Index(pub usize);
impl std::ops::Deref for Index {
    type Target = usize;
    fn deref(&self) -> &Self::Target { &self.0 }
}
impl Indexable for Index {
    fn to_index(&self) -> Index { *self }
}


/// Error type(s) used throughout the crate.
/// 
/// Contains [AccessError] for error reporting when and how operations fail.
mod enums {
    use super::Index;
    /// Errors which may occur while accessing and modifying memory.
    #[derive(Debug)]
    pub enum AccessError {
        /// Returned when attempting to access an index beyond the length of [MemHeap]'s internal storage
        OutOfBoundsMemory(Index),
        /// Returned when attempting to access an index which isn't currently allocated
        FreeMemory(Index),
        /// Returned when the type of data requested doesn't match the type of data stored
        MisalignedTypes,
        /// Catch all for operation failure
        OperationFailed,
    }

    #[derive(Debug)]
    pub(crate) enum ReferenceError {
        OverUnder,
        Dangling,
    }
}
use enums::{AccessError, ReferenceError};
/// Internal container type(s) for memory management.
mod containers {
    use super::*;

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Steward<T> {
        pub data : T,
        pub rc : RefCount,
    }

    #[derive(Debug, Serialize, Deserialize)]
    #[repr(transparent)]
    pub struct RefCount(NonZeroUsize);
    impl RefCount {
        pub(crate) fn new() -> Self {
            Self(NonZeroUsize::new(1).unwrap())
        }

        pub(crate) fn modify_ref(&mut self, delta:isize) -> Result<NonZeroUsize, ReferenceError> {
            let current = self.0.get(); // get the raw usize
            let new_ref_count = if delta < 0 {
                if let Some(count) = current.checked_sub(delta.abs() as usize) {
                    count
                } else { return Err(ReferenceError::OverUnder) }
            } else {
                if let Some(count) = current.checked_add(delta as usize) {
                    count
                } else { return Err(ReferenceError::OverUnder) }
            };
            match NonZeroUsize::new(new_ref_count) {
                None => Err(ReferenceError::Dangling),
                Some(ref_count) => {
                    self.0 = ref_count;
                    Ok(ref_count)
                }
            }
        }

        pub fn ref_count(&self) -> NonZeroUsize { self.0 }
    }
    
    /// The container placed in each slot of allocated memory
    #[derive(Debug, Serialize, Deserialize)]
    pub enum MemorySlot<T> {
        /// Notes this memory slot is free and points to the next free slot
        Free(Index),
        /// Notes this memory slot contains data
        Occupied(Steward<T>),
    }
    impl<T> MemorySlot<T> {
        pub(crate) fn steward(data:T) -> Self {
            Self::Occupied(Steward {
                data,
                rc: RefCount::new(),
            })
        }
        /// Handles boilerplate for unwrapping a &[Steward] from a [MemorySlot]
        pub fn unwrap_steward(&self) -> Result<&Steward<T>, AccessError> {
            if let MemorySlot::Occupied(steward) = self {
                Ok(steward)
            } else { Err( AccessError::MisalignedTypes ) }
        }
        /// Handles boilerplate for unwrapping a &mut [Steward] from a [MemorySlot]
        pub fn unwrap_steward_mut(&mut self) -> Result<&mut Steward<T>, AccessError> {
            if let MemorySlot::Occupied(steward) = self {
                Ok(steward)
            } else { Err( AccessError::MisalignedTypes ) }
        }
    }

}
use containers::*;

/// Used to allocate space on the heap, read from that space, and write to it.
#[derive(Serialize, Deserialize, Debug)]
pub struct NodeField<T:Clone> {
    /// The container used to manage allocated memory
    memory : Vec< MemorySlot<T> >,
    first_free : Index,
}

// Private methods
impl<T:Clone> NodeField<T> {
    fn last_index(&self) -> Index {
        Index(self.memory.len() - 1)
    }

    fn mut_slot(&mut self, index:Index) -> Result<&mut MemorySlot<T>, AccessError> {
        match index {
            bad_index if index > self.last_index() => Err( AccessError::OutOfBoundsMemory(bad_index) ),
            index => match &mut self.memory[*index] {
                MemorySlot::Free { .. } => Err( AccessError::FreeMemory(index) ),
                MemorySlot::Occupied { .. } => Ok( &mut self.memory[*index] ),
            }
        }
    }

    fn slot(&self, index:Index) -> Result<&MemorySlot<T>, AccessError> {
        match index {
            bad_index if index > self.last_index() => Err( AccessError::OutOfBoundsMemory(bad_index) ),
            index => match &self.memory[*index] {
                MemorySlot::Free { .. } => Err( AccessError::FreeMemory(index) ),
                MemorySlot::Occupied { .. } => Ok( &self.memory[*index] ),
            }
        }
    }

    fn free_index(&mut self, index:Index) -> Option<T> {
        let data = {
            let Ok(slot) = self.mut_slot(index) else { return None };
            match slot.unwrap_steward() {
                Ok(steward) => steward.data.clone(),
                Err(error) => {
                    dbg!("Error while freeing index {}: {}", index, error);
                    return None
                },
            }
        };
        let next_free = if *self.first_free == self.memory.len() { index } else { self.first_free };
        self.memory[*index] = MemorySlot::Free(next_free);
        self.first_free = index;
        Some(data)
    }

    fn reserve_index(&mut self) -> Option<Index> {
        let first_free = self.first_free;
        if *first_free == self.memory.len() { 
            self.first_free = Index(self.memory.len() + 1);
            return None 
        }
        let next_free = if let MemorySlot::Free(next_free) = self.memory[*first_free]  {
            next_free
        } else { 
            dbg!("Repairing allocator");
            self.repair_and_sort_allocator();
            let MemorySlot::Free(next_free) = self.memory[*first_free] else { panic!("Failed to repair allocator") };
            next_free
        };
        if next_free == first_free { 
            if self.memory.len() == 0 { self.first_free = Index(1) }
            else { self.first_free = Index(self.memory.len()) }
        } else { self.first_free = next_free; }
        Some(first_free)
    }
}

//Public functions
impl<T:Clone> NodeField<T> {
    /// Constructs a new `NodeField` which can store data of type `T` 
    /// # Example
    /// ```
    /// //Stores i32s
    /// let mut storage = NodeField::<i32>::new();
    /// ```
    pub fn new() -> Self {
        Self {
            memory : Vec::new(),
            first_free : Index(0),
        }
    }

    /// Returns an immutable reference to the data stored at the requested index, or an [AccessError] if there is a problem.
    pub fn data<I:Indexable>(&self, index:I) -> Result<&T, AccessError> {
        Ok(&self.slot(index.to_index())?.unwrap_steward()?.data)
    }

    /// Returns a mutable reference to the data stored at the requested index, or an [AccessError] if there is a problem.
    pub fn mut_data<I:Indexable>(&mut self, index:I) -> Result<&mut T, AccessError> {
        Ok(&mut self.mut_slot(index.to_index())?.unwrap_steward_mut()?.data)
    }

    /// Tells the NodeField that something else references the data at `index`.
    /// So long as the NodeField thinks there is at least one reference, the data won't be freed.
    /// 
    /// Failure to properly track references will lead to either freeing data you wanted or leaking data you didn't.
    pub fn add_ref<I:Indexable>(&mut self, index:I) -> Result<(), AccessError> {
        match self.mut_slot(index.to_index())?.unwrap_steward_mut()?.rc.modify_ref(1) {
            Err(ReferenceError::OverUnder) => Err(AccessError::OperationFailed),
            _ => Ok(())
        }
    }

    /// Tells the NodeField that something no longer references the data at `index`.
    /// If calling this function renders the refcount 0, the data will be freed and returned.
    /// 
    /// Failure to properly track references will lead to either freeing data you wanted or leaking data you didn't.
    pub fn remove_ref<I:Indexable>(&mut self, index:I) -> Result<Option<T>, AccessError> {
        let internal_index = index.to_index();
        if let Err(ReferenceError::Dangling) = self.mut_slot(internal_index)?.unwrap_steward_mut()?.rc.modify_ref(-1) {
            Ok( self.free_index(internal_index) )
        } else { Ok(None) }
    }

    /// Returns the number of references the data at `index` has or an [AccessError] if the request has a problem
    pub fn status<I:Indexable>(&self, index:I) -> Result<NonZeroUsize, AccessError> {
        Ok(self.slot(index.to_index())?.unwrap_steward()?.rc.ref_count())
    }

    /// Pushes `data` into the NodeField, returning the index it was stored at.
    /// 
    /// Once you recieve the index the data was stored at, it is your responsibility to manage its references.
    /// The data will start with one reference.
    #[must_use]
    pub fn push(&mut self, data:T) -> Index {
        match self.reserve_index() {
            Some(index) => {
                self.memory[*index] = MemorySlot::steward(data);
                index
            },
            None => {
                self.memory.push(MemorySlot::steward(data));
                Index(self.memory.len() - 1)
            },
        }
    }

    /// Replaces the data at `index` with `new_data`, returning the replaced data on success and an [AccessError] on failure.
    /// You may not replace an index which is currently free. 
    /// There isn't currently a way to select the index you wish to insert at, though you can view the next free index with [NodeField::next_allocated]
    pub fn replace<I:Indexable>(&mut self, index:I, new_data:T) -> Result<T, AccessError> {
        let wrapper = self.mut_slot(index.to_index())?.unwrap_steward_mut()?;
        let old_data = wrapper.data.clone();
        wrapper.data = new_data;
        Ok(old_data)
    }

    /// Returns an immutable reference to the internal memory Vec 
    pub fn internal_memory(&self) -> &Vec< MemorySlot<T> > { &self.memory } 
    
    /// Returns the next index which will be allocated on a [NodeField::push] call
    pub fn next_allocated(&self) -> Index { self.first_free }
    
    /// Travels through memory and refreshes all free slots. 
    /// This process also re-arranges the memory allocation order such that until a new slot is freed allocation will occur in order from the smallest index to the largest.
    /// 
    /// This operation is O(n) to the number of slots in memory.
    pub fn repair_and_sort_allocator(&mut self) {
        let mut next_free = None;
        for (index, slot) in self.memory.iter_mut().enumerate().rev() {
            if let MemorySlot::Free(_) = slot {
                *slot = MemorySlot::Free(next_free.unwrap_or(Index(index)));
                next_free = Some(Index(index));
            }
        }
        self.first_free = next_free.unwrap_or(Index(self.memory.len()));
    }

    /// Travels through memory and re-arranges slots so that they are contiguous in memory, with no free slots in between occupied ones.
    /// The hashmap returned can be used to remap your references to their new locations. (Key:Old, Value:New)
    /// 
    /// Slots at the back of memory will be placed in the first free slot, until the above condition is met.
    /// The allocator will then be repaired using [NodeField::repair_and_sort_allocator].
    /// 
    /// This operation is O(n) to the number of slots in memory.
    #[must_use]
    pub fn defrag(&mut self) -> HashMap<Index, Index> {
        let mut remapped = HashMap::new();
        let mut solid_until = 0;
        if solid_until == self.memory.len() { return remapped }
        let mut free_until = self.memory.len() - 1;
        'defrag: loop {
            while let MemorySlot::Occupied(_) = self.memory[solid_until] { 
                solid_until += 1;
                if solid_until == free_until { break 'defrag }
            }
            while let MemorySlot::Free(_) = self.memory[free_until] { 
                free_until -= 1;
                if free_until == solid_until { break 'defrag }
            }
            remapped.insert(Index(free_until), Index(solid_until));
            self.memory.swap(free_until, solid_until);
        }
        self.repair_and_sort_allocator();
        remapped
    }

    /// [NodeField::defrag]s the memory, then shrinks the internal memory Vec to the size of the block of occupied memory.
    #[must_use]
    pub fn trim(&mut self) -> HashMap<Index, Index> {
        let remap = self.defrag();
        self.memory.truncate(*self.first_free);
        remap
    }

}
