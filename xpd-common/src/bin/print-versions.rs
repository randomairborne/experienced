use xpd_common::{CURRENT_GIT_REV_COUNT, CURRENT_GIT_SHA};
fn main() {
    println!("Commit number {CURRENT_GIT_REV_COUNT} commit {CURRENT_GIT_SHA}");
}
