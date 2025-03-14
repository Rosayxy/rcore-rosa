//! rosa's trace array
use core::sync::atomic::{AtomicIsize, Ordering};

static mut GLOBAL_ARRAY_TRACE: [AtomicIsize; 8] = [
    AtomicIsize::new(0),
    AtomicIsize::new(0),
    AtomicIsize::new(0),
    AtomicIsize::new(0),
    AtomicIsize::new(0),
    AtomicIsize::new(0),
    AtomicIsize::new(0),
    AtomicIsize::new(0),
];
/// writing to the global array
pub fn write_to_array(index: usize, value: isize) -> Result<(), &'static str> {
    if index >= 8 {
        return Err("Index out of bounds");
    }
    
    // Safety: We're using atomic operations, so this is thread-safe
    unsafe {
        GLOBAL_ARRAY_TRACE[index].store(value, Ordering::SeqCst);
    }
    Ok(())
}

/// Safe wrapper function to read from the array
pub fn read_from_array(index: usize) -> Result<isize, &'static str> {
    if index >= 8 {
        return Err("Index out of bounds");
    }
    
    // Safety: We're using atomic operations, so this is thread-safe
    unsafe {
        Ok(GLOBAL_ARRAY_TRACE[index].load(Ordering::SeqCst))
    }
}
/// incline the value in the array
pub fn incl_array(index:usize) -> Result<(), &'static str>{
    if index >= 8 {
        return Err("Index out of bounds");
    }
    unsafe {
        GLOBAL_ARRAY_TRACE[index].fetch_add(1, Ordering::SeqCst);
    }
    Ok(())
}
/// function for zeroing every syscall count, called when initializing for running user program

pub fn zero_out_array(){
    unsafe {
        for i in 0..8 {
            GLOBAL_ARRAY_TRACE[i].store(0, Ordering::SeqCst);
        }
    }
}