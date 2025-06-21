mod binary_tree;
use binary_tree::FullFlatBinaryTree;
// use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::mem::MaybeUninit;

#[derive(Debug)]
pub struct NodeField<T> {
  data : Vec< MaybeUninit<T> >,
  list: FullFlatBinaryTree
}

// Private methods
impl<T> NodeField<T> {

  fn is_reserved(&self, idx: usize) -> bool {
    match self.list.get_leaf(idx) {
      Some(result) => result,
      None => false
    }
  }

  fn first_free(&self) -> Option<usize> { self.list.get_first_empty_leaf() }

  fn mark_free(&mut self, idx:usize) { self.list.set_leaf(idx, false).unwrap(); }

  fn mark_reserved(&mut self, idx:usize) { self.list.set_leaf(idx, true).unwrap(); }

  #[must_use]
  fn reserve(&mut self) -> usize {
    let pot_idx = self.first_free();
    let idx = if pot_idx.is_some() { pot_idx.unwrap() }
    else {
      let old_len = self.data.len();
      self.list.resize(old_len + 1);
      self.data.reserve(1);
      unsafe { self.data.set_len(old_len + 1) }
      old_len
    };
    self.mark_reserved(idx);
    idx
  }

}

// Public functions
impl<T> NodeField<T> {
  pub fn new() -> Self {
    Self {
      data : Vec::new(),
      list: FullFlatBinaryTree::new(),
    }
  }

  /// Returns the next index which will be allocated on a [NodeField::alloc] call
  pub fn next_allocated(&self) -> usize { self.first_free().unwrap_or(self.data.len()) }

  /// Sets NodeField to hold `size` elements. If size < self.data().len(), excess data will be truncated
  /// and dropped. Use care when calling this function.
  pub fn resize(&mut self, size: usize) {
    let additional = size.saturating_sub(self.data.len());
    // If we don't free each set cell manually, it won't get dropped and we'll leak whatever
    // resources they owned
    // while let Some(idx) = self.list.get_last_full_node() {
      // if idx < size { break }
      // // Releases the value from the vec, then drops it when we loop and the scope resets.
      // self.free(idx);
    // }
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

  /// Stores `data` in the NodeField, returning it's memory index.
  #[must_use]
  pub fn alloc(&mut self, data:T) -> usize {
    let idx = self.reserve();
    self.data[idx].write(data);
    idx
  }

  /// Frees the data at `index`, returning it on success or None on failure.
  /// Failure means you were trying to free a node which was already free.
  pub fn free(&mut self, idx:usize) -> Option<T> {
    if !self.is_reserved(idx) { return None }
    self.mark_free(idx);
    Some( unsafe { self.data[idx].assume_init_read() } )
  }

  /// Replaces the data at `index` with `new_data`, returning the original data on success or None on failure.
  /// You may not replace an index which is currently free. 
  pub fn replace(&mut self, idx:usize, new_data:T) -> Option<T> {
    if !self.is_reserved(idx) { return None }
    let old_value = unsafe { self.data[idx].assume_init_read() };
    self.data[idx].write(new_data);
    Some(old_value)
  }

}


// This all has to be rewritten once I finish fixing the binarytree

  // /// Travels through memory and re-arranges slots so that they are contiguous in memory, with no free slots in between occupied ones.
  // /// The hashmap returned can be used to remap your references to their new locations. (Key:Old, Value:New)
  // /// 
  // /// Slots at the back of memory will be placed in the first free slot, until the above condition is met.
  // /// 
  // /// This operation is O(KlogN), where K is the number of swaps required to make data contiguous.
  // /// This isn't technically correct, only half the lookups use the binary tree bc I'm silly, but
  // /// it's close enough and if you care that much make an issue and I'll fix it.
  // /// and logN is the height of the internal freetree. Barring degenerate cases where most of your
  // /// free nodes are clumped at the front and most of your data is in the back, this should probably be faster than the O(N) alternative. 
  // /// If you feel differently, make an issue and I'll revive the original linear search function as an alternative
//   #[must_use]
//   pub fn defrag(&mut self) -> HashMap<usize, usize> {
//     let mut remapped = HashMap::new();
//     if self.data.len() == 0 { return remapped }
//     let mut full_search = self.data.len() - 1;
//     'defrag: loop {
//       if let Some(free) = self.list.get_first_empty_leaf() {
//         while self.data[full_search].is_null() { 
//           if free >= full_search { break 'defrag }
//           full_search -= 1;
//         }
//         if free >= full_search { break 'defrag }
//         remapped.insert(full_search, free);
//         self.data.swap(free, full_search);
//         self.list.set_leaf(full_search, false).unwrap();
//         self.list.set_leaf(free, true).unwrap();
//       } else { break 'defrag }
//     }
//     remapped
//   }
//
//   /// [NodeField::defrag]s the memory, then shrinks the internal memory Vec to the size of the block of occupied memory.
//   #[must_use]
//   pub fn trim(&mut self) -> HashMap<usize, usize> {
//     let remap = self.defrag();
//     if let Some(first_free) = self.first_free() {
//       self.resize(first_free)
//     }
//     remap
//   }
// }
//
