---
name: cdx
description: "Trigger: cdx, directory navigation, fuzzy directory jump, zoxide, content search, cdx wrapper, or cdx config. Operates and configures cdx."
---

## Activation Contract

Load for cdx navigation, search, wrappers, or configuration.

## Hard Rules

- cdx prints a path; it cannot change the caller shell directory.
- Resolve an argument as a path, then a zoxide query, then a TUI query.
- Use `-g` for content search. It requires `rg`.

## Execution Steps

1. Run `cdx`, `cdx <target>`, or `cdx -g <query>`.
2. Capture stdout when a script must change directory.
3. Use the shell wrapper to run `cd`.
4. Edit `~/.config/cdx/config.toml` for configuration.

## Output Contract

Report the command, resolved path or results, and wrapper requirement.

