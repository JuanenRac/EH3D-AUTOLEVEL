//! The size limit of the files the command line reads: a file within it is read, a bigger one is refused with its name and nothing is loaded.

use eh3d_autolevel::window::read_limited;
use std::fs;

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "eh3d_autolevel_limit_{}_{name}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_file_within_the_limit_is_read_and_a_bigger_one_is_refused() {
    let dir = scratch("size");
    let file = dir.join("program.nc");
    fs::write(&file, "G0 X1\n").unwrap();
    assert_eq!(read_limited(&file, 6).unwrap(), "G0 X1\n");
    assert_eq!(read_limited(&file, 1_000).unwrap(), "G0 X1\n");
    let why = read_limited(&file, 5).unwrap_err();
    assert!(
        why.contains("program.nc") && why.contains("larger than 5 bytes"),
        "{why}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_missing_file_is_an_error_that_names_it() {
    let dir = scratch("missing");
    let why = read_limited(&dir.join("nothing.nc"), 1_000).unwrap_err();
    assert!(why.contains("nothing.nc"), "{why}");
    let _ = fs::remove_dir_all(&dir);
}
