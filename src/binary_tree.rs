use std::fmt;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Serialize, Deserialize)]
struct PackedNode(u8);
// xxxx left_has_empty left_has_full right_has_empty right_has_full
impl PackedNode {
  const fn empty() -> Self { Self(0b0000_1010) }

  fn read(self, left: bool, full: bool) -> bool {
    ((self.0 >> 2 * left as u8) >> !full as u8) & 0b1 == 1
  }
  // Data should be a u2
  fn write(&mut self, left: bool, data: u8) {
    // Mask out existing data
    self.0 &= !(0b11 << 2 * left as u8);
    self.0 |= data << 2 * left as u8;
  }
  fn combine(self) -> u8 { (self.0 & 0b11) | (self.0 >> 2) }
}
impl fmt::Debug for PackedNode {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{:04b}", self.0)
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BinaryTree {
  tree: Vec<PackedNode>,
  height: u8,
  size: usize, // Artificial limit for api
}
impl BinaryTree {
  pub fn new() -> Self {
    Self {
      tree: Vec::new(),
      height: 0,
      size: 0,
    }
  }

  fn capacity(&self) -> usize { self.tree.len() + 1}
  /// Don't call if size == 0
  // We want the node halfway through the tree. Divide capacity by 2 for the halfway point.
  // Subtract 1 is the 0-based index
  fn root(&self) -> usize { (self.capacity() >> 1) - 1 }

  /// Sets the number of leaves this tree tracks (this was really clever of me)
  pub fn resize(&mut self, size: usize) {
    if self.size == size { return }
    let last_safe_idx = size.min(self.size).saturating_sub(1);
    let last_val = self.is_full(last_safe_idx);
    let new_capacity = if size == 0 { 0 } else { size.next_power_of_two().max(2) };
    self.height = if new_capacity == 0 { 0 } else { (new_capacity >> 1).ilog2() as u8 };
    self.tree.truncate(size.saturating_sub(1)); // Eliminate any now-invalid data (decreasing only)
    self.tree.resize(new_capacity.saturating_sub(1), PackedNode::empty()); // Replace architecture
    self.size = size;
    if let Some(val) = last_val { self.set_leaf(last_safe_idx, val); } // Rebuild path
    self.tree.shrink_to_fit();
  }

  /// Don't call this function with false, false
  /// You'll just get None unless size == capacity
  fn find_leaf(&self, left: bool, full: bool) -> Option<usize> {
    if self.size == 0 { return None }
    let mut cur_idx = self.root();
    for i in (0 .. self.height).rev() {
      let step = 1 << i;
      if self.tree[cur_idx].read(left, full) { cur_idx = if left { cur_idx - step } else {cur_idx + step} }
      else if self.tree[cur_idx].read(!left, full) { cur_idx = if left { cur_idx + step } else {cur_idx - step} }
      else { return None }
    }
    let result = cur_idx + 
      if self.tree[cur_idx].read(left, full) { !left as usize }
      else if self.tree[cur_idx].read(!left, full) { left as usize }
    else { return None };
    (result < self.size).then_some(result)
  }

  pub fn find_first_free(&self) -> Option<usize> { self.find_leaf(true, false)}
  pub fn find_last_full(&self) -> Option<usize> { self.find_leaf(false, true)}

  pub fn set_leaf(&mut self, idx: usize, full: bool) -> Option<()> {
    if idx >= self.size { return None }
    let mut step = 1;
    let mut cur_idx = idx & !1;
    // We're just packing this silly stuff, we want has_empty to be !full and has_full to be full
    self.tree[cur_idx].write(idx & step == 0, ((!full as u8) << 1) | full as u8);
    for _ in 0 .. self.height {
      let combined = self.tree[cur_idx].combine();
      let on_left = idx & (step << 1) == 0;
      cur_idx = if on_left { cur_idx + step } else { cur_idx - step };
      self.tree[cur_idx].write(on_left, combined);
      step <<= 1;
    }
    Some(())
  }

  pub fn is_full(&self, idx: usize) -> Option<bool> {
    if idx >= self.size { return None }
    Some(self.tree[idx & !1].read(idx & 1 == 0, true))
  }

}

#[test]
fn write() {
  let mut tree = BinaryTree::new();
  tree.resize(7);

  // Does setting work correctly
  tree.set_leaf(1, true);
  assert_eq!(tree.is_full(1).unwrap(), true);

  // Make sure setting and unsetting work
  tree.set_leaf(3, true);
  assert_eq!(tree.is_full(3).unwrap(), true);
  tree.set_leaf(3, false);
  assert_eq!(tree.is_full(3).unwrap(), false);

  // Do we correctly catch sets outside of bounds
  assert_eq!(tree.set_leaf(7, false), None);
}

// We have to verify the paths were built correctly by descending, since normal reads are O(1)
#[test]
fn paths() {
  let mut tree = BinaryTree::new();
  tree.resize(8);

  tree.set_leaf(0, true);
  tree.set_leaf(1, true);
  tree.set_leaf(6, true);

  assert_eq!(tree.find_first_free().unwrap(), 2);
  assert_eq!(tree.find_leaf(true, true).unwrap(), 0);
  assert_eq!(tree.find_last_full().unwrap(), 6);
}

#[test]
fn resize() {
  let mut tree = BinaryTree::new();
  tree.resize(8);
  
  // Test resizing
  tree.set_leaf(6, true);
  tree.set_leaf(2, true);
  // Ensure we also crop the old root head
  tree.resize(3);
  assert_eq!(tree.is_full(2).unwrap(), true);
  tree.resize(8);
  assert_eq!(tree.is_full(6).unwrap(), false); // The 6 was reset as it's out of bounds
  assert_eq!(tree.is_full(2).unwrap(), true); // The 2 wasn't because it remained in bounds
  dbg!(&tree.tree);
  assert_eq!(tree.find_leaf(true, true).unwrap(), 2);
  assert_eq!(tree.find_last_full().unwrap(), 2);

}

