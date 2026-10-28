use std::{
    fs,
    path::{Path, PathBuf},
};

/// This file, which assembles the forbidden strings instead of spelling them out.
const GUARD_SOURCE: &str = "crates/quboid-core/tests/source_separation.rs";

/// Files allowed to name the legacy document sections, because they migrate or
/// test documents written by older builds.
const SECTION_NAME_ALLOWLIST: [&str; 2] = [
    "crates/quboid-core/src/config.rs",
    "crates/quboid-core/tests/config_public.rs",
];

/// Directories that hold build output or checkout metadata rather than sources.
const IGNORED_DIRECTORIES: [&str; 4] = [".git", ".idea", "dist", "target"];

/// The file kinds published with this repository.
const SCANNED_EXTENSIONS: [&str; 10] = [
    "json", "manifest", "md", "ps1", "rc", "rs", "toml", "wxs", "yaml", "yml",
];

/// Files the guard must reach: narrowing its reach has to fail the test instead
/// of quietly stopping the check.
const REQUIRED_COVERAGE: [&str; 9] = [
    "Cargo.toml",
    "README.md",
    "docs/BUILDING.md",
    "docs/RELEASING.md",
    "crates/quboid-app/quboid.rc",
    "crates/quboid-core/src/config.rs",
    "packaging/wix/Quboid.wxs",
    "scripts/package.ps1",
    GUARD_SOURCE,
];

/// Assembled at compile time so this guard never matches its own source.
fn forbidden_names() -> [&'static str; 10] {
    [
        concat!("Layout", "Config"),
        concat!("Zone", "Config"),
        concat!("Application", "Rule"),
        concat!("Action", "Cycle"),
        concat!("layouts", "_page"),
        concat!("applications", "_page"),
        concat!("zone", "_editor"),
        concat!("layout", "_canvas"),
        concat!("Apply", "Zone"),
        concat!("Zone", "Applied"),
    ]
}

/// Assembled the same way: the sections a pre-split document stored beside the
/// Base settings.
fn legacy_section_names() -> [&'static str; 2] {
    [
        concat!("action", "_cycles"),
        concat!("application", "_rules"),
    ]
}

const FORBIDDEN_NETWORK_CRATES: [&str; 17] = [
    "curl",
    "h2",
    "hyper",
    "isahc",
    "mio",
    "native-tls",
    "openssl",
    "reqwest",
    "rustls",
    "socket2",
    "surf",
    "tokio",
    "tungstenite",
    "ureq",
    "webpki",
    "winhttp",
    "ws2_32-sys",
];

#[test]
fn the_dependency_tree_contains_no_networking_crates() {
    let root = workspace_root();
    let lockfile = fs::read_to_string(root.join("Cargo.lock")).expect("Cargo.lock");
    for crate_name in FORBIDDEN_NETWORK_CRATES {
        let pattern = format!("name = \"{crate_name}\"");
        assert!(
            !lockfile.contains(&pattern),
            "Cargo.lock contains networking crate {crate_name}"
        );
    }

    let manifest = fs::read_to_string(root.join("Cargo.toml")).expect("Cargo.toml");
    assert!(
        !manifest.contains("Win32_Networking"),
        "Cargo.toml enables Win32_Networking features"
    );
}

#[test]
fn public_sources_contain_no_separately_licensed_types_or_pages() {
    let mut failures = Vec::new();
    for file in workspace_sources() {
        let relative = relative_path(&file);
        let contents = fs::read_to_string(&file).unwrap_or_default();
        for needle in forbidden_names() {
            if contents.contains(needle) {
                failures.push(format!("{relative} still mentions {needle}"));
            }
        }
    }

    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn only_the_migration_path_names_legacy_document_sections() {
    let mut failures = Vec::new();
    for file in workspace_sources() {
        let relative = relative_path(&file);
        if SECTION_NAME_ALLOWLIST.contains(&relative.as_str()) {
            continue;
        }
        let contents = fs::read_to_string(&file).unwrap_or_default();
        for needle in legacy_section_names() {
            if contents.contains(needle) {
                failures.push(format!("{relative} still mentions {needle}"));
            }
        }
    }

    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn the_guard_reads_sources_documentation_scripts_and_packaging() {
    let scanned = workspace_sources()
        .iter()
        .map(|file| relative_path(file))
        .collect::<Vec<_>>();

    for required in REQUIRED_COVERAGE {
        assert!(
            scanned.iter().any(|file| file == required),
            "the guard no longer reads {required}"
        );
    }
    assert!(
        !scanned.iter().any(|file| {
            IGNORED_DIRECTORIES
                .iter()
                .any(|ignored| file.split('/').any(|segment| segment == *ignored))
        }),
        "the guard read build output or checkout metadata"
    );
}

#[test]
fn the_guard_never_matches_the_strings_it_assembles() {
    let source = fs::read_to_string(workspace_root().join(GUARD_SOURCE)).unwrap();

    for needle in forbidden_names().into_iter().chain(legacy_section_names()) {
        assert!(
            !source.contains(needle),
            "the guard's own source spells out {needle}"
        );
    }
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root")
}

fn relative_path(file: &Path) -> String {
    file.strip_prefix(workspace_root())
        .unwrap_or(file)
        .to_string_lossy()
        .replace('\\', "/")
}

fn workspace_sources() -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect(&workspace_root(), &mut files);
    assert!(
        files.len() >= 25,
        "the guard scanned only {} files; the repository layout changed",
        files.len()
    );
    files
}

fn collect(directory: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path
                .file_name()
                .is_some_and(|name| IGNORED_DIRECTORIES.iter().any(|ignored| name == *ignored))
            {
                continue;
            }
            collect(&path, files);
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| SCANNED_EXTENSIONS.contains(&extension))
        {
            files.push(path);
        }
    }
}
