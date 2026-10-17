//! Negative: four `bool` fields where an enum or a flag set belongs.

pub struct Attributes {
    pub hidden: bool,
    pub read_only: bool,
    pub system: bool,
    pub archive: bool,
}
