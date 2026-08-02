//! Command-line and executable path matching edge cases.

use path_rs::{
    CommandLinePathMatchOptions, ExecutableMatchOptions, PathError, command_line_contains_path,
    executable_paths_match,
};

fn default_cli() -> CommandLinePathMatchOptions {
    CommandLinePathMatchOptions {
        case_insensitive: true,
        normalize_separators: true,
        support_quotes: true,
        support_wsl_translation: false,
        match_basename: false,
        require_component_boundary: true,
    }
}

#[test]
fn slash_and_backslash_executables() {
    let o = ExecutableMatchOptions::default();
    assert!(
        executable_paths_match(
            r"C:\Program Files\XBP\xbp.exe",
            r"c:/program files/xbp/xbp.exe",
            o
        )
        .unwrap()
    );
    assert!(
        !executable_paths_match(
            r"C:\Program Files\XBP\xbp.exe",
            r"C:\Program Files\Other\xbp.exe",
            o
        )
        .unwrap()
    );
}

#[test]
fn verbatim_vs_ordinary() {
    let o = ExecutableMatchOptions {
        strip_verbatim_prefix: true,
        normalize_case: true,
        resolve_existing_symlinks: false,
    };
    // Lexical comparison of simplified forms.
    let a = r"\\?\C:\Program Files\XBP\xbp.exe";
    let b = r"C:\Program Files\XBP\xbp.exe";
    // On non-Windows, \\?\ is just a path string — still should not panic.
    let _ = executable_paths_match(a, b, o);
}

#[test]
fn case_only_differences() {
    let sensitive = ExecutableMatchOptions {
        normalize_case: false,
        ..ExecutableMatchOptions::default()
    };
    let a = r"C:\Tools\App.exe";
    let b = r"c:\tools\app.exe";
    // Without case normalize, identity still uses path separators normalize;
    // PlatformDefault case is off so Preserve — may differ.
    let _ = executable_paths_match(a, b, sensitive).unwrap();

    let insensitive = ExecutableMatchOptions {
        normalize_case: true,
        ..ExecutableMatchOptions::default()
    };
    assert!(executable_paths_match(a, b, insensitive).unwrap());
}

#[test]
fn quoted_double_and_single() {
    let o = default_cli();
    assert!(
        command_line_contains_path(
            r#"tool "C:\Users\Floris\repo" --flag"#,
            r"C:\Users\Floris\repo",
            o
        )
        .unwrap()
    );
    assert!(
        command_line_contains_path(
            r"tool 'C:\Users\Floris\repo' --flag",
            r"C:\Users\Floris\repo",
            o
        )
        .unwrap()
    );
}

#[test]
fn path_with_spaces() {
    let o = default_cli();
    assert!(
        command_line_contains_path(
            r#"run "C:\Program Files\My App\bin""#,
            r"C:\Program Files\My App\bin",
            o
        )
        .unwrap()
    );
}

#[test]
fn prefix_collision_rejected() {
    let o = default_cli();
    assert!(!command_line_contains_path(r"tool C:\repository\file", r"C:\repo", o).unwrap());
    assert!(command_line_contains_path(r"tool C:\repo\file", r"C:\repo", o).unwrap());
    assert!(!command_line_contains_path(r"tool C:\repoextra", r"C:\repo", o).unwrap());
}

#[test]
fn basename_match_fuzzy() {
    let o = CommandLinePathMatchOptions {
        match_basename: true,
        require_component_boundary: true,
        case_insensitive: true,
        ..default_cli()
    };
    assert!(
        command_line_contains_path(r"run mytool.exe --x", r"C:\Program Files\mytool.exe", o)
            .unwrap()
    );
}

#[test]
fn empty_target_errors() {
    let o = default_cli();
    assert!(matches!(
        command_line_contains_path("foo", "", o),
        Err(PathError::EmptyInput)
    ));
}

#[test]
fn empty_command_line_no_match() {
    let o = default_cli();
    assert!(!command_line_contains_path("", r"C:\repo", o).unwrap());
}

#[test]
fn wsl_form_in_command_line() {
    let o = CommandLinePathMatchOptions {
        support_wsl_translation: true,
        case_insensitive: true,
        ..default_cli()
    };
    assert!(
        command_line_contains_path(
            "tool /mnt/c/Users/Floris/repo",
            "/mnt/c/Users/Floris/repo",
            o
        )
        .unwrap()
    );
}

#[test]
fn substring_without_boundary_can_false_positive() {
    let loose = CommandLinePathMatchOptions {
        require_component_boundary: false,
        case_insensitive: true,
        match_basename: false,
        ..default_cli()
    };
    // Without boundaries, "repo" is a substring of "repository".
    assert!(command_line_contains_path(r"C:\repository", r"C:\repo", loose).unwrap());

    let strict = default_cli();
    assert!(!command_line_contains_path(r"C:\repository", r"C:\repo", strict).unwrap());
}

#[test]
fn forward_slash_in_cli() {
    let o = default_cli();
    assert!(
        command_line_contains_path(r"tool C:/Users/Floris/repo", r"C:\Users\Floris\repo", o)
            .unwrap()
    );
}
