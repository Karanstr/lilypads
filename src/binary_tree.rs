use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Node {
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
      tree: vec![Node::new(); 1],
      height: 0,
      size: 0, // Artificial limit to make api simpler
      capacity: 2, // Actual limit without growing, one node = 2 leaves
    }
  }

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

  pub fn get_first_empty_leaf(&self) -> Option<usize> {
    let mut cur_idx = (1 << self.height) - 1;
    for i in (0 .. self.height).rev() {
      let step = 1 << i;
      if self.tree[cur_idx].left == false { cur_idx -= step }
      else if self.tree[cur_idx].right == false { cur_idx += step }
      else { return None }
    }
    let result = cur_idx + if self.tree[cur_idx].left == false { 0 }
    else if self.tree[cur_idx].right == false { 1 }
    else { return None };
    (result < self.size).then_some(result)
  }
  
  // This isn't real unless we double the data we store (basically a tree for both set and unset nodes)
  // pub fn get_last_full_leaf(&self) -> Option<usize> {
  //   let mut cur_idx = (1 << self.height) - 1;
  //   for i in (0 .. self.height).rev() {
  //     let step = 1 << i;
  //     if self.tree[cur_idx].right == true { cur_idx += step }
  //     else if self.tree[cur_idx].left == true { cur_idx -= step }
  //     else { return None }
  //   }
  //   let result = cur_idx + self.tree[cur_idx].left as usize;
  //   (result < self.size).then_some(result)
  // }
  
  pub fn set_leaf(&mut self, idx: usize, full: bool) -> Option<()> {
    if idx >= self.size { return None }
    self.set(idx, full, 0)
  }

  pub fn get_leaf(&self, idx: usize) -> Option<bool> {
    if idx >= self.size { return None }
    self.get(idx, 0)
  }

  fn set(&mut self, path: usize, full: bool, steps_from_leaf: u8) -> Option<()> {
    let mut cur_idx = (path & (!0 << 1)) << steps_from_leaf; // The last bit is left vs right, the leaf's parent node is at the even index
    if cur_idx > self.capacity { return None } // Prevent out of bound indexing
    if path & 1 == 0 { self.tree[cur_idx].left = full; } else { self.tree[cur_idx].right = full; }
    let mut combined = self.tree[cur_idx].left & self.tree[cur_idx].right;
    for i in steps_from_leaf .. self.height {
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
    Some(())
  }

  fn get(&self, path: usize, steps_from_leaf: u8) -> Option<bool> {
    let cur_idx = (path & (!0 << 1)) << steps_from_leaf;
    if cur_idx > self.capacity { return None } // Prevent out of bound indexing
    Some( if path & 1 == 0 { self.tree[cur_idx].left } else { self.tree[cur_idx].right } )
  }
}

#[test]
fn test_tree() {
  let mut tree = FullFlatBinaryTree::new();
  tree.resize(8);
  assert_eq!(tree.get_first_empty_leaf().unwrap(), 0);
  for i in 0 .. tree.size { tree.set_leaf(i, true); }
  assert_eq!(tree.get_first_empty_leaf(), None);
  tree.set_leaf(7, false);
  assert_eq!(tree.get_first_empty_leaf().unwrap(), 7);
  // Can't set out of bounds
  assert_eq!(tree.set_leaf(8, false), None);

  tree.resize(3);
  assert_eq!(tree.set_leaf(4, false), None);
  assert_eq!(tree.set_leaf(1, false), Some(()));
  assert_eq!(tree.get_first_empty_leaf(), Some(1));
}

// Write a test for resize logic


