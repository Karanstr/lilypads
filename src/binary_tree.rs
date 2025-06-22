use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
struct Node {
  left: [bool; 2], // [has_empty, has_full]
  right: [bool; 2], // [has_empty, has_full]
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FullFlatBinaryTree{
  tree: Vec<Node>,
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

  // This is incorrect
  // This is messy but I just want my tests to pass so I can fix everything
  /// Sets the number of leaves this tree tracks (this was really clever of me)
  pub fn resize(&mut self, size: usize) {
    let old_size = self.size;
    // let shrinking = self.size > size;
    if self.size == size { return }
    self.size = size;
    // This is stupid and what I get when I cheat and add edge cases. This problem goes away once I replace the struct
    self.capacity = self.size.next_power_of_two().max(2); // How many leaves we can handle
    self.height = (self.capacity >> 1).ilog2() as u8;
    if old_size > self.size && self.size != 0 {
      let last_safe_idx = self.size - 1;
      let last_val = self.get_leaf(last_safe_idx).unwrap();
      self.tree.truncate(self.size - 1); // Eliminate any now-invalid data
      self.tree.resize(self.capacity - 1, Node::new()); // Replace culled architecture
      self.set_leaf(last_safe_idx, last_val); // Restore the path lost in the cull
      self.tree.shrink_to_fit();
    }
    else if old_size > self.size {
      self.tree.truncate(0); // Eliminate any now-invalid data
      self.tree.resize(self.capacity - 1, Node::new()); // Replace culled architecture
      self.tree.shrink_to_fit();
    }
    else if old_size == 0 {
      self.tree.resize(self.capacity - 1, Node::new());
    }
    else {
      let last_old_idx = old_size - 1;
      let last_val = self.get_leaf(last_old_idx).unwrap();
      self.tree.resize(self.capacity - 1, Node::new());
      self.set_leaf(last_old_idx, last_val);
    }
  }


  // Might work, check height logic
  pub fn find_first_leaf(&self, val: bool) -> Option<usize> {
    if self.size == 0 { return None }
    let mut cur_idx = (2 << self.height) - 1;
    for i in (0 .. self.height - 1).rev() {
      let step = 1 << i;
      if self.tree[cur_idx].left[val as usize] { cur_idx -= step }
      else if self.tree[cur_idx].right[val as usize] { cur_idx += step }
      else { return None }
    }
    let result = cur_idx + if self.tree[cur_idx].left[val as usize] == false { 0 }
    else if self.tree[cur_idx].right[val as usize] == false { 1 }
    else { return None };
    (result < self.size).then_some(result)
  }
  
  // Might work, check height logic
  pub fn find_last_leaf(&self, val: bool) -> Option<usize> {
    if self.size == 0 { return None }
    let mut cur_idx = (2 << self.height) - 1;
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
  
  // Should work
  pub fn set_leaf(&mut self, idx: usize, full: bool) -> Option<()> {
    if idx >= self.size { return None }
    let mut cur_idx = idx & !1; // The last bit is left vs right, the leaf's parent node is at the even index
    if idx & 1 == 0 { self.tree[cur_idx].left = [!full, full]; } 
    else { self.tree[cur_idx].right = [!full, full]; }

    let mut combined = [
      self.tree[cur_idx].left[0] & self.tree[cur_idx].right[0],
      self.tree[cur_idx].left[1] & self.tree[cur_idx].right[1]
    ];
    for i in 0 .. self.height - 1 {
      let step = 1 << i;
      if cur_idx & (1 << (i + 1)) == 0 { 
        cur_idx += step;
        self.tree[cur_idx].left = combined;
      } else {
        cur_idx -= step;
        self.tree[cur_idx].right = combined;
      }
      combined = [
        self.tree[cur_idx].left[0] & self.tree[cur_idx].right[0],
        self.tree[cur_idx].left[1] & self.tree[cur_idx].right[1]
      ];
    }
    Some(())
  }

  // Should work
  pub fn is_full(&self, idx: usize) -> Option<bool> {
    if idx >= self.size { return None }
    let cur_idx = idx & !1;
    Some( if idx & 1 == 0 { self.tree[cur_idx].left[1] } else { self.tree[cur_idx].right[1] } )
  }

}

#[test]
fn test_tree() {
  let mut tree = FullFlatBinaryTree::new();
}

// Write a test for resize logic


