//! rosa's trace array
use core::sync::atomic::{AtomicIsize, Ordering};

static mut GLOBAL_ARRAY_TRACE: [AtomicIsize; 5] = [
    AtomicIsize::new(0),
    AtomicIsize::new(0),
    AtomicIsize::new(0),
    AtomicIsize::new(0),
    AtomicIsize::new(0),
];
/// writing to our global array
pub fn write_to_array(index: usize, value: isize) -> Result<(), &'static str> {
    if index >= 5 {
        return Err("Index out of bounds");
    }

    unsafe {
        GLOBAL_ARRAY_TRACE[index].store(value, Ordering::SeqCst);
    }
    Ok(())
}

/// Safety wrapper function to read from the array
pub fn read_from_array(index: usize) -> Result<isize, &'static str> {
    if index >= 5 {
        return Err("Index out of bounds");
    }

    unsafe {
        Ok(GLOBAL_ARRAY_TRACE[index].load(Ordering::SeqCst))
    }
}

/// incline the value by 1 in the array
pub fn incl_array(index:usize) -> Result<(), &'static str>{
    if index >= 5 {
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
        for i in 0..5 {
            GLOBAL_ARRAY_TRACE[i].store(0, Ordering::SeqCst);
        }
    }
}