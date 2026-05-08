use std::path::PathBuf;

pub fn query(query_str: &str) -> Option<PathBuf> {
    let output = std::process::Command::new("zoxide")
        .args(["query", query_str])
        .output()
        .ok()?;
    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Some(PathBuf::from(path));
        }
    }
    None
}

pub fn get_list() -> Vec<PathBuf> {
    std::process::Command::new("zoxide")
        .args(["query", "--list"])
        .output()
        .ok()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter(|l| !l.is_empty())
                .map(PathBuf::from)
                .collect()
        })
        .unwrap_or_default()
}
