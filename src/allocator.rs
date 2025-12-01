use crate::QueueStructure;
use alloc::boxed::Box;
use crossbeam_queue::ArrayQueue;
#[cfg(feature = "std")]
use std::sync::OnceLock;

#[cfg(not(feature = "std"))]
use once_cell::sync::OnceCell as OnceLock;

const QUEUE_POOL_CAPACITY: usize = 128;

static QUEUE_ALLOCATOR: OnceLock<ArrayQueue<Box<QueueStructure>>> = OnceLock::new();

fn get_queue_allocator() -> &'static ArrayQueue<Box<QueueStructure>> {
    QUEUE_ALLOCATOR.get_or_init(|| {
        let queue = ArrayQueue::new(QUEUE_POOL_CAPACITY);
        for _ in 0..QUEUE_POOL_CAPACITY {
            let _ = queue.push(Box::new(QueueStructure::new()));
        }
        queue
    })
}

#[inline(always)]
pub(crate) fn allocate_queue() -> Box<QueueStructure> {
    get_queue_allocator()
        .pop()
        .unwrap_or_else(|| Box::new(QueueStructure::new()))
}

#[inline(always)]
pub(crate) fn deallocate_queue(element: Box<QueueStructure>) {
    _ = get_queue_allocator().push(element)
}
