use lilypads::Pond;

fn rand_int(seed: u64) -> u64 {
  seed.wrapping_mul(6364136223846793005).wrapping_add(1)
}

#[test]
/// Tests that we can insert values and retrieve them
fn get_and_retrieve() {
  let mut pool = Pond::new();
  let idx1 = pool.insert(42);
  let idx2 = pool.insert(123);

  assert_eq!(*pool.get(idx1).unwrap(), 42);
  assert_eq!(*pool.get(idx2).unwrap(), 123);
}

#[test]
/// Ensures get works on an empty pond
fn empty_get() {
  let pool: Pond<u32> = Pond::new();
  assert_eq!(pool.get(0), None)
}

#[test]
fn mut_get() {
  let mut pool = Pond::new();
  let idx = pool.insert(42);
  let data2 = pool.get_mut(idx).unwrap();
  let new_val = 13;
  *data2 = new_val;
  assert_eq!(*pool.get(idx).unwrap(), new_val);
}

#[test] 
fn free() {
  let mut pool = Pond::new();
  let idx = pool.insert(42);
  // Was data set?
  assert_eq!(*pool.get(idx).unwrap(), 42);

  // Can data be freed?
  assert_eq!(pool.free(idx), Some(42));
  assert_eq!(pool.get(idx), None);

  // Check double freeing
  assert_eq!(pool.free(idx), None);
}

#[test]
fn write() {
  let mut pool = Pond::new();
  let idx = pool.insert(42);

  let old = pool.write(idx, 155).unwrap();
  // Verify old data was returned and new data is in place
  assert_eq!(old, 42);
  assert_eq!(*pool.get(idx).unwrap(), 155);

  let idx2 = 13;
  pool.write(idx2, 29);
  // Ensure the data was saved at the correct idx
  assert_eq!(*pool.get(idx2).unwrap(), 29);

  // Ensure middle slots remain empty
  let empty_idx = 5;
  assert_eq!(pool.get(empty_idx), None);
}

#[test]
fn resize_bigger() {
  let mut pool = Pond::new();
  let idx1 = pool.insert(42);
  let idx2 = pool.insert(12);
  // Resize via insert
  assert_eq!(pool.len(), 2);
  
  pool.resize(100);
  // Operation successful?
  assert_eq!(pool.len(), 100);

  // Ensure data survived
  assert_eq!(*pool.get(idx1).unwrap(), 42);
  assert_eq!(*pool.get(idx2).unwrap(), 12);
}

#[test]
fn resize_smaller() {
  let mut pool = Pond::new();
  pool.resize(100);
  pool.write(99, 42);
  pool.write(25, 13);
  pool.write(24, 63);

  pool.resize(25);

  // Resize successful?
  assert_eq!(pool.len(), 25);
  assert_eq!(pool.get(25), None);
  assert_eq!(pool.get(24), Some(&63));
}

#[test]
fn memory_reuse() {
  let mut pool = Pond::new();
  let idx1 = pool.insert(1);
  let idx2 = pool.insert(2);
  pool.free(idx1);
  let idx3 = pool.insert(3);

  // Verify slot reuse
  assert_eq!(idx1, idx3);
  // Verify data
  assert_eq!(*pool.get(idx2).unwrap(), 2);
  assert_eq!(*pool.get(idx3).unwrap(), 3);
}

#[test]
fn defrag() {
  let mut pool = Pond::new();
  let mut indices: Vec<usize> = (0..5).map(|i| pool.insert(i) ).collect();
  // Remove some items to create gaps
  pool.free(indices[1]).unwrap();
  pool.free(indices[3]).unwrap();

  // Defrag and verify remapping
  let remapped = pool.defrag();
  for (old, new) in remapped.iter() { indices[*old] = *new }

  // Verify data is preserved and contiguous
  assert_eq!(*pool.get(indices[0]).unwrap(), 0);
  assert_eq!(*pool.get(indices[2]).unwrap(), 2);
  assert_eq!(*pool.get(indices[4]).unwrap(), 4);
  assert_eq!(pool.next_index(), 3);
}

#[test]
fn trim_normal() {
  let mut pool = Pond::new();
  let mut indices: Vec<usize> = (0..5).map(|i| pool.insert(i)).collect();

  // Remove last two items
  pool.free(indices[3]).unwrap();
  pool.free(indices[4]).unwrap();

  // Trim and verify
  let remapped = pool.trim();
  for (old, new) in remapped.iter() { indices[*old] = *new }

  // Verify new length
  assert_eq!(pool.len(), 3);

  // Verify memory state after trim
  assert!(matches!(pool.get(2), Some(_)));
  assert!(matches!(pool.get(3), None));

  // Verify inserter state after trim
  assert_eq!(pool.next_index(), 3);

  // Verify remaining data
  assert_eq!(*pool.get(indices[0]).unwrap(), 0);
  assert_eq!(*pool.get(indices[1]).unwrap(), 1);
  assert_eq!(*pool.get(indices[2]).unwrap(), 2);
}

#[test]
fn trim_empty() {
  let mut pool = Pond::<i32>::new();
  let _ = pool.trim();

  // Verify new length
  assert_eq!(pool.len(), 0);

  // Verify inserter state after trim
  assert_eq!(pool.next_index(), 0);
}

#[test]
fn trim_free() {
  let mut pool = Pond::<i32>::new();
  pool.resize(12);
  let _ = pool.trim();
  
  // Verify new length
  assert_eq!(pool.len(), 0);

  // Verify inserter state after trim
  assert_eq!(pool.next_index(), 0);
}

#[test]
fn extended_use() {
    let mut pool = Pond::new();
    let mut indices = Vec::new();
    let mut seed = 12345;

    for i in 0..100_000 {
        seed = rand_int(seed);
        if seed >> 60 == 1 && !indices.is_empty() {
            let idx = indices.pop().unwrap();
            pool.free(idx);
        } else {
            let idx = pool.insert(i);
            indices.push(idx);
        }
    }
    // Verify all remaining indices are valid
    for idx in indices { assert!(pool.get(idx).is_some()); }
}
