#![warn(missing_docs)]
use crate::bitmap::AcceleratedBitmap;
use std::collections::HashMap;
use std::mem::MaybeUninit;

/// The struct used to pool T.
///
/// The first available node will be assigned when you call [Pond::insert],
/// intending to keep the data as contiguous as possible. If you need total contiguity,
/// [Pond::defrag] and [Pond::trim] should help with that.
#[derive(Debug)]
pub struct Pond<T> {
  data : Vec< MaybeUninit<T> >,
  bitmap: AcceleratedBitmap,
}
impl<T> Pond<T> {

  /// THIS FUNCTION DOESN'T BOUND CHECK
  fn mark_free(&mut self, idx:usize) { self.bitmap.set(idx, false) }

  /// THIS FUNCTION DOESN'T BOUND CHECK
  fn mark_reserved(&mut self, idx:usize) { self.bitmap.set(idx, true); }

  #[must_use]
  fn reserve(&mut self) -> usize {
    let idx = self.bitmap.first_free().unwrap_or(self.len());
    if idx >= self.len() { self.resize(idx + 1) }
    self.mark_reserved(idx);
    idx
  }

}
impl<T> Pond<T> {
  /// Creates a new instance of [Pond]
  pub fn new() -> Self {
    Self {
      data : Vec::new(),
      bitmap: AcceleratedBitmap::new(3),
    }
  }
  
  /// Checks whether the provided index has an associated value
  pub fn is_occupied(&self, idx: usize) -> bool {
    if idx < self.data.len() { self.bitmap.is_set(idx) } else { false }
  }

  /// Returns the number of slots held internally, both free and full.
  pub fn len(&self) -> usize { self.data.len() }

  /// Returns the next index which will be assigned on a [Pond::insert] call. If you need to
  /// guarantee a specific index, use [Pond::write] instead.
  pub fn next_index(&self) -> usize { self.bitmap.first_free().unwrap_or(self.len()) }

  /// Sets Pond to hold `size` elements. If size < self.len(), excess data will be truncated and dropped.
  pub fn resize(&mut self, size: usize) {
    for idx in size .. self.len() {
      if self.bitmap.is_set(idx) { unsafe { self.data[idx].assume_init_drop(); } }
    }
    self.data.reserve(size.saturating_sub(self.len()));
    unsafe { self.data.set_len(size); }
    self.bitmap.resize(size);
  }

  /// Returns an immutable reference to the data stored at the requested index, or None if the index isn't reserved
  pub fn get(&self, idx:usize) -> Option<&T> {
    if !self.is_occupied(idx) { return None }
    Some( unsafe { self.data[idx].assume_init_ref() } )
  }

  /// Returns a mutable reference to the data stored at the requested index, or None if the index isn't reserved
  pub fn get_mut(&mut self, idx:usize) -> Option<&mut T> {
    if !self.is_occupied(idx) { return None }
    Some( unsafe { self.data[idx].assume_init_mut() } )
  }

  /// Stores `data` in PoolField, returning a reference index.
  #[must_use]
  pub fn insert(&mut self, data:T) -> usize {
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
    let old_value = if self.is_occupied(idx) { 
      Some( unsafe { self.data[idx].assume_init_read() } ) 
    } else { None };
    self.data[idx].write(new_data);
    self.mark_reserved(idx);
    old_value
  }

  /// Frees the data at `index`, returning it on success or None on failure.
  /// Failure means you were trying to free a node which was already free.
  pub fn free(&mut self, idx:usize) -> Option<T> {
    if !self.is_occupied(idx) { return None }
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
    while let Some(free) = self.bitmap.first_free() {
      for idx in (free .. last_full).rev() {
        if self.bitmap.is_set(idx) { full = idx; break }
      }
      if full == last_full { break }
      remapped.insert(full, free);
      self.data.swap(free, full);
      self.bitmap.set(full, false);
      self.bitmap.set(free, true);
      last_full = full;
    }
    remapped
  }

  /// [Pond::defrag]s the memory, then shrinks the internal vec to fit remaining data.
  #[must_use]
  pub fn trim(&mut self) -> HashMap<usize, usize> {
    let remap = self.defrag();
    if let Some(first_free) = self.bitmap.first_free() { self.resize(first_free) }
    remap
  }

  /// Returns a safe, readonly version of the internal vec.
  pub fn safe_data(&self) -> Vec<Option<&T>> {
    let mut safe_data = Vec::with_capacity(self.data.len());
    for idx in 0 .. self.data.len() { safe_data.push( self.get(idx)) }
    safe_data
  }

  /// Returns the readonly, unsafe reference to the internal vec. 
  /// This should only be used when you have some sort of
  /// access scheme (such as a tree) which can be used to safely navigate the unsafe data
  pub fn unsafe_data(&self) -> &Vec<MaybeUninit<T>> { &self.data }
}

// Iterators
impl<T> Pond<T> {

  /// Returns an iterator over all valid items stored in this pond, in order.
  ///
  /// This iterator covers (item_idx, &T)
  pub fn iter(&self) -> impl Iterator<Item = (usize, &T)> {
    let bitmap = &self.bitmap;
    self.data.iter().enumerate().filter_map(|(idx, data)| {
      // This is a safe call because we're iterating over avaliable slots already
      if bitmap.is_set(idx) {
        Some( (idx, unsafe { data.assume_init_ref() }) )
      } else { None }
    })
  }

  /// Returns an iterator over all valid items stored in this pond, in order.
  ///
  /// This iterator covers (item_idx, &mut T)
  pub fn iter_mut(&mut self) -> impl Iterator<Item = (usize, &mut T)> {
    let bitmap = &self.bitmap;
    self.data.iter_mut().enumerate().filter_map(|(idx, data)| {
      // This is a safe call because we're iterating over avaliable slots already
      if bitmap.is_set(idx) {
        Some( (idx, unsafe { data.assume_init_mut() }) )
      } else { None }
    } ) 
  }

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
