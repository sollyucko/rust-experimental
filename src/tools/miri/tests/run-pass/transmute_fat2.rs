// ignore-windows: Unwind panicking does not currently work on Windows
fn main() {
    #[cfg(target_pointer_width="64")]
    let bad = unsafe {
        std::mem::transmute::<u128, &[u8]>(42)
    };
    #[cfg(target_pointer_width="32")]
    let bad = unsafe {
        std::mem::transmute::<u64, &[u8]>(42)
    };
    // This created a slice with length 0, so the following will fail the bounds check.
    bad[0];
}
