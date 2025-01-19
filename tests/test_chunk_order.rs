mod integration_common;
use assert_cmd::Command;
use integration_common::{create_file, setup_temp_repo};
use tracing::Level;
use tracing_subscriber::fmt;

/// This test ensures that the last-written chunk contains the highest-priority file.
#[test]
fn chunk_order_reflects_priority() {
    // Setup logging
    fmt()
        .with_max_level(Level::DEBUG)
        .with_target(false)
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_ansi(true)
        .try_init()
        .ok();

    let repo = setup_temp_repo();

    // Create a Yek config that makes `high_priority/` have a very high score
    create_file(
        repo.path(),
        "yek.toml",
        r#"
[[priority_rules]]
score = 10
patterns = ["^low_priority/"]

[[priority_rules]]
score = 999
patterns = ["^high_priority/"]
"#,
    );

    // Create a small file in low_priority
    create_file(repo.path(), "low_priority/foo.txt", "low priority content");

    // Create a bigger file in high_priority so that
    // it definitely splits or at least goes into a later chunk.
    //  We'll just create multiple lines to push the chunk size.
    let big_content = "HIGH PRIORITY\n".repeat(1000);
    create_file(repo.path(), "high_priority/foo.txt", &big_content);

    // We'll force extremely small max-size to ensure multiple chunks.
    let mut cmd = Command::cargo_bin("yek").unwrap();
    let assert = cmd
        .current_dir(repo.path())
        .arg("--max-size")
        .arg("1KB") // force chunking
        .arg("--debug")
        .env("TERM", "xterm-256color")
        .assert()
        .success();

    // Run the command and capture output
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    println!("STDOUT:\n{}", stdout);

    // Instead of comparing positions, we'll check the order of chunks
    let mut found_low_priority = false;
    let mut found_high_priority = false;

    for line in stdout.lines() {
        if line.contains("low_priority/foo.txt") {
            found_low_priority = true;
        } else if line.contains("high_priority/foo.txt") {
            found_high_priority = true;
            // Once we find high priority, low priority should have been found already
            assert!(
                found_low_priority,
                "Low priority file should appear before high priority file"
            );
        }
    }

    // Verify both files were found
    assert!(
        found_low_priority,
        "Low priority file should be in the output"
    );
    assert!(
        found_high_priority,
        "High priority file should be in the output"
    );
}
