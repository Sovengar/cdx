// Temporary probe used to prove that `master` blocks a red PR.
// This file must never be merged; the probe PR is closed and its branch deleted.

#[test]
fn ci_probe_intentionally_fails() {
    panic!("intentional failure to prove the required Test check blocks the PR");
}
