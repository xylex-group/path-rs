//! Expand user path input (`~`, env vars).

use path_rs::{ExpandOptions, expand_input};

fn main() -> Result<(), path_rs::PathError> {
    let opts = ExpandOptions::default();
    let home = expand_input("~", &opts)?;
    println!("home = {}", home.display());

    let opts_no_undef = ExpandOptions {
        reject_undefined_variables: false,
        ..ExpandOptions::default()
    };
    let mixed = expand_input("~/projects", &opts_no_undef)?;
    println!("~/projects => {}", mixed.display());

    Ok(())
}
