#[deny(missing_docs)]
mod binary_tree;
use binary_tree::BinaryTree;
use std::collections::HashMap;
use std::mem::MaybeUninit;

#[derive(Debug)]
pub struct NodePool<T> {
  data : Vec< MaybeUninit<T> >,
  list: BinaryTree,
}
// Private methods
impl<T> NodePool<T> {

  fn is_reserved(&self, idx: usize) -> bool {
    match self.list.is_full(idx) {
      Some(result) => result,
      None => false
    }
  }

  fn first_free(&self) -> Option<usize> { self.list.find_first_free() }

  fn mark_free(&mut self, idx:usize) { self.list.set_leaf(idx, false).unwrap(); }

  fn mark_reserved(&mut self, idx:usize) { self.list.set_leaf(idx, true).unwrap(); }

  #[must_use]
  fn reserve(&mut self) -> usize {
    let pot_idx = self.first_free();
    let idx = if pot_idx.is_some() { pot_idx.unwrap() }
    else {
      let old_len = self.len();
      self.resize(old_len + 1);
      old_len
    };
    self.mark_reserved(idx);
    idx
  }

}
// Public functions
impl<T> NodePool<T> {
  pub fn new() -> Self {
    Self {
      data : Vec::new(),
      list: BinaryTree::new(),
    }
  }

  pub fn len(&self) -> usize { self.data.len() }

  /// Returns the next index which will be allocated on a [NodePool::alloc] call. If you need to
  /// guarantee a certain value, use [NodePool::write] instead.
  pub fn next_allocated(&self) -> usize { self.first_free().unwrap_or(self.data.len()) }

  /// Sets NodePool to hold `size` elements. If size < self.data().len(), excess data will be truncated
  /// and dropped. Use care when calling this function.
  pub fn resize(&mut self, size: usize) {
    let additional = size.saturating_sub(self.data.len());
    while let Some(idx) = self.list.find_last_full() {
      if idx < size { break }
      // Releases the value from the vec, then drops it when we loop and the scope resets.
      self.free(idx);
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
  /// This function will [NodePool::resize] if `idx` is beyond [NodePool::len], guaranteeing
  /// your data will be written to the requested slot.
  pub fn write(&mut self, idx:usize, new_data:T) -> Option<T> {
    if idx >= self.len() { self.resize(idx + 1) }
    let old_value = if !self.is_reserved(idx) { None } 
    else { Some( unsafe { self.data[idx].assume_init_read() } ) };
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
  /// This operation is O(KlogN), where K is the number of swaps required to make data contiguous
  /// and logN is the height of the internal freetree. Barring degenerate cases where most of your
  /// free nodes are clumped at the front and most of your data is in the back, this should probably be faster than the O(N) alternative. 
  /// If you feel differently, make an issue and I'll revive the original linear search function as an alternative
  #[must_use]
  pub fn defrag(&mut self) -> HashMap<usize, usize> {
    let mut remapped = HashMap::new();
    if self.data.len() == 0 { return remapped }
    'defrag: loop {
      match (self.list.find_first_free(), self.list.find_last_full()) {
        (Some(free), Some(full)) => {
          if free >= full { break 'defrag }
          remapped.insert(full, free);
          self.data.swap(free, full);
          self.list.set_leaf(full, false).unwrap();
          self.list.set_leaf(free, true).unwrap();
        }
        _ => break 'defrag
      }
    }
    remapped
  }

  /// [NodePool::defrag]s the memory, then shrinks the internal vec to fit remaining data.
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
impl<T> Serialize for NodePool<T> where T: Serialize {
  fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    let mut seq = serializer.serialize_seq(Some(self.data.len()))?;
    for idx in 0 .. self.data.len() { seq.serialize_element(&self.get(idx))?; }
    seq.end()
  }
}

impl<'de, T> Deserialize<'de> for NodePool<T> where T: Deserialize<'de> {
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
