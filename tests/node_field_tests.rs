use lilypads::Pond;

#[test]
fn insert() {
  let mut pool = Pond::new();
  let idx1 = pool.insert(42);
  let idx2 = pool.insert(123);

  assert_eq!(*pool.get(idx1).unwrap(), 42);
  assert_eq!(*pool.get(idx2).unwrap(), 123);
}

#[test]
fn get() {
  let mut pool = Pond::new();
  let idx = pool.insert(42);
  // Ensure we can access reserved data and can't access free slots
  assert_eq!(*pool.get(idx).unwrap(), 42);
  assert_eq!(pool.get(idx + 1), None);
}

#[test]
fn mut_get() {
  let mut pool = Pond::new();
  let idx = pool.insert(42);
  let data2 = pool.get_mut(idx).unwrap();
  *data2 = 13;
  assert_eq!(*pool.get(idx).unwrap(), 13);
}

#[test] 
fn free() {
  let mut pool = Pond::new();
  let idx = pool.insert(42);
  // Was data set?
  assert_eq!(*pool.get(idx).unwrap(), 42);
  pool.free(idx);
  // Was data unset?
  assert_eq!(pool.get(idx), None);
  // Ensure we can't double free
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
  // Ensure the vec was properly resize and the data was marked as reserved
  assert_eq!(*pool.get(idx2).unwrap(), 29);
}

#[test]
fn memory_reuse() {
  let mut pool = Pond::new();
  let idx1 = pool.insert(1);
  let idx2 = pool.insert(2);
  pool.free(idx1);
  let idx3 = pool.insert(3);

  // Verify reuse
  assert_eq!(idx1, idx3);
  // Verify data
  assert_eq!(*pool.get(idx2).unwrap(), 2);
  assert_eq!(*pool.get(idx3).unwrap(), 3);
}

#[test]
fn defrag() {
  let mut pool = Pond::new();
  let mut indices: Vec<_> = (0..5).map(|i| pool.insert(i) ).collect();
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
  let mut indices: Vec<_> = (0..5).map(|i| pool.insert(i)).collect();

  // Remove last two items
  pool.free(indices[3]).unwrap();
  pool.free(indices[4]).unwrap();

  // Trim and verify
  let remapped = pool.trim();
  for (old, new) in remapped.iter() { indices[*old] = *new }

  // Verify memory state after trim
  assert!(matches!(pool.get(2), Some(_)));
  assert!(matches!(pool.get(3), None));

  // Verify insertator state after trim
  assert_eq!(pool.next_index(), 3);

  // Verify remaining data
  assert_eq!(*pool.get(indices[0]).unwrap(), 0);
  assert_eq!(*pool.get(indices[1]).unwrap(), 1);
  assert_eq!(*pool.get(indices[2]).unwrap(), 2);
}

#[test]
fn trim_all_free() {
  let mut pool = Pond::new();

  let idx1 = pool.insert(1);
  let idx2 = pool.insert(2);

  //Set all slots to free
  pool.free(idx1).unwrap();
  pool.free(idx2).unwrap();

  _ = pool.trim();

  // Verify memory state
  assert_eq!(pool.get(0), None);

  // Verify insertator state after trim
  assert_eq!(pool.next_index(), 0);
}

#[test]
fn trim_empty() {
  let mut pool = Pond::<i32>::new();
  _ = pool.trim();

  // Verify memory state
  assert_eq!(pool.get(0), None);

  // Verify insertator state after trim
  assert_eq!(pool.next_index(), 0);
}

#[test]
fn trim_free() {
  let mut pool = Pond::<i32>::new();
  pool.resize(16);
  _ = pool.trim();
  
  // Verify memory state
  assert_eq!(pool.get(0), None);

  // Verify insertator state after trim
  assert_eq!(pool.next_index(), 0);
}

#[test]
fn stress() {
  const N: u32 = 1_000_000_0;
  let mut pool = Pond::new();
  pool.resize(N as usize);

  // Push a bunch of values into the insertator
  for i in 0..N { let _ = pool.insert(i); }
}

#[test]
fn bitmap_resize_boundary() {
  let mut pool = Pond::new();
  pool.resize(63);
  pool.write(62, 5);
  pool.resize(64);
  assert_eq!(*pool.get(62).unwrap(), 5);
}

