pub const EXCLUDE_DIRS: &[&str] = &[
    "node_modules", ".git", ".cache", "cache", "licenses",
    "vendor", "target", "build", "dist", "Modules", "modules",
    "lib", "platform",
];

pub const EXCLUDE_WIN_DIRS: &[&str] = &[
    "AppData", "ProgramData",
];

pub const EXCLUDE_PATH_GLOBS: &[&str] = &[
    "**/go/pkg/mod",
];

pub const PRIORITY_ROOTS: &[&str] = &["dev", ".config"];

pub const MAX_PRIORITY_DEPTH: usize = 6;
pub const MAX_SECONDARY_DEPTH: usize = 5;
