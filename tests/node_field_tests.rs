use vec_mem_heap::NodePool;

#[test]
fn alloc() {
  let mut storage = NodePool::new();
  let idx1 = storage.alloc(42);
  let idx2 = storage.alloc(123);

  assert_eq!(*storage.get(idx1).unwrap(), 42);
  assert_eq!(*storage.get(idx2).unwrap(), 123);
}

#[test]
fn get() {
  let mut storage = NodePool::new();
  let idx = storage.alloc(42);
  // Ensure we can access reserved data and can't access free slots
  assert_eq!(*storage.get(idx).unwrap(), 42);
  assert_eq!(storage.get(idx + 1), None);
}

#[test] 
fn free() {
  let mut storage = NodePool::new();
  let idx = storage.alloc(42);
  // Was data set?
  assert_eq!(*storage.get(idx).unwrap(), 42);
  storage.free(idx);
  // Was data unset?
  assert_eq!(storage.get(idx), None);
  // Ensure we can't double free
  assert_eq!(storage.free(idx), None);
}

#[test]
fn write() {
  let mut storage = NodePool::new();
  let idx = storage.alloc(42);

  let old = storage.write(idx, 155).unwrap();
  // Verify old data was returned and new data is in place
  assert_eq!(old, 42);
  assert_eq!(*storage.get(idx).unwrap(), 155);

  let idx2 = 13;
  storage.write(idx2, 29);
  // Ensure the vec was properly resize and the data was marked as reserved
  assert_eq!(*storage.get(idx2).unwrap(), 29);
}

#[test]
fn memory_reuse() {
  let mut storage = NodePool::new();
  let idx1 = storage.alloc(1);
  let idx2 = storage.alloc(2);
  storage.free(idx1);
  let idx3 = storage.alloc(3);

  // Verify reuse
  assert_eq!(idx1, idx3);
  // Verify data
  assert_eq!(*storage.get(idx2).unwrap(), 2);
  assert_eq!(*storage.get(idx3).unwrap(), 3);
}

#[test]
fn defrag() {
  let mut storage = NodePool::new();
  let mut indices: Vec<_> = (0..5).map(|i| storage.alloc(i) ).collect();
  // Remove some items to create gaps
  storage.free(indices[1]).unwrap();
  storage.free(indices[3]).unwrap();

  // Defrag and verify remapping
  let remapped = storage.defrag();
  for (old, new) in remapped.iter() { indices[*old] = *new }

  // Verify data is preserved and contiguous
  assert_eq!(*storage.get(indices[0]).unwrap(), 0);
  assert_eq!(*storage.get(indices[2]).unwrap(), 2);
  assert_eq!(*storage.get(indices[4]).unwrap(), 4);
  assert_eq!(storage.next_allocated(), 3);
}

#[test]
fn trim_normal() {
  let mut storage = NodePool::new();
  let mut indices: Vec<_> = (0..5).map(|i| storage.alloc(i)).collect();

  // Remove last two items
  storage.free(indices[3]).unwrap();
  storage.free(indices[4]).unwrap();

  // Trim and verify
  let remapped = storage.trim();
  for (old, new) in remapped.iter() { indices[*old] = *new }

  // Verify memory state after trim
  assert!(matches!(storage.get(2), Some(_)));
  assert!(matches!(storage.get(3), None));

  // Verify allocator state after trim
  assert_eq!(storage.next_allocated(), 3);

  // Verify remaining data
  assert_eq!(*storage.get(indices[0]).unwrap(), 0);
  assert_eq!(*storage.get(indices[1]).unwrap(), 1);
  assert_eq!(*storage.get(indices[2]).unwrap(), 2);
}

#[test]
fn trim_all_free() {
  let mut storage = NodePool::new();

  let idx1 = storage.alloc(1);
  let idx2 = storage.alloc(2);

  //Set all slots to free
  storage.free(idx1).unwrap();
  storage.free(idx2).unwrap();

  _ = storage.trim();

  // Verify memory state
  assert_eq!(storage.get(0), None);

  // Verify allocator state after trim
  assert_eq!(storage.next_allocated(), 0);
}

#[test]
fn trim_empty() {
  let mut storage = NodePool::<i32>::new();
  _ = storage.trim();

  // Verify memory state
  assert_eq!(storage.get(0), None);

  // Verify allocator state after trim
  assert_eq!(storage.next_allocated(), 0);
}

#[test]
fn trim_free() {
  let mut storage = NodePool::<i32>::new();
  storage.resize(16);
  _ = storage.trim();
  
  // Verify memory state
  assert_eq!(storage.get(0), None);

  // Verify allocator state after trim
  assert_eq!(storage.next_allocated(), 0);
}

#[test]
fn stress() {
  const N: u32 = 1_000_000;
  let mut storage = NodePool::new();
  storage.resize(N as usize);

  // Push a bunch of values into the allocator
  for i in 0..N { let _ = storage.alloc(i); }
}

