use nix_build as nix;

fn main() {
    let outputs = nix::Config::new()
        .target_flake("nixpkgs#hello")
        .build()
        .expect("nix build to work");

    let hello = outputs[0].out().expect("default output to be available");

    let stdout = std::process::Command::new(format!("{}/bin/hello", hello.display().to_string()))
        .output()
        .expect("hello to succeed")
        .stdout;

    let stdout = std::str::from_utf8(&stdout).expect("stdout to be UTF-8");
    assert!(stdout == "Hello, world!\n");

    print!("{}", stdout);
}
