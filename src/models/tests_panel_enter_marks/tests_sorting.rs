//! Purpose: unit tests for sort mode cycling and directories-before-files ordering.

use super::*;

/// `cycle_sort_mode` cycles through all three modes
#[test]
fn test_cycle_sort_mode_all_three() {
    let temp_dir = TempDir::new().expect("temp dir");
    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("panel");

    assert_eq!(panel.sort_mode, SortMode::Name);
    panel.cycle_sort_mode();
    assert_eq!(panel.sort_mode, SortMode::Extension);
    panel.cycle_sort_mode();
    assert_eq!(panel.sort_mode, SortMode::Size);
    panel.cycle_sort_mode();
    assert_eq!(panel.sort_mode, SortMode::Name);
}

// Kill: delete ! in (!e.kind.is_dir(), ext, ...) in Extension sort
#[test]
fn test_extension_sort_dirs_before_files() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    fs::create_dir(temp_dir.path().join("zdir"))?;
    fs::write(temp_dir.path().join("aaa.txt"), "a")?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    panel.sort_mode = SortMode::Extension;
    panel.load_entries()?;

    // Find positions: ".." first, then dirs, then files
    let dir_pos = panel.entries.iter().position(|e| e.name == "zdir").unwrap();
    let file_pos = panel
        .entries
        .iter()
        .position(|e| e.name == "aaa.txt")
        .unwrap();
    assert!(
        dir_pos < file_pos,
        "directories must sort before files in Extension mode: dir@{dir_pos} file@{file_pos}"
    );

    Ok(())
}

// Kill: delete match arm (true, false) => Less in Size sort
// Kill: delete match arm (false, true) => Greater in Size sort
#[test]
fn test_size_sort_dirs_before_files() -> io::Result<()> {
    let temp_dir = TempDir::new()?;
    // Create multiple dirs and files interspersed alphabetically
    // so the sort must encounter both (dir,file) and (file,dir) comparisons
    fs::create_dir(temp_dir.path().join("adir"))?;
    fs::create_dir(temp_dir.path().join("mdir"))?;
    fs::create_dir(temp_dir.path().join("zdir"))?;
    fs::write(temp_dir.path().join("bfile.bin"), vec![0u8; 10000])?;
    fs::write(temp_dir.path().join("nfile.txt"), vec![0u8; 5000])?;
    fs::write(temp_dir.path().join("yfile.dat"), "x")?;

    let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
    panel.sort_mode = SortMode::Size;
    panel.load_entries()?;

    assert_directories_before_files(&panel);
    assert_files_descending(&panel);

    Ok(())
}

fn assert_directories_before_files(panel: &Panel) {
    // All directories must appear before all files
    let all_dir_positions: Vec<usize> = panel
        .entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.kind.is_dir() && e.name != "..")
        .map(|(i, _)| i)
        .collect();
    let all_file_positions: Vec<usize> = panel
        .entries
        .iter()
        .enumerate()
        .filter(|(_, e)| !e.kind.is_dir())
        .map(|(i, _)| i)
        .collect();

    let max_dir = all_dir_positions.iter().max().copied().unwrap_or(0);
    let min_file = all_file_positions
        .iter()
        .min()
        .copied()
        .unwrap_or(usize::MAX);
    assert!(
        max_dir < min_file,
        "all dirs must come before all files: max_dir_pos={max_dir} min_file_pos={min_file}"
    );
}

fn assert_files_descending(panel: &Panel) {
    // Files should be ordered by size descending
    let file_sizes: Vec<u64> = panel
        .entries
        .iter()
        .filter(|e| !e.kind.is_dir())
        .map(|e| e.size.bytes())
        .collect();
    for w in file_sizes.windows(2) {
        assert!(
            w[0] >= w[1],
            "files must be sorted by size descending: {} < {}",
            w[0],
            w[1]
        );
    }
}
