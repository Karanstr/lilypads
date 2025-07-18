use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct Bitmap { 
  layers: [Vec<u64>; 3],
  size: usize, // Artificial API limit for bound checking
}
impl Bitmap {

  pub fn new() -> Self { Self { 
    layers: [Vec::new(), Vec::new(), Vec::new()],
    size: 0,
  } }

  pub fn resize(&mut self, mut size: usize) {
    self.size = size;
    for i in 0 .. 3 {
      let last_bit = size & 63;
      size += 63;
      size >>= 6;
      self.layers[i].resize(size, 0);
      if self.layers[i].len() == 0 || last_bit != 0 { continue }
      let last_bocks = self.layers[i].len() - 1;
      self.layers[i][last_bocks] &= (1 << last_bit) - 1;
    }
  }

  pub fn find_first_free(&self) -> Option<usize> {
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
    (idx < self.size).then_some(idx)
  }

  pub fn set(&mut self, mut idx: usize, mut full:bool) -> Option<()> {
    if idx >= self.size { return None }
    for layer in &mut self.layers {
      let offset = idx as u64 & 63;
      idx >>= 6;
      let bit = 1 << offset;
      if full { layer[idx] |= bit } else { layer[idx] &= !bit }
      full = layer[idx] == u64::MAX;
    }
    Some(())
  }

  pub fn is_full(&self, idx: usize) -> Option<bool> {
    if idx >= self.size { return None }
    let offset = idx & 63;
    Some(0 != (self.layers[0][idx >> 6] & (1 << offset)))
  }

}

#[test]
fn write() {
  let mut tree = Bitmap::new();
  tree.resize(7);

  tree.set(0, true);
  // Does setting work correctly
  tree.set(1, true);
  assert_eq!(tree.is_full(1).unwrap(), true);

  // Make sure setting and unsetting work
  tree.set(2, true);
  assert_eq!(tree.is_full(2).unwrap(), true);
  tree.set(2, false);
  assert_eq!(tree.is_full(2).unwrap(), false);

  assert_eq!(tree.find_first_free(), Some(2));

  assert_eq!(tree.is_full(7), None);
}
