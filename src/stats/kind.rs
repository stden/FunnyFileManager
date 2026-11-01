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
        let extension = Path::new(name)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(str::to_ascii_lowercase)
            .unwrap_or_default();

        match extension.as_str() {
            "rs" | "c" | "h" | "cc" | "cpp" | "hpp" | "cs" | "go" | "java" | "js" | "jsx"
            | "ts" | "tsx" | "py" | "rb" | "php" | "kt" | "swift" | "lua" | "sh" | "bash"
            | "ps1" | "cmd" | "bat" | "sql" | "css" | "scss" | "html" | "htm" | "vue" => {
                Self::Source
            }
            "md" | "markdown" | "txt" | "rst" | "adoc" | "pdf" | "doc" | "docx" | "odt" | "rtf"
            | "tex" | "epub" => Self::Document,
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "svg" | "webp" | "ico" | "tif" | "tiff" => {
                Self::Image
            }
            "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" | "opus" | "wma" => Self::Audio,
            "mp4" | "mkv" | "avi" | "mov" | "webm" | "wmv" | "flv" | "m4v" => Self::Video,
            "zip" | "tar" | "gz" | "bz2" | "xz" | "zst" | "7z" | "rar" | "jar" | "whl" => {
                Self::Archive
            }
            "exe" | "dll" | "so" | "dylib" | "a" | "o" | "bin" | "msi" | "deb" | "rpm" => {
                Self::Executable
            }
            "json" | "yaml" | "yml" | "toml" | "xml" | "csv" | "tsv" | "db" | "sqlite" | "log"
            | "lock" | "parquet" => Self::Data,
            _ => Self::Other,
        }
    }
}
