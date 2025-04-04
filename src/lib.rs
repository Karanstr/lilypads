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
//!     dbg!( storage.data( data1 ) );
//!     // Err( AccessError::FreeMemory(Index(1)) ) -> The data at Index(1) was freed when its last reference was removed.
//!     dbg!( storage.data( Index(1) ) );
//! 
//! }
//! ```

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::mem::replace;

/// Common types and traits exported for convenience.
/// 
/// This module re-exports the essential types and traits.
/// Import everything from this module with `use vec_mem_heap::prelude::*`.
pub mod prelude {
    pub use super::{
        NodeField, 
        Indexable,
        AccessError
    };
}

/// A trait which allows you to customize how indexes are stored on your side of the api
pub trait Indexable {
    ///Allows the library to convert your type to its internal [Index] representation
    fn to_index(&self) -> Index;
}
type Index = usize;
impl Indexable for usize {
    fn to_index(&self) -> Index { *self }
}


/// Errors which may occur while accessing and modifying memory.
#[derive(Debug)]
pub enum AccessError {
    /// Returned when attempting to access an index beyond the length of [MemHeap]'s internal storage
    OutOfBoundsMemory(Index),
    /// Returned when attempting to access an index which isn't currently allocated
    FreeMemory(Index),
    /// Returned when the type of data requested doesn't match the type of data stored
    MisalignedTypes,
    /// Returned when a reference operation causes an over/underflow
    ReferenceOverflow,
    /// Catch all for operation failure
    OperationFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Steward<T> {
    data : T,
    rc : NonZeroUsize,
}
impl<T> Steward<T> {
    pub fn new(data: T) -> Self {
        Self { data, rc: NonZeroUsize::new(1).unwrap() }
    }

    pub(crate) fn modify_ref(&mut self, delta:isize) -> Result<bool, AccessError> {
        let current = self.rc.get();
        let new_ref_count = match delta.is_negative() {
            true => current.checked_sub(delta.abs() as usize),
            false => current.checked_add(delta as usize)
        }.ok_or(AccessError::ReferenceOverflow)?;
        Ok( match NonZeroUsize::new(new_ref_count) {
            Some(count) => { self.rc = count; true },
            None => false
        })
    }

}
    
/// Used to allocate space on the heap, read from that space, and write to it.
#[derive(Serialize, Deserialize, Debug)]
pub struct NodeField<T:Clone> {
    /// The container used to manage memory
    memory : Vec< Option< Steward<T> > >,
    /// A bitmap of allocated and free memory slots
    slot_mask : Vec<usize>,
}

// Private methods
impl<T:Clone> NodeField<T> {
    fn last_index(&self) -> Index {
        self.memory.len() - 1
    }

    fn mut_slot(&mut self, index:Index) -> Result<&mut Steward<T>, AccessError> {
        if self.memory.is_empty() { return Err(AccessError::OutOfBoundsMemory(index)) }
        match index {
            bad_index if index > self.last_index() => Err( AccessError::OutOfBoundsMemory(bad_index) ),
            index => match &mut self.memory[index] {
                Some(steward) => Ok( steward ),
                None => Err( AccessError::FreeMemory(index) ),
            }   
        }
    }

    fn slot(&self, index:Index) -> Result<&Steward<T>, AccessError> {
        if self.memory.is_empty() { return Err(AccessError::OutOfBoundsMemory(index)) }
        match index {
            bad_index if index > self.last_index() => Err( AccessError::OutOfBoundsMemory(bad_index) ),
            index => match &self.memory[index] {
                Some(steward) => Ok( steward ),
                None => Err( AccessError::FreeMemory(index) ),
            }
        }
    }

    fn first_free(&self) -> Option<Index> {
        let bits_per_cell = usize::BITS as usize;
        for (cell, mask) in self.slot_mask.iter().enumerate() {
            if *mask != usize::MAX {
                return Some(cell * bits_per_cell + mask.trailing_ones() as usize);
            }
        }
        None
    }

    fn mark_free(&mut self, index:Index) {
        let bits_per_cell = usize::BITS as usize;
        let cell = index / bits_per_cell;
        let mask = 1 << (index % bits_per_cell);
        self.slot_mask[cell] &= !mask;
    }

    fn mark_reserved(&mut self, index:Index) {
        let bits_per_cell = usize::BITS as usize;
        let cell = index / bits_per_cell;
        let mask = 1 << (index % bits_per_cell);
        self.slot_mask[cell] |= mask;
    }

    fn release(&mut self, index:Index) -> T {
        self.mark_free(index);
        let Ok(_) = self.mut_slot(index) else { panic!("Tried to release a free/OoB slot"); };
        self.memory[index].take().unwrap().data
    }

    #[must_use]
    fn reserve(&mut self) -> Index {
        let index = match self.first_free() {
            Some(index) => {
                if self.memory.is_empty() || index > self.last_index() {
                    self.memory.push(None);
                }
                index
            },
            None => {
                self.memory.push(None);
                self.slot_mask.push(0);
                self.last_index()
            }
        };
        self.mark_reserved(index);
        index
    }

}

// Public functions
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
            slot_mask : vec![0],
        }
    }

    /// Returns an immutable reference to the data stored at the requested index, or an [AccessError] if there is a problem.
    pub fn data<I:Indexable>(&self, index:I) -> Result<&T, AccessError> {
        Ok(&self.slot(index.to_index())?.data)
    }

    /// Returns a mutable reference to the data stored at the requested index, or an [AccessError] if there is a problem.
    pub fn mut_data<I:Indexable>(&mut self, index:I) -> Result<&mut T, AccessError> {
        Ok(&mut self.mut_slot(index.to_index())?.data)
    }

    /// Tells the NodeField that something else references the data at `index`.
    /// So long as the NodeField thinks there is at least one reference, the data won't be freed.
    /// 
    /// Failure to properly track references will lead to either freeing data you wanted or leaking data you didn't.
    pub fn add_ref<I:Indexable>(&mut self, index:I) -> Result<(), AccessError> {
        self.mut_slot(index.to_index())?.modify_ref(1)?;
        Ok(())
    }

    /// Tells the NodeField that something no longer references the data at `index`.
    /// If calling this function renders the refcount 0, the data will be freed and returned.
    /// 
    /// Failure to properly track references will lead to either freeing data you wanted or leaking data you didn't.
    pub fn remove_ref<I:Indexable>(&mut self, index:I) -> Result<Option<T>, AccessError> {
        let internal_index = index.to_index();
        match self.mut_slot(internal_index)?.modify_ref(-1)? {
            false => Ok( Some( self.release(internal_index) ) ),
            true => Ok(None)
        }
    }

    /// Returns the number of references the data at `index` has or an [AccessError] if the request has a problem
    pub fn status<I:Indexable>(&self, index:I) -> Result<NonZeroUsize, AccessError> {
        Ok(self.slot(index.to_index())?.rc)
    }

    /// Pushes `data` into the NodeField, returning the index it was stored at.
    /// 
    /// Once you recieve the index the data was stored at, it is your responsibility to manage its references.
    /// The data will start with one reference.
    #[must_use]
    pub fn push(&mut self, data:T) -> Index {
        let index = self.reserve();
        self.memory[index] = Some(Steward::new(data));
        index
    }

    /// Replaces the data at `index` with `new_data`, returning the replaced data on success and an [AccessError] on failure.
    /// You may not replace an index which is currently free. 
    pub fn replace<I:Indexable>(&mut self, index:I, new_data:T) -> Result<T, AccessError> {
        let data = &mut self.mut_slot(index.to_index())?.data;
        Ok( replace(data, new_data) )
    }

    /// Returns the next index which will be allocated on a [NodeField::push] call
    pub fn next_allocated(&self) -> Index { 
        self.first_free().unwrap_or(self.memory.len())
    }
    
    /// Travels through each slot in the vec checks whether it currently contains data, updating the allocator accordingly. 
    pub fn repair_allocator(&mut self) {
        for i in 0 .. self.memory.len() {
            self.mark_free(i);
            if self.memory[i].is_some() { self.mark_reserved(i) }
        }
    }

    /// Travels through memory and re-arranges slots so that they are contiguous in memory, with no free slots in between occupied ones.
    /// The hashmap returned can be used to remap your references to their new locations. (Key:Old, Value:New)
    /// 
    /// Slots at the back of memory will be placed in the first free slot, until the above condition is met.
    /// 
    /// This operation is O(n) to the number of slots in memory.
    #[must_use]
    pub fn defrag(&mut self) -> HashMap<Index, Index> {
        let mut remapped = HashMap::new();
        let mut solid_until = 0;
        if solid_until == self.memory.len() { return remapped }
        let mut free_until = self.memory.len() - 1;
        'defrag: loop {
            while let Some(_) = self.memory[solid_until] { 
                solid_until += 1;
                if solid_until == free_until { break 'defrag }
            }
            while let None = self.memory[free_until] { 
                free_until -= 1;
                if free_until == solid_until { break 'defrag }
            }
            remapped.insert(free_until, solid_until);
            self.memory.swap(free_until, solid_until);
            self.mark_free(free_until);
            self.mark_reserved(solid_until);
        }
        remapped
    }

    /// [NodeField::defrag]s the memory, then shrinks the internal memory Vec to the size of the block of occupied memory.
    #[must_use]
    pub fn trim(&mut self) -> HashMap<Index, Index> {
        let remap = self.defrag();
        if let Some(first_free) = self.first_free() {
            self.memory.truncate(first_free);
            self.memory.shrink_to_fit();
            if first_free == 0 { self.slot_mask.clear() } else {
                let cell_bits = usize::BITS as usize;
                let last_cell = first_free / cell_bits;
                self.slot_mask.truncate(last_cell + 1);
                self.slot_mask.shrink_to_fit();
            }
        }
        remap
    }

    /// Returns the current bitfield of allocated/free memory slots
    pub fn mask(&self) -> &Vec<usize> {
        &self.slot_mask
    }

}
