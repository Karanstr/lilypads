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

use serde::{Serialize, Deserialize};
use std::{collections::HashMap, fs::read_link};

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

#[derive(Clone, Copy, Debug)]
struct Node {
  left: bool,
  right: bool,
}
impl Node {
  fn new() -> Self {
    Self {
      left: false, 
      right: false,
    }
  }
}
struct FullFlatBinaryTree{
  pub tree: Vec<Node>,
  height: u8,
}
impl FullFlatBinaryTree {
  pub fn new(height: u8) -> Self {
    Self {
      tree: vec![Node::new(); (1 << (height + 1)) - 1],
      height
    }
  }

  pub fn set_height(&mut self, new_height: u8) {
    self.height = new_height;
    self.tree.resize((1 << (new_height + 1)) - 1, Node::new());
  }
  
  pub fn get_first_empty_leaf(&self) -> Option<usize> {
    let mut cur_idx = (1 << self.height) - 1;
    for i in (0 .. self.height).rev() {
      let step = 1 << i;
      if self.tree[cur_idx].left == false { cur_idx -= step }
      else if self.tree[cur_idx].right == false { cur_idx += step }
      else { return None }
    }
    Some(cur_idx + self.tree[cur_idx].left as usize)
  }
  
  pub fn set_leaf(&mut self, idx: usize, full: bool) {
    let mut cur_idx = idx & (!0 << 1); // The last bit is left vs right, the leaf's parent node is at the even index
    if idx & 1 == 0 { self.tree[cur_idx].left = full; } else { self.tree[cur_idx].right = full; }
    let mut combined = self.tree[cur_idx].left & self.tree[cur_idx].right;
    // Or cur_node's children then walk it up the tree
    for i in 0 .. self.height {
      let step = 1 << i;
      if cur_idx & (1 << (i + 1)) == 0 { 
        cur_idx += step;
        self.tree[cur_idx].left = combined;
      } else {
        cur_idx -= step;
        self.tree[cur_idx].right = combined;
      }
      combined = self.tree[cur_idx].left & self.tree[cur_idx].right;
    }
  }
}

#[test]
fn test_tree() {
  let mut tree = FullFlatBinaryTree::new(3);
  assert_eq!(tree.get_first_empty_leaf().unwrap(), 0);
  for i in 0 .. 16 { tree.set_leaf(i, true); }
  assert_eq!(tree.get_first_empty_leaf(), None);
  tree.set_leaf(7, false);
  assert_eq!(tree.get_first_empty_leaf().unwrap(), 7);
}


/// Trait any data stored in the NodeField must implement, guaranteeing there's a value we can use as null for uninitialized cells. 
/// [Option<T>] where T:[Sized] is implemented for you, so if you don't care/understand why you'd
/// want this just wrap your data in an [Option]
pub trait Nullable: Sized {
  //! The main reason you'd manually implement this is if you don't want to deal with a wrapper type eating up your bits and would rather just define a custom null sentinel.
  //!
  //! # Example
  //! ```
  //! #[derive(PartialEq)]
  //! struct NoZeroU32(u32);
  //! impl Nullable for NoZeroU32 {
  //!   const NULL_VAL: Self = NoZeroU32(u32::MAX);
  //!   fn is_null(&self) -> bool { self != &Self::NULL_VAL }
  //! }
  //! ```
  //! Implementing Nullable on a wrapper type
  //!
  //! # Example
  //! ```
  //! impl<T> Nullable for Option<T> { 
  //!   const NULL_VAL: Self = None; 
  //!   fn is_null(&self) -> bool { self.is_none() }
  //!   fn take(&mut self) -> Self { self.take() }
  //! }
  //! ```
  //! Implementing Nullable for Option<T>, this is the canon implementation within this crate

  /// The null sentinel used 
  const NULL_VAL: Self;
  /// Returns true if the data matches its type's null condition. In [Option]'s case, this is the same as calling [Option::is_none]
  /// I could've provided this with a default impl, but I would have to bind Self: PartialEq and I
  /// don't want to do that.
  fn is_null(&self) -> bool;
  /// Takes replaces the value at &mut self with Self::NULL_VAL, returning the original value. In [Option]'s case, this is the same as calling [Option::take]
  fn take(&mut self) -> Self { std::mem::replace(self, Self::NULL_VAL) }
}

impl<T> Nullable for Option<T> { 
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
}

// Private methods
impl<T:Nullable> NodeField<T> {
  fn last_index(&self) -> usize { self.data.len() - 1 }

  fn first_free(&self) -> Option<usize> {
    for (index, reference) in self.refs.iter().enumerate() {
      if reference.is_none() { return Some(index) }
    }
    None
  }

  fn mark_free(&mut self, idx:usize) { self.refs[idx] = None; }

  fn mark_reserved(&mut self, idx:usize) { self.refs[idx] = Some(1); }

  fn release(&mut self, idx:usize) -> T {
    let data = self.data[idx].take();
    if data.is_null() { panic!("Tried to release free slot") } else {
      self.mark_free(idx);
      data
    }
  }

  /// Right now reserve sets data to have a single reference (through mark_reserved). I haven't
  /// decided whether this is good or not yet, but for now it's how it'll be
  #[must_use]
  fn reserve(&mut self) -> usize {
    let idx = if let Some(idx) = self.first_free() { idx } else {
      self.data.push(T::NULL_VAL);
      self.refs.push(None);
      self.last_index()
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
    }
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

  /// Returns the next index which will be allocated on a [NodeField::push] call
  pub fn next_allocated(&self) -> usize { self.first_free().unwrap_or(self.data.len()) }

  /// Travels through memory and re-arranges slots so that they are contiguous in memory, with no free slots in between occupied ones.
  /// The hashmap returned can be used to remap your references to their new locations. (Key:Old, Value:New)
  /// 
  /// Slots at the back of memory will be placed in the first free slot, until the above condition is met.
  /// 
  /// This operation is O(n) to the number of slots in memory.
  #[must_use]
  pub fn defrag(&mut self) -> HashMap<usize, usize> {
    let mut remapped = HashMap::new();
    let mut solid_until = 0;
    if solid_until == self.data.len() { return remapped }
    let mut free_until = self.data.len() - 1;
    'defrag: loop {
      while !self.data[solid_until].is_null() { 
        solid_until += 1;
        if solid_until == free_until { break 'defrag }
      }
      while self.data[free_until].is_null() { 
        free_until -= 1;
        if free_until == solid_until { break 'defrag }
      }
      remapped.insert(free_until, solid_until);
      self.data.swap(free_until, solid_until);
      self.refs.swap(free_until, solid_until);
    }
    remapped
  }

  /// [NodeField::defrag]s the memory, then shrinks the internal memory Vec to the size of the block of occupied memory.
  #[must_use]
  pub fn trim(&mut self) -> HashMap<usize, usize> {
    let remap = self.defrag();
    if let Some(first_free) = self.first_free() {
      self.data.truncate(first_free);
      self.data.shrink_to_fit();
      self.refs.truncate(first_free);
      self.refs.shrink_to_fit();
    }
    remap
  }

  /// Returns a reference to the internal data Vec
  pub fn data(&self) -> &Vec<T> { &self.data }

  /// Returns a reference to the internal reference Vec
  pub fn refs(&self) -> &Vec< Option<usize> > { &self.refs }
}
