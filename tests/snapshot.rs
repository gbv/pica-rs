#[test]
fn snapshots() {
    trycmd::TestCases::new()
        .case("tests/snapshot/cat/*.trycmd")
        .case("tests/snapshot/cat/*.toml")
        .case("tests/snapshot/completions/*.trycmd")
        .case("tests/snapshot/completions/*.toml")
        .case("tests/snapshot/invalid/*.trycmd")
        .case("tests/snapshot/invalid/*.toml");
}