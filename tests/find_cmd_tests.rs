// Copyright 2021 Chad Williamson <chad@dahc.us>
//
// Use of this source code is governed by an MIT-syle license that can be
// found in the LICENSE file or at https://opensource.org/licenses/MIT.

// This file contains integration tests for the find command.
//
// Note: the `serial` macro is used on tests that make assumptions about the
// working directory, since we have at least one test that needs to change it.

use assert_cmd::Command;
use predicates::prelude::*;
use serial_test::serial;
use std::fs::File;
use std::io::Write;
use std::{env, io::ErrorKind};
use tempfile::Builder;

#[cfg(unix)]
use std::os::unix::fs::symlink;

#[cfg(windows)]
use std::os::windows::fs::{symlink_dir, symlink_file};

use common::test_helpers::*;

mod common;

// Variants of fix_up_slashes that properly escape the forward slashes for being
// in a regex.
#[cfg(windows)]
fn fix_up_regex_slashes(re: &str) -> String {
    re.replace("/", "\\\\")
}

#[cfg(not(windows))]
fn fix_up_regex_slashes(re: &str) -> String {
    re.to_owned()
}

#[serial(working_dir)]
#[test]
fn no_args() {
    Command::cargo_bin("find")
        .expect("found binary")
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains("test_data"));
}

#[serial(working_dir)]
#[test]
fn two_matchers_both_match() {
    Command::cargo_bin("find")
        .expect("found binary")
        .args(&["-type", "d", "-name", "test_data"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains("test_data"));
}

#[serial(working_dir)]
#[test]
fn two_matchers_one_matches() {
    Command::cargo_bin("find")
        .expect("found binary")
        .args(&["-type", "f", "-name", "test_data"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::is_empty());
}

#[test]
fn matcher_with_side_effects_at_end() {
    let temp_dir = Builder::new().prefix("find_cmd_").tempdir().unwrap();

    let temp_dir_path = temp_dir.path().to_string_lossy();
    let test_file = temp_dir.path().join("test");
    File::create(&test_file).expect("created test file");

    Command::cargo_bin("find")
        .expect("found binary")
        .args(&[&temp_dir_path, "-name", "test", "-delete"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::is_empty());

    assert!(!test_file.exists(), "test file should be deleted");
    assert!(temp_dir.path().exists(), "temp dir should NOT be deleted");
}

#[test]
fn matcher_with_side_effects_in_front() {
    let temp_dir = Builder::new().prefix("find_cmd_").tempdir().unwrap();

    let temp_dir_path = temp_dir.path().to_string_lossy();
    let test_file = temp_dir.path().join("test");
    File::create(&test_file).expect("created test file");

    Command::cargo_bin("find")
        .expect("found binary")
        .args(&[&temp_dir_path, "-delete", "-name", "test"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::is_empty());

    assert!(!test_file.exists(), "test file should be deleted");
    assert!(!temp_dir.path().exists(), "temp dir should also be deleted");
}

// This could be covered by a unit test in principle... in practice, changing
// the working dir can't be done safely in unit tests unless `--test-threads=1`
// or `serial` goes everywhere, and it doesn't seem possible to get an
// appropriate `walkdir::DirEntry` for "." without actually changing dirs
// (or risking deletion of the repo itself).
#[serial(working_dir)]
#[test]
fn delete_on_dot_dir() {
    let temp_dir = Builder::new().prefix("example").tempdir().unwrap();
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(&temp_dir.path()).expect("working dir changed");

    // "." should be matched (confirmed by the print), but not deleted.
    Command::cargo_bin("find")
        .expect("found binary")
        .args(&[".", "-delete", "-print"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::diff(".\n"));

    env::set_current_dir(original_dir).expect("restored original working dir");

    assert!(temp_dir.path().exists(), "temp dir should still exist");
}

#[test]
fn regex_types() {
    let temp_dir = Builder::new().prefix("find_cmd_").tempdir().unwrap();

    let temp_dir_path = temp_dir.path().to_string_lossy();
    let test_file = temp_dir.path().join("teeest");
    File::create(&test_file).expect("created test file");

    Command::cargo_bin("find")
        .expect("found binary")
        .args(&[&temp_dir_path, "-regex", &fix_up_regex_slashes(".*/tE+st")])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::is_empty());

    Command::cargo_bin("find")
        .expect("found binary")
        .args(&[&temp_dir_path, "-iregex", &fix_up_regex_slashes(".*/tE+st")])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains("teeest"));

    Command::cargo_bin("find")
        .expect("found binary")
        .args(&[
            &temp_dir_path,
            "-regextype",
            "posix-basic",
            "-regex",
            &fix_up_regex_slashes(r".*/te\{1,3\}st"),
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains("teeest"));

    Command::cargo_bin("find")
        .expect("found binary")
        .args(&[
            &temp_dir_path,
            "-regextype",
            "posix-extended",
            "-regex",
            &fix_up_regex_slashes(".*/te{1,3}st"),
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains("teeest"));

    Command::cargo_bin("find")
        .expect("found binary")
        .args(&[
            &temp_dir_path,
            "-regextype",
            "ed",
            "-regex",
            &fix_up_regex_slashes(r".*/te\{1,3\}st"),
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains("teeest"));

    Command::cargo_bin("find")
        .expect("found binary")
        .args(&[
            &temp_dir_path,
            "-regextype",
            "sed",
            "-regex",
            &fix_up_regex_slashes(r".*/te\{1,3\}st"),
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains("teeest"));
}

#[test]
fn empty_files() {
    let temp_dir = Builder::new().prefix("find_cmd_").tempdir().unwrap();
    let temp_dir_path = temp_dir.path().to_string_lossy();

    Command::cargo_bin("find")
        .expect("found binary")
        .args(&[&temp_dir_path, "-empty"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(fix_up_slashes(&format!("{}\n", temp_dir_path)));

    let test_file_path = temp_dir.path().join("test");
    let mut test_file = File::create(&test_file_path).unwrap();

    Command::cargo_bin("find")
        .expect("found binary")
        .args(&[&temp_dir_path, "-empty"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(fix_up_slashes(&format!(
            "{}\n",
            test_file_path.to_string_lossy()
        )));

    let subdir_path = temp_dir.path().join("subdir");
    std::fs::create_dir(&subdir_path).unwrap();

    Command::cargo_bin("find")
        .expect("found binary")
        .args(&[&temp_dir_path, "-empty", "-sorted"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(fix_up_slashes(&format!(
            "{}\n{}\n",
            subdir_path.to_string_lossy(),
            test_file_path.to_string_lossy()
        )));

    write!(test_file, "x").unwrap();
    test_file.sync_all().unwrap();

    Command::cargo_bin("find")
        .expect("found binary")
        .args(&[&temp_dir_path, "-empty", "-sorted"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(fix_up_slashes(&format!(
            "{}\n",
            subdir_path.to_string_lossy(),
        )));
}

#[serial(working_dir)]
#[test]
fn find_printf() {
    #[cfg(unix)]
    {
        if let Err(e) = symlink("abbbc", "test_data/links/link-f") {
            if e.kind() != ErrorKind::AlreadyExists {
                panic!("Failed to create sym link: {:?}", e);
            }
        }
        if let Err(e) = symlink("subdir", "test_data/links/link-d") {
            if e.kind() != ErrorKind::AlreadyExists {
                panic!("Failed to create sym link: {:?}", e);
            }
        }
        if let Err(e) = symlink("missing", "test_data/links/link-missing") {
            if e.kind() != ErrorKind::AlreadyExists {
                panic!("Failed to create sym link: {:?}", e);
            }
        }
        if let Err(e) = symlink("abbbc/x", "test_data/links/link-notdir") {
            if e.kind() != ErrorKind::AlreadyExists {
                panic!("Failed to create sym link: {:?}", e);
            }
        }
        if let Err(e) = symlink("link-loop", "test_data/links/link-loop") {
            if e.kind() != ErrorKind::AlreadyExists {
                panic!("Failed to create sym link: {:?}", e);
            }
        }
    }
    #[cfg(windows)]
    {
        if let Err(e) = symlink_file("abbbc", "test_data/links/link-f") {
            if e.kind() != ErrorKind::AlreadyExists {
                panic!("Failed to create sym link: {:?}", e);
            }
        }
        if let Err(e) = symlink_dir("subdir", "test_data/links/link-d") {
            if e.kind() != ErrorKind::AlreadyExists {
                panic!("Failed to create sym link: {:?}", e);
            }
        }
        if let Err(e) = symlink_file("missing", "test_data/links/link-missing") {
            if e.kind() != ErrorKind::AlreadyExists {
                panic!("Failed to create sym link: {:?}", e);
            }
        }
        if let Err(e) = symlink_file("abbbc/x", "test_data/links/link-notdir") {
            if e.kind() != ErrorKind::AlreadyExists {
                panic!("Failed to create sym link: {:?}", e);
            }
        }
    }

    Command::cargo_bin("find")
        .expect("found binary")
        .args(&[
            &fix_up_slashes("./test_data/simple"),
            "-sorted",
            "-printf",
            "%f %d %h %H %p %P %y\n",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::diff(fix_up_slashes(
            "simple 0 ./test_data ./test_data/simple \
            ./test_data/simple  d\n\
            abbbc 1 ./test_data/simple ./test_data/simple \
            ./test_data/simple/abbbc abbbc f\n\
            subdir 1 ./test_data/simple ./test_data/simple \
            ./test_data/simple/subdir subdir d\n\
            ABBBC 2 ./test_data/simple/subdir ./test_data/simple \
            ./test_data/simple/subdir/ABBBC subdir/ABBBC f\n",
        )));

    Command::cargo_bin("find")
        .expect("found binary")
        .args(&[
            &fix_up_slashes("./test_data/links"),
            "-sorted",
            "-type",
            "l",
            "-printf",
            "%f %l %y %Y\n",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::diff(
            [
                "link-d subdir l d\n",
                "link-f abbbc l f\n",
                #[cfg(unix)]
                "link-loop link-loop l L\n",
                "link-missing missing l N\n",
                // We can't detect ENOTDIR on non-unix platforms yet.
                #[cfg(not(unix))]
                "link-notdir abbbc/x l ?\n",
                #[cfg(unix)]
                "link-notdir abbbc/x l N\n",
            ]
            .join(""),
        ));
}

#[cfg(unix)]
#[serial(working_dir)]
#[test]
fn find_perm() {
    Command::cargo_bin("find")
        .expect("found binary")
        .args(&["-perm", "+rwx"])
        .assert()
        .success();

    Command::cargo_bin("find")
        .expect("found binary")
        .args(&["-perm", "u+rwX"])
        .assert()
        .success();

    Command::cargo_bin("find")
        .expect("found binary")
        .args(&["-perm", "u=g"])
        .assert()
        .success();
}

#[cfg(unix)]
#[serial(working_dir)]
#[test]
fn find_inum() {
    use std::fs::metadata;
    use std::os::unix::fs::MetadataExt;

    let inum = metadata("test_data/simple/abbbc")
        .expect("metadata for abbbc")
        .ino()
        .to_string();

    Command::cargo_bin("find")
        .expect("found binary")
        .args(&["test_data", "-inum", &inum])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains("abbbc"));
}

#[cfg(unix)]
#[serial(working_dir)]
#[test]
fn find_links() {
    Command::cargo_bin("find")
        .expect("found binary")
        .args(&["test_data", "-links", "1"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains("abbbc"));
}

#[serial(working_dir)]
#[test]
fn find_mount_xdev() {
    // Make sure that -mount/-xdev doesn't prune unexpectedly.
    // TODO: Test with a mount point in the search.

    Command::cargo_bin("find")
        .expect("found binary")
        .args(&["test_data", "-mount"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains("abbbc"));

    Command::cargo_bin("find")
        .expect("found binary")
        .args(&["test_data", "-xdev"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains("abbbc"));
}

#[serial(working_dir)]
#[test]
fn find_accessable() {
    Command::cargo_bin("find")
        .expect("found binary")
        .args(&["test_data", "-readable"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains("abbbc"));

    Command::cargo_bin("find")
        .expect("found binary")
        .args(&["test_data", "-writable"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains("abbbc"));

    #[cfg(unix)]
    Command::cargo_bin("find")
        .expect("found binary")
        .args(&["test_data", "-executable"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains("abbbc").not());
}
