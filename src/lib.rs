#![warn(missing_docs)]
//! Fun little arena allocator.
//! 
//! This is a learning experience for me and should be used with a mountain of salt.
//! At some point I need to rename it, but I don't have a good one yet.
//! 
//! This crate is intended for the creation of graphs and similar data structures, with a focus on storing data contiguously in memory while allowing it to have multiple owners. Internally the data is stored in a [Vec].
//! 
//! This crate does not yet support Weak or Atomic references to data, that's on the todo list (maybe).
//! 
//! Errors will cancel the request and returning an [AccessError].
//! 
//! # Example
//! ```
//! use vec_mem_heap::prelude::*;
//! 
//! fn main() {
//! 
//!     let mut storage : NodeField<Option<u32>> = NodeField::new();
//! 
//!     // When you push data into the structure, it returns the index that data was stored at and sets the reference count to 1.
//!     let data1 = storage.push(Some(15)); // data1 == 0
//!
//!     {
//!         let data2 = storage.push(Some(72)); // data2 == 1
//! 
//!         // Now that a second reference to the data at index 0 exists, we have to manually add to the reference count.
//!         let data3 = data1;
//!         storage.add_ref(data3);
//!     
//!         // data2 and data3 are about to go out of scope, so we have to manually remove their references.
//!         // returns Ok( Some( Some(72) ) ) -> The data at index 1 only had one reference, so it was freed.
//!         storage.remove_ref(data2);
//! 
//!         // returns Ok( None ) -> The data at index 0 had two references, now one.
//!         storage.remove_ref(data3); 
//!     }
//! 
//!     // returns Ok( &Some(15) ) -> The data at index 0 (data1) still has one reference.
//!     dbg!( storage.get( data1 ) );
//!     // Err( AccessError::FreeMemory(1) ) -> The data at index 1 was freed when its last reference was removed.
//!     dbg!( storage.get( 1 ) );
//! 
//! }
//! ```

mod binary_tree;
use binary_tree::FullFlatBinaryTree;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Datastructure, ErrorEnum, and Required Trait all bundled together for convenience
/// 
/// This module re-exports the essential types and traits.
/// Import everything from this module with `use vec_mem_heap::prelude::*`.
pub mod prelude {
  pub use super::{
    NodeField, 
    AccessError,
    Nullable,
  };
}

/// Errors which may occur while accessing and modifying memory.
#[derive(Debug)]
pub enum AccessError {
  /// Returned when attempting to access an index which isn't currently allocated
  FreeMemory(usize),
  /// Returned when a reference operation causes an over/underflow
  ReferenceOverflow,
}

/// Trait any data stored in the NodeField must implement, guaranteeing there's a value we can use as null for uninitialized cells. 
/// [Option<T>] where T:[Sized] is implemented for you, so if you don't care/understand why you'd
/// want this just wrap your data in an [Option]
pub trait Nullable: Sized + Clone {
  //! The main reason you'd manually implement this is if you don't want to deal with a wrapper type eating up your bits and would rather just define a custom null sentinel.
  //!
  //! # Example
  //! ```
  //! use vec_mem_heap::Nullable;
  //! #[derive(PartialEq, Clone)]
  //! struct NoZeroU32(u32);
  //! impl Nullable for NoZeroU32 {
  //!   const NULL_VAL: Self = NoZeroU32(u32::MAX);
  //!   fn is_null(&self) -> bool { self != &Self::NULL_VAL }
  //! }
  //! ```
  //! Implementing Nullable on a wrapper type
  //!
  //! # Example
  //! ```ignore
  //! use vec_mem_heap::Nullable;
  //! impl<T> Nullable for Option<T> { 
  //!   const NULL_VAL: Self = None; 
  //!   fn is_null(&self) -> bool { self.is_none() }
  //!   fn take(&mut self) -> Self { self.take() }
  //! }
  //! ```
  //! Implementing Nullable for Option<T>, this is the canon implementation within this crate. Due
  //! to the orphan rule, you actually can't do this without a wrapper in your project :(
  //!
  //! If you need a type implemented, complain in issues and I'll learn macros or something like
  //! that

  /// The null sentinel used 
  const NULL_VAL: Self;
  /// Returns true if the data matches its type's null condition. In [Option]'s case, this is the same as calling [Option::is_none]
  /// I could've provided this with a default impl, but I would have to bind Self: PartialEq and I
  /// don't want to do that.
  fn is_null(&self) -> bool;
  /// Takes replaces the value at &mut self with Self::NULL_VAL, returning the original value. In [Option]'s case, this is the same as calling [Option::take]
  fn take(&mut self) -> Self { std::mem::replace(self, Self::NULL_VAL) }
}

impl<T: Clone> Nullable for Option<T> { 
  const NULL_VAL: Self = None; 
  fn is_null(&self) -> bool { self.is_none() }
  fn take(&mut self) -> Self { self.take() }
}

/// Used to allocate space on the heap, read from that space, and write to it.
#[derive(Serialize, Deserialize, Debug)]
pub struct NodeField<T> where T: Nullable {
  /// List of all data stored within this structure
  data : Vec< T >,
  /// A reference count for each data slot
  refs : Vec<Option<usize>>,
  /// A binary tree marking whether each cell is free (0) or reserved (1)
  pub list: FullFlatBinaryTree
}

// Private methods
impl<T:Nullable> NodeField<T> {

  fn first_free(&self) -> Option<usize> { self.list.get_first_empty_leaf() }

  fn mark_free(&mut self, idx:usize) { 
    self.refs[idx] = None;
    self.list.set_leaf(idx, false);
  }

  fn mark_reserved(&mut self, idx:usize) {
    self.refs[idx] = Some(1);
    self.list.set_leaf(idx, true);
  }

  fn release(&mut self, idx:usize) -> T {
    let data = self.data[idx].take();
    if data.is_null() { panic!("Tried to release free slot") } else {
      self.mark_free(idx);
      data
    }
  }

  #[must_use]
  fn reserve(&mut self) -> usize {
    let pot_idx = self.first_free();
    let idx = if pot_idx.is_some() { pot_idx.unwrap() }
    else {
      self.list.resize(self.data.len() + 1);
      self.data.push(T::NULL_VAL);
      self.refs.push(None);
      self.data.len() - 1
    };
    self.mark_reserved(idx);
    idx
  }

}

// Public functions
impl<T:Nullable> NodeField<T> {
  /// Constructs a new `NodeField` which can store data of type `T` 
  /// # Example
  /// ```
  /// use vec_mem_heap::prelude::*;
  /// //Stores i32s
  /// let mut storage = NodeField::<Option<i32>>::new();
  /// ```
  pub fn new() -> Self {
    Self {
      data : Vec::new(),
      refs : Vec::new(),
      list: FullFlatBinaryTree::new(),
    }
  }

  /// Returns a reference to the internal data Vec
  pub fn data(&self) -> &Vec<T> { &self.data }

  /// Returns a reference to the internal reference Vec
  pub fn refs(&self) -> &Vec< Option<usize> > { &self.refs }

  /// Returns the next index which will be allocated on a [NodeField::push] call
  pub fn next_allocated(&self) -> usize { self.first_free().unwrap_or(self.data.len()) }

  /// Sets NodeField to hold `size` elements. If size < self.data().len(), excess data will be truncated
  /// and will be lost. Use care when calling this function.
  pub fn resize(&mut self, size: usize) {
    self.data.resize(size, T::NULL_VAL);
    self.data.shrink_to_fit();
    self.refs.resize(size, None);
    self.refs.shrink_to_fit();
    self.list.resize(size);
  }

  /// Returns an immutable reference to the data stored at the requested index, or an [AccessError] if there is a problem.
  pub fn get(&self, idx:usize) -> Result<&T, AccessError> {
    if let Some(data) = self.data.get(idx) {
      if data.is_null() { Err(AccessError::FreeMemory(idx)) } else { Ok( data )}
    } else { Err(AccessError::FreeMemory(idx)) }
  }

  /// Returns a mutable reference to the data stored at the requested index, or an [AccessError] if there is a problem.
  pub fn get_mut(&mut self, idx:usize) -> Result<&mut T, AccessError> {
    if let Some(data) = self.data.get_mut(idx) {
      if data.is_null() { Err(AccessError::FreeMemory(idx)) } else { Ok( data )}
    } else { Err(AccessError::FreeMemory(idx)) }
  }

  /// Tells the NodeField that something else references the data at `index`.
  /// So long as the NodeField thinks there is at least one reference, the data won't be freed.
  /// 
  /// Failure to properly track references will lead to either freeing data you wanted or leaking data you didn't.
  pub fn add_ref(&mut self, idx:usize) -> Result<(), AccessError> {
    if let Some(Some(count)) = self.refs.get_mut(idx) {
      *count = count.checked_add(1).ok_or(AccessError::ReferenceOverflow)?;
      Ok(())
    } else { Err(AccessError::FreeMemory(idx)) }
  }

  /// Tells the NodeField that something no longer references the data at `index`.
  /// If calling this function renders the refcount 0 the data will be freed and returned.
  /// 
  /// Failure to properly track references will lead to either freeing data you wanted or leaking data you didn't.
  pub fn remove_ref(&mut self, idx:usize) -> Result<Option<T>, AccessError> {
    let internal_index = idx;
    if let Some(Some(count)) = self.refs.get_mut(internal_index) {
      *count = count.checked_sub(1).ok_or(AccessError::ReferenceOverflow)?;
      if *count == 0 { Ok( Some( self.release(internal_index) ) ) } else { Ok(None) }
    } else { Err(AccessError::FreeMemory(internal_index)) }
  }

  /// Returns the number of references the data at `index` has or an [AccessError] if there is a problem.
  pub fn status(&self, idx:usize) -> Result<usize, AccessError> {
    if let Some(Some(count)) = self.refs.get(idx) {
      Ok(*count)
    } else { Err(AccessError::FreeMemory(idx)) }
  }

  /// Pushes `data` into the NodeField, returning the index it was stored at.
  /// 
  /// Once you recieve the index the data was stored at, it is your responsibility to manage its references.
  /// The data will start with one reference.
  #[must_use]
  pub fn push(&mut self, data:T) -> usize {
    let idx = self.reserve();
    self.data[idx] = data;
    idx
  }

  /// Replaces the data at `index` with `new_data`, returning the original data on success and an [AccessError] on failure.
  /// You may not replace an index which is currently free. 
  pub fn replace(&mut self, idx:usize, new_data:T) -> Result<T, AccessError> {
    if let Some(Some(_)) = self.refs.get(idx) {
      Ok(std::mem::replace(&mut self.data[idx], new_data))
    } else { Err(AccessError::FreeMemory(idx)) }
  }

  /// Travels through memory and re-arranges slots so that they are contiguous in memory, with no free slots in between occupied ones.
  /// The hashmap returned can be used to remap your references to their new locations. (Key:Old, Value:New)
  /// 
  /// Slots at the back of memory will be placed in the first free slot, until the above condition is met.
  /// 
  /// This operation is O(KlogN), where K is the number of swaps required to make data contiguous.
  /// This isn't technically correct, only half the lookups use the binary tree bc I'm silly, but
  /// it's close enough and if you care that much make an issue and I'll fix it.
  /// and logN is the height of the internal freetree. Barring degenerate cases where most of your
  /// free nodes are clumped at the front and most of your data is in the back, this should probably be faster than the O(N) alternative. 
  /// If you feel differently, make an issue and I'll revive the original linear search function as an alternative
  #[must_use]
  pub fn defrag(&mut self) -> HashMap<usize, usize> {
    let mut remapped = HashMap::new();
    if self.data.len() == 0 { return remapped }
    let mut full_search = self.data.len() - 1;
    'defrag: loop {
      if let Some(free) = self.list.get_first_empty_leaf() {
        while self.data[full_search].is_null() { 
          if free >= full_search { break 'defrag }
          full_search -= 1;
        }
        if free >= full_search { break 'defrag }
        remapped.insert(full_search, free);
        self.data.swap(free, full_search);
        self.refs.swap(free, full_search);
        self.list.set_leaf(full_search, false).unwrap();
        self.list.set_leaf(free, true).unwrap();
      } else { break 'defrag }
    }
    remapped
  }

  /// [NodeField::defrag]s the memory, then shrinks the internal memory Vec to the size of the block of occupied memory.
  #[must_use]
  pub fn trim(&mut self) -> HashMap<usize, usize> {
    let remap = self.defrag();
    if let Some(first_free) = self.first_free() {
      self.resize(first_free)
    }
    remap
  }
}

