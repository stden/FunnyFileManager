//! Purpose: Answer the cursor query `ConPTY` requires before its child can start.

#[derive(Default)]
enum Prefix {
    #[default]
    None,
    Escape,
    Csi,
    Cursor,
}

#[derive(Default)]
pub(super) struct Queries(Prefix);

pub(super) enum Reply {
    CursorPosition,
}

impl Reply {
    pub const fn bytes() -> &'static [u8] {
        b"\x1b[1;1R"
    }
}

impl Queries {
    pub fn feed(&mut self, bytes: &[u8]) -> usize {
        let mut queries = 0;
        for &byte in bytes {
            queries += usize::from(matches!((&self.0, byte), (Prefix::Cursor, b'n')));
            self.0 = match (&self.0, byte) {
                (Prefix::Escape, b'[') => Prefix::Csi,
                (Prefix::Csi, b'6') => Prefix::Cursor,
                (_, 0x1b) => Prefix::Escape,
                _ => Prefix::None,
            };
        }
        queries
    }
}

#[cfg(test)]
mod tests;
