use crate::domain::{
    EngineInfo, EngineSource, GitDiff, GitFileStatus, GitStatus, IntegrationReadiness,
    ProjectSummary, SkillSummary,
};
use std::{
    collections::BTreeSet,
    ffi::OsStr,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

const RUBY_PROBE_TIMEOUT: Duration = Duration::from_secs(5);
use thiserror::Error;

const MAX_DIFF_BYTES: usize = 1_048_576;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("Project path must be an existing directory: {0}")]
    InvalidProject(String),
    #[error("Unable to inspect project: {0}")]
    Io(#[from] std::io::Error),
    #[error("Git is not available or the directory is not a Git repository")]
    GitUnavailable,
    #[error("Unable to manage the isolated Git worktree: {0}")]
    Worktree(String),
    #[error("Agent launch blocked by an untrusted project feature: {0}")]
    UntrustedProject(String),
    #[error("Unable to integrate the isolated worktree: {0}")]
    Integration(String),
}

pub struct WorktreeAllocation {
    pub path: PathBuf,
    pub base_commit: String,
}

pub fn create_isolated_worktree(
    project: &Path,
    runs_root: &Path,
) -> Result<WorktreeAllocation, ServiceError> {
    let root = git_root(project)?;
    fs::create_dir_all(runs_root)?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    // The stable `workspace` leaf prevents Rubyn's legacy lexical path check
    // from sharing a prefix with another run directory.
    let worktree = runs_root
        .join(format!("run-{}-{stamp}", std::process::id()))
        .join("workspace");
    if let Some(container) = worktree.parent() {
        fs::create_dir_all(container)?;
    }
    let output = git(
        &root,
        [
            OsStr::new("worktree"),
            OsStr::new("add"),
            OsStr::new("--detach"),
            worktree.as_os_str(),
            OsStr::new("HEAD"),
        ],
    )?;
    if !output.status.success() {
        return Err(ServiceError::Worktree(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    let worktree = fs::canonicalize(worktree).map_err(ServiceError::Io)?;
    if let Err(error) = ensure_agent_safe_project(&worktree) {
        let _ = remove_isolated_worktree(&root, &worktree, runs_root);
        return Err(error);
    }
    let base_commit = git_text(&worktree, ["rev-parse", "HEAD"])?;
    Ok(WorktreeAllocation {
        path: worktree,
        base_commit,
    })
}

pub fn remove_isolated_worktree(
    project: &Path,
    worktree: &Path,
    runs_root: &Path,
) -> Result<(), ServiceError> {
    let worktree = validate_managed_worktree(worktree, runs_root)?;
    let root = git_root(project)?;
    let output = git(
        &root,
        [
            OsStr::new("worktree"),
            OsStr::new("remove"),
            OsStr::new("--force"),
            worktree.as_os_str(),
        ],
    )?;
    if !output.status.success() {
        let registered = git(&root, ["worktree", "list", "--porcelain"])?;
        let listed = registered.status.success()
            && String::from_utf8_lossy(&registered.stdout)
                .lines()
                .filter_map(|line| line.strip_prefix("worktree "))
                .any(|path| Path::new(path) == worktree);
        if listed || !registered.status.success() {
            return Err(ServiceError::Worktree(
                String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            ));
        }
        fs::remove_dir_all(&worktree).map_err(|error| {
            ServiceError::Worktree(format!(
                "Git deregistered the worktree, but its managed directory could not be removed: {error}"
            ))
        })?;
    }
    if let Some(container) = worktree.parent() {
        let _ = fs::remove_dir(container);
    }
    Ok(())
}

pub fn inspect_isolated_worktree(
    worktree: &Path,
    runs_root: &Path,
    base_commit: &str,
) -> Result<(GitStatus, GitDiff), ServiceError> {
    validate_git_oid(base_commit)?;
    let worktree = validate_managed_worktree(worktree, runs_root)?;
    let path = worktree.to_string_lossy();
    let mut status = git_status(&path)?;
    let committed = git(&worktree, ["diff", "--name-only", "-z", base_commit, "--"])?;
    if !committed.status.success() {
        return Err(ServiceError::Worktree(git_error(&committed)));
    }
    for path in committed
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|path| String::from_utf8_lossy(path).into_owned())
    {
        if !status.files.iter().any(|file| file.path == path) {
            status.files.push(GitFileStatus {
                path,
                index_status: "C".into(),
                worktree_status: " ".into(),
            });
        }
    }
    Ok((status, git_diff_from_base(&worktree, base_commit)?))
}

pub fn inspect_integration_readiness(
    project: &Path,
    base_commit: &str,
) -> Result<IntegrationReadiness, ServiceError> {
    validate_git_oid(base_commit)?;
    let root = git_root(project)?;
    let source_head = git_text(&root, ["rev-parse", "HEAD"])?;
    let source_clean = git_status(root.to_string_lossy().as_ref())?
        .files
        .is_empty();
    let source_matches_base = source_head == base_commit;
    let mut blockers = Vec::new();
    if !source_clean {
        blockers.push("The source repository has uncommitted changes. Commit or stash them, then refresh review.".into());
    }
    if !source_matches_base {
        blockers.push("The source repository moved after this worktree was created. Start a fresh run from the current source revision.".into());
    }
    Ok(IntegrationReadiness {
        source_head,
        recorded_base: base_commit.into(),
        source_clean,
        source_matches_base,
        blockers,
    })
}

pub fn integrate_isolated_worktree(
    project: &Path,
    worktree: &Path,
    runs_root: &Path,
    base_commit: &str,
    run_id: u64,
) -> Result<String, ServiceError> {
    validate_git_oid(base_commit)?;
    let root = git_root(project)?;
    let worktree = validate_managed_worktree(worktree, runs_root)?;
    ensure_agent_safe_project(&worktree)?;
    let disabled_hooks = runs_root.join(".disabled-hooks");
    fs::create_dir_all(&disabled_hooks)?;
    let readiness = inspect_integration_readiness(&root, base_commit)?;
    if !readiness.blockers.is_empty() {
        return Err(ServiceError::Integration(readiness.blockers.join(" ")));
    }

    let head_before = git_text(&worktree, ["rev-parse", "HEAD"])?;
    let ancestor = git(
        &worktree,
        ["merge-base", "--is-ancestor", base_commit, &head_before],
    )?;
    if !ancestor.status.success() {
        return Err(ServiceError::Integration(
            "the retained worktree is no longer based on its recorded source revision".into(),
        ));
    }
    let status = git_status(worktree.to_string_lossy().as_ref())?;
    if status.files.is_empty() {
        if head_before == base_commit {
            return Err(ServiceError::Integration(
                "the run worktree has no changes to integrate".into(),
            ));
        }
    } else {
        let added = git(&worktree, ["add", "--all", "--"])?;
        if !added.status.success() {
            return Err(ServiceError::Integration(git_error(&added)));
        }
        let message = format!("Integrate Rubyn Harness run {run_id}");
        let committed = git_without_hooks(
            &worktree,
            ["commit", "--no-gpg-sign", "-m", &message],
            &disabled_hooks,
        )
        .map(|mut command| {
            command
                .env("GIT_AUTHOR_NAME", "Rubyn Harness")
                .env("GIT_AUTHOR_EMAIL", "rubyn-harness@localhost")
                .env("GIT_COMMITTER_NAME", "Rubyn Harness")
                .env("GIT_COMMITTER_EMAIL", "rubyn-harness@localhost")
                .output()
        })??;
        if !committed.status.success() {
            let _ = git(&worktree, ["reset", "--mixed", "HEAD"]);
            return Err(ServiceError::Integration(git_error(&committed)));
        }
    }

    let reset = git(&worktree, ["reset", "--soft", base_commit])?;
    if !reset.status.success() {
        return Err(ServiceError::Integration(git_error(&reset)));
    }
    let message = format!("Integrate Rubyn Harness run {run_id}");
    let squashed = git_without_hooks(
        &worktree,
        ["commit", "--no-gpg-sign", "-m", &message],
        &disabled_hooks,
    )
    .map(|mut command| {
        command
            .env("GIT_AUTHOR_NAME", "Rubyn Harness")
            .env("GIT_AUTHOR_EMAIL", "rubyn-harness@localhost")
            .env("GIT_COMMITTER_NAME", "Rubyn Harness")
            .env("GIT_COMMITTER_EMAIL", "rubyn-harness@localhost")
            .output()
    })??;
    if !squashed.status.success() {
        return Err(ServiceError::Integration(git_error(&squashed)));
    }
    let commit = git_text(&worktree, ["rev-parse", "HEAD"])?;
    let source_tree = git_text(&root, ["rev-parse", "HEAD^{tree}"])?;
    let worktree_tree = git_text(&worktree, ["rev-parse", &format!("{commit}^{{tree}}")])?;
    if source_tree == worktree_tree {
        return git_text(&root, ["rev-parse", "HEAD"]);
    }
    let already_integrated = git(&root, ["merge-base", "--is-ancestor", &commit, "HEAD"])?;
    if already_integrated.status.success() {
        return Ok(commit);
    }
    let cherry_pick =
        git_without_hooks(&root, ["cherry-pick", &commit], &disabled_hooks)?.output()?;
    if !cherry_pick.status.success() {
        let detail = git_error(&cherry_pick);
        let _ = git(&root, ["cherry-pick", "--abort"]);
        return Err(ServiceError::Integration(format!(
            "cherry-pick was aborted: {detail}"
        )));
    }
    git_text(&root, ["rev-parse", "HEAD"])
}

fn ensure_agent_safe_project(project: &Path) -> Result<(), ServiceError> {
    for risky in [".mcp.json", ".rubyn-code/settings.json"] {
        if project.join(risky).exists() {
            return Err(ServiceError::UntrustedProject(format!(
                "{risky} can execute project-defined commands and is not allowed in the first beta"
            )));
        }
    }
    let mut visited = 0;
    reject_symlinks(project, project, &mut visited)
}

fn reject_symlinks(root: &Path, directory: &Path, visited: &mut usize) -> Result<(), ServiceError> {
    for entry in fs::read_dir(directory)?.flatten() {
        *visited += 1;
        if *visited > 100_000 {
            return Err(ServiceError::UntrustedProject(
                "project is too large to validate safely".into(),
            ));
        }
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(ServiceError::UntrustedProject(format!(
                "symbolic link {} could escape the isolated worktree",
                path.strip_prefix(root).unwrap_or(&path).display()
            )));
        }
        if path == root.join(".git") {
            continue;
        }
        if metadata.is_dir() {
            reject_symlinks(root, &path, visited)?;
        }
    }
    Ok(())
}

pub fn canonical_project(path: &str) -> Result<PathBuf, ServiceError> {
    let resolved = fs::canonicalize(path).map_err(|_| ServiceError::InvalidProject(path.into()))?;
    if resolved.is_dir() {
        Ok(resolved)
    } else {
        Err(ServiceError::InvalidProject(path.into()))
    }
}

pub fn scan_projects(roots: &[String]) -> Result<Vec<ProjectSummary>, ServiceError> {
    let mut candidates = BTreeSet::new();
    for root in roots.iter().take(12) {
        let root = canonical_project(root)?;
        candidates.insert(root.clone());
        for entry in fs::read_dir(root)?.flatten().take(128) {
            let path = entry.path();
            if path.is_dir() && !is_hidden(&path) {
                candidates.insert(path);
            }
        }
    }

    let mut projects: Vec<_> = candidates
        .into_iter()
        .filter(|path| looks_like_project(path))
        .map(|path| {
            let git_root = git_root(&path).ok();
            ProjectSummary::from_path(path, git_root)
        })
        .collect();
    projects.sort_by_key(|project| project.name.to_lowercase());
    Ok(projects)
}

pub fn inspect_project(path: &str) -> Result<ProjectSummary, ServiceError> {
    let path = canonical_project(path)?;
    Ok(ProjectSummary::from_path(
        path.clone(),
        git_root(&path).ok(),
    ))
}

pub fn create_project_skill(
    project: &Path,
    name: &str,
    content: &str,
) -> Result<SkillSummary, ServiceError> {
    let project = fs::canonicalize(project)?;
    let name = name.trim();
    if name.is_empty() || name.len() > 120 || content.trim().is_empty() || content.len() > 100_000 {
        return Err(ServiceError::UntrustedProject(
            "skill name and content must be present and within local limits".into(),
        ));
    }
    let slug = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        return Err(ServiceError::UntrustedProject(
            "skill name is invalid".into(),
        ));
    }
    let directory = project.join(".rubyn-code/skills/harness");
    fs::create_dir_all(&directory)?;
    let path = directory.join(format!("{slug}.md"));
    if path.exists() {
        return Err(ServiceError::UntrustedProject(format!(
            "a project skill named {slug} already exists"
        )));
    }
    let temporary = path.with_extension("md.tmp");
    let document = format!("# {name}\n\n{}\n", content.trim());
    fs::write(&temporary, document)?;
    fs::rename(&temporary, &path)?;
    Ok(SkillSummary {
        name: name.to_owned(),
        path: path.to_string_lossy().into_owned(),
        description: content.lines().next().unwrap_or_default().trim().to_owned(),
    })
}

pub fn git_status(path: &str) -> Result<GitStatus, ServiceError> {
    let project = canonical_project(path)?;
    let output = git(&project, ["status", "--porcelain=v1", "-z", "--branch"])?;
    if !output.status.success() {
        return Err(ServiceError::GitUnavailable);
    }
    Ok(parse_git_status(&output.stdout))
}

pub fn git_diff(path: &str, staged: bool) -> Result<GitDiff, ServiceError> {
    let project = canonical_project(path)?;
    let mut args = vec!["diff", "--no-ext-diff", "--unified=3"];
    if staged {
        args.push("--cached");
    }
    let output = git(&project, args)?;
    if !output.status.success() {
        return Err(ServiceError::GitUnavailable);
    }
    let truncated = output.stdout.len() > MAX_DIFF_BYTES;
    let bytes = &output.stdout[..output.stdout.len().min(MAX_DIFF_BYTES)];
    Ok(GitDiff {
        diff: String::from_utf8_lossy(bytes).into_owned(),
        truncated,
    })
}

fn git_diff_from_base(project: &Path, base_commit: &str) -> Result<GitDiff, ServiceError> {
    let output = git(
        project,
        [
            "diff",
            "--no-ext-diff",
            "--binary",
            "--unified=3",
            base_commit,
            "--",
        ],
    )?;
    if !output.status.success() {
        return Err(ServiceError::GitUnavailable);
    }
    let untracked = git(project, ["ls-files", "--others", "--exclude-standard"])?;
    let mut bytes = output.stdout;
    let mut content_truncated = false;
    if untracked.status.success() && !untracked.stdout.is_empty() {
        bytes.extend_from_slice(b"\n# Untracked files (included when integrating):\n");
        for relative in untracked
            .stdout
            .split(|byte| *byte == b'\n')
            .filter(|path| !path.is_empty())
        {
            if bytes.len() >= MAX_DIFF_BYTES {
                content_truncated = true;
                break;
            }
            let relative = String::from_utf8_lossy(relative);
            let path = project.join(relative.as_ref());
            let Ok(metadata) = fs::symlink_metadata(&path) else {
                continue;
            };
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                continue;
            }
            bytes.extend_from_slice(
                format!(
                    "\ndiff --git a/{0} b/{0}\nnew file mode 100644\n--- /dev/null\n+++ b/{0}\n",
                    relative
                )
                .as_bytes(),
            );
            let remaining = MAX_DIFF_BYTES.saturating_sub(bytes.len());
            if metadata.len() > remaining as u64 {
                content_truncated = true;
            }
            let mut content = Vec::new();
            if let Ok(file) = fs::File::open(&path) {
                let _ = file.take(remaining as u64).read_to_end(&mut content);
            }
            if content.contains(&0) {
                bytes.extend_from_slice(b"Binary file contents omitted\n");
            } else {
                for line in String::from_utf8_lossy(&content).lines() {
                    bytes.extend_from_slice(b"+");
                    bytes.extend_from_slice(line.as_bytes());
                    bytes.extend_from_slice(b"\n");
                    if bytes.len() >= MAX_DIFF_BYTES {
                        break;
                    }
                }
            }
        }
    }
    let truncated = content_truncated || bytes.len() > MAX_DIFF_BYTES;
    bytes.truncate(MAX_DIFF_BYTES);
    Ok(GitDiff {
        diff: String::from_utf8_lossy(&bytes).into_owned(),
        truncated,
    })
}

fn validate_managed_worktree(worktree: &Path, runs_root: &Path) -> Result<PathBuf, ServiceError> {
    let root = fs::canonicalize(runs_root).map_err(ServiceError::Io)?;
    let worktree = fs::canonicalize(worktree).map_err(ServiceError::Io)?;
    let relative = worktree.strip_prefix(&root).map_err(|_| {
        ServiceError::UntrustedProject(
            "the run worktree is outside the Harness-managed worktree directory".into(),
        )
    })?;
    let components: Vec<_> = relative.components().collect();
    let valid = components.len() == 2
        && components[0]
            .as_os_str()
            .to_str()
            .is_some_and(|name| name.starts_with("run-"))
        && components[1].as_os_str() == OsStr::new("workspace");
    if !valid {
        return Err(ServiceError::UntrustedProject(
            "the run worktree does not have a managed run/workspace layout".into(),
        ));
    }
    Ok(worktree)
}

pub fn git_root(path: &Path) -> Result<PathBuf, ServiceError> {
    let output = git(path, ["rev-parse", "--show-toplevel"])?;
    if !output.status.success() {
        return Err(ServiceError::GitUnavailable);
    }
    let raw = String::from_utf8_lossy(&output.stdout);
    let root = raw.trim();
    if root.is_empty() {
        Err(ServiceError::GitUnavailable)
    } else {
        fs::canonicalize(root).map_err(ServiceError::Io)
    }
}

fn git<I, S>(project: &Path, args: I) -> Result<Output, ServiceError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Command::new("git")
        .arg("-C")
        .arg(project)
        .arg("--no-pager")
        .args(args)
        .env("GIT_PAGER", "cat")
        .output()
        .map_err(ServiceError::Io)
}

fn git_without_hooks<I, S>(
    project: &Path,
    args: I,
    disabled_hooks: &Path,
) -> Result<Command, ServiceError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let disabled_hooks = fs::canonicalize(disabled_hooks).map_err(ServiceError::Io)?;
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(project)
        .arg("-c")
        .arg(format!("core.hooksPath={}", disabled_hooks.display()))
        .arg("--no-pager")
        .args(args)
        .env("GIT_PAGER", "cat");
    Ok(command)
}

fn git_text<I, S>(project: &Path, args: I) -> Result<String, ServiceError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = git(project, args)?;
    if !output.status.success() {
        return Err(ServiceError::Worktree(git_error(&output)));
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if value.is_empty() {
        Err(ServiceError::Worktree(
            "Git returned an empty revision".into(),
        ))
    } else {
        Ok(value)
    }
}

fn git_error(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    if stderr.is_empty() {
        format!("Git exited with {}", output.status)
    } else {
        stderr
    }
}

fn validate_git_oid(value: &str) -> Result<(), ServiceError> {
    if matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(ServiceError::Worktree(
            "the run has an invalid recorded base revision".into(),
        ))
    }
}

pub fn list_skills(root: &Path) -> Result<Vec<SkillSummary>, ServiceError> {
    list_skill_directory(&root.join("skills"))
}

pub fn list_project_skills(project: &Path) -> Result<Vec<SkillSummary>, ServiceError> {
    let directory = project.join(".rubyn-code/skills");
    if !directory.is_dir() {
        return Ok(Vec::new());
    }
    list_skill_directory(&directory)
}

pub fn read_skill_file(skills_root: &Path, relative_path: &str) -> Result<String, ServiceError> {
    let root = fs::canonicalize(skills_root)?;
    let candidate = fs::canonicalize(root.join(relative_path))?;
    let metadata = fs::symlink_metadata(&candidate)?;
    if !candidate.starts_with(&root)
        || metadata.file_type().is_symlink()
        || !metadata.is_file()
        || candidate.extension() != Some(OsStr::new("md"))
        || metadata.len() > 262_144
    {
        return Err(ServiceError::UntrustedProject(
            "skill path is outside the selected skill source or exceeds the read limit".into(),
        ));
    }
    fs::read_to_string(candidate).map_err(ServiceError::Io)
}

fn list_skill_directory(skills_root: &Path) -> Result<Vec<SkillSummary>, ServiceError> {
    let skills_root = fs::canonicalize(skills_root).map_err(ServiceError::Io)?;
    let mut pending = vec![skills_root.clone()];
    let mut skills = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory)?.flatten() {
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path)?;
            if metadata.file_type().is_symlink() {
                continue;
            }
            if metadata.is_dir() {
                pending.push(path);
                continue;
            }
            if path.extension() != Some(OsStr::new("md")) || metadata.len() > 262_144 {
                continue;
            }
            let relative = path.strip_prefix(&skills_root).unwrap_or(&path);
            let contents = fs::read_to_string(&path)?;
            let title = markdown_frontmatter_value(&contents, "name")
                .or_else(|| {
                    contents
                        .lines()
                        .find_map(|line| line.trim().strip_prefix("# "))
                        .map(str::trim)
                        .filter(|title| !title.is_empty())
                        .map(str::to_owned)
                })
                .unwrap_or_else(|| {
                    relative
                        .with_extension("")
                        .to_string_lossy()
                        .replace(['/', '_'], " ")
                });
            let description = markdown_frontmatter_value(&contents, "description")
                .unwrap_or_else(|| {
                    contents
                        .lines()
                        .map(str::trim)
                        .find(|line| !line.is_empty() && !line.starts_with('#') && *line != "---")
                        .unwrap_or("Rubyn Code engineering guidance")
                        .to_owned()
                })
                .chars()
                .take(240)
                .collect();
            skills.push(SkillSummary {
                name: title,
                path: relative.to_string_lossy().into_owned(),
                description,
            });
            if skills.len() >= 500 {
                break;
            }
        }
        if skills.len() >= 500 {
            break;
        }
    }
    skills.sort_by_key(|skill| skill.name.to_lowercase());
    Ok(skills)
}

fn markdown_frontmatter_value(contents: &str, key: &str) -> Option<String> {
    let mut lines = contents.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }
    for line in lines {
        let line = line.trim();
        if line == "---" {
            break;
        }
        let Some((candidate, value)) = line.split_once(':') else {
            continue;
        };
        if candidate.trim() == key {
            let value = value.trim().trim_matches(['"', '\'']);
            return (!value.is_empty()).then(|| value.to_owned());
        }
    }
    None
}

fn looks_like_project(path: &Path) -> bool {
    path.join(".git").exists()
        || path.join("Gemfile").is_file()
        || path.join("package.json").is_file()
        || path.join("Cargo.toml").is_file()
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with('.'))
}

fn parse_git_status(bytes: &[u8]) -> GitStatus {
    let mut branch = None;
    let mut files = Vec::new();
    let mut records = bytes
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty());
    while let Some(record) = records.next() {
        let value = String::from_utf8_lossy(record);
        if let Some(value) = value.strip_prefix("## ") {
            branch = Some(value.split("...").next().unwrap_or(value).to_owned());
            continue;
        }
        if record.len() < 4 {
            continue;
        }
        let index_status = String::from_utf8_lossy(&record[0..1]).into_owned();
        let worktree_status = String::from_utf8_lossy(&record[1..2]).into_owned();
        let mut path = String::from_utf8_lossy(&record[3..]).into_owned();
        // In porcelain -z, rename/copy records carry the old path as the next NUL record.
        if matches!(record[0], b'R' | b'C') {
            if let Some(old_path) = records.next() {
                path = format!("{} → {}", String::from_utf8_lossy(old_path), path);
            }
        }
        files.push(GitFileStatus {
            path,
            index_status,
            worktree_status,
        });
    }
    GitStatus { branch, files }
}

pub fn engine_info(bundled_root: Option<PathBuf>) -> EngineInfo {
    if let Some(root) = bundled_root.filter(|root| root.join("exe/rubyn-code").is_file()) {
        let runtime = ruby_runtime_for(&root);
        let version = runtime
            .as_deref()
            .and_then(|ruby| read_engine_version(&root, ruby));
        let healthy = version.is_some();
        return EngineInfo {
            available: true,
            healthy,
            source: EngineSource::Bundled,
            executable: root.join("exe/rubyn-code").to_string_lossy().into_owned(),
            version,
            detail: (!healthy).then(|| {
                "Bundled Rubyn Code requires Ruby 4.0.2+ with its runtime gems. Install it with rbenv or Homebrew, then relaunch Harness.".into()
            }),
        };
    }
    if let Some(version) = read_installed_engine_version() {
        return EngineInfo {
            available: true,
            healthy: true,
            source: EngineSource::Installed,
            executable: "rubyn-code".into(),
            version: Some(version),
            detail: None,
        };
    }
    EngineInfo {
        available: false,
        healthy: false,
        source: EngineSource::Unavailable,
        executable: "rubyn-code".into(),
        version: None,
        detail: Some("Install Rubyn Code or include the bundled engine resources.".into()),
    }
}

pub fn ruby_runtime_for(root: &Path) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(explicit) = std::env::var_os("RUBYN_HARNESS_RUBY") {
        candidates.push(PathBuf::from(explicit));
    }
    if let Some(rbenv_root) = std::env::var_os("RBENV_ROOT") {
        candidates.push(PathBuf::from(rbenv_root).join("shims/ruby"));
    }
    if let Some(user_home) = std::env::var_os("HOME") {
        candidates.push(PathBuf::from(user_home).join(".rbenv/shims/ruby"));
    }
    candidates.extend([
        PathBuf::from("/opt/homebrew/opt/ruby/bin/ruby"),
        PathBuf::from("/opt/homebrew/bin/ruby"),
        PathBuf::from("/usr/local/opt/ruby/bin/ruby"),
        PathBuf::from("/usr/local/bin/ruby"),
        PathBuf::from("ruby"),
    ]);
    candidates.into_iter().find(|ruby| {
        let mut command = Command::new(ruby);
        command
            .arg("-I")
            .arg(root.join("lib"))
            .args([
                "-e",
                "abort unless Gem::Version.new(RUBY_VERSION) >= Gem::Version.new('4.0.2'); require 'rubyn_code'",
            ])
            .current_dir(root);
        command_output_with_timeout(&mut command, RUBY_PROBE_TIMEOUT)
            .is_some_and(|output| output.status.success())
    })
}

fn read_engine_version(root: &Path, ruby: &Path) -> Option<String> {
    let mut command = Command::new(ruby);
    command
        .args(["-I", "lib", "exe/rubyn-code", "--version"])
        .current_dir(root);
    command_output_with_timeout(&mut command, RUBY_PROBE_TIMEOUT)
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|version| !version.is_empty())
}

fn read_installed_engine_version() -> Option<String> {
    let mut command = Command::new("rubyn-code");
    command.arg("--version");
    command_output_with_timeout(&mut command, RUBY_PROBE_TIMEOUT)
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|version| !version.is_empty())
}

fn command_output_with_timeout(command: &mut Command, timeout: Duration) -> Option<Output> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command.spawn().ok()?;
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return child.wait_with_output().ok(),
            Ok(None) => {}
            Err(_) => {
                terminate_probe(&mut child);
                return None;
            }
        }
        if Instant::now() >= deadline {
            terminate_probe(&mut child);
            return None;
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn terminate_probe(child: &mut std::process::Child) {
    #[cfg(unix)]
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    #[cfg(not(unix))]
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_probe_returns_output_before_its_deadline() {
        let mut command = Command::new("/bin/sh");
        command.args(["-c", "printf ready"]);

        let output = command_output_with_timeout(&mut command, Duration::from_secs(1)).unwrap();

        assert!(output.status.success());
        assert_eq!(output.stdout, b"ready");
    }

    #[test]
    fn command_probe_terminates_a_hung_process_group() {
        let mut command = Command::new("/bin/sh");
        command.args(["-c", "sleep 10"]);
        let started = Instant::now();

        assert!(command_output_with_timeout(&mut command, Duration::from_millis(50)).is_none());
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn parses_branch_and_multiple_status_records() {
        let result =
            parse_git_status(b"## main...origin/main\0 M app/models/user.rb\0A  README.md\0");
        assert_eq!(result.branch.as_deref(), Some("main"));
        assert_eq!(result.files.len(), 2);
        assert_eq!(result.files[0].worktree_status, "M");
        assert_eq!(result.files[1].index_status, "A");
    }

    #[test]
    fn identifies_a_rails_project_from_conventions() {
        let directory = std::env::temp_dir().join(format!("rubyn-rails-{}", std::process::id()));
        fs::create_dir_all(directory.join("config")).unwrap();
        fs::write(directory.join("Gemfile"), "source 'https://rubygems.org'").unwrap();
        fs::write(
            directory.join("config/application.rb"),
            "module Example; end",
        )
        .unwrap();
        assert!(
            inspect_project(directory.to_str().unwrap())
                .unwrap()
                .is_rails
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn inspects_status_and_diff_for_a_real_git_project() {
        let directory = std::env::temp_dir().join(format!(
            "rubyn-git-flow-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        fs::create_dir_all(directory.join("config")).unwrap();
        fs::write(directory.join("Gemfile"), "source 'https://rubygems.org'\n").unwrap();
        fs::write(
            directory.join("config/application.rb"),
            "module Example; end\n",
        )
        .unwrap();

        for args in [
            vec!["init"],
            vec!["config", "user.email", "qa@example.test"],
            vec!["config", "user.name", "Rubyn QA"],
            vec!["add", "."],
            vec!["commit", "-m", "baseline"],
        ] {
            let output = git(&directory, args).unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        fs::write(
            directory.join("config/application.rb"),
            "module Example\n  RELEASE = true\nend\n",
        )
        .unwrap();

        let project = inspect_project(directory.to_str().unwrap()).unwrap();
        let status = git_status(directory.to_str().unwrap()).unwrap();
        let diff = git_diff(directory.to_str().unwrap(), false).unwrap();

        assert!(project.is_rails);
        assert!(project.git_root.is_some());
        assert!(status
            .files
            .iter()
            .any(|file| { file.path == "config/application.rb" && file.worktree_status == "M" }));
        assert!(diff.diff.contains("+  RELEASE = true"));
        assert!(!diff.truncated);

        fs::remove_dir_all(directory).unwrap();
    }

    fn initialize_test_repository(directory: &Path) {
        fs::create_dir_all(directory).unwrap();
        fs::write(directory.join("README.md"), "baseline\n").unwrap();
        for args in [
            vec!["init"],
            vec!["config", "user.email", "qa@example.test"],
            vec!["config", "user.name", "Rubyn QA"],
            vec!["add", "."],
            vec!["commit", "-m", "baseline"],
        ] {
            let output = git(directory, args).unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    #[test]
    fn integrates_and_cleans_a_managed_worktree() {
        let container = std::env::temp_dir().join(format!(
            "rubyn-integrate-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let project = container.join("source");
        let runs_root = container.join("runs");
        initialize_test_repository(&project);
        let allocation = create_isolated_worktree(&project, &runs_root).unwrap();
        fs::write(allocation.path.join("README.md"), "integrated\n").unwrap();
        fs::write(allocation.path.join("new.rb"), "puts :integrated\n").unwrap();
        for args in [vec!["add", "--all"], vec!["commit", "-m", "agent work"]] {
            let output = git(&allocation.path, args).unwrap();
            assert!(output.status.success(), "{}", git_error(&output));
        }

        let (status, diff) =
            inspect_isolated_worktree(&allocation.path, &runs_root, &allocation.base_commit)
                .unwrap();
        assert_eq!(status.files.len(), 2);
        assert!(diff.diff.contains("+integrated"));
        assert!(diff.diff.contains("new.rb"));

        let commit = integrate_isolated_worktree(
            &project,
            &allocation.path,
            &runs_root,
            &allocation.base_commit,
            41,
        )
        .unwrap();
        assert_eq!(commit, git_text(&project, ["rev-parse", "HEAD"]).unwrap());
        assert_eq!(
            fs::read_to_string(project.join("README.md")).unwrap(),
            "integrated\n"
        );
        assert_eq!(
            fs::read_to_string(project.join("new.rb")).unwrap(),
            "puts :integrated\n"
        );

        remove_isolated_worktree(&project, &allocation.path, &runs_root).unwrap();
        assert!(!allocation.path.exists());
        let registered = git(&project, ["worktree", "list", "--porcelain"]).unwrap();
        assert!(!String::from_utf8_lossy(&registered.stdout)
            .contains(allocation.path.to_string_lossy().as_ref()));
        fs::remove_dir_all(container).unwrap();
    }

    #[test]
    fn review_includes_untracked_file_contents() {
        let container = std::env::temp_dir().join(format!(
            "rubyn-untracked-review-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let project = container.join("source");
        let runs_root = container.join("runs");
        initialize_test_repository(&project);
        let allocation = create_isolated_worktree(&project, &runs_root).unwrap();
        fs::write(
            allocation.path.join("untracked.rb"),
            "class ReviewedEvidence\nend\n",
        )
        .unwrap();

        let (_, diff) =
            inspect_isolated_worktree(&allocation.path, &runs_root, &allocation.base_commit)
                .unwrap();
        assert!(diff
            .diff
            .contains("diff --git a/untracked.rb b/untracked.rb"));
        assert!(diff.diff.contains("+class ReviewedEvidence"));
        remove_isolated_worktree(&project, &allocation.path, &runs_root).unwrap();
        fs::remove_dir_all(container).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn integration_does_not_execute_repository_git_hooks() {
        use std::os::unix::fs::PermissionsExt;

        let container = std::env::temp_dir().join(format!(
            "rubyn-hook-safe-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let project = container.join("source");
        let runs_root = container.join("runs");
        initialize_test_repository(&project);
        let allocation = create_isolated_worktree(&project, &runs_root).unwrap();
        fs::write(allocation.path.join("README.md"), "safe integration\n").unwrap();
        let hook = project.join(".git/hooks/pre-commit");
        let marker = container.join("hook-ran");
        fs::write(&hook, format!("#!/bin/sh\ntouch '{}'\n", marker.display())).unwrap();
        let mut permissions = fs::metadata(&hook).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&hook, permissions).unwrap();

        integrate_isolated_worktree(
            &project,
            &allocation.path,
            &runs_root,
            &allocation.base_commit,
            44,
        )
        .unwrap();
        assert!(!marker.exists(), "repository hook must never execute");
        remove_isolated_worktree(&project, &allocation.path, &runs_root).unwrap();
        fs::remove_dir_all(container).unwrap();
    }

    #[test]
    fn integration_refuses_to_overwrite_a_dirty_source_project() {
        let container = std::env::temp_dir().join(format!(
            "rubyn-dirty-integrate-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let project = container.join("source");
        let runs_root = container.join("runs");
        initialize_test_repository(&project);
        let allocation = create_isolated_worktree(&project, &runs_root).unwrap();
        fs::write(allocation.path.join("README.md"), "agent change\n").unwrap();
        fs::write(project.join("README.md"), "user change\n").unwrap();

        assert!(matches!(
            integrate_isolated_worktree(
                &project,
                &allocation.path,
                &runs_root,
                &allocation.base_commit,
                42,
            ),
            Err(ServiceError::Integration(_))
        ));
        assert_eq!(
            fs::read_to_string(project.join("README.md")).unwrap(),
            "user change\n"
        );
        remove_isolated_worktree(&project, &allocation.path, &runs_root).unwrap();
        fs::remove_dir_all(container).unwrap();
    }

    #[test]
    fn integration_refuses_clean_source_drift_and_retains_the_worktree() {
        let container = std::env::temp_dir().join(format!(
            "rubyn-drift-integrate-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let project = container.join("source");
        let runs_root = container.join("runs");
        initialize_test_repository(&project);
        let allocation = create_isolated_worktree(&project, &runs_root).unwrap();
        fs::write(allocation.path.join("agent.rb"), "puts :agent\n").unwrap();
        fs::write(project.join("source.rb"), "puts :source\n").unwrap();
        for args in [
            vec!["add", "source.rb"],
            vec!["commit", "-m", "source drift"],
        ] {
            let output = git(&project, args).unwrap();
            assert!(output.status.success(), "{}", git_error(&output));
        }

        let readiness = inspect_integration_readiness(&project, &allocation.base_commit).unwrap();
        assert!(readiness.source_clean);
        assert!(!readiness.source_matches_base);
        assert!(readiness.blockers[0].contains("moved"));
        let error = integrate_isolated_worktree(
            &project,
            &allocation.path,
            &runs_root,
            &allocation.base_commit,
            43,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("moved"));
        assert!(allocation.path.join("agent.rb").exists());
        assert!(!project.join("agent.rb").exists());
        remove_isolated_worktree(&project, &allocation.path, &runs_root).unwrap();
        fs::remove_dir_all(container).unwrap();
    }

    #[test]
    fn integration_refuses_a_no_change_worktree() {
        let container = std::env::temp_dir().join(format!(
            "rubyn-no-change-integrate-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let project = container.join("source");
        let runs_root = container.join("runs");
        initialize_test_repository(&project);
        let allocation = create_isolated_worktree(&project, &runs_root).unwrap();
        let error = integrate_isolated_worktree(
            &project,
            &allocation.path,
            &runs_root,
            &allocation.base_commit,
            45,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("no changes"));
        assert!(allocation.path.exists());
        remove_isolated_worktree(&project, &allocation.path, &runs_root).unwrap();
        fs::remove_dir_all(container).unwrap();
    }

    #[test]
    fn discarding_removes_only_the_managed_worktree() {
        let container = std::env::temp_dir().join(format!(
            "rubyn-discard-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let project = container.join("source");
        let runs_root = container.join("runs");
        initialize_test_repository(&project);
        let allocation = create_isolated_worktree(&project, &runs_root).unwrap();
        fs::write(allocation.path.join("README.md"), "discard me\n").unwrap();

        remove_isolated_worktree(&project, &allocation.path, &runs_root).unwrap();
        assert!(!allocation.path.exists());
        assert_eq!(
            fs::read_to_string(project.join("README.md")).unwrap(),
            "baseline\n"
        );
        assert!(git_status(project.to_string_lossy().as_ref())
            .unwrap()
            .files
            .is_empty());
        fs::remove_dir_all(container).unwrap();
    }

    #[test]
    fn cleanup_recovers_an_orphaned_managed_worktree_directory() {
        let container = std::env::temp_dir().join(format!(
            "rubyn-orphan-cleanup-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let project = container.join("source");
        let runs_root = container.join("runs");
        initialize_test_repository(&project);
        let allocation = create_isolated_worktree(&project, &runs_root).unwrap();
        let git_file = allocation.path.join(".git");
        let admin = fs::read_to_string(&git_file)
            .unwrap()
            .trim()
            .strip_prefix("gitdir: ")
            .map(PathBuf::from)
            .unwrap();
        fs::remove_dir_all(admin).unwrap();
        fs::remove_file(git_file).unwrap();

        remove_isolated_worktree(&project, &allocation.path, &runs_root).unwrap();
        assert!(!allocation.path.exists());
        assert_eq!(
            fs::read_to_string(project.join("README.md")).unwrap(),
            "baseline\n"
        );
        fs::remove_dir_all(container).unwrap();
    }

    #[test]
    fn enumerates_real_markdown_skills_with_descriptions() {
        let directory = std::env::temp_dir().join(format!(
            "rubyn-skills-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        fs::create_dir_all(directory.join("skills/rails")).unwrap();
        fs::write(
            directory.join("skills/rails/safe_migrations.md"),
            "# Safe Rails migrations\n\nShip schema changes in compatible phases.\n",
        )
        .unwrap();

        let skills = list_skills(&directory).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "Safe Rails migrations");
        assert_eq!(skills[0].path, "rails/safe_migrations.md");
        assert!(skills[0].description.contains("compatible phases"));
        assert!(
            read_skill_file(&directory.join("skills"), "rails/safe_migrations.md")
                .unwrap()
                .contains("compatible phases")
        );
        assert!(read_skill_file(&directory.join("skills"), "../outside.md").is_err());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn enumerates_skill_frontmatter_instead_of_showing_yaml_delimiters() {
        let directory = std::env::temp_dir().join(format!(
            "rubyn-frontmatter-skills-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        fs::create_dir_all(directory.join("skills/wayfinder")).unwrap();
        fs::write(
            directory.join("skills/wayfinder/wayfinder.md"),
            "---\nname: wayfinder\ndescription: Map fog as decision tickets.\ndisable-model-invocation: true\n---\n\nSkill body.\n",
        )
        .unwrap();

        let skills = list_skills(&directory).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "wayfinder");
        assert_eq!(skills[0].description, "Map fog as decision tickets.");
        assert_eq!(skills[0].path, "wayfinder/wayfinder.md");
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn creates_and_enumerates_project_skills_without_overwriting() {
        let directory = std::env::temp_dir().join(format!(
            "rubyn-project-skills-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        fs::create_dir_all(&directory).unwrap();
        let created = create_project_skill(
            &directory,
            "Safe Rails migrations",
            "Inspect locks before changing a busy table.",
        )
        .unwrap();
        assert!(Path::new(&created.path).is_file());
        let skills = list_project_skills(&directory).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "Safe Rails migrations");
        assert!(matches!(
            create_project_skill(&directory, "Safe Rails migrations", "replacement"),
            Err(ServiceError::UntrustedProject(_))
        ));
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn blocks_project_defined_process_configuration() {
        let directory = std::env::temp_dir().join(format!(
            "rubyn-untrusted-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join(".mcp.json"), "{}").unwrap();
        assert!(matches!(
            ensure_agent_safe_project(&directory),
            Err(ServiceError::UntrustedProject(_))
        ));
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn blocks_symlinks_nested_in_node_modules() {
        use std::os::unix::fs::symlink;

        let directory = std::env::temp_dir().join(format!(
            "rubyn-symlink-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let target = directory.with_extension("outside");
        fs::create_dir_all(directory.join("node_modules/vendor")).unwrap();
        fs::create_dir_all(&target).unwrap();
        symlink(&target, directory.join("node_modules/vendor/escape")).unwrap();
        assert!(matches!(
            ensure_agent_safe_project(&directory),
            Err(ServiceError::UntrustedProject(_))
        ));
        fs::remove_dir_all(directory).unwrap();
        fs::remove_dir_all(target).unwrap();
    }
}
