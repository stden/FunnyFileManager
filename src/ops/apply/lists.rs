//! Temporary selection lists with Far's modifiers and native code pages.

use std::io::{self, Write};
use std::path::Path;

use tempfile::{Builder, TempPath};

use crate::models::apply::{ListEncoding, ListOptions, NameForm, PanelContext};

pub(super) struct Lists<'a> {
    directory: &'a Path,
    files: Vec<TempPath>,
}

impl<'a> Lists<'a> {
    pub(super) const fn new(directory: &'a Path) -> Self {
        Self {
            directory,
            files: Vec::new(),
        }
    }

    pub(super) fn write(
        &mut self,
        panel: &PanelContext,
        options: ListOptions,
    ) -> io::Result<String> {
        let mut file = Builder::new()
            .prefix("ffm-apply-")
            .suffix(".lst")
            .tempfile_in(self.directory)?;
        file.write_all(&encode(&listing(panel, options), options.encoding)?)?;
        let path = file.into_temp_path();
        let name = match options.form {
            NameForm::Long => super::context::display_path(&path),
            NameForm::Short => super::context::display_path(&super::context::short_path(&path)),
        };
        self.files.push(path);
        Ok(name)
    }
}

fn listing(panel: &PanelContext, options: ListOptions) -> String {
    let mut text = String::new();
    for name in &panel.selected {
        let mut name = if options.full_paths {
            Path::new(panel.paths.logical.get(options.form))
                .join(name.get(options.form))
                .display()
                .to_string()
        } else {
            name.get(options.form).to_owned()
        };
        if options.forward_slashes {
            name = name.replace('\\', "/");
        }
        text.push_str(&options.quote.apply(&name));
        text.push_str(if cfg!(windows) { "\r\n" } else { "\n" });
    }
    text
}

#[cfg_attr(
    not(windows),
    expect(
        clippy::unnecessary_wraps,
        reason = "only the Windows code-page conversion can fail"
    )
)]
fn encode(text: &str, encoding: ListEncoding) -> io::Result<Vec<u8>> {
    match encoding {
        ListEncoding::Utf8 => Ok(text.as_bytes().to_vec()),
        ListEncoding::Utf16 => Ok(text.encode_utf16().flat_map(u16::to_le_bytes).collect()),
        #[cfg(windows)]
        ListEncoding::Oem | ListEncoding::Ansi => native_encode(text, encoding),
        #[cfg(not(windows))]
        ListEncoding::Oem | ListEncoding::Ansi => Ok(text.as_bytes().to_vec()),
    }
}

#[cfg(windows)]
fn native_encode(text: &str, encoding: ListEncoding) -> io::Result<Vec<u8>> {
    use windows_sys::Win32::Globalization::{CP_ACP, CP_OEMCP};

    let wide: Vec<_> = text.encode_utf16().collect();
    if wide.is_empty() {
        return Ok(Vec::new());
    }
    let code_page = if encoding == ListEncoding::Ansi {
        CP_ACP
    } else {
        CP_OEMCP
    };
    let mut bytes = vec![0; native_convert(&wide, code_page, &mut [])?];
    let written = native_convert(&wide, code_page, &mut bytes)?;
    bytes.truncate(written);
    Ok(bytes)
}

#[cfg(windows)]
fn native_convert(wide: &[u16], code_page: u32, bytes: &mut [u8]) -> io::Result<usize> {
    use windows_sys::Win32::Globalization::WideCharToMultiByte;
    let length = i32::try_from(wide.len()).map_err(io::Error::other)?;
    let capacity = i32::try_from(bytes.len()).map_err(io::Error::other)?;
    let output = if bytes.is_empty() {
        std::ptr::null_mut()
    } else {
        bytes.as_mut_ptr()
    };
    // SAFETY: both buffers remain valid for the call, with their exact lengths.
    let written = unsafe {
        WideCharToMultiByte(
            code_page,
            0,
            wide.as_ptr(),
            length,
            output,
            capacity,
            std::ptr::null(),
            std::ptr::null_mut(),
        )
    };
    if written == 0 {
        return Err(io::Error::last_os_error());
    }
    usize::try_from(written).map_err(io::Error::other)
}

#[cfg(test)]
mod tests;
