use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct Node {
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
  tree: Vec<Node>,
  height: u8,
}
impl FullFlatBinaryTree {
  pub fn new(height: u8) -> Self {
    Self {
      tree: vec![Node::new(); (1 << (height + 1)) - 1],
      height
    }
  }

  pub fn height(&self) -> u8 { self.height }

  pub fn set_height(&mut self, new_height: u8) {
    self.height = new_height;
    self.tree.resize((1 << (new_height + 1)) - 1, Node::new());
  }
  
  pub fn get_first_empty_leaf(&self) -> Option<usize> {
    let mut cur_idx = (1 << self.height) - 1;
    for i in (0 .. self.height).rev() {
      let step = 1 << i;
      if self.tree[cur_idx].left == false { cur_idx -= step }
      else if self.tree[cur_idx].right == false { cur_idx += step }
      else { return None }
    }
    Some(cur_idx + self.tree[cur_idx].left as usize)
  }
  
  pub fn set_leaf(&mut self, idx: usize, full: bool) {
    let mut cur_idx = idx & (!0 << 1); // The last bit is left vs right, the leaf's parent node is at the even index
    if idx & 1 == 0 { self.tree[cur_idx].left = full; } else { self.tree[cur_idx].right = full; }
    let mut combined = self.tree[cur_idx].left & self.tree[cur_idx].right;
    for i in 0 .. self.height {
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
  }
}

#[test]
fn test_tree() {
  let mut tree = FullFlatBinaryTree::new(3);
  assert_eq!(tree.get_first_empty_leaf().unwrap(), 0);
  for i in 0 ..= tree.tree.len() { tree.set_leaf(i, true); }
  assert_eq!(tree.get_first_empty_leaf(), None);
  tree.set_leaf(7, false);
  assert_eq!(tree.get_first_empty_leaf().unwrap(), 7);
}

