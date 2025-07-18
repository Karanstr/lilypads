#![warn(missing_docs)]
//! Fun little object pool allocator.
//!
//! This is a learning experience for me, but I'm fairly confident it doesn't suck that badly.
//!
//! This crate was originally intended for the creation of tree-like datastructures,
//! where indexes could be used instead of dealing with rust's reference/pointer system.
//! The vision of the project has somewhat shifted since v0.8 and is now intended as a 
//! general purpose object pool, for whatever you need to be pooling. It attempts to keep data as
//! contiguous as possible, [Pond::alloc] reserves the first (sequentially) free node and [Pond::defrag] +
//! [Pond::trim] are provided to maintain contiguity on otherwise sparse allocations.
//!
//! This crate isn't yet thread safe, but that's eventually on the todo list probably.
//!
//! # Example
//! ```
//! use lilypads::Pond;
//!
//! fn main() {
//!   let mut pool = Pond::new();
//!   // You can push data into the pond and recieve their index.
//!   let idx1 = pool.alloc(57);
//!   let idx2 = pool.alloc(42);
//!
//!   // Data is retrieved with get
//!   let data1 = pool.get(idx1).unwrap();
//!   assert_eq!(*data1, 57);
//!   // And get_mut
//!   let data2 = pool.get_mut(idx2).unwrap();
//!   *data2 = 13;
//!   assert_eq!(*pool.get(idx2).unwrap(), 13);
//!
//!   // Data can be freed with free, which will return the data stored at the index.
//!   let freed1 = pool.free(idx1).unwrap();
//!   assert_eq!(freed1, 57);
//!   assert_eq!(pool.get_mut(idx1), None);
//!
//!   // You can request a specific index with write, overwriting the existing data 
//!   // and returning whatever used to be there
//!   let replaced = pool.write(idx2, 98);
//!   assert_eq!(*pool.get(idx2).unwrap(), 98);
//!
//!   let far_idx = 17;
//!   let nothing = pool.write(far_idx, 1000);
//!   assert_eq!(nothing, None);
//!   assert_eq!(*pool.get(far_idx).unwrap(), 1000);
//!   
//! }
//! ```

mod bitmap;
use bitmap::Bitmap;
use std::collections::HashMap;
use std::mem::MaybeUninit;

/// The struct used to pool T.
///
/// The first available node will be allocated when you call [Pond::alloc],
/// intending to keep the data as contiguous as possible. If you need total contiguity,
/// [Pond::defrag] and [Pond::trim] should help with that.
#[derive(Debug)]
pub struct Pond<T> {
  data : Vec< MaybeUninit<T> >,
  list: Bitmap,
}
impl<T> Pond<T> {

  fn is_reserved(&self, idx: usize) -> bool {
    if let Some(result) = self.list.is_full(idx) { result } else { false }
  }

  fn first_free(&self) -> Option<usize> { self.list.find_first_free() }

  fn mark_free(&mut self, idx:usize) { self.list.set(idx, false).unwrap(); }

  fn mark_reserved(&mut self, idx:usize) { self.list.set(idx, true).unwrap(); }

  #[must_use]
  fn reserve(&mut self) -> usize {
    let idx = self.first_free().unwrap_or_else(|| {
      self.resize(self.len() + 1);
      self.len() - 1
    });
    self.mark_reserved(idx);
    idx
  }

}
impl<T> Pond<T> {
  /// Creates a new instance of [Pond]
  pub fn new() -> Self {
    Self {
      data : Vec::new(),
      list: Bitmap::new(),
    }
  }

  /// Returns the number of slots held internally, both free and full.
  pub fn len(&self) -> usize { self.data.len() }

  /// Returns the next index which will be allocated on a [Pond::alloc] call. If you need to
  /// guarantee a certain value, use [Pond::write] instead.
  pub fn next_allocated(&self) -> usize { self.first_free().unwrap_or(self.len()) }

  /// Sets Pond to hold `size` elements. If size < self.len(), excess data will be truncated and dropped.
  pub fn resize(&mut self, size: usize) {
    let additional = size.saturating_sub(self.len());
    for idx in (size .. self.len()).rev() {
      if self.list.is_full(idx).unwrap() { unsafe { self.data[idx].assume_init_drop(); } }
    }
    self.data.reserve(additional);
    unsafe { self.data.set_len(size); }
    self.list.resize(size);
  }

  /// Returns an immutable reference to the data stored at the requested index, or an [AccessError] if there is a problem.
  pub fn get(&self, idx:usize) -> Option<&T> {
    if !self.is_reserved(idx) { return None }
    Some( unsafe { self.data[idx].assume_init_ref() } )
  }

  /// Returns a mutable reference to the data stored at the requested index, or an [AccessError] if there is a problem.
  pub fn get_mut(&mut self, idx:usize) -> Option<&mut T> {
    if !self.is_reserved(idx) { return None }
    Some( unsafe { self.data[idx].assume_init_mut() } )
  }

  /// Stores `data` in PoolField, returning it's memory index.
  #[must_use]
  pub fn alloc(&mut self, data:T) -> usize {
    let idx = self.reserve();
    self.data[idx].write(data);
    idx
  }
  
  /// Overwrite and reserve the data at `idx`. 
  /// Returns Some(old_data) or None, depending whether the slot was previously reserved.
  ///
  /// This function will [Pond::resize] if `idx` is beyond [Pond::len], guaranteeing
  /// your data will be written to the requested slot.
  pub fn write(&mut self, idx:usize, new_data:T) -> Option<T> {
    if idx >= self.len() { self.resize(idx + 1) }
    let old_value = if self.is_reserved(idx) { 
      Some( unsafe { self.data[idx].assume_init_read() } ) 
    } else { None };
    self.data[idx].write(new_data);
    self.mark_reserved(idx);
    old_value
  }

  /// Frees the data at `index`, returning it on success or None on failure.
  /// Failure means you were trying to free a node which was already free.
  pub fn free(&mut self, idx:usize) -> Option<T> {
    if !self.is_reserved(idx) { return None }
    self.mark_free(idx);
    Some( unsafe { self.data[idx].assume_init_read() } )
  }

  /// Travels through memory and re-arranges slots so that they are contiguous in memory, with no free slots in between occupied ones.
  /// The hashmap returned can be used to remap your references to their new locations. (Key:Old, Value:New)
  /// 
  /// Slots at the back of memory will be placed in the first free slot, until the above condition is met.
  /// 
  // Note to self, figure out time complexity
  #[must_use]
  pub fn defrag(&mut self) -> HashMap<usize, usize> {
    let mut remapped = HashMap::new();
    if self.len() == 0 { return remapped }
    let mut full = self.len();
    let mut last_full = full;
    while let Some(free) = self.list.find_first_free() {
      for idx in (free .. last_full).rev() {
        if self.list.is_full(idx).unwrap() { full = idx; break }
      }
      if full == last_full { break }
      remapped.insert(full, free);
      self.data.swap(free, full);
      self.list.set(full, false).unwrap();
      self.list.set(free, true).unwrap();
      last_full = full;
    }
    remapped
  }

  /// [Pond::defrag]s the memory, then shrinks the internal vec to fit remaining data.
  #[must_use]
  pub fn trim(&mut self) -> HashMap<usize, usize> {
    let remap = self.defrag();
    if let Some(first_free) = self.first_free() { self.resize(first_free) }
    remap
  }

  /// Returns a safe, readonly version of the allocated memory.
  pub fn safe_data(&self) -> Vec<Option<&T>> {
    let mut safe_data = Vec::with_capacity(self.data.len());
    for idx in 0 .. self.data.len() { safe_data.push( self.get(idx)) }
    safe_data
  }
  
  /// Returns the unsafe data. This should only be used when you have some sort of
  /// access scheme (a tree) which can be used to safely navigate the partially-allocated data
  pub fn unsafe_data(&self) -> &Vec<MaybeUninit<T>> { &self.data }
}


use serde::{Serialize, Serializer, ser::SerializeSeq, Deserialize, Deserializer};
impl<T> Serialize for Pond<T> where T: Serialize {
  fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    let mut seq = serializer.serialize_seq(Some(self.data.len()))?;
    for idx in 0 .. self.data.len() { seq.serialize_element(&self.get(idx))?; }
    seq.end()
  }
}
impl<'de, T> Deserialize<'de> for Pond<T> where T: Deserialize<'de> {
  fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    let data: Vec<Option<T>> = Vec::deserialize(deserializer)?;
    let mut pool = Self::new();
    pool.resize(data.len());
    for (idx, pot_val) in data.into_iter().enumerate() {
      if let Some(val) = pot_val { pool.write(idx, val); }
    }
    Ok(pool)
  }
}
