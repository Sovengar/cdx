use crate::config;

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
