use funny_file_manager::run;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    run()
}

#[cfg(coverage)]
#[test]
fn test_main_coverage() {
    assert!(main().is_ok());
}
