use vergen::{generate_cargo_keys, ConstantsFlags};

fn main() {
    println!("cargo:rerun-if-changed=.env",);
    let dotpath = dotenvy::dotenv();

    if dotpath.is_ok() {
        for env_var in dotenvy::dotenv_iter().unwrap() {
            let (key, value) = env_var.unwrap();
            println!("cargo:rustc-env={key}={value}");
        }
    }

    let flags = ConstantsFlags::empty().union(ConstantsFlags::SHA_SHORT);
    generate_cargo_keys(flags).expect("Unable to generate the cargo keys!");
}
