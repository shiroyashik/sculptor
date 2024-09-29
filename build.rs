use std::process::Command;

fn main() {
    let output = match Command::new("git").args(&["rev-parse", "HEAD"]).output() {
        Ok(d) => d,
        Err(err) => {
            eprintln!("Can't run git and get last commit due: {err:?}");
            println!("cargo:rustc-env=GIT_HASH=0000000000000000000000000000000000000000");
            return
        }
    };
    let mut git_hash = String::from_utf8(output.stdout).unwrap();
    if &git_hash == "HEAD" || git_hash.len() < 7 {
        git_hash = String::from("0000000000000000000000000000000000000000");
    }
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
}