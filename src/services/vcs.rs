use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use git2::{
    build::CheckoutBuilder,
    BranchType, DiffFormat, IndexAddOption, Repository, Signature, Sort, StatusOptions,
};
use serde_json;

use crate::domain::collection::{Collection, SavedRequest};

fn open_at(path: &PathBuf) -> Result<Repository, String> {
    std::fs::create_dir_all(path).map_err(|e| e.to_string())?;
    match Repository::open(path) {
        Ok(repo) => Ok(repo),
        Err(_) => Repository::init(path).map_err(|e| e.to_string()),
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct CollectionFile {
    collection: Collection,
    requests: Vec<SavedRequest>,
}

/// Filename the global script pair is stored under, at the repo root
/// (sibling to the per-collection `{id}.json` files). Its `.json`
/// extension means it would otherwise be picked up by `load_collections`'s
/// blanket `*.json` scan — but that scan only keeps files that successfully
/// deserialize as `CollectionFile`, so it's silently skipped there.
const GLOBAL_SCRIPT_FILE: &str = "global.json";

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct GlobalScriptFile {
    pre_request_script: String,
    test_script: String,
}

/// Reads the global script pair out of a repo's current working tree.
/// `None` if the repo has none saved (never committed, or this is a repo
/// that predates the global script feature).
pub fn read_global_script(data_dir: &PathBuf) -> Option<(String, String)> {
    let repo = open_at(data_dir).ok()?;
    let workdir = repo.workdir()?;
    let json = std::fs::read_to_string(workdir.join(GLOBAL_SCRIPT_FILE)).ok()?;
    let file: GlobalScriptFile = serde_json::from_str(&json).ok()?;
    Some((file.pre_request_script, file.test_script))
}

/// Reads the global script pair as it existed at a specific past commit —
/// the counterpart to `read_commit_collections`, used when restoring a
/// commit rather than reading the live working tree.
pub fn read_commit_global_script(data_dir: &PathBuf, commit_id: &str) -> Option<(String, String)> {
    let repo = open_at(data_dir).ok()?;
    let oid = git2::Oid::from_str(commit_id).ok()?;
    let commit = repo.find_commit(oid).ok()?;
    let tree = commit.tree().ok()?;
    let entry = tree.get_path(Path::new(GLOBAL_SCRIPT_FILE)).ok()?;
    let object = entry.to_object(&repo).ok()?;
    let blob = object.as_blob()?;
    let file: GlobalScriptFile = serde_json::from_slice(blob.content()).ok()?;
    Some((file.pre_request_script, file.test_script))
}

pub fn load_collections(repo: &Repository) -> Result<Vec<(Collection, Vec<SavedRequest>)>, String> {
    let workdir = repo.workdir().unwrap();
    let mut results = Vec::new();

    let entries = std::fs::read_dir(workdir).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            if let Ok(cf) = serde_json::from_str::<CollectionFile>(&json) {
                results.push((cf.collection, cf.requests));
            }
        }
    }
    Ok(results)
}

#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub id: String,
    pub message: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone)]
pub struct GitIdentity {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: String,
    pub state: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BranchInfo {
    pub name: String,
    pub is_head: bool,
}

#[derive(Debug, Clone, Default)]
pub struct RepoStatus {
    pub branch: String,
    pub changes: Vec<FileChange>,
    pub remote_url: Option<String>,
    pub ahead: usize,
    pub behind: usize,
}

pub fn set_identity(data_dir: &PathBuf, name: &str, email: &str) -> Result<(), String> {
    let repo = open_at(data_dir)?;
    let mut cfg = repo.config().map_err(|e| e.to_string())?;
    cfg.set_str("user.name", name).map_err(|e| e.to_string())?;
    cfg.set_str("user.email", email).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn resolve_identity(data_dir: &PathBuf) -> Option<GitIdentity> {
    if let Ok(repo) = open_at(data_dir) {
        if let Ok(cfg) = repo.config() {
            let name = cfg.get_string("user.name").ok().filter(|s| !s.is_empty());
            let email = cfg.get_string("user.email").ok().filter(|s| !s.is_empty());
            if let (Some(name), Some(email)) = (name, email) {
                return Some(GitIdentity { name, email });
            }
        }
    }
    None
}

pub fn status(data_dir: &PathBuf) -> Result<RepoStatus, String> {
    let repo = open_at(data_dir)?;
    let mut opts = StatusOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true);
    let statuses = repo.statuses(Some(&mut opts)).map_err(|e| e.to_string())?;

    let mut changes = Vec::new();
    for entry in statuses.iter() {
        let s = entry.status();
        let state = if s.is_wt_new() || s.is_index_new() {
            "new"
        } else if s.is_wt_deleted() || s.is_index_deleted() {
            "deleted"
        } else if s.is_wt_renamed() || s.is_index_renamed() {
            "renamed"
        } else if s.is_wt_modified() || s.is_index_modified() {
            "modified"
        } else {
            "changed"
        };
        if let Ok(path) = entry.path() {
            changes.push(FileChange { path: path.to_owned(), state: state.to_owned() });
        }
    }

    let branch = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().ok().map(|s| s.to_owned()))
        .unwrap_or_else(|| "main".to_owned());
    let remote_url = repo
        .find_remote("origin")
        .ok()
        .and_then(|r| r.url().ok().map(|s| s.to_owned()));
    let (ahead, behind) = ahead_behind(&repo, &branch).unwrap_or((0, 0));

    Ok(RepoStatus { branch, changes, remote_url, ahead, behind })
}

fn ahead_behind(repo: &Repository, branch: &str) -> Option<(usize, usize)> {
    let local = repo.refname_to_id(&format!("refs/heads/{branch}")).ok()?;
    let upstream = repo.refname_to_id(&format!("refs/remotes/origin/{branch}")).ok()?;
    repo.graph_ahead_behind(local, upstream).ok()
}

pub fn commit(data_dir: &PathBuf, identity: &GitIdentity, message: &str) -> Result<String, String> {
    let repo = open_at(data_dir)?;
    let mut index = repo.index().map_err(|e| e.to_string())?;
    index
        .add_all(["*"], IndexAddOption::DEFAULT, None)
        .map_err(|e| e.to_string())?;
    index.write().map_err(|e| e.to_string())?;
    let tree_id = index.write_tree().map_err(|e| e.to_string())?;
    let tree = repo.find_tree(tree_id).map_err(|e| e.to_string())?;
    let sig = Signature::now(&identity.name, &identity.email).map_err(|e| e.to_string())?;
    let parents: Vec<_> = repo
        .head()
        .ok()
        .and_then(|h| h.peel_to_commit().ok())
        .into_iter()
        .collect();
    let parent_refs: Vec<&git2::Commit> = parents.iter().collect();
    let oid = repo
        .commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs)
        .map_err(|e| e.to_string())?;
    Ok(oid.to_string())
}

pub fn full_log(data_dir: &PathBuf, limit: usize) -> Vec<CommitInfo> {
    let repo = match open_at(data_dir) {
        Ok(r) => r,
        Err(_) => return vec![],
    };
    let mut revwalk = match repo.revwalk() {
        Ok(r) => r,
        Err(_) => return vec![],
    };
    if revwalk.push_head().is_err() {
        return vec![];
    }
    let _ = revwalk.set_sorting(Sort::TIME);
    revwalk
        .filter_map(|oid| {
            let oid = oid.ok()?;
            let commit = repo.find_commit(oid).ok()?;
            Some(CommitInfo {
                id: oid.to_string(),
                message: commit.message().unwrap_or("").to_owned(),
                timestamp: commit.time().seconds(),
            })
        })
        .take(limit)
        .collect()
}

pub fn working_diff(data_dir: &PathBuf) -> Result<String, String> {
    let repo = open_at(data_dir)?;
    let head_tree = repo.head().ok().and_then(|h| h.peel_to_tree().ok());
    let diff = repo
        .diff_tree_to_workdir_with_index(head_tree.as_ref(), None)
        .map_err(|e| e.to_string())?;
    let mut out = String::new();
    diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
        let origin = line.origin();
        if origin == '+' || origin == '-' || origin == ' ' {
            out.push(origin);
        }
        out.push_str(&String::from_utf8_lossy(line.content()));
        true
    })
    .map_err(|e| e.to_string())?;
    Ok(out)
}

pub fn branches(data_dir: &PathBuf) -> Result<Vec<BranchInfo>, String> {
    let repo = open_at(data_dir)?;
    let iter = repo.branches(Some(BranchType::Local)).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for item in iter {
        let (branch, _) = item.map_err(|e| e.to_string())?;
        let is_head = branch.is_head();
        if let Ok(Some(name)) = branch.name() {
            out.push(BranchInfo { name: name.to_owned(), is_head });
        }
    }
    Ok(out)
}

pub fn create_branch(data_dir: &PathBuf, name: &str) -> Result<(), String> {
    let repo = open_at(data_dir)?;
    let commit = repo
        .head()
        .map_err(|e| e.to_string())?
        .peel_to_commit()
        .map_err(|e| e.to_string())?;
    repo.branch(name, &commit, false).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn checkout_branch(data_dir: &PathBuf, name: &str) -> Result<(), String> {
    let repo = open_at(data_dir)?;
    let refname = format!("refs/heads/{name}");
    let object = repo.revparse_single(&refname).map_err(|e| e.to_string())?;
    let mut checkout = CheckoutBuilder::new();
    checkout.force();
    repo.checkout_tree(&object, Some(&mut checkout)).map_err(|e| e.to_string())?;
    repo.set_head(&refname).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn read_commit_collections(
    data_dir: &PathBuf,
    commit_id: &str,
) -> Result<Vec<(Collection, Vec<SavedRequest>)>, String> {
    let repo = open_at(data_dir)?;
    let oid = git2::Oid::from_str(commit_id).map_err(|e| e.to_string())?;
    let commit = repo.find_commit(oid).map_err(|e| e.to_string())?;
    let tree = commit.tree().map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for entry in tree.iter() {
        if entry.name().map(|n| n.ends_with(".json")).unwrap_or(false) {
            if let Ok(object) = entry.to_object(&repo) {
                if let Some(blob) = object.as_blob() {
                    if let Ok(file) = serde_json::from_slice::<CollectionFile>(blob.content()) {
                        out.push((file.collection, file.requests));
                    }
                }
            }
        }
    }
    Ok(out)
}

pub fn remote_url(data_dir: &PathBuf) -> Option<String> {
    let repo = open_at(data_dir).ok()?;
    repo.find_remote("origin").ok().and_then(|r| r.url().ok().map(|s| s.to_owned()))
}

pub fn set_remote(data_dir: &PathBuf, url: &str) -> Result<(), String> {
    let repo = open_at(data_dir)?;
    match repo.find_remote("origin") {
        Ok(_) => repo.remote_set_url("origin", url).map_err(|e| e.to_string())?,
        Err(_) => {
            repo.remote("origin", url).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn run_git(repo: &Path, args: &[&str]) -> Result<String, String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(|e| format!("could not run git (is it installed?): {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}

pub fn clone_repo(url: &str, dest: &Path) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    if dest.join(".git").is_dir() {

        return Ok(());
    }
    let output = std::process::Command::new("git")
        .arg("clone")
        .arg(url)
        .arg(dest)
        .output()
        .map_err(|e| format!("could not run git (is it installed?): {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}

pub fn fetch(data_dir: &PathBuf) -> Result<(), String> {
    run_git(data_dir, &["fetch", "--all"]).map(|_| ())
}

pub fn push(data_dir: &PathBuf) -> Result<(), String> {
    run_git(data_dir, &["push", "-u", "origin", "HEAD"]).map(|_| ())
}

pub fn pull(data_dir: &PathBuf) -> Result<String, String> {
    let output = run_git(data_dir, &["pull", "--ff-only"])?;
    let trimmed = output.trim();
    Ok(if trimmed.is_empty() { "Up to date".to_owned() } else { trimmed.to_owned() })
}

pub fn write_working_tree(
    data_dir: &PathBuf,
    collections: &[Collection],
    requests: &HashMap<String, Vec<SavedRequest>>,
    global_pre_request_script: &str,
    global_test_script: &str,
) -> Result<(), String> {
    let repo = open_at(data_dir)?;
    let workdir = repo
        .workdir()
        .ok_or_else(|| "repository has no working directory".to_owned())?
        .to_path_buf();

    let current: HashSet<String> = collections.iter().map(|c| c.id.clone()).collect();
    if let Ok(entries) = std::fs::read_dir(&workdir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.file_name().and_then(|n| n.to_str()) == Some(GLOBAL_SCRIPT_FILE) {
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if !current.contains(stem) {
                        let _ = std::fs::remove_file(&path);
                    }
                }
            }
        }
    }

    for collection in collections {
        let reqs = requests.get(&collection.id).cloned().unwrap_or_default();
        let file = CollectionFile { collection: collection.clone(), requests: reqs };
        let json = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
        std::fs::write(workdir.join(format!("{}.json", collection.id)), json)
            .map_err(|e| e.to_string())?;
    }

    let global_path = workdir.join(GLOBAL_SCRIPT_FILE);
    if global_pre_request_script.is_empty() && global_test_script.is_empty() {
        let _ = std::fs::remove_file(&global_path);
    } else {
        let file = GlobalScriptFile {
            pre_request_script: global_pre_request_script.to_owned(),
            test_script: global_test_script.to_owned(),
        };
        let json = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
        std::fs::write(&global_path, json).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn load_collections_from(data_dir: &PathBuf) -> Result<Vec<(Collection, Vec<SavedRequest>)>, String> {
    let repo = open_at(data_dir)?;
    load_collections(&repo)
}
