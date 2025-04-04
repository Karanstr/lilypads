use vec_mem_heap::prelude::*;

#[test]
fn test_push() {
    let mut storage = NodeField::<i32>::new();
    let idx1 = storage.push(42);
    let idx2 = storage.push(123);
    
    assert_eq!(*storage.data(idx1).unwrap(), 42);
    assert_eq!(*storage.data(idx2).unwrap(), 123);
}

#[test]
fn test_error_handling() {
    let mut storage = NodeField::<i32>::new();
    let idx = storage.push(42);
    
    // Test invalid index
    assert!(matches!(storage.data(999), Err(AccessError::OutOfBoundsMemory(_))));
    
    // Test double free
    storage.remove_ref(idx).unwrap();
    assert!(matches!(storage.remove_ref(idx), Err(AccessError::FreeMemory(_))));
}

#[test]
fn test_replace() {
    let mut storage = NodeField::<String>::new();
    let idx = storage.push("old".to_string());
    
    // Replace and verify old data is returned
    let old = storage.replace(idx, "new".to_string()).unwrap();
    assert_eq!(old, "old");
    
    // Verify new data is in place
    assert_eq!(*storage.data(idx).unwrap(), "new");
}

#[test]
fn test_reference_counting() {
    let mut storage = NodeField::<String>::new();
    let idx = storage.push("test".to_string());
    
    // Add reference
    storage.add_ref(idx).unwrap();
    
    // First remove should return None (still has one ref)
    assert!(storage.remove_ref(idx).unwrap().is_none());
    
    // Second remove should return the data
    assert_eq!(storage.remove_ref(idx).unwrap().unwrap(), "test");
    
    // Third remove should fail
    assert!(storage.remove_ref(idx).is_err());
}

#[test]
fn test_memory_reuse() {
    let mut storage = NodeField::<i32>::new();
    let idx1 = storage.push(1);
    let idx2 = storage.push(2);
    
    // Remove first item
    storage.remove_ref(idx1).unwrap();
    
    // New push should reuse idx1
    let idx3 = storage.push(3);
    assert_eq!(idx1, idx3);
    
    // Verify data
    assert_eq!(*storage.data(idx2).unwrap(), 2);
    assert_eq!(*storage.data(idx3).unwrap(), 3);
}

#[test]
fn test_repair() {
    let mut storage = NodeField::<i32>::new();
    let indices: Vec<_> = (0..5).map(|i| storage.push(i)).collect();
    
    // Create some gaps
    storage.remove_ref(indices[1]).unwrap();
    storage.remove_ref(indices[3]).unwrap();
    
    // Should be in order after repair
    storage.repair_allocator();
    
    // Next allocation should use the lowest free index
    let new_idx = storage.push(42);
    assert_eq!(new_idx, 1); // Should reuse the first freed slot
    
    let another_idx = storage.push(100);
    assert_eq!(another_idx, 3); // Should reuse the second freed slot
}

#[test]
fn test_defrag() {
    let mut storage = NodeField::<i32>::new();
    let mut indices: Vec<_> = (0..5).map(|i| storage.push(i)).collect();
    
    // Remove some items to create gaps
    storage.remove_ref(indices[1]).unwrap();
    storage.remove_ref(indices[3]).unwrap();
    
    // Defrag and verify remapping
    let remapped = storage.defrag();
    for (old, new) in remapped.iter() { indices[*old] = *new }


    // Verify data is preserved
    assert_eq!(*storage.data(indices[0]).unwrap(), 0);
    assert_eq!(*storage.data(indices[2]).unwrap(), 2);
    assert_eq!(*storage.data(indices[4]).unwrap(), 4);
}

#[test]
fn test_trim() {
    let mut storage = NodeField::<i32>::new();
    let mut indices: Vec<_> = (0..5).map(|i| storage.push(i)).collect();
    
    // Remove last two items
    storage.remove_ref(indices[3]).unwrap();
    storage.remove_ref(indices[4]).unwrap();
    
    // Trim and verify
    let remapped = storage.trim();
    for (old, new) in remapped.iter() { indices[*old] = *new }

    // Verify memory state after trim
    assert!(!matches!(storage.data(2), Err(AccessError::OutOfBoundsMemory(_))));
    assert!(matches!(storage.data(3), Err(AccessError::OutOfBoundsMemory(_))));
    
    // Verify allocator state after trim
    assert!(storage.next_allocated() == 3);

    
    // Verify remaining data
    assert_eq!(*storage.data(indices[0]).unwrap(), 0);
    assert_eq!(*storage.data(indices[1]).unwrap(), 1);
    assert_eq!(*storage.data(indices[2]).unwrap(), 2);
}

#[test]
fn test_trim_allocator() {
    let mut storage = NodeField::<i32>::new();
    
    // Create a large gap by pushing many values and then freeing most of them
    let indices: Vec<usize> = (0..100).map(|i| storage.push(i)).collect();
    for &idx in &indices[0..99] {
        storage.remove_ref(idx).unwrap();
    }
    
    // Verify the last element is still accessible
    assert!(storage.data(indices[99]).is_ok());
    let _ = storage.trim();
    
    // Verify memory state after trim
    assert!(matches!(storage.data(0), Ok(_))); // Last element is now at index 0
    assert!(matches!(storage.data(1), Err(AccessError::OutOfBoundsMemory(_)))); // No index 1 after trim
    
    // Verify allocator state after trim
    assert!(storage.next_allocated() == 1);
    
    // Verify mask verification
    assert!(storage.allocation_map().len() == 1);
}

#[test]
fn test_trim_all_free() {
    let mut storage = NodeField::<i32>::new();

    let idx1 = storage.push(1);
    let idx2 = storage.push(2);

    //Set all slots to free
    storage.remove_ref(idx1).unwrap();
    storage.remove_ref(idx2).unwrap();

    _ = storage.trim();

    // Verify memory state
    assert!(matches!(storage.data(0), Err(AccessError::OutOfBoundsMemory(0))));

    // Verify allocator state after trim
    assert!(storage.next_allocated() == 0);
    
    // Verify mask verification
    assert!(storage.allocation_map().len() == 0);
}

#[test]
fn test_trim_empty() {
    let mut storage = NodeField::<i32>::new();

    _ = storage.trim();

    // Verify memory state
    assert!(matches!(storage.data(0), Err(AccessError::OutOfBoundsMemory(0))));
    
    // Verify allocator state after trim
    assert!(storage.next_allocated() == 0);
    
    // Verify mask vec
    assert!(storage.allocation_map().len() == 0);
}
    