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
  
  pub fn set_leaf(&mut self, mut idx: usize, full: bool) -> Option<()> {
    if idx >= self.size { return None }
    let mut step = 1;
    self.tree[idx][idx & step] = [!full, full];
    for _ in 0 .. self.height {
      // Positive step if we're on the left, negative step if we're on the right
      idx = if (idx & (step << 1)) == 0 { idx + step } else { idx - step };
      let combined = [
        self.tree[idx][0][0] | self.tree[idx][1][0],
        self.tree[idx][0][1] | self.tree[idx][1][1]
      ];
      step <<= 1;
      self.tree[idx][idx & step] = combined;
    }
    Some(())
  }

  pub fn is_full(&self, idx: usize) -> Option<bool> {
    if idx >= self.size { return None }
    Some(self.tree[idx & !1][idx & 1][1])
  }

}

#[test]
fn test_tree() {
  let mut tree = BinaryTree::new();
  tree.resize(8);
  
  // Test basic setting
  assert_eq!(tree.is_full(0).unwrap(), false);
  tree.set_leaf(0, true);
  assert_eq!(tree.is_full(0).unwrap(), true);

  assert_eq!(tree.find_first_leaf(false).unwrap(), 1);
  assert_eq!(tree.find_last_leaf(false).unwrap(), 7);
  
  tree.set_leaf(6, true);
  assert_eq!(tree.is_full(6).unwrap(), true);

  tree.resize(5);
  dbg!(&tree.tree);
  tree.resize(8);
  assert_eq!(tree.is_full(6).unwrap(), false);


}

