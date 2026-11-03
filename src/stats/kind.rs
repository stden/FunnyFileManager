//! Purpose: Broad content categories, and which one a file name falls into.

use std::path::Path;

/// Broad content categories, which is the level a person asks the question at.
///
/// Far coarser than a MIME table on purpose: "is this code or media" is
/// answerable at a glance and useful; "is this `image/x-portable-anymap`" is
/// neither.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ContentKind {
    Source,
    Document,
    Image,
    Audio,
    Video,
    Archive,
    Executable,
    /// Databases, logs, dumps — data that is neither prose nor code.
    Data,
    Other,
}

impl ContentKind {
    pub(super) const COUNT: usize = 9;

    pub(super) const ALL: [Self; Self::COUNT] = [
        Self::Source,
        Self::Document,
        Self::Image,
        Self::Audio,
        Self::Video,
        Self::Archive,
        Self::Executable,
        Self::Data,
        Self::Other,
    ];

    pub(super) const fn index(self) -> usize {
        match self {
            Self::Source => 0,
            Self::Document => 1,
            Self::Image => 2,
            Self::Audio => 3,
            Self::Video => 4,
            Self::Archive => 5,
            Self::Executable => 6,
            Self::Data => 7,
            Self::Other => 8,
        }
    }

    /// A short label for the panel.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Source => "Source",
            Self::Document => "Documents",
            Self::Image => "Images",
            Self::Audio => "Audio",
            Self::Video => "Video",
            Self::Archive => "Archives",
            Self::Executable => "Programs",
            Self::Data => "Data",
            Self::Other => "Other",
        }
    }

    /// Which category a file name falls into.
    ///
    /// By extension, because that is what the user can see too. Sniffing the
    /// contents would be more accurate and would also mean opening every file
    /// in the tree, which is the one thing this walk must not do.
    #[must_use]
    pub fn of(name: &str) -> Self {
        let Some(ext) = Path::new(name).extension().and_then(|ext| ext.to_str()) else {
            return Self::Other;
        };
        let mut buf = [0_u8; 8];
        let Some(dest) = buf.get_mut(..ext.len()) else {
            return Self::Other;
        };
        for (dst, src) in dest.iter_mut().zip(ext.bytes()) {
            *dst = src.to_ascii_lowercase();
        }

        match &*dest {
            b"rs" | b"c" | b"h" | b"cc" | b"cpp" | b"hpp" | b"cs" | b"go" | b"java" | b"js"
            | b"jsx" | b"ts" | b"tsx" | b"py" | b"rb" | b"php" | b"kt" | b"swift" | b"lua"
            | b"sh" | b"bash" | b"ps1" | b"cmd" | b"bat" | b"sql" | b"css" | b"scss" | b"html"
            | b"htm" | b"vue" => Self::Source,
            b"md" | b"markdown" | b"txt" | b"rst" | b"adoc" | b"pdf" | b"doc" | b"docx"
            | b"odt" | b"rtf" | b"tex" | b"epub" => Self::Document,
            b"png" | b"jpg" | b"jpeg" | b"gif" | b"bmp" | b"svg" | b"webp" | b"ico" | b"tif"
            | b"tiff" => Self::Image,
            b"mp3" | b"wav" | b"flac" | b"ogg" | b"m4a" | b"aac" | b"opus" | b"wma" => Self::Audio,
            b"mp4" | b"mkv" | b"avi" | b"mov" | b"webm" | b"wmv" | b"flv" | b"m4v" => Self::Video,
            b"zip" | b"tar" | b"gz" | b"bz2" | b"xz" | b"zst" | b"7z" | b"rar" | b"jar"
            | b"whl" => Self::Archive,
            b"exe" | b"dll" | b"so" | b"dylib" | b"a" | b"o" | b"bin" | b"msi" | b"deb"
            | b"rpm" => Self::Executable,
            b"json" | b"yaml" | b"yml" | b"toml" | b"xml" | b"csv" | b"tsv" | b"db" | b"sqlite"
            | b"log" | b"lock" | b"parquet" => Self::Data,
            _ => Self::Other,
        }
    }
}
