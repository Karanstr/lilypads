// #![warn(missing_docs)]
//! Silly virtual memory allocator.
//! 
//! This is a learning experience for me and should be used with a mountain of salt.
//! 
//! This crate is intended for the creation of graphs and similar data structures, with a focus on storing data contiguously in memory while allowing it to have multiple owners. Internally the data is stored in a [Vec].
//! 
//! This crate does not yet support Weak or Atomic references to data, that's on the todo list.
//! 
//! # Example
//! ```
//! use vec_mem_heap::*;
//! 
//! fn main() {
//! 
//!     let mut mem_heap : MemHeap<u32> = MemHeap::new();
//! 
//!     let data1 = mem_heap.push(15); //data1 == Index(0)
//!     //Normally you'd write matches here to catch AccessErrors, but that's a lot of writing I don't want to do
//!     _ = mem_heap.add_owner(data1);
//!
//!     {
//!         let data2 = mem_heap.push(72); // data2 == Index(1)
//!         //Index derives copy, so it can be passed around as parameters without worrying about references/ownership.
//!         _ = mem_heap.add_owner(data2);
//! 
//!         let data3 = data1;
//!         //The value stored in mem_heap.data(Index(0)) now has two owners.
//!         _ = mem_heap.add_owner(data3);
//!     
//!         //data2 and data3 are about to go out of scope, so we have to manually remove them as owners.
//!         //Ok( Some(72) ) -> The data at Index(1) only had one owner, so it was collected
//!         _ = mem_heap.remove_owner(data2);
//!         // Err( AccessError::ProtectedMemory( Index(2) ) ) -> The data at Index(2) was protected, we can't modify its owner_count
//!         _ = mem_heap.remove_owner(data3); 
//!         // Ok( None ) -> The data at Index(0) had two owners, now has one owner. Nothing needs to be done
//!         _ = mem_heap.remove_owner(data1); 
//!     }
//!     // Ok( &15 ) -> The data at Index(0) still has one owner (data1). If the data didn't derive copy, we would recieve &data instead.
//!     _ = dbg!( mem_heap.data( Index(0) ) );
//!     // Err( AccessError::FreeMemory(Index(1)) ) -> The data at Index(1) was garbage collected when its final owner was removed
//!     _ = dbg!( mem_heap.data( Index(1) ) );
//! 
//! }
//! ```

use serde::{Serialize, Deserialize};

pub trait Indexable {
    fn index(&self) -> usize;
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Index(usize);
impl std::ops::Deref for Index {
    type Target = usize;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Indexable for Index {
    fn index(&self) -> usize { self.0 }
}

/// A collection of errors which may occur while handling memory.
#[derive(Debug)]
pub enum AccessError {
    /// Returned when attempting to access an index beyond the length of [MemHeap]'s internal storage
    OutOfBoundsMemory(Index),
    /// Returned when attempting to access an index which isn't currently allocated
    FreeMemory(Index),
    /// Returned when modification of an index's owner count overflows
    OwnershipOverflow,
    /// Returned when the type of data requested doesn't match the type of data stored
    MisalignedTypes,
}
/// The current status of data ownership
pub enum Ownership {
    /// There are `usize` owners of the data
    Fine(usize),
    /// Nobody owns the data, it's dangling and should be freed.
    Dangling,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct Marker {
    last_marker : Index,
    next_marker : Index,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Steward<T> {
    data : T,
    rc : RefCount,
}
#[derive(Debug, Serialize, Deserialize)]
#[repr(transparent)]
struct RefCount(usize);
impl RefCount {
    pub fn modify_owners(&mut self, delta:isize) -> Result<Ownership, AccessError> {
        let new_ref_count = if delta < 0 {
            if let Some(count) = self.0.checked_sub(delta.abs() as usize) {
                count
            } else { return Result::Err( AccessError::OwnershipOverflow ) }
        } else {
            if let Some(count) = self.0.checked_add(delta as usize) {
                count
            } else { return Result::Err( AccessError::OwnershipOverflow ) }
        };
        self.0 = new_ref_count;
        Result::Ok(Ownership::Fine(new_ref_count))
    }
    pub fn status(&self) -> Ownership {
        match self.0 {
            0 => Ownership::Dangling,
            _ => Ownership::Fine(self.0)
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MemorySlot<T> {
    Free,
    Occupied(Steward<T>),
    Marker(Marker),
}
impl<T> MemorySlot<T> {
    pub fn steward(data:T) -> Self {
        Self::Occupied(Steward {
            data,
            rc : RefCount(1),
        })
    }
    pub fn unwrap_steward(&self) -> Result<&Steward<T>, AccessError> {
        if let MemorySlot::Occupied(steward) = self {
            Result::Ok(steward)
        } else { Result::Err( AccessError::MisalignedTypes ) }
    }
    pub fn unwrap_steward_mut(&mut self) -> Result<&mut Steward<T>, AccessError> {
        if let MemorySlot::Occupied(steward) = self {
            Result::Ok(steward)
        } else { Result::Err( AccessError::MisalignedTypes ) }
    }
    pub fn marker(last_marker:Index, next_marker:Index) -> Self {
        Self::Marker(Marker {
            last_marker,
            next_marker,
        })
    }
    pub fn unwrap_marker(&self) -> Result<&Marker, AccessError> {
        if let MemorySlot::Marker(marker) = self {
            Result::Ok(marker)
        } else { Result::Err( AccessError::MisalignedTypes ) }
    }
    pub fn unwrap_marker_mut(&mut self) -> Result<&mut Marker, AccessError> {
        if let MemorySlot::Marker(marker) = self {
            Result::Ok(marker)
        } else { Result::Err( AccessError::MisalignedTypes ) }
    }
}


/// Used to allocate space on the heap, read from that space, and write to it.
#[derive(Serialize, Deserialize)]
pub struct Garden<T:Clone> {
    /// The container used to manage allocated memory
    memory : Vec< MemorySlot<T> >,
    /// Stores list of indexes which can be written to 
    free_indexes : Vec<Index>,
}

//Private functions
impl<T:Clone> Garden<T> {
    fn last_index(&self) -> Index {
        Index(self.memory.len() - 1)
    }

    fn mut_slot(&mut self, index:Index) -> Result<&mut MemorySlot<T>, AccessError> {
        match index {
            bad_index if index > self.last_index() => Err( AccessError::OutOfBoundsMemory(bad_index) ),
            index => match &mut self.memory[*index] {
                MemorySlot::Free | MemorySlot::Marker { .. } => Err( AccessError::FreeMemory(index) ),
                MemorySlot::Occupied { .. } => Ok( &mut self.memory[*index] ),
            }
        }
    }

    fn slot(&self, index:Index) -> Result<&MemorySlot<T>, AccessError> {
        match index {
            bad_index if index > self.last_index() => Err( AccessError::OutOfBoundsMemory(bad_index) ),
            index => match &self.memory[*index] {
                MemorySlot::Free | MemorySlot::Marker { .. } => Err( AccessError::FreeMemory(index) ),
                MemorySlot::Occupied { .. } => Ok( &self.memory[*index] ),
            }
        }
    }

    //Rewrite these two to work with the new memory system
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

        self.memory[*index] = MemorySlot::Free;
        self.free_indexes.push(index);
        Some(data)
    }

    fn reserve_index(&mut self) -> Index {
        match self.free_indexes.pop() {
            Some(index) => index,
            None => {
                self.memory.push(MemorySlot::Free);
                self.last_index()
            }
        }        
    }
}

//Public functions
impl<T:Clone> Garden<T> {
    /// Constructs a new `MemHeap` which can store data of type `T` 
    /// # Example
    /// ```
    /// //Stores u32's in each index.
    /// let mut mem_heap:MemHeap<u32> = MemHeap::new();
    /// ```
    pub fn new() -> Self {
        Self {
            memory : Vec::new(),
            free_indexes : Vec::new(),
        }
    }

    /// Returns the number of indexes the MemHeap currently has allocated.
    pub fn length(&self) -> usize {
        self.memory.len()
    }

    /// Frees all data which has no owners
    /// 
    /// This operation is O(n) to the total number of allocated indexes (which can be found using [MemHeap::length]).
    pub fn remove_memory_leaks(&mut self) {
        for cell in 0 .. self.memory.len() {
            let index = Index(cell);
            let Ok(slot) = self.mut_slot(index) else { continue };
            if let Ownership::Dangling = slot.unwrap_steward().unwrap().rc.status() { self.free_index(index); }
        }
    }

    /// Returns an immutable reference to the data stored at the requested index, or an [AccessError] if there is a problem.
    /// 
    /// The equivalent to using & to borrow variables in normal Rust.
    pub fn data<I:Indexable>(&self, index:I) -> Result<&T, AccessError> {
        Ok(&self.slot(Index(index.index()))?.unwrap_steward()?.data)
    }

    /// Tells the MemHeap that something else owns the data at `index`.
    /// So long as MemHeap thinks there is at least one owner, the data won't be garbage collected.
    /// 
    /// Failure to properly track ownership will lead to either garbage collection of active data or leaking of inactive data
    pub fn add_owner<I:Indexable>(&mut self, index:I) -> Result<(), AccessError> {
        self.mut_slot(Index(index.index()))?.unwrap_steward_mut()?.rc.modify_owners(1)?;
        Ok(())
    }

    /// Tells the MemHeap that something no longer owns the data at `index`.
    /// By default, if calling this function renders the ownercount of data 0, it will automatically be garbage collected and returned.
    /// 
    /// Failure to properly track ownership will lead to either garbage collection of active data or leaking of inactive data.
    pub fn remove_owner<I:Indexable>(&mut self, index:I) -> Result<Option<T>, AccessError> {
        let internal_index = Index(index.index());
        if let Ownership::Dangling = self.mut_slot(internal_index)?.unwrap_steward_mut()?.rc.modify_owners(-1)? {
            Ok( self.free_index(internal_index) )
        } else { Ok(None) }
    }

    /// Frees the data at `index` and returns it wrapped in an [Option::Some] wrapped in a [Result::Ok] if the data is ownerless.
    /// If there are still owners, [Option::None] will be returned in the [Result::Ok] instead.
    /// If the index is invalid, or the data cannot be freed for some reason, returns an [AccessError].
    pub fn free_if_dangling<I:Indexable>(&mut self, index:I) -> Result<Option<T>, AccessError> {
        let internal_index = Index(index.index());
        match self.status(internal_index)? {
            Ownership::Fine(_) => Ok(None),
            Ownership::Dangling => Ok(self.free_index(internal_index)),
        }
    }

    /// Returns the [Ownership] of the data at `index`, or an [AccessError] if the request has a problem
    pub fn status<I:Indexable>(&self, index:I) -> Result<Ownership, AccessError> {
        Ok(self.slot(Index(index.index()))?.unwrap_steward()?.rc.status())
    }

    /// Pushes `data` into the MemHeap, selecting the first free index for insertion and returning that index.
    /// 
    /// Once you recieve the index the data was stored at, it is your responsibility to manage its owners.
    /// The data will start with one owner.
    pub fn push(&mut self, data:T) -> Index {
        let index = self.reserve_index();
        self.memory[*index] = MemorySlot::steward(data);
        index
    }

    /// Replaces the data at `index` with `new_data`, returning the replaced data on success and an [AccessError] on failure.
    /// You may only replace reserved data. Free indexes should be filled with [MemHeap::push].
    pub fn replace<I:Indexable>(&mut self, index:I, new_data:T) -> Result<T, AccessError> {
        let wrapper = self.mut_slot(Index(index.index()))?.unwrap_steward_mut()?;
        let old_data = wrapper.data.clone();
        wrapper.data = new_data;
        Ok(old_data)
    }

    /// Returns an immutable reference to the internal memory Vec
    pub fn peek(&self) -> &Vec< MemorySlot<T> > {
        &self.memory
    } 

}

