# vec_mem_heap
An unfinished and probably bad memory allocator and ownership sharer wrapped in Rust's Vec because I don't know how to handle raw memory yet.
Documentation was done a bit hastily, expect stuff like use examples to be added as I feel like it.

There should be two major reworks before this project is considered 'complete' (unless I think of more):
- Removing the middle man (Vec<>) and handling memory management directly.
- Adding multiple 'buckets' to hold data of different sizes to minimize fragmentation.

These changes could come in any order, at any time, so I wouldn't rely on this unless you're me until it hits version 1.
