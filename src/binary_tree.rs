use serde::{Deserialize, Serialize};

const NEW_NODE: [[bool; 2]; 2] = [[true, false]; 2];
#[derive(Clone, Serialize, Deserialize)]
pub struct BinaryTree {
  tree: Vec<[[bool; 2]; 2]>, // [[left_has_empty, left_has_full], [right_has_empty, right_has_full]]
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
  // We want the node halfway through the tree. Divided capacity by 2 for the halfway point.
  // Subtract 1 is the 0-based index
  fn root(&self) -> usize { (self.capacity() >> 1) - 1 }

  /// Sets the number of leaves this tree tracks (this was really clever of me)
  pub fn resize(&mut self, size: usize) {
    if self.size == size { return }
    let last_safe_idx = size.min(self.size).saturating_sub(1);
    let last_val = self.is_full(last_safe_idx);
    let new_capacity = if size == 0 { 0 } else { size.next_power_of_two().max(2) };
    self.height = (new_capacity >> 1).ilog2() as u8;
    self.tree.truncate(size.saturating_sub(1)); // Eliminate any now-invalid data (decreasing only)
    self.tree.resize(new_capacity.saturating_sub(1), NEW_NODE); // Replace architecture
    self.size = size;
    if let Some(val) = last_val { self.set_leaf(last_safe_idx, val); } // Rebuild path
    self.tree.shrink_to_fit();
  }

  pub fn find_first_leaf(&self, full: bool) -> Option<usize> {
    if self.size == 0 { return None }
    let mut cur_idx = self.root();
    for i in (0 .. self.height).rev() {
      let step = 1 << i;
      if self.tree[cur_idx][0][full as usize] { cur_idx -= step }
      else if self.tree[cur_idx][1][full as usize] { cur_idx += step }
      else { return None }
    }
    let result = cur_idx + if self.tree[cur_idx][0][full as usize] { 0 }
    else if self.tree[cur_idx][1][full as usize] { 1 }
    else { return None };
    (result < self.size).then_some(result)
  }
  
  /// # WARNING 
  /// Calling find_last_leaf(false) will return None unless self.size == self.capacity()
  pub fn find_last_leaf(&self, full: bool) -> Option<usize> {
    if self.size == 0 { return None }
    let mut cur_idx = self.root();
    for i in (0 .. self.height).rev() {
      let step = 1 << i;
      if self.tree[cur_idx][1][full as usize] { cur_idx += step }
      else if self.tree[cur_idx][0][full as usize] { cur_idx -= step }
      else { return None }
    }
    let result = cur_idx + if self.tree[cur_idx][1][full as usize] { 1 }
    else if self.tree[cur_idx][0][full as usize] { 0 }
    else { return None };
    (result < self.size).then_some(result)
  }
  
  pub fn set_leaf(&mut self, idx: usize, full: bool) -> Option<()> {
    if idx >= self.size { return None }
    let mut step = 1;
    let mut cur_idx = idx & !1;
    self.tree[cur_idx][idx & step] = [!full, full];
    for _ in 0 .. self.height {
      let combined = [
        self.tree[cur_idx][0][0] | self.tree[cur_idx][1][0],
        self.tree[cur_idx][0][1] | self.tree[cur_idx][1][1]
      ];
      let left = idx & (step << 1) == 0;
      cur_idx = if left { cur_idx + step } else { cur_idx - step };
      self.tree[cur_idx][!left as usize] = combined;
      step <<= 1;
    }
    Some(())
  }

  pub fn is_full(&self, idx: usize) -> Option<bool> {
    if idx >= self.size { return None }
    Some(self.tree[idx & !1][idx & 1][1])
  }

}

// Because we have O(1) read times, we can't verify the path is created the easy way
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

#[test]
fn read() {
  let mut tree = BinaryTree::new();
  tree.resize(8);

  tree.set_leaf(0, true);
  tree.set_leaf(1, true);
  tree.set_leaf(6, true);

  assert_eq!(tree.find_first_leaf(false).unwrap(), 2);
  assert_eq!(tree.find_first_leaf(true).unwrap(), 0);
  assert_eq!(tree.find_last_leaf(true).unwrap(), 6);
}

#[test]
fn resize() {
  let mut tree = BinaryTree::new();
  tree.resize(8);
  
  // Test resizing
  tree.set_leaf(6, true);
  tree.set_leaf(4, true);
  tree.resize(5);
  assert_eq!(tree.is_full(4).unwrap(), true);
  tree.resize(8);
  assert_eq!(tree.is_full(6).unwrap(), false); // The 6 was reset as it's out of bounds
  assert_eq!(tree.is_full(4).unwrap(), true); // The 4 wasn't because it remained in bounds
  assert_eq!(tree.find_first_leaf(true).unwrap(), 4);
  assert_eq!(tree.find_last_leaf(true).unwrap(), 4);

}

