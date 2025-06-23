# lilypads
Fun little object pool allocator.

This is a learning experience for me, but I'm fairly confident it doesn't suck that badly.

This crate was originally intended for the creation of tree-like datastructures,
where indexes could be used instead of dealing with rust's reference/pointer system.
The vision of the project has somewhat shifted since v0.8 and is now intended as a 
general purpose object pool, for whatever you need to be pooling. It attempts to keep data as
contiguous as possible, reserving memory close to the front and providing 
inbuilt defragmentation and trimming utilities.

This crate isn't yet thread safe, but that's eventually on the todo list probably.
If you run into any issues, complaints, or suggestions, feel free to open an issue.
