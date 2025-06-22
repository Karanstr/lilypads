use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct Node {
  left: [bool; 2], // [has_empty, has_full]
  right: [bool; 2], // [has_empty, has_full]
}
impl Node {
  fn new() -> Self {
    Self {
      left: [true, false],
      right: [true, false],
    }
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FullFlatBinaryTree{
  pub tree: Vec<Node>,
  height: u8,
  size: usize,
  capacity: usize
}
impl FullFlatBinaryTree {
  pub fn new() -> Self {
    Self {
      tree: Vec::new(),
      height: 0,
      size: 0, // Artificial limit for api
      capacity: 0, // Actual limit without restructuring
    }
  }

  // height == 0 capacity 0 len 0
  // height == 1 capacity 2 len 1
  // height == 2 capacity 4 len 3

  /// Sets the number of leaves this tree tracks (this was really clever of me)
  pub fn resize(&mut self, size: usize) {
    if self.size == size { return }
    else if size < self.size {
      let last_safe_idx = size.saturating_sub(1);
      let last_val = self.is_full(last_safe_idx);
      self.size = size;
      self.capacity = if self.size == 0 { 0 } else { self.size.next_power_of_two() };
      self.height = (self.capacity >> 1).ilog2() as u8;
      self.tree.truncate(self.size.saturating_sub(1)); // Eliminate any now-invalid data
      self.tree.resize(self.capacity.saturating_sub(1), Node::new()); // Replace culled architecture
      if let Some(val) = last_val { self.set_leaf(last_safe_idx, val); } // Restore path lost in cull
      self.tree.shrink_to_fit();
    }
    else if size > self.size {
      let last_old_idx = self.size.saturating_sub(1);
      let last_val = self.is_full(last_old_idx);
      self.size = size;
      self.capacity = if self.size == 0 { 0 } else { self.size.next_power_of_two() };
      self.height = (self.capacity >> 1).ilog2() as u8;
      self.tree.resize(self.capacity.saturating_sub(1), Node::new());
      if let Some(val) = last_val { self.set_leaf(last_old_idx, val); }
    }
  }

  pub fn find_first_leaf(&self, val: bool) -> Option<usize> {
    if self.size == 0 { return None }
    let mut cur_idx = (self.capacity >> 1) - 1;
    for i in (0 .. self.height).rev() {
      let step = 1 << i;
      if self.tree[cur_idx].left[val as usize] { cur_idx -= step }
      else if self.tree[cur_idx].right[val as usize] { cur_idx += step }
      else { return None }
    }
    let result = cur_idx + if self.tree[cur_idx].left[val as usize] { 0 }
    else if self.tree[cur_idx].right[val as usize] { 1 }
    else { return None };
    (result < self.size).then_some(result)
  }
  
  pub fn find_last_leaf(&self, val: bool) -> Option<usize> {
    if self.size == 0 { return None }
    let mut cur_idx = (self.capacity >> 1) - 1;
    for i in (0 .. self.height).rev() {
      let step = 1 << i;
      if self.tree[cur_idx].right[val as usize] { cur_idx += step }
      else if self.tree[cur_idx].left[val as usize] { cur_idx -= step }
      else { return None }
    }
    let result = cur_idx + if self.tree[cur_idx].right[val as usize] { 1 }
    else if self.tree[cur_idx].left[val as usize] { 0 }
    else { return None };
    (result < self.size).then_some(result)
  }
  
  pub fn set_leaf(&mut self, idx: usize, full: bool) -> Option<()> {
    if idx >= self.size { return None }
    let mut cur_idx = idx & !1; // The last bit is left vs right, the leaf's parent node is at the even index
    if idx & 1 == 0 { self.tree[cur_idx].left = [!full, full]; } 
    else { self.tree[cur_idx].right = [!full, full]; }
    let mut combined = [
      self.tree[cur_idx].left[0] | self.tree[cur_idx].right[0],
      self.tree[cur_idx].left[1] | self.tree[cur_idx].right[1]
    ];
    for i in 0 .. self.height {
      let step = 1 << i;
      if cur_idx & (1 << (i + 1)) == 0 { 
        cur_idx += step;
        self.tree[cur_idx].left = combined;
      } else {
        cur_idx -= step;
        self.tree[cur_idx].right = combined;
      }
      combined = [
        self.tree[cur_idx].left[0] | self.tree[cur_idx].right[0],
        self.tree[cur_idx].left[1] | self.tree[cur_idx].right[1]
      ];
    }
    Some(())
  }

  pub fn is_full(&self, idx: usize) -> Option<bool> {
    if idx >= self.size { return None }
    let cur_idx = idx & !1;
    Some( if idx & 1 == 0 { self.tree[cur_idx].left[1] } else { self.tree[cur_idx].right[1] } )
  }

}

#[test]
fn test_tree() {
  let mut tree = FullFlatBinaryTree::new();
  tree.resize(5);
  
  // Test basic setting
  assert_eq!(tree.is_full(0).unwrap(), false);
  tree.set_leaf(0, true);
  assert_eq!(tree.is_full(0).unwrap(), true);
  dbg!(&tree.tree);

  assert_eq!(tree.find_first_leaf(false).unwrap(), 1);
  assert_eq!(tree.find_last_leaf(false).unwrap(), 5);


}

