# vec_mem_heap
An unfinished and probably bad memory allocator wrapped in Rust's Vec because I don't know how to handle raw memory yet.
Expect documentation to come over the next few days.
There should be two major reworks before this project is considered 'complete'.
- Removing the middle man (Vec<>) and handling memory management directly.
- Adding multiple 'buckets' to hold data of different sizes to minimize fragmentation.

These changes could come in any order, at any time, so I wouldn't rely on this unless you're me until it hits version 1.
