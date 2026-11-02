//! Complementary generic instantiations plus one genuinely unexecuted line.

fn choose<T: Default>(flag: bool) -> T {
    if flag {
        T::default()
    } else {
        T::default()
    }
}

fn main() {
    if std::hint::black_box(false) {
        return;
    }
    let _ = choose::<u8>(true);
    let _ = choose::<u16>(false);
}
