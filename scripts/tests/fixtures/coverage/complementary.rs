//! Complementary generic instantiations whose physical lines all execute.

fn choose<T: Default>(flag: bool) -> T {
    if flag {
        T::default()
    } else {
        T::default()
    }
}

fn main() {
    let _ = choose::<u8>(true);
    let _ = choose::<u16>(false);
}
