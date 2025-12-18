#[allow(missing_docs)]

use crate::bitmap::AcceleratedBitmap;

pub trait DynamicStruct {
  /// Should be an enum
  type Fields;

  fn update(&mut self, data: &mut [Option<Self::Fields>] ) {
    for field in data {
      if let Some(owned) = field.take() {
        self.update_field(owned);
      }
    }
  }

  fn update_field(&mut self, field: Self::Fields);
}

pub struct PondSoa<T: DynamicStruct> {
  bitmap: AcceleratedBitmap,
  soa: T,
  len: usize
}

impl<T> PondSoa<T> where T: DynamicStruct {

  /// THIS FUNCTION DOESN'T BOUND CHECK
  fn mark_free(&mut self, idx:usize) { self.bitmap.set(idx, false) }

  /// THIS FUNCTION DOESN'T BOUND CHECK
  fn mark_reserved(&mut self, idx:usize) { self.bitmap.set(idx, true); }

  #[must_use]
  fn reserve(&mut self) -> usize {
    let idx = self.bitmap.first_free().unwrap_or(self.len);
    if idx >= self.len { self.resize(idx + 1) }
    self.mark_reserved(idx);
    idx
  }

}
impl<T> PondSoa<T> where T: DynamicStruct {

  pub fn resize(&mut self, size: usize) {

  }

}
