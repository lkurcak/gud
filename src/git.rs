use anyhow::{Context, Result, bail};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn git(args: &[&str]) -> Result<Output> {
    Command::new("git")
        .args(args)
        .output()
        .context("failed to run git")
}

fn git_with_input(args: &[&str], input: &[u8]) -> Result<Output> {
    let mut child = Command::new("git")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to run git")?;
    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(input)?;
    Ok(child.wait_with_output()?)
}

fn stdout_of(args: &[&str]) -> Result<String> {
    let out = git(args)?;
    if !out.status.success() {
        bail!("{}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

pub struct Branch {
    pub name: String,
    pub is_current: bool,
    /// Checked out in another worktree.
    pub is_worktree: bool,
    /// For a remote-only branch, the remote ref (e.g. `origin/foo`); `name` is the local name to create.
    pub remote: Option<String>,
}

/// Local branches, most recently committed first. With `include_remote`, followed by
/// remote branches without a local counterpart, also most recently committed first.
pub fn branches(include_remote: bool) -> Result<Vec<Branch>> {
    let mut all = local_branches()?;
    if !include_remote {
        return Ok(all);
    }
    let remotes = stdout_of(&[
        "for-each-ref",
        "--sort=refname",
        "--sort=-committerdate",
        "--format=%(refname)",
        "refs/remotes/",
    ])?;
    let mut seen: std::collections::HashSet<String> = all.iter().map(|b| b.name.clone()).collect();
    for full in remotes.lines() {
        let Some(short) = full.strip_prefix("refs/remotes/") else {
            continue;
        };
        let Some((_, name)) = short.split_once('/') else {
            continue;
        };
        if name == "HEAD" || !seen.insert(name.to_string()) {
            continue;
        }
        all.push(Branch {
            name: name.to_string(),
            is_current: false,
            is_worktree: false,
            remote: Some(short.to_string()),
        });
    }
    Ok(all)
}

fn local_branches() -> Result<Vec<Branch>> {
    let out = stdout_of(&[
        "for-each-ref",
        "--sort=refname",
        "--sort=-committerdate",
        "--format=%(HEAD)%(worktreepath)\t%(refname:short)",
        "refs/heads/",
    ])?;
    Ok(out
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(|l| {
            let (head, rest) = l.split_at(1);
            let (worktree, name) = rest.split_once('\t')?;
            let is_current = head == "*";
            Some(Branch {
                name: name.to_string(),
                is_current,
                is_worktree: !is_current && !worktree.is_empty(),
                remote: None,
            })
        })
        .collect())
}

/// The local default branch: what `origin/HEAD` points to, else `main` or `master`.
fn default_branch() -> Option<String> {
    let exists = |name: &str| {
        git(&[
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{name}"),
        ])
        .is_ok_and(|o| o.status.success())
    };
    let remote = stdout_of(&["symbolic-ref", "--short", "refs/remotes/origin/HEAD"])
        .ok()
        .and_then(|s| s.trim().strip_prefix("origin/").map(str::to_string));
    remote
        .into_iter()
        .chain(["main", "master"].map(String::from))
        .find(|name| exists(name))
}

fn failure(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).trim().to_string()
}

/// Deletes `name`. If it is the checked-out branch, first switches to the
/// default branch, switching back if the deletion is then refused.
pub fn delete_branch(name: &str, is_current: bool, force: bool) -> Result<(bool, String)> {
    if !is_current {
        return run_delete(name, force);
    }
    let Some(default) = default_branch() else {
        return Ok((
            false,
            "Cannot delete the current branch: no default branch found".into(),
        ));
    };
    if default == name {
        return Ok((
            false,
            format!("Cannot delete the default branch '{default}' while it is checked out"),
        ));
    }
    let out = git(&["switch", "--quiet", &default])?;
    if !out.status.success() {
        return Ok((false, failure(&out)));
    }
    let (ok, msg) = run_delete(name, force)?;
    if ok {
        return Ok((true, format!("Switched to '{default}'. {msg}")));
    }
    // Leave the user where they started rather than stranded on another branch.
    let back = git(&["switch", "--quiet", name])?;
    if !back.status.success() {
        return Ok((
            false,
            format!("{msg} (and failed to switch back: {})", failure(&back)),
        ));
    }
    Ok((false, msg))
}

/// Runs `git branch -d/-D`, returning git's message and whether it succeeded.
fn run_delete(name: &str, force: bool) -> Result<(bool, String)> {
    let flag = if force { "-D" } else { "-d" };
    let out = git(&["branch", flag, name])?;
    let text = if out.status.success() {
        &out.stdout
    } else {
        &out.stderr
    };
    Ok((
        out.status.success(),
        String::from_utf8_lossy(text).trim().to_string(),
    ))
}

/// Runs `git switch` with inherited stdio so the user sees git's output.
/// For a remote-only branch, `remote` creates a local branch tracking it.
pub fn switch(name: &str, remote: Option<&str>) -> Result<i32> {
    let mut cmd = Command::new("git");
    cmd.arg("switch");
    match remote {
        Some(r) => cmd.args(["--track", r]),
        None => cmd.arg(name),
    };
    let status = cmd.status().context("failed to run git")?;
    Ok(status.code().unwrap_or(1))
}

pub struct Commit {
    pub hash: String,
    pub short: String,
    pub refs: String,
    pub subject: String,
}

/// Up to `limit` commits reachable from HEAD, newest first.
pub fn commits(limit: usize) -> Result<Vec<Commit>> {
    let out = stdout_of(&[
        "log",
        "--format=%H%x1f%h%x1f%D%x1f%s%x1e",
        &format!("-n{limit}"),
        "HEAD",
        "--",
    ])?;
    Ok(out
        .split('\x1e')
        .map(|r| r.trim_start_matches('\n'))
        .filter(|r| !r.is_empty())
        .filter_map(|r| {
            let mut f = r.splitn(4, '\x1f');
            Some(Commit {
                hash: f.next()?.to_string(),
                short: f.next()?.to_string(),
                refs: f.next()?.to_string(),
                subject: f.next()?.to_string(),
            })
        })
        .collect())
}

/// Header, message and diffstat of a commit, sized for a pane `width` columns wide.
pub fn preview(hash: &str, width: usize) -> Result<String> {
    stdout_of(&[
        "show",
        "--no-color",
        "--no-ext-diff",
        &format!("--stat={}", width.max(20)),
        "--format=commit %H%nAuthor: %an <%ae>%nDate:   %ad%n%n%B",
        hash,
        "--",
    ])
}

/// Runs `git show` with inherited stdio so it goes through the user's pager.
pub fn show(hash: &str) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.args(["show", hash, "--"]);
    // git defaults to LESS=FRX, which exits immediately on short output and
    // would drop straight back into the TUI before the diff can be read.
    if std::env::var_os("LESS").is_none() {
        cmd.env("LESS", "R");
    }
    cmd.status().context("failed to run git")?;
    Ok(())
}

/// Runs `git reset --soft/--hard`, returning git's message and whether it succeeded.
pub fn reset(hash: &str, hard: bool) -> Result<(bool, String)> {
    let flag = if hard { "--hard" } else { "--soft" };
    let out = git(&["reset", flag, hash])?;
    let text = if out.status.success() {
        &out.stdout
    } else {
        &out.stderr
    };
    Ok((
        out.status.success(),
        String::from_utf8_lossy(text).trim().to_string(),
    ))
}

/// Runs `git tag`, returning git's message and whether it succeeded.
/// With `message_file`, creates an annotated tag whose message is read from it.
pub fn tag(name: &str, hash: &str, message_file: Option<&Path>) -> Result<(bool, String)> {
    let mut args = vec!["tag".to_string()];
    if let Some(path) = message_file {
        args.extend(["-a".into(), "-F".into(), path.display().to_string()]);
    }
    args.extend([name.to_string(), hash.to_string()]);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let out = git(&args)?;
    Ok((
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).trim().to_string(),
    ))
}

/// Full commit message, re-encoded to UTF-8.
pub fn message(hash: &str) -> Result<String> {
    stdout_of(&["log", "-1", "--format=%B", hash, "--"])
}

/// Path of a file inside the repository's git directory.
pub fn git_path(name: &str) -> Result<PathBuf> {
    Ok(PathBuf::from(
        stdout_of(&["rev-parse", "--git-path", name])?.trim(),
    ))
}

/// The character git treats as starting a comment line in commit messages.
pub fn comment_char() -> char {
    stdout_of(&["config", "--get", "core.commentChar"])
        .ok()
        .and_then(|s| {
            let s = s.trim();
            let mut chars = s.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => Some(c),
                _ => None,
            }
        })
        .unwrap_or('#')
}

/// Cleans up a message the way `git commit` does: strips comments and excess whitespace.
pub fn stripspace(text: &str) -> Result<String> {
    let out = git_with_input(&["stripspace", "--strip-comments"], text.as_bytes())?;
    if !out.status.success() {
        bail!("{}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Opens `path` in the user's configured git editor, returning whether it exited cleanly.
pub fn edit_file(path: &Path) -> Result<bool> {
    let editor = stdout_of(&["var", "GIT_EDITOR"])?.trim().to_string();
    // Like git, run the editor through a shell so values such as `code --wait` work.
    #[cfg(not(windows))]
    let mut cmd = {
        let mut c = Command::new("sh");
        c.arg("-c")
            .arg(format!("{editor} \"$@\""))
            .arg(&editor)
            .arg(path);
        c
    };
    #[cfg(windows)]
    let mut cmd = {
        use std::os::windows::process::CommandExt;
        let mut c = Command::new("cmd");
        c.arg("/C")
            .raw_arg(format!("{editor} \"{}\"", path.display()));
        c
    };
    Ok(cmd.status().context("failed to run editor")?.success())
}

/// Replaces the message of `hash` and re-creates every commit between it and HEAD
/// on top of the result, then moves HEAD (and the branch it points to) there.
///
/// Only messages change, never trees, so this cannot conflict, handles merges,
/// and leaves the index and working tree alone. Returns the new hash of the
/// reworded commit and the number of commits rewritten.
pub fn reword(hash: &str, message: &str) -> Result<(String, usize)> {
    let head = stdout_of(&["rev-parse", "HEAD"])?.trim().to_string();
    let descendants = stdout_of(&[
        "rev-list",
        "--topo-order",
        "--reverse",
        "--ancestry-path",
        &format!("{hash}..{head}"),
    ])?;

    let mut rewritten = HashMap::new();
    let new_hash = rewrite_commit(hash, &rewritten, Some(message))?;
    rewritten.insert(hash.to_string(), new_hash.clone());
    for old in descendants.lines().filter(|l| !l.is_empty()) {
        let new = rewrite_commit(old, &rewritten, None)?;
        rewritten.insert(old.to_string(), new);
    }

    let new_head = rewritten
        .get(&head)
        .context("HEAD is not a descendant of the reworded commit")?;
    let short = &hash[..hash.len().min(7)];
    let out = git(&[
        "update-ref",
        "-m",
        &format!("gud: reword {short}"),
        "HEAD",
        new_head,
        &head,
    ])?;
    if !out.status.success() {
        bail!("{}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok((new_hash, rewritten.len()))
}

/// Writes a copy of commit `hash` with parents mapped through `rewritten` and,
/// if given, a new message. Signatures are dropped since they no longer match.
fn rewrite_commit(
    hash: &str,
    rewritten: &HashMap<String, String>,
    message: Option<&str>,
) -> Result<String> {
    let out = git(&["cat-file", "commit", hash])?;
    if !out.status.success() {
        bail!("{}", String::from_utf8_lossy(&out.stderr).trim());
    }
    let raw = out.stdout;
    let (header, body) = raw
        .windows(2)
        .position(|w| w == b"\n\n")
        .map_or_else(|| (&raw[..], &[][..]), |i| (&raw[..i], &raw[i + 2..]));

    let mut new = Vec::with_capacity(raw.len());
    let mut in_signature = false;
    for line in header.split(|&b| b == b'\n') {
        // Multi-line headers continue on lines starting with a space.
        if line.starts_with(b" ") {
            if !in_signature {
                new.extend_from_slice(line);
                new.push(b'\n');
            }
            continue;
        }
        in_signature = line.starts_with(b"gpgsig");
        if in_signature || (message.is_some() && line.starts_with(b"encoding ")) {
            continue;
        }
        let parent = line
            .strip_prefix(b"parent ")
            .and_then(|p| rewritten.get(std::str::from_utf8(p).ok()?));
        match parent {
            Some(p) => new.extend_from_slice(format!("parent {p}").as_bytes()),
            None => new.extend_from_slice(line),
        }
        new.push(b'\n');
    }
    new.push(b'\n');
    new.extend_from_slice(message.map_or(body, str::as_bytes));

    let out = git_with_input(&["hash-object", "-t", "commit", "-w", "--stdin"], &new)?;
    if !out.status.success() {
        bail!("{}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}
