use std::path::PathBuf;

use git2::{Repository, Signature};
use serde_json;

use crate::domain::collection::{Collection, SavedRequest};

const COLLECTIONS_DIR: &str = "collections";

pub fn open_repo(data_dir: &PathBuf) -> Result<Repository, String> {
    let repo_path = data_dir.join(COLLECTIONS_DIR);
    std::fs::create_dir_all(&repo_path).map_err(|e| e.to_string())?;
    match Repository::open(&repo_path) {
        Ok(r) => Ok(r),
        Err(_) => Repository::init(&repo_path).map_err(|e| e.to_string()),
    }
}

fn sig() -> Result<Signature<'static>, String> {
    Signature::now("Rustman", "rustman@local").map_err(|e| e.to_string())
}

fn collection_path(repo: &Repository, collection_id: &str) -> PathBuf {
    repo.workdir().unwrap().join(format!("{collection_id}.json"))
}

#[derive(serde::Serialize, serde::Deserialize)]
struct CollectionFile {
    collection: Collection,
    requests: Vec<SavedRequest>,
}

pub fn save_collection(
    repo: &Repository,
    collection: &Collection,
    requests: &[SavedRequest],
) -> Result<(), String> {
    let file = CollectionFile { collection: collection.clone(), requests: requests.to_vec() };
    let json = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
    let path = collection_path(repo, &collection.id);
    std::fs::write(&path, json).map_err(|e| e.to_string())?;

    let workdir = repo.workdir().unwrap();
    let rel = path.strip_prefix(workdir).unwrap();

    let mut index = repo.index().map_err(|e| e.to_string())?;
    index.add_path(rel).map_err(|e| e.to_string())?;
    index.write().map_err(|e| e.to_string())?;

    let tree_id = index.write_tree().map_err(|e| e.to_string())?;
    let tree = repo.find_tree(tree_id).map_err(|e| e.to_string())?;
    let sig = sig()?;

    let parent: Vec<_> = repo
        .head()
        .ok()
        .and_then(|h| h.peel_to_commit().ok())
        .into_iter()
        .collect();

    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        &format!("Update collection: {}", collection.name),
        &tree,
        parent.iter().collect::<Vec<_>>().as_slice(),
    )
    .map_err(|e| e.to_string())?;

    Ok(())
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

pub fn delete_collection(repo: &Repository, collection_id: &str) -> Result<(), String> {
    let path = collection_path(repo, collection_id);
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }

    let workdir = repo.workdir().unwrap();
    let rel_path = path.strip_prefix(workdir).unwrap().to_path_buf();

    let mut index = repo.index().map_err(|e| e.to_string())?;
    let _ = index.remove_path(&rel_path);
    index.write().map_err(|e| e.to_string())?;

    let tree_id = index.write_tree().map_err(|e| e.to_string())?;
    let tree = repo.find_tree(tree_id).map_err(|e| e.to_string())?;
    let sig = sig()?;

    let parent: Vec<_> = repo
        .head()
        .ok()
        .and_then(|h| h.peel_to_commit().ok())
        .into_iter()
        .collect();

    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        &format!("Delete collection: {collection_id}"),
        &tree,
        parent.iter().collect::<Vec<_>>().as_slice(),
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub id: String,
    pub message: String,
    pub timestamp: i64,
}

pub fn collection_log(repo: &Repository, collection_id: &str) -> Vec<CommitInfo> {
    let filename = format!("{collection_id}.json");
    let mut revwalk = match repo.revwalk() {
        Ok(r) => r,
        Err(_) => return vec![],
    };
    let _ = revwalk.push_head();
    let _ = revwalk.set_sorting(git2::Sort::TIME);

    revwalk
        .filter_map(|oid| {
            let oid = oid.ok()?;
            let commit = repo.find_commit(oid).ok()?;
            let tree = commit.tree().ok()?;
            tree.get_name(&filename)?; // only include commits touching this file
            Some(CommitInfo {
                id: oid.to_string(),
                message: commit.message().unwrap_or("").to_owned(),
                timestamp: commit.time().seconds(),
            })
        })
        .take(50)
        .collect()
}
