//! Expand user path input: tilde, `%VAR%`, `$VAR` / `${VAR}`, and the high-level API.

use path_rs::{
    ExpandOptions, expand_dollar_variables, expand_input, expand_percent_variables, expand_tilde,
};

fn main() -> Result<(), path_rs::PathError> {
    // --- expand_tilde ---
    let home = expand_tilde("~")?;
    println!("expand_tilde(\"~\")           => {home}");
    let projects = expand_tilde("~/projects")?;
    println!("expand_tilde(\"~/projects\")  => {projects}");

    // --- expand_percent_variables (Windows-style %VAR%) ---
    // Permissive: undefined tokens are left unchanged.
    let percent = expand_percent_variables("%TEMP%\\path-rs", false)?;
    println!("expand_percent_variables     => {percent}");

    // --- expand_dollar_variables (Unix-style $VAR / ${VAR}) ---
    let dollar = expand_dollar_variables("$HOME/work", false)?;
    println!("expand_dollar_variables      => {dollar}");

    // --- expand_input (combined pipeline via ExpandOptions) ---
    let opts = ExpandOptions::default();
    let full = expand_input("~", &opts)?;
    println!("expand_input(\"~\")            => {}", full.display());

    let opts_permissive = ExpandOptions {
        reject_undefined_variables: false,
        ..ExpandOptions::default()
    };
    let mixed = expand_input("~/projects", &opts_permissive)?;
    println!("expand_input(\"~/projects\")   => {}", mixed.display());

    // Disable all expansion.
    let none = ExpandOptions::none();
    let literal = expand_input("~", &none)?;
    println!("ExpandOptions::none()        => {}", literal.display());

    Ok(())
}
