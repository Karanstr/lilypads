use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct Bitmap { layers: [Vec<u64>; 3] }
impl Bitmap {

  pub fn new() -> Self { Self { layers: [Vec::new(), Vec::new(), Vec::new()], } }

  pub fn resize(&mut self, mut size: usize) {
    for i in 0 .. 3 {
      let last_bit = size & 63;
      size >>= 6;
      self.layers[i].resize(size + 1, 0);
      let last_bocks = self.layers[i].len() - 1;
      self.layers[i][last_bocks] &= !(!0 << last_bit);
    }
  }

  pub fn first_free(&self) -> Option<usize> {
    let mut layer_down = None;
    for (val, bocks) in self.layers[2].iter().enumerate() {
      if *bocks != u64::MAX {
        layer_down = Some((val << 6) + bocks.trailing_ones() as usize);
        break;
      }
    }
    let mut idx = layer_down?;
    for layer in (0 .. 2).rev() {
      let offset = self.layers[layer][idx].trailing_ones() as usize;
      idx = (idx << 6) + offset;
    }
    Some(idx)
  }

  // Panics if out of bound attempt
  pub fn set(&mut self, mut idx: usize, mut full:bool) {
    for layer in &mut self.layers {
      let offset = idx & 63;
      let bit = 1 << offset;
      idx >>= 6;
      if full { layer[idx] |= bit } else { layer[idx] &= !bit }
      full = layer[idx] == u64::MAX;
    }
  }

  // Panics if out of bound attempt
  pub fn is_full(&self, idx: usize) -> bool {
    let offset = idx & 63;
    0 != (self.layers[0][idx >> 6] & (1 << offset))
  }

}

#[test]
fn write() {
  let mut tree = Bitmap::new();
  tree.resize(7);

  tree.set(0, true);
  // Does setting work correctly
  tree.set(1, true);
  assert_eq!(tree.is_full(1), true);

  // Make sure setting and unsetting work
  tree.set(2, true);
  assert_eq!(tree.is_full(2), true);
  tree.set(2, false);
  assert_eq!(tree.is_full(2), false);

  assert_eq!(tree.first_free(), Some(2));

}
