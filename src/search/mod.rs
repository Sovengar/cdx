use std::collections::HashSet;
use std::path::PathBuf;
use std::io::Write;

use crate::config;

pub fn global_search(query: &str) -> anyhow::Result<()> {
    if query.is_empty() {
        eprintln!("[i] -g requires a query");
        return Ok(());
    }

    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("HOME not set"))?;
    let priority_roots: Vec<PathBuf> = config::get().priority_roots
        .iter()
        .map(|r| home.join(r))
        .filter(|p| p.exists())
        .collect();

    let content_matches = search_content(query, &priority_roots, &home)?;
    let filename_matches = search_filenames(query, &priority_roots, &home)?;
    let dirname_matches = search_dirnames(query, &priority_roots, &home)?;

    let all: Vec<String> = [content_matches, filename_matches, dirname_matches]
        .concat()
        .into_iter()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    if all.is_empty() {
        eprintln!("[i] No matches found for '{}'", query);
        return Ok(());
    }

    present_results(&all, query)
}

fn search_content(query: &str, priority: &[PathBuf], home: &PathBuf) -> anyhow::Result<Vec<String>> {
    let mut results = Vec::new();

    for root in priority {
        let mut cmd = std::process::Command::new("rg");
        cmd.args(["--files-with-matches", "--smart-case", "--hidden"]);
        cmd.args(build_exclude_args());
        cmd.args(["--max-depth", &config::get().max_priority_depth.to_string()]);
        cmd.arg(query).arg(root);

        if let Ok(out) = cmd.output() {
            if out.status.success() {
                for line in String::from_utf8_lossy(&out.stdout).lines() {
                    results.push(line.to_string());
                }
            }
        }
    }

    let mut cmd = std::process::Command::new("rg");
    cmd.args(["--files-with-matches", "--smart-case", "--hidden"]);
    cmd.args(build_exclude_args());
    cmd.args(["--max-depth", &config::get().max_secondary_depth.to_string()]);
    cmd.arg(query).arg(home);

    if let Ok(out) = cmd.output() {
        if out.status.success() {
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                let path = PathBuf::from(line);
                if !priority.iter().any(|p| path.starts_with(p)) {
                    results.push(line.to_string());
                }
            }
        }
    }

    Ok(results)
}

fn search_filenames(query: &str, priority: &[PathBuf], home: &PathBuf) -> anyhow::Result<Vec<String>> {
    let mut results = Vec::new();

    for root in priority {
        let mut cmd = std::process::Command::new("rg");
        cmd.args(["--files", "--hidden", "--smart-case"]);
        cmd.args(build_exclude_args());
        cmd.arg(root);

        if let Ok(out) = cmd.output() {
            let files = String::from_utf8_lossy(&out.stdout);
            for line in files.lines() {
                let path = PathBuf::from(line);
                if let Some(name) = path.file_name() {
                    let name = name.to_string_lossy().to_lowercase();
                    if name.contains(&query.to_lowercase()) {
                        results.push(line.to_string());
                    }
                }
            }
        }
    }

    let mut cmd = std::process::Command::new("rg");
    cmd.args(["--files", "--hidden", "--smart-case"]);
    cmd.args(build_exclude_args());
    cmd.arg(home);

    if let Ok(out) = cmd.output() {
        let files = String::from_utf8_lossy(&out.stdout);
        for line in files.lines() {
            let path = PathBuf::from(line);
            if priority.iter().any(|p| path.starts_with(p)) {
                continue;
            }
            if let Some(name) = path.file_name() {
                let name = name.to_string_lossy().to_lowercase();
                if name.contains(&query.to_lowercase()) {
                    results.push(line.to_string());
                }
            }
        }
    }

    Ok(results)
}

fn search_dirnames(query: &str, priority: &[PathBuf], home: &PathBuf) -> anyhow::Result<Vec<String>> {
    let mut results = Vec::new();
    let query_lower = query.to_lowercase();

    for root in priority {
        collect_matching_dirs(root, &query_lower, config::get().max_priority_depth, &mut results);
    }

    collect_matching_dirs(home, &query_lower, config::get().max_secondary_depth, &mut results);

    results.retain(|r| {
        let path = PathBuf::from(r);
        !priority.iter().any(|p| path.starts_with(p))
    });

    Ok(results)
}

fn collect_matching_dirs(root: &PathBuf, query: &str, max_depth: usize, results: &mut Vec<String>) {
    let mut builder = ignore::WalkBuilder::new(root);
    builder.max_depth(Some(max_depth));
    builder.hidden(false);
    builder.require_git(false);
    builder.filter_entry(|entry| {
        let name = entry.file_name().to_string_lossy();
        if config::get().exclude_dirs.contains(&name.as_ref()) {
            return false;
        }
        true
    });

    for result in builder.build() {
        if let Ok(entry) = result {
            if entry.file_type().map_or(false, |ft| ft.is_dir()) {
                if let Some(name) = entry.file_name().to_str() {
                    if name.to_lowercase().contains(query) {
                        let path_str = entry.path().to_string_lossy().to_string();
                        results.push(path_str);
                    }
                }
            }
        }
    }
}

fn build_exclude_args() -> Vec<String> {
    let cfg = config::get();
    let mut args = Vec::new();
    for d in &cfg.exclude_dirs {
        args.push("--glob".into());
        args.push(format!("!{}", d));
    }
    for d in &cfg.exclude_win_dirs {
        args.push("--glob".into());
        args.push(format!("!{}", d));
    }
    for p in &cfg.exclude_path_globs {
        args.push("--glob".into());
        args.push(format!("!{}", p));
    }
    args
}

fn present_results(results: &[String], query: &str) -> anyhow::Result<()> {
    let input = results.join("\n");

    let mut child = std::process::Command::new("fzf")
        .args(["--query", query, "--select-1", "--exit-0"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input.as_bytes())?;
    }

    let output = child.wait_with_output()?;
    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            println!("{}", path);
        }
    }
    Ok(())
}
