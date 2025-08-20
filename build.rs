use vergen_git2::{Emitter, Git2Builder};

fn main() {
    println!("cargo:rerun-if-changed=.env",);
    let dotpath = dotenvy::dotenv();

    if dotpath.is_ok() {
        for env_var in dotenvy::dotenv_iter().unwrap() {
            let (key, value) = env_var.unwrap();
            println!("cargo:rustc-env={key}={value}");
        }
    }

    let flags = Git2Builder::default().sha(true).build().unwrap();

    Emitter::default()
        .add_instructions(&flags)
        .unwrap()
        .emit()
        .unwrap();
}
