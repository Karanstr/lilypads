#![warn(missing_docs)]
//! Unoptimized virtual memory allocator.
//! 
//! This is a learning experience for me and should be used with a mountain of salt.
//! 
//! Allocates an area of memory using Rust's [Vec], storing fixed size blocks of data (until I learn to handle raw memory).
//! This is a rather low level library, handling garbage collection semi-manually (see *link to reference stuff*) and intended to be integrated into larger projects, specifically datastructures like trees where Rust's lifetime system makes things difficult. You will not be protected from memory leaks, though leakage can be mitigated with calls to [MemHeap::remove_memory_leaks].

use std::ops::Deref;

/// A collection of errors which may occur while handling memory.
#[derive(Debug)]
pub enum AccessError {
    /// Returned when attempting to access an Index beyond the length of [MemHeap]'s internal storage
    OutOfBoundsMemory(usize),
    /// Returned when attempting to access an Index marked as Protected
    ProtectedMemory(usize),
    /// Returned when attempting to access an Index which isn't currently allocated
    FreeMemory(usize),
    /// Returned when attempting to do something which isn't supported
    InvalidRequest,
}


mod reference_management {

    #[derive(PartialEq)]
    /// The current status of the MemHeap's reference tracking for a piece of data
    pub enum ReferenceStatus {
        /// There are `usize` references to the data
        Fine(usize),
        /// There are no references to the data, it's dangling and should be freed.
        Dangling,
    }
    
    pub struct ReferenceWrapper<T> {
        pub data : T,
        ref_count:usize,
    }
    
    impl<T> ReferenceWrapper<T> {
    
        pub fn new(data:T) -> Self {
            Self {
                data,
                ref_count : 0,
            }
        }
    
        pub fn modify_ref(&mut self, delta:isize) -> Result<ReferenceStatus, super::AccessError> {
            if delta < 0 {
                match self.ref_count.checked_sub(delta.abs() as usize) {
                    Some(zero) if zero == 0 => {
                        self.ref_count = 0;
                        Result::Ok(ReferenceStatus::Dangling)
                    },
                    Some(new_ref) => {
                        self.ref_count = new_ref;
                        Result::Ok(ReferenceStatus::Fine(self.ref_count))
                    },
                    None => Result::Err( super::AccessError::InvalidRequest )
                }     
            } else {
                self.ref_count += delta as usize;
                Result::Ok(ReferenceStatus::Fine(self.ref_count))
            }
        }
    
        pub fn status(&self) -> ReferenceStatus {
            if self.ref_count == 0 {
                ReferenceStatus::Dangling
            } else {
                ReferenceStatus::Fine(self.ref_count)
            }
        }
    
    }

}

/// A [new_type](<https://doc.rust-lang.org/rust-by-example/generics/new_types.html>) used to help prevent improper access to memory.
#[derive(Debug, PartialEq, PartialOrd, Clone, Copy, Hash, Eq)]
pub struct Index(pub usize);
impl Deref for Index {
    type Target = usize;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}


pub use reference_management::ReferenceStatus;
use reference_management::ReferenceWrapper;

/// Used to allocate space on the stack, read from that space, and write to it.
pub struct MemHeap<T:Clone> {
    /// The container used to manage allocated memory
    memory:Vec< Option< ReferenceWrapper< T > > >,
    /// Stores list of indexes which can be written to 
    free_indexes : Vec<Index>,
    /// Stores list of indexes which can't be referenced mutably
    protected_indexes : Vec<Index>,
}

impl<T:Clone> MemHeap<T> {

    /// Constructs a new `MemHeap` which can store data of type `T` 
    /// # Examples
    /// `
    /// //Stores u32's in each index.
    /// let mut mem_heap:MemHeap<u32> = MemHeap::new();
    /// `
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

    fn mut_wrapper(&mut self, index:Index) -> Result<&mut ReferenceWrapper<T>, AccessError> {
        match index {
            bad_index if index >= self.upper_bound() => Err( AccessError::OutOfBoundsMemory(*bad_index) ),
            protected_index if self.is_protected(protected_index) => Err( AccessError::ProtectedMemory(*protected_index) ),
            index => match &mut self.memory[*index] {
                None => Err( AccessError::FreeMemory(*index) ),
                Some(tracker) => Ok(tracker)
            }
        }
    }

    fn wrapper(&self, index:Index) -> Result<&ReferenceWrapper<T>, AccessError> {
        match index {
            bad_index if index >= self.upper_bound() => Err( AccessError::OutOfBoundsMemory(*bad_index) ),
            index => match &self.memory[*index] {
                None => Err( AccessError::FreeMemory(*index) ),
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

    /// Removes protection from an index, doing nothing if the index isn't protected.
    /// This should be used when you need to mutate the data at that index for some reason.
    /// If this is a permanent exposure (you don't plan to call [MemHeap::protect] afterwards), ensure you use [MemHeap::add_ref] or [MemHeap::free_if_dangling] to prevent memory leaking.
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

    /// Protects an index, ensuring it's reference_tracker can't be modified/it can't be garbage collected by:
    /// - [MemHeap::add_ref]
    /// - [MemHeap::remove_ref]
    /// - [MemHeap::remove_memory_leaks]
    /// - [MemHeap::free_if_dangling]
    /// For now free indexes (non-reserved) cannot be protected. This is subject to change as I feel like it.
    pub fn protect(&mut self, index:Index) -> Result<(), AccessError> {
        //We don't protect free indexes. Reserving indexes should be done with reserve_index/reserve_protected
        _ = self.wrapper(index)?;
        self.protected_indexes.push(index);
        Ok(())
    }

    /// Frees every index which meets the following requirements:
    /// - Has zero references to it
    /// - Is not protected
    /// This operation is O(n) to the total number of allocated indexes (which can be found using [MemHeap::length]).
    pub fn remove_memory_leaks(&mut self) {
        for cell in 0 .. self.memory.len() {
            let index = Index(cell);
            if let Ok(wrapper) = self.mut_wrapper(index) {
                if let ReferenceStatus::Dangling = wrapper.status() {
                    self.free_index(index);
                }
            }
        }
    }

    /// Returns an immutable reference to the data stored at the requested index, or an [AccessError] if there is a problem.
    pub fn data(&self, index:Index) -> Result<&T, AccessError> {
        Ok(&self.wrapper(index)?.data)
    }

    /// Tells the MemHeap that something is referencing the data at `index`.
    /// So long as MemHeap thinks there is at least one reference to the data, it will not be garbage collected.
    /// Due to the nature of Rust's lifetime system the MemHeap cannot verify the data's reference count, it will trust you completely.
    /// If you want to store data regardless of whether something is currently referencing it, please use:
    /// - [MemHeap::protect] for existing data
    /// - [MemHeap::push()] with `protected = true` for new data
    pub fn add_ref(&mut self, index:Index) -> Result<(), AccessError> {
        self.mut_wrapper(index)?.modify_ref(1)?;
        Ok(())
    }

    /// Tells the MemHeap that something which was referencing the data at `index` isn't anymore.
    /// MemHeap cannot verify this and will trust you. If the MemHeap thinks there is nothing else referencing the data after removal, it will be garbage collected.
    /// If you want to store data regardless of whether something is currently referencing it, please use:
    /// - [MemHeap::protect] for existing data
    /// - [MemHeap::push] with `protected` set to true for new data
    /// If instead you're trying to garbage collect protected data, please:
    /// 1. Remove the data's protection using [MemHeap::expose]
    /// 2. Remove any existing references using [MemHeap::remove_ref] (can check if needed with [MemHeap::status])
    /// 3. Free the data using [MemHeap::free_if_dangling]
    pub fn remove_ref(&mut self, index:Index) -> Result<Option<T>, AccessError> {
        if let ReferenceStatus::Dangling = self.mut_wrapper(index)?.modify_ref(-1)? {
            match self.free_index(index) {
                Some(data) => Ok( Some(data) ),
                None => Ok(None)
            }
        } else { Ok(None) }
    }

    /// Frees the `index` and returns the data stored there wrapped in an [Option::Some] wrapped in a [Result::Ok] if the MemHeap doesn't know about any references to the data.
    /// If there are still references, [Option::None] will be returned in the [Result::Ok] instead.
    /// If the index is invalid, or cannot be freed for some reason, returns an [AccessError].
    pub fn free_if_dangling(&mut self, index:Index) -> Result<Option<T>, AccessError> {
        match self.status(index)? {
            ReferenceStatus::Fine(_) => Ok(None),
            ReferenceStatus::Dangling => Ok(self.free_index(index)),
        }
    }

    /// Returns the [reference_management::ReferenceStatus] of the Index, or an [AccessError] if the request has a problem
    pub fn status(&self, index:Index) -> Result<reference_management::ReferenceStatus, AccessError> {
        Ok(self.wrapper(index)?.status())
    }

    /// Pushes `data` into the MemHeap, selecting the most *recently freed index for insertion and returning the index the data is placed at.
    /// *Subject to change. In the future the plan is to return the first free index, sequentially, to leave less holes in reserved memory.
    /// If `protected` is true, the data will be marked as immutable. This means the MemHeap will 'freeze' the data's reference count when calling [MemHeap::add_ref] or [MemHeap::remove_ref] and won't garbage collect the data, even if it's [reference_management::ReferenceStatus::Dangling]
    /// Protection can be removed with a call to [MemHeap::expose], which will unfreeze the data's reference count.
    /// If you don't intend to re[MemHeap::protect] the data, please either garbage collect with [MemHeap::free_if_dangling] or give it a reference with [MemHeap::add_ref].
    /// Remember: **References should correlate to locations an [Index] is stored**. DO NOT just call [MemHeap::add_ref] and forget about it, unless you want to deal with memory leakage.
    /// Once you recieve the index the data was stored at, it is your responsibility to manage references to it, the MemHeap can't hold your hand here.
    pub fn push(&mut self, data:T, protected:bool) -> Index {
        let index = if protected { self.reserve_protected() } else { self.reserve_index() };
        self.memory[*index] = Some( ReferenceWrapper::new(data) );
        index
    }

    /// Replaces the data at `index` with `new_data`, returning the replaced data on success and an [AccessError] on failure.
    /// You may only replaced reserved, non-protected data. Free indexes should be filled with [MemHeap::push].
    /// If you want to replace protected data:
    /// 1. Call [MemHeap::expose] on the index to remove its protection
    /// 2. Call [MemHeap::replace] to replace the data
    /// 3. Call [MemHeap::protect] to restore its protection (if desired)
    pub fn replace(&mut self, index:Index, new_data:T) -> Result<T, AccessError> {
        let wrapper = self.mut_wrapper(index)?;
        let old_data = wrapper.data.clone();
        wrapper.data = new_data;
        Ok(old_data)
    }

}