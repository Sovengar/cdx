use std::path::{Path, PathBuf};

use crate::config;
use crate::walker::DirEntryItem;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct GrepMatch {
    pub file_path: String,
    pub line_number: u32,
    pub match_text: String,
}

pub fn run_rg_command(
    root: &std::path::Path,
    query: &str,
    show_winhidden: bool,
    show_dotfiles: bool,
    json: bool,
) -> std::process::Output {
    let mut cmd = std::process::Command::new("rg");
    if json {
        cmd.arg("--json");
    } else {
        cmd.arg("--vimgrep");
    }
    cmd.args(["--smart-case"]);
    cmd.args(["--max-depth", &config::get().grep_max_depth.to_string()]);

    for d in config::get().exclude_dirs.iter() {
        cmd.args(["--glob", &format!("!{}", d)]);
    }
    for p in config::get().exclude_path_globs.iter() {
        cmd.args(["--glob", &format!("!{}", p)]);
    }
    if !show_winhidden {
        for d in config::get().exclude_win_dirs.iter() {
            cmd.args(["--glob", &format!("!{}", d)]);
        }
    }
    if !show_dotfiles {
        cmd.args(["--glob", "!.*"]);
    }

    cmd.arg(query).arg(root);

    cmd.output()
        .unwrap_or_else(|_| std::process::Output {
            status: std::process::ExitStatus::default(),
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
}

pub fn parse_rg_json(stdout: &str) -> Vec<GrepMatch> {
    stdout
        .lines()
        .filter_map(|line| {
            if line.is_empty() || !line.starts_with('{') {
                return None;
            }
            let v: serde_json::Value = serde_json::from_str(line).ok()?;
            if v.get("type")?.as_str()? != "match" {
                return None;
            }
            let data = v.get("data")?;
            let file_path = data.get("path")?.get("text")?.as_str()?.to_string();
            let line_number = data.get("line_number")?.as_u64()? as u32;
            let match_text = data.get("lines")?.get("text")?.as_str()?.trim_end().to_string();
            Some(GrepMatch {
                file_path,
                line_number,
                match_text,
            })
        })
        .collect()
}

pub fn parse_rg_vimgrep(stdout: &str) -> Vec<GrepMatch> {
    stdout
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(4, ':').collect();
            if parts.len() < 4 {
                return None;
            }
            Some(GrepMatch {
                file_path: parts[0].to_string(),
                line_number: parts[1].parse().unwrap_or(0),
                match_text: parts[3].to_string(),
            })
        })
        .collect()
}

pub fn to_dir_entries(results: &[GrepMatch], root: &Path) -> Vec<DirEntryItem> {
    results
        .iter()
        .map(|m| {
            let full_path = if Path::new(&m.file_path).is_absolute() {
                PathBuf::from(&m.file_path)
            } else {
                root.join(&m.file_path)
            };
            let display_path = if let Ok(rel) = Path::new(&m.file_path).strip_prefix(root) {
                rel.to_string_lossy().replace('\\', "/")
            } else {
                m.file_path.clone()
            };
            DirEntryItem {
                display: format!("{}:{}", display_path, m.line_number),
                rel_path: full_path.to_string_lossy().replace('\\', "/"),
                full_path,
                is_zoxide: false,
                is_dir: false,
            }
        })
        .collect()
}

pub fn execute_search(
    query: &str,
    root: &Path,
    show_winhidden: bool,
    show_dotfiles: bool,
) -> (Vec<GrepMatch>, Vec<DirEntryItem>) {
    let output = run_rg_command(root, query, show_winhidden, show_dotfiles, true);

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Detect if rg supports --json: first non-empty line should start with '{'
    let is_json = stdout.lines().find(|l| !l.is_empty()).is_some_and(|l| l.starts_with('{'));

    let results = if is_json {
        parse_rg_json(&stdout)
    } else {
        // Fallback: rg < 13.0.0 doesn't support --json, re-run with --vimgrep
        let fallback = run_rg_command(root, query, show_winhidden, show_dotfiles, false);
        let fallback_stdout = String::from_utf8_lossy(&fallback.stdout);
        parse_rg_vimgrep(&fallback_stdout)
    };

    let items = to_dir_entries(&results, root);
    (results, items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rg_json_valid_match() {
        let json = r#"{"type":"match","data":{"path":{"text":"src/main.rs"},"line_number":42,"lines":{"text":"    fn main() {\n"}}}"#;
        let results = parse_rg_json(json);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].file_path, "src/main.rs");
        assert_eq!(results[0].line_number, 42);
        assert!(results[0].match_text.contains("fn main()"));
    }

    #[test]
    fn test_parse_rg_json_multiple_matches() {
        let json = r#"{"type":"match","data":{"path":{"text":"a.rs"},"line_number":1,"lines":{"text":"line1"}}}
{"type":"match","data":{"path":{"text":"b.rs"},"line_number":5,"lines":{"text":"line2"}}}"#;
        let results = parse_rg_json(json);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_parse_rg_json_ignores_non_match_lines() {
        let json = r#"{"type":"begin","data":{"path":{"text":"a.rs"}}}
{"type":"match","data":{"path":{"text":"a.rs"},"line_number":1,"lines":{"text":"found it"}}}
{"type":"end","data":{"path":{"text":"a.rs"}}}"#;
        let results = parse_rg_json(json);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].match_text, "found it");
    }

    #[test]
    fn test_parse_rg_json_empty() {
        assert!(parse_rg_json("").is_empty());
    }

    #[test]
    fn test_parse_rg_json_invalid() {
        assert!(parse_rg_json("not json at all").is_empty());
    }

    #[test]
    fn test_parse_rg_vimgrep_valid() {
        let vimgrep = "src/main.rs:42:10:    fn main() {";
        let results = parse_rg_vimgrep(vimgrep);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].file_path, "src/main.rs");
        assert_eq!(results[0].line_number, 42);
    }

    #[test]
    fn test_parse_rg_vimgrep_multiple() {
        let vimgrep = "a.rs:1:0:line1\nb.rs:5:0:line2";
        let results = parse_rg_vimgrep(vimgrep);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_parse_rg_vimgrep_empty() {
        assert!(parse_rg_vimgrep("").is_empty());
    }

    #[test]
    fn test_parse_rg_vimgrep_invalid_line() {
        assert!(parse_rg_vimgrep("not-a-vimgrep-line").is_empty());
    }

    #[test]
    fn test_to_dir_entries_relative_path() {
        let root = Path::new("/home/user/project");
        let matches = vec![
            GrepMatch {
                file_path: "src/main.rs".to_string(),
                line_number: 10,
                match_text: "fn main()".to_string(),
            },
        ];
        let entries = to_dir_entries(&matches, root);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].display, "src/main.rs:10");
        assert!(entries[0].full_path.to_string_lossy().contains("src/main.rs"));
        assert!(!entries[0].is_dir);
    }

    #[test]
    fn test_to_dir_entries_absolute_path() {
        let root = Path::new("/home/user/project");
        let matches = vec![
            GrepMatch {
                file_path: "/home/user/project/lib.rs".to_string(),
                line_number: 5,
                match_text: "code".to_string(),
            },
        ];
        let entries = to_dir_entries(&matches, root);
        assert_eq!(entries[0].display, "lib.rs:5");
    }

    #[test]
    fn test_execute_search_real_file() {
        config::init();
        let root = std::env::temp_dir().join(format!("cdx-grep-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("test.txt"), "hello world\nfoo bar\nhello again\n").unwrap();

        let (results, items) = execute_search("hello", &root, true, true);
        assert_eq!(results.len(), 2, "should find 2 matches for 'hello'");
        assert_eq!(items.len(), 2);
        assert!(items[0].display.contains("test.txt:"));
        assert!(!items[0].is_dir);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn test_execute_search_no_matches() {
        config::init();
        let root = std::env::temp_dir().join(format!("cdx-grep-nomatch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("test.txt"), "hello world\n").unwrap();

        let (results, _) = execute_search("NONEXISTENT_STRING_XYZ", &root, true, true);
        assert!(results.is_empty());

        let _ = std::fs::remove_dir_all(&root);
    }
}
