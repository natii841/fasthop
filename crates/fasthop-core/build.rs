use std::{fs, path::Path};

fn main() {
    let dist = Path::new("../../dashboard/dist");
    if !dist.exists() {
        fs::create_dir_all(dist).expect("create dashboard placeholder directory");
        fs::write(
            dist.join("index.html"),
            "<!doctype html><title>FastHop</title><p>dashboard not built</p>",
        )
        .expect("write dashboard placeholder");
    }
    println!("cargo:rerun-if-changed=../../dashboard/dist");
}
