use std::num::NonZero;
use serde::{Deserialize, Serialize};

type Word = u64;
const WORD_BITS: usize = Word::BITS as usize;
/// When accessing the base layer, idx & BASE_MASK returns the desired bit idx
const WORD_MASK: usize = WORD_BITS - 1;
/// When accessing the base layer, idx >> BASE_SHIFT returns the desired word idx
const WORD_SHIFT: usize = WORD_MASK.trailing_ones() as usize;

#[derive(Deserialize, Serialize, Debug)]
pub struct BitMap(Vec<Word>);
// Consider adding lower level functions to reduce ops
// ex. write_exact(self, idx, offset, value)
impl BitMap {
  fn new() -> Self { Self(Vec::new()) }
  
  pub fn resize(&mut self, size: usize, flip_default: bool) {
    let full_word_count = size >> WORD_SHIFT;
    // All full words + the extra partial word
    let default_val = if flip_default { Word::MAX } else { 0 };
    self.0.resize(full_word_count + 1, default_val);
    if let Some(word) = self.0.last_mut() {
      let partial_word_size = size & WORD_MASK;
      *word &= !(Word::MAX << partial_word_size);
    }
  }
  
  /// Panics on out of bounds write
  pub fn write(&mut self, idx: usize, value: bool) {
    let offset = idx & WORD_MASK;
    let idx = idx >> WORD_SHIFT;
    let bit = 1 << offset;
    if value { self.0[idx] |= bit } else { self.0[idx] &= !bit }
  }

  /// Panics on out of bounds read
  pub fn read(&self, idx: usize) -> bool {
    let offset = idx & WORD_MASK;
    let idx = idx >> WORD_SHIFT;
    let bit = 1 << offset;
    self.0[idx] & bit != 0
  }

  pub fn get_word(&self, idx: usize) -> Word {
    self.0[idx >> WORD_SHIFT]
  }
}

fn first_set(structure: &Vec<BitMap>) -> Option<usize> {
    let top_layer = structure.last()?;
    let mut idx = top_layer.0.iter().position(|word| *word != 0)?;

    for layer in structure.iter().rev() {
      let word = layer.0[idx];
      let bit_pos = word.trailing_zeros() as usize;
      idx = (idx << WORD_SHIFT) + bit_pos;
    }

    Some(idx)
}

fn last_set(structure: &Vec<BitMap>) -> Option<usize> {
    let top_layer = structure.last()?;
    let mut idx = top_layer.0.iter().rposition(|word| *word != 0)?;
    
    for layer in structure.iter().rev() {
      let word = layer.0[idx];
      let bit_pos = WORD_MASK - word.leading_zeros() as usize;
      idx = (idx << WORD_SHIFT) + bit_pos;
    }

    Some(idx)
}

#[derive(Deserialize, Serialize, Debug)]
pub struct AcceleratedBitmap {
  base: BitMap,
  set_layers: Vec<BitMap>,
  unset_layers: Vec<BitMap>,
}
impl AcceleratedBitmap {

  pub fn new(layers: NonZero<usize>) -> Self {
    let mut set_layers = Vec::new();
    set_layers.resize_with(layers.get(), BitMap::new);

    let mut unset_layers = Vec::new();
    unset_layers.resize_with(layers.get(), BitMap::new);
    
    Self { 
      base: BitMap::new(),
      set_layers,
      unset_layers
    }
  }

  pub fn resize(&mut self, mut size: usize) {
    self.base.resize(size, false);
    for (set_layer, unset_layer) in &mut self.set_layers.iter_mut().zip(self.unset_layers.iter_mut()) {
      size >>= WORD_SHIFT;
      set_layer.resize(size.max(1), false);
      unset_layer.resize(size.max(1), true);
    }
  }

  /// Returns the first set index, or none if the bitmap is all unset
  #[allow(unused)]
  pub fn first_set(&self) -> Option<usize> {
    let idx = first_set(&self.set_layers)?;
    let bit_pos = self.base.0[idx].trailing_zeros() as usize;
    Some((idx << WORD_SHIFT) + bit_pos)
  }

  /// Returns the first set index, or none if the bitmap is all set
  pub fn first_unset(&self) -> Option<usize> {
    let idx = first_set(&self.unset_layers)?;
    let bit_pos = self.base.0[idx].trailing_ones() as usize;
    Some((idx << WORD_SHIFT) + bit_pos)
  }

  /// Returns the last set index, or none if the bitmap is all unset
  pub fn last_set(&self) -> Option<usize> {
    let idx = last_set(&self.set_layers)?;
    let bit_pos = WORD_MASK - self.base.0[idx].leading_zeros() as usize;
    Some((idx << WORD_SHIFT) + bit_pos)
  }

  /// Returns the last set index, or none if the bitmap is all set
  #[allow(unused)]
  pub fn last_unset(&self) -> Option<usize> {
    let idx = last_set(&self.unset_layers)?;
    let bit_pos = WORD_MASK - self.base.0[idx].leading_ones() as usize;
    Some((idx << WORD_SHIFT) + bit_pos)
  }


  /// Panics if out of bound attempt
  pub fn set(&mut self, mut idx: usize, value: bool) {
    self.base.write(idx, value);
    let mut has_set = self.base.get_word(idx) != 0;
    let mut has_unset = self.base.get_word(idx) != Word::MAX;

    for (set_layer, unset_layer) in self.set_layers.iter_mut().zip(self.unset_layers.iter_mut()) {
      idx >>= WORD_SHIFT;
      set_layer.write(idx, has_set);
      unset_layer.write(idx, has_unset);
      has_set = set_layer.get_word(idx) != 0;
      has_unset = unset_layer.get_word(idx) != 0;
    }
  }

  /// Panics if out of bound attempt
  pub fn is_set(&self, idx: usize) -> bool { self.base.read(idx) }

}


// Written by AI
#[cfg(test)]
mod bitmap_tests {
    use super::*;

    #[test]
    fn resize_zeroed() {
        let mut bm = BitMap::new();
        bm.resize(100, false);

        for i in 0..100 {
            assert_eq!(bm.read(i), false);
        }
    }

    #[test]
    fn resize_flipped() {
        let mut bm = BitMap::new();
        bm.resize(100, true);

        for i in 0..100 {
            assert_eq!(bm.read(i), true);
        }
    }

    #[test]
    fn write_and_read() {
        let mut bm = BitMap::new();
        bm.resize(64, false);

        bm.write(10, true);
        bm.write(20, true);

        assert!(bm.read(10));
        assert!(bm.read(20));
        assert!(!bm.read(0));
    }

    #[test]
    fn overwrite_bit() {
        let mut bm = BitMap::new();
        bm.resize(32, false);

        bm.write(5, true);
        assert!(bm.read(5));

        bm.write(5, false);
        assert!(!bm.read(5));
    }

    #[test]
    fn word_access() {
        let mut bm = BitMap::new();
        bm.resize(64, false);

        bm.write(1, true);
        bm.write(63, true);

        let word = bm.get_word(0);
        assert!(word & (1 << 1) != 0);
        assert!(word & (1 << 63) != 0);
    }

    #[test]
    fn partial_word_masking() {
        let mut bm = BitMap::new();
        bm.resize(35, false);

        // Ensure bits beyond size are zeroed
        let last_word = bm.0.last().unwrap();
        let valid_bits = 35 & WORD_MASK;

        let mask = if valid_bits == 0 {
            0
        } else {
            (1 << valid_bits) - 1
        };

        assert_eq!(*last_word & !mask, 0);
    }
}
#[cfg(test)]
mod structure_tests {
    use super::*;

    fn make_layer(words: &[Word]) -> BitMap {
        BitMap(words.to_vec())
    }

    #[test]
    fn first_set_basic() {
        let layers = vec![
            make_layer(&[0b0010]), // base
            make_layer(&[0b0001]), // summary
        ];

        let idx = first_set(&layers).unwrap();
        assert_eq!(idx, 1);
    }

    #[test]
    fn last_set_basic() {
        let layers = vec![
            make_layer(&[0b1000]),
            make_layer(&[0b0001]),
        ];

        let idx = last_set(&layers).unwrap();
        assert_eq!(idx, 3);
    }

    #[test]
    fn first_set_none() {
        let layers = vec![
            make_layer(&[0]),
            make_layer(&[0]),
        ];

        assert!(first_set(&layers).is_none());
    }

    #[test]
    fn last_set_none() {
        let layers = vec![
            make_layer(&[0]),
            make_layer(&[0]),
        ];

        assert!(last_set(&layers).is_none());
    }
}
#[cfg(test)]
mod accel_tests {
    use super::*;
    use std::num::NonZero;

    fn new_bitmap() -> AcceleratedBitmap {
        let mut bm = AcceleratedBitmap::new(NonZero::new(2).unwrap());
        bm.resize(128);
        bm
    }

    #[test]
    fn basic_set_and_get() {
        let mut bm = new_bitmap();

        bm.set(10, true);
        bm.set(20, true);

        assert!(bm.is_set(10));
        assert!(bm.is_set(20));
        assert!(!bm.is_set(0));
    }

    #[test]
    fn first_set_simple() {
        let mut bm = new_bitmap();

        bm.set(15, true);
        bm.set(30, true);

        assert_eq!(bm.first_set(), Some(15));
    }

    #[test]
    fn first_unset_simple() {
        let mut bm = new_bitmap();

        // everything starts unset
        assert_eq!(bm.first_unset(), Some(0));

        bm.set(0, true);
        assert_eq!(bm.first_unset(), Some(1));
    }

    #[test]
    fn last_set_simple() {
        let mut bm = new_bitmap();

        bm.set(5, true);
        bm.set(100, true);

        assert_eq!(bm.last_set(), Some(100));
    }

    #[test]
    fn last_unset_simple() {
        let mut bm = new_bitmap();

        // all unset → last unset is last index
        assert_eq!(bm.last_unset(), Some(127));

        bm.set(127, true);
        assert_eq!(bm.last_unset(), Some(126));
    }

    #[test]
    fn set_toggle() {
        let mut bm = new_bitmap();

        bm.set(42, true);
        assert!(bm.is_set(42));

        bm.set(42, false);
        assert!(!bm.is_set(42));
    }

    #[test]
    fn dense_pattern() {
        let mut bm = new_bitmap();

        for i in 0..128 {
            if i % 2 == 0 {
                bm.set(i, true);
            }
        }

        assert_eq!(bm.first_set(), Some(0));
        assert_eq!(bm.first_unset(), Some(1));
        assert_eq!(bm.last_set(), Some(126));
        assert_eq!(bm.last_unset(), Some(127));
    }

    #[test]
    fn stress_sparse() {
        let mut bm = new_bitmap();

        for i in (0..128).step_by(17) {
            bm.set(i, true);
        }

        assert_eq!(bm.first_set(), Some(0));
        assert!(bm.last_set().unwrap() >= 102);
    }
}
