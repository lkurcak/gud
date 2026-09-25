use anyhow::{Context, Result, bail};
use std::process::{Command, Output};

fn git(args: &[&str]) -> Result<Output> {
    Command::new("git")
        .args(args)
        .output()
        .context("failed to run git")
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
}

/// Local branches, most recently committed first.
pub fn branches() -> Result<Vec<Branch>> {
    let out = stdout_of(&[
        "for-each-ref",
        "--sort=refname",
        "--sort=-committerdate",
        "--format=%(HEAD)%(refname:short)",
        "refs/heads/",
    ])?;
    Ok(out
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| {
            let (head, name) = l.split_at(1);
            Branch {
                name: name.to_string(),
                is_current: head == "*",
            }
        })
        .collect())
}

/// Runs `git branch -d/-D`, returning git's message and whether it succeeded.
pub fn delete_branch(name: &str, force: bool) -> Result<(bool, String)> {
    let flag = if force { "-D" } else { "-d" };
    let out = git(&["branch", flag, name])?;
    let text = if out.status.success() { &out.stdout } else { &out.stderr };
    Ok((
        out.status.success(),
        String::from_utf8_lossy(text).trim().to_string(),
    ))
}

/// Runs `git switch` with inherited stdio so the user sees git's output.
pub fn switch(name: &str) -> Result<i32> {
    let status = Command::new("git")
        .args(["switch", name])
        .status()
        .context("failed to run git")?;
    Ok(status.code().unwrap_or(1))
}
