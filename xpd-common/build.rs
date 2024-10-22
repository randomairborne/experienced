use git2::Repository;

fn main() {
    let head_hash = Repository::open_from_env()
        .unwrap()
        .head()
        .unwrap()
        .peel_to_commit()
        .unwrap()
        .id()
        .to_string();
    println!("cargo:rustc-env=GIT_HASH_EXPERIENCED={}", head_hash);
}
