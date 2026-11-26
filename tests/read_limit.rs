//! The size limit of the files the command line reads: a file within it is read, a bigger one is refused with its name and nothing is loaded.

use eh3d_autolevel::window::{read_limited, write_atomic};
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

/// A device tells no size: the limit is on what is read.
#[cfg(unix)]
#[test]
fn a_file_that_tells_no_size_is_still_read_only_up_to_the_limit() {
    let why = read_limited(std::path::Path::new("/dev/zero"), 1_000).unwrap_err();
    assert!(why.contains("larger than 1000 bytes"), "{why}");
}

#[test]
fn a_write_replaces_the_whole_file_and_takes_no_file_of_the_person_for_its_temporary_one() {
    let dir = scratch("atomic");
    let file = dir.join("out.nc");
    let theirs = dir.join("out.nc.tmp");
    fs::write(&theirs, "mine").unwrap();
    fs::write(&file, "old program\n").unwrap();
    // another name of the old file keeps the old text: the file was replaced and never cut short
    let other = dir.join("kept.nc");
    fs::hard_link(&file, &other).unwrap();
    write_atomic(&file, "new\n").unwrap();
    assert_eq!(fs::read_to_string(&file).unwrap(), "new\n");
    assert_eq!(fs::read_to_string(&other).unwrap(), "old program\n");
    assert_eq!(fs::read_to_string(&theirs).unwrap(), "mine");
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 3);
    assert!(write_atomic(&dir.join("none").join("x.nc"), "x").is_err());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn writers_at_the_same_time_leave_one_whole_file() {
    let dir = scratch("together");
    let file = dir.join("out.nc");
    let texts: Vec<String> = (0..8)
        .map(|k| format!("G0 X{k}\n").repeat(2_000 + k * 100))
        .collect();
    let written: usize = std::thread::scope(|scope| {
        let handles: Vec<_> = texts
            .iter()
            .map(|t| scope.spawn(|| usize::from(write_atomic(&file, t).is_ok())))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).sum()
    });
    // on Windows a rename over a file that another rename holds at that moment can be refused: the file is whole all the same
    assert!(written >= 1);
    assert!(texts.contains(&fs::read_to_string(&file).unwrap()));
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
    let _ = fs::remove_dir_all(&dir);
}
