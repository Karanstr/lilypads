#![warn(missing_docs)]
//! Unoptimized virtual memory allocator.
//! 
//! This is a learning experience for me and should be used with a mountain of salt.
//! 
//! Effectively a custom implementation of [std::rc] with a focus on streamlining the creation of a large number of shared-ownership data and ensuring all of that data is stored (more or less) contiguously in memory. Data is stored in a [Vec] (until I learn how to handle raw memory), and [Index]es are used to read and write.
//! 
//! # Example
//! ```
//! use vec_mem_heap::*;
//! 
//! fn main() {
//! 
//!     let mut mem_heap : MemHeap<u32> = MemHeap::new();
//! 
//!     let data1 = mem_heap.push(15, false); //data1 == Index(0)
//!     //Normally you'd write matches here to catch AccessErrors, but that's a lot of writing I don't want to do
//!     _ = mem_heap.add_owner(data1);
//!
//!     {
//!         let data2 = mem_heap.push(72, false); // data2 == Index(1)
//!         //Index derives copy, so it can be passed around as parameters without worrying about references/ownership.
//!         _ = mem_heap.add_owner(data2);
//! 
//!         let data3 = mem_heap.push(7, true); // data3 == Index(2)
//!         //The data located at data3 (Index(2)) is protected, meaning calling add_owner will result in an AccessError::ProtectedMemory, which can be caught with a match
//!         match mem_heap.add_owner(data3) {
//!             Ok(_) => {
//!                 //Whatever
//!             },
//!             Err(AccessError:ProtectedMemory(_)) => println!("Attempted to modify protected memory"),
//!             Err(error) => _ = dbg!(error),
//!         }
//! 
//!         let data4 = data1;
//!         //The value stored in mem_heap.data(Index(0)) now has two owners.
//!         _ = mem_heap.add_owner(data4);
//!     
//!         //data2, data3, and data4 are about to go out of scope, so we have to manually remove them as owners.
//!         //Ok( Some(72) ) -> The data at Index(1) only had one owner, so it was collected
//!         _ = mem_heap.remove_owner(data2);
//!         // Err( AccessError::ProtectedMemory( Index(2) ) ) -> The data at Index(2) was protected, we can't modify its owner_count
//!         _ = mem_heap.remove_owner(data3); 
//!         // Ok( None ) -> The data at Index(0) had two owners, now has one owner. Nothing needs to be done
//!         _ = mem_heap.remove_owner(data4); 
//!     }
//!     // Ok( &15 ) -> The data at Index(0) still has one owner (data1). If the data didn't derive copy, we would recieve &data instead.
//!     _ = dbg!( mem_heap.data( Index(0) ) );
//!     // Err( AccessError::FreeMemory(Index(1)) ) -> The data at Index(1) was garbage collected when its final owner was removed
//!     _ = dbg!( mem_heap.data( Index(1) ) );
//! 
//! }
//! ```

use std::ops::Deref;

/// A [new_type](<https://doc.rust-lang.org/rust-by-example/generics/new_types.html>) used to help prevent improper access to memory.
#[derive(Debug, PartialEq, PartialOrd, Clone, Copy, Hash, Eq)]
pub struct Index(pub usize);
impl Deref for Index {
    type Target = usize;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}


/// A collection of errors which may occur while handling memory.
#[derive(Debug)]
pub enum AccessError {
    /// Returned when attempting to access an index beyond the length of [MemHeap]'s internal storage
    OutOfBoundsMemory(Index),
    /// Returned when attempting to access an index marked as protected
    ProtectedMemory(Index),
    /// Returned when attempting to access an index which isn't currently allocated
    FreeMemory(Index),
    /// Returned when attempting to do something which isn't supported
    InvalidRequest,
}


mod owner_tracking {

    #[derive(PartialEq)]
    /// The current status of data ownership
    pub enum Ownership {
        /// There are `usize` owners of the data
        Fine(usize),
        /// Nobody owns the data, it's dangling and should be freed.
        Dangling,
    }
    
    pub struct Steward<T> {
        pub data : T,
        owner_count:usize,
    }
    
    impl<T> Steward<T> {
    
        pub fn new(data:T) -> Self {
            Self {
                data,
                owner_count : 0,
            }
        }
    
        pub fn modify_owners(&mut self, delta:isize) -> Result<Ownership, super::AccessError> {
            if delta < 0 {
                match self.owner_count.checked_sub(delta.abs() as usize) {
                    Some(zero) if zero == 0 => {
                        self.owner_count = 0;
                        Result::Ok(Ownership::Dangling)
                    },
                    Some(new_ref) => {
                        self.owner_count = new_ref;
                        Result::Ok(Ownership::Fine(self.owner_count))
                    },
                    None => Result::Err( super::AccessError::InvalidRequest )
                }     
            } else {
                self.owner_count += delta as usize;
                Result::Ok(Ownership::Fine(self.owner_count))
            }
        }
    
        pub fn status(&self) -> Ownership {
            if self.owner_count == 0 {
                Ownership::Dangling
            } else {
                Ownership::Fine(self.owner_count)
            }
        }
    
    }

}

pub use owner_tracking::Ownership;
use owner_tracking::Steward;

/// Used to allocate space on the stack, read from that space, and write to it.
pub struct MemHeap<T:Clone> {
    /// The container used to manage allocated memory
    memory:Vec< Option< Steward< T > > >,
    /// Stores list of indexes which can be written to 
    free_indexes : Vec<Index>,
    /// Stores list of indexes which can't be referenced mutably
    protected_indexes : Vec<Index>,
}

impl<T:Clone> MemHeap<T> {

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
            protected_indexes : Vec::new(),
        }
    }

    fn upper_bound(&self) -> Index {
        Index(self.memory.len())
    }

    fn last_index(&self) -> Index {
        Index(self.memory.len() - 1)
    }

    fn is_protected(&self, index:Index) -> bool {
        self.protected_indexes.contains(&index)
    }

    fn mut_wrapper(&mut self, index:Index) -> Result<&mut Steward<T>, AccessError> {
        match index {
            bad_index if index >= self.upper_bound() => Err( AccessError::OutOfBoundsMemory(bad_index) ),
            protected_index if self.is_protected(protected_index) => Err( AccessError::ProtectedMemory(protected_index) ),
            index => match &mut self.memory[*index] {
                None => Err( AccessError::FreeMemory(index) ),
                Some(tracker) => Ok(tracker)
            }
        }
    }

    fn wrapper(&self, index:Index) -> Result<&Steward<T>, AccessError> {
        match index {
            bad_index if index >= self.upper_bound() => Err( AccessError::OutOfBoundsMemory(bad_index) ),
            index => match &self.memory[*index] {
                None => Err( AccessError::FreeMemory(index) ),
                Some(tracker) => Ok(tracker)
            }
        }
    }

    fn free_index(&mut self, index:Index) -> Option<T> {
        let data = match self.mut_wrapper(index) {
            Err( error ) => { dbg!(error); panic!() },
            Ok(wrapper) => wrapper.data.clone()
        };
        self.memory[*index] = None;
        self.free_indexes.push(index);
        Some(data)
    }

    fn reserve_index(&mut self) -> Index {
        match self.free_indexes.pop() {
            Some(index) => index,
            None => {
                self.memory.push(None);
                self.last_index()
            }
        }        
    }

    fn reserve_protected(&mut self) -> Index {
        let index = match self.free_indexes.pop() {
            Some(index) => index,
            None => {
                self.memory.push(None);
                self.last_index()
            }
        };
        self.protected_indexes.push(index);
        index
    }

    /// Returns the number of indexes the MemHeap currently has allocated.
    pub fn length(&self) -> usize {
        self.memory.len()
    }

    /// Removes protection from a piece of data, doing nothing if it isn't protected.
    /// 
    /// If this is a permanent exposure (you don't plan to call [MemHeap::protect] afterwards), ensure you use [MemHeap::add_owner] or [MemHeap::free_if_dangling] to prevent memory leaking.
    pub fn expose(&mut self, index:Index) {
        if self.is_protected(index) {
            for protected in 0 .. self.protected_indexes.len() {
                if self.protected_indexes[protected] == index {
                    self.protected_indexes.remove(protected);
                    break
                }
            }
        }
    }

    /// Protects a piece of data, ensuring its ownership tracking won't be altered and the data won't be garbage collected by any of the following:
    /// - [MemHeap::add_owner]
    /// - [MemHeap::remove_owner]
    /// - [MemHeap::remove_memory_leaks]
    /// - [MemHeap::free_if_dangling]
    /// - [MemHeap::replace]
    ///
    ///  For now free indexes (non-reserved) cannot be protected. This is subject to change as I feel like it.
    pub fn protect(&mut self, index:Index) -> Result<(), AccessError> {
        //We don't protect free indexes. Reserving indexes should be done with reserve_index/reserve_protected
        _ = self.wrapper(index)?;
        self.protected_indexes.push(index);
        Ok(())
    }

    /// Frees all data which meets the following requirements:
    /// - Has no owners
    /// - Is not protected
    /// 
    /// This operation is O(n) to the total number of allocated indexes (which can be found using [MemHeap::length]).
    pub fn remove_memory_leaks(&mut self) {
        for cell in 0 .. self.memory.len() {
            let index = Index(cell);
            if let Ok(wrapper) = self.mut_wrapper(index) {
                if let Ownership::Dangling = wrapper.status() {
                    self.free_index(index);
                }
            }
        }
    }

    /// Returns an immutable reference to the data stored at the requested index, or an [AccessError] if there is a problem.
    /// 
    /// The equivalent to using & to borrow variables in normal Rust.
    pub fn data(&self, index:Index) -> Result<&T, AccessError> {
        Ok(&self.wrapper(index)?.data)
    }

    /// Tells the MemHeap that something else owns the data at `index`.
    /// So long as MemHeap thinks there is at least one owner, the data won't be garbage collected.
    /// 
    /// Failure to properly track ownership will lead to either garbage collection of active data or leaking of inactive data
    /// 
    /// If you want to store data regardless of whether something is currently owning it, please use:
    /// - [MemHeap::protect] for existing data
    /// - [MemHeap::push()] with `protected = true` for new data
    pub fn add_owner(&mut self, index:Index) -> Result<(), AccessError> {
        self.mut_wrapper(index)?.modify_owners(1)?;
        Ok(())
    }

    /// Tells the MemHeap that something no longer owns the data at `index`.
    /// By default, if calling this function renders the ownercount of data 0, it will automatically be garbage collected and returned.
    /// 
    /// Failure to properly track ownership will lead to either garbage collection of active data or leaking of inactive data.
    /// 
    /// If you want to store data regardless of whether something owns it, please use:
    /// - [MemHeap::protect] for existing data
    /// - [MemHeap::push] with `protected` set to true for new data
    /// 
    /// If instead you're trying to free protected data, please:
    /// 1. Remove the data's protection using [MemHeap::expose]
    /// 2. Remove any existing references using [MemHeap::remove_owner] (can check if needed with [MemHeap::status])
    /// 3. Free the data using [MemHeap::free_if_dangling]
    pub fn remove_owner(&mut self, index:Index) -> Result<Option<T>, AccessError> {
        if let Ownership::Dangling = self.mut_wrapper(index)?.modify_owners(-1)? {
            match self.free_index(index) {
                Some(data) => Ok( Some(data) ),
                None => Ok(None)
            }
        } else { Ok(None) }
    }

    /// Frees the data at `index` and returns it wrapped in an [Option::Some] wrapped in a [Result::Ok] if the data is ownerless.
    /// If there are still owners, [Option::None] will be returned in the [Result::Ok] instead.
    /// If the index is invalid, or the data cannot be freed for some reason, returns an [AccessError].
    pub fn free_if_dangling(&mut self, index:Index) -> Result<Option<T>, AccessError> {
        match self.status(index)? {
            Ownership::Fine(_) => Ok(None),
            Ownership::Dangling => Ok(self.free_index(index)),
        }
    }

    /// Returns the [owner_tracking::Ownership] of the data at `index`, or an [AccessError] if the request has a problem
    pub fn status(&self, index:Index) -> Result<owner_tracking::Ownership, AccessError> {
        Ok(self.wrapper(index)?.status())
    }

    /// Pushes `data` into the MemHeap, selecting the most *recently freed index for insertion and returning the index the data is placed at.
    /// 
    /// *Subject to change. In the future the plan is to return the first free index, sequentially, to leave less holes in reserved memory.
    /// 
    /// If `protected` is true, the data will be marked as immutable. 
    /// This means the MemHeap will 'freeze' the data's ownership tracking when calling [MemHeap::add_owner] or [MemHeap::remove_owner] and won't garbage collect the data, even if it is [owner_tracking::Ownership::Dangling]
    /// 
    /// Protection can be removed with a call to [MemHeap::expose], which will unfreeze the data's ownership tracking.
    /// If you don't intend to re[MemHeap::protect] the data, please either garbage collect with [MemHeap::free_if_dangling] or give it an owner with [MemHeap::add_owner].
    ///
    ///  Remember: **Owners should correlate to locations an [Index] is stored**. DO NOT just call [MemHeap::add_owner] and forget about it unless you want to deal with memory leakage.
    /// Once you recieve the index the data was stored at, it is your responsibility to manage its owners.
    pub fn push(&mut self, data:T, protected:bool) -> Index {
        let index = if protected { self.reserve_protected() } else { self.reserve_index() };
        self.memory[*index] = Some( Steward::new(data) );
        index
    }

    /// Replaces the data at `index` with `new_data`, returning the replaced data on success and an [AccessError] on failure.
    /// You may only replaced reserved, non-protected data. Free indexes should be filled with [MemHeap::push].
    /// 
    /// If you want to replace protected data:
    /// 1. Call [MemHeap::expose] on the data's index to remove its protection
    /// 2. Call [MemHeap::replace] to replace the data
    /// 3. Call [MemHeap::protect] to restore its protection (if desired)
    pub fn replace(&mut self, index:Index, new_data:T) -> Result<T, AccessError> {
        let wrapper = self.mut_wrapper(index)?;
        let old_data = wrapper.data.clone();
        wrapper.data = new_data;
        Ok(old_data)
    }

    /// Returns an immutable reference to the internal memory Vec
    /// to enable profiling on memory usage.
    pub fn profile(&self) -> &Vec< Option< Steward<T> > > {
        &self.memory
    } 

}