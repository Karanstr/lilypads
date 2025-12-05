#![warn(missing_docs)]
//! Fun little object pool allocator.
//!
//! This is a learning experience for me, but I'm fairly confident it doesn't suck that badly.
//!
//! This crate was originally intended for the creation of tree-like datastructures,
//! where indexes could be used instead of dealing with rust's reference/pointer system.
//! The vision of the project has somewhat shifted since v0.8 and is now intended as a 
//! general purpose object pool, for whatever you need to be pooling. It attempts to keep data as
//! contiguous as possible, [Pond::insert] reserves the first (sequentially) free node and [Pond::defrag] +
//! [Pond::trim] are provided to maintain contiguity on otherwise sparse allocations.
//!
//! This crate isn't yet thread safe, but that's eventually on the todo list probably.
//!
//! # Example
//! ```
//! use lilypads::Pond;
//!
//! fn main() {
//!   let mut pool = Pond::new();
//!   // You can push data into the pond and recieve their index.
//!   let idx1 = pool.insert(57);
//!   let idx2 = pool.insert(42);
//!
//!   // Data is retrieved with get
//!   let data1 = pool.get(idx1).unwrap();
//!   assert_eq!(*data1, 57);
//!   // And get_mut
//!   let data2 = pool.get_mut(idx2).unwrap();
//!   *data2 = 13;
//!   assert_eq!(*pool.get(idx2).unwrap(), 13);
//!
//!   // Data can be freed with free, which will return the data stored at the index.
//!   let freed1 = pool.free(idx1).unwrap();
//!   assert_eq!(freed1, 57);
//!   assert_eq!(pool.get_mut(idx1), None);
//!
//!   // You can request a specific index with write, overwriting the existing data 
//!   // and returning whatever used to be there
//!   let replaced = pool.write(idx2, 98);
//!   assert_eq!(*pool.get(idx2).unwrap(), 98);
//!
//!   let far_idx = 17;
//!   let nothing = pool.write(far_idx, 1000);
//!   assert_eq!(nothing, None);
//!   assert_eq!(*pool.get(far_idx).unwrap(), 1000);
//!   
//! }
//! ```

mod bitmap;
mod pondaos;
// mod pondsoa;

pub use pondaos::Pond;
// pub use pondsoa::PondSoa;

