use serde::{Deserialize, Serialize};

// Given an index, the first 6 bits represents the offset in self.base[box] to find the packed bit.
// This is because it takes 6 bits to represent any bit of the u64 (0 - 63)
// All following layers use 5 bits, representing the 32 2bit slots
// We pack 2 bits per index in acceleration layers to represent the has_full and has_empty states
// Each slot is packed right to left, so data & 1 is the first slot
// Yes I know this is annoying, reading backwards when we step through the vecs forwards.. shut up.

const BASE_MASK: usize = 0b111111; // 63
const BASE_SHIFT: usize = 6;
const ACCEL_MASK: usize = 0b11111; // 31
const ACCEL_SHIFT: usize = 5;
const UNSET_FULL: u64 = !0 << 32; // SECOND 32 BITS
const SET_FULL: u64 = !0 >> 32; // FIRST 32 BITS

// First 32 bits of accel_layers are full_tracking, second 32 are empty_tracking
#[derive(Deserialize, Serialize, Debug)]
pub struct AcceleratedBitmap {
  base: Vec<u64>,
  accel_layers: Vec< Vec<u64> >,
}
impl AcceleratedBitmap {

  pub fn new(layers: usize) -> Self {
    let mut accel_layers = Vec::new();
    accel_layers.resize_with(layers, Vec::new);

    Self { 
      base: Vec::new(),
      accel_layers,
    }
  }

  pub fn resize(&mut self, size: usize) {
    let offset = size & BASE_MASK;
    let mut full_word_count = size >> BASE_SHIFT;
    self.base.resize(full_word_count + 1, 0);
    // This line zeros any leftovers after the requested size
    // It generates a bitstring of 1s via not,
    // creates 0s in the front via shift,
    // and inverts the string via not
    self.base[full_word_count] &= !(!0 << offset);
    for layer in &mut self.accel_layers {
        let offset = full_word_count & ACCEL_MASK;
        full_word_count >>= ACCEL_SHIFT;
        layer.resize(full_word_count + 1, 0);
        let set_mask = SET_FULL >> (32 - offset);
        let unset_mask = UNSET_FULL >> (32 - offset) & UNSET_FULL;
        layer[full_word_count] &= unset_mask | set_mask;
    }
  }


  /// Returns the first free index, or none if the bitmap is saturated
  // Warning, this rewrite was written with significant help from AI,
  // I'm fairly certain it functions correctly, but you never know
  pub fn first_free(&self) -> Option<usize> {
    if self.base.len() == 0 { return None }
    let mut idx = 0;
    for layer in self.accel_layers.iter().rev() {
      let word = layer[idx];
      let free_bits = !((word as u32) & 0xFFFF_FFFF); // lower 32 bits track empties
      if free_bits == 0 { return None }
      let bit_pos = free_bits.trailing_zeros() as usize;
      idx = (idx << ACCEL_SHIFT) + bit_pos;
    }

    // Finally scan the base layer
    let word = self.base[idx];
    let free_bits = !((word as u32) & 0xFFFF_FFFF);
    if free_bits == 0 { return None }
    let bit_pos = free_bits.trailing_zeros() as usize;

    Some((idx << BASE_SHIFT) + bit_pos)
  }

  /// Panics if out of bound attempt
  pub fn set(&mut self, mut idx: usize, value: bool) {
    let offset = idx & BASE_MASK;
    let bit = 1 << offset;
    idx >>= BASE_SHIFT;
    if value { self.base[idx] |= bit } else { self.base[idx] &= !bit }
    
    let mut is_full = self.base[idx] == u64::MAX;
    let mut is_empty = self.base[idx] == 0;

    for layer in &mut self.accel_layers {
      let offset = idx & ACCEL_MASK;
      let bit = 1 << offset;
      idx >>= ACCEL_SHIFT;
      if is_full { layer[idx] |= bit } else { layer[idx] &= !bit }
      if is_empty { layer[idx] |= bit << 32 } else { layer[idx] &= !(bit << 32) }
      is_full = layer[idx] & SET_FULL == SET_FULL;
      is_empty = layer[idx] & UNSET_FULL == UNSET_FULL;
    }
  }

  // Panics if out of bound attempt
  pub fn is_set(&self, idx: usize) -> bool {
    let offset = idx & BASE_MASK;
    0 != (self.base[idx >> BASE_SHIFT] & (1 << offset))
  }

}


#[cfg(test)]
mod tests {
  use super::AcceleratedBitmap;
  #[test]
  fn write() {
    let mut tree = AcceleratedBitmap::new(2);
    tree.resize(7);

    tree.set(0, true);
    // Does setting work correctly
    tree.set(1, true);
    assert_eq!(tree.is_set(1), true);

    // Make sure setting and unsetting work
    tree.set(2, true);
    assert_eq!(tree.is_set(2), true);
    tree.set(2, false);
    assert_eq!(tree.is_set(2), false);
    
    tree.set(1, false);

    assert_eq!(tree.first_free(), Some(1));
  }


  #[test]
  fn resize_boundary() {
    let mut tree = AcceleratedBitmap::new(2);
    tree.resize(63);
    tree.set(62, true);
    tree.resize(64);
    assert_eq!(tree.is_set(62), true);
  }

}
