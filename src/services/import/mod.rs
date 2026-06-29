pub mod httpie;
pub mod httpie_export;
pub mod postman;

use serde::{Deserialize, Serialize};

use crate::domain::collection::{Collection, SavedRequest};

#[derive(Serialize, Deserialize)]
struct NativeCollectionFile {
    collection: Collection,
    requests: Vec<SavedRequest>,
}

pub fn native_export(collection: &Collection, requests: &[SavedRequest]) -> String {
    let file = NativeCollectionFile {
        collection: collection.clone(),
        requests: requests.to_vec(),
    };
    serde_json::to_string_pretty(&file).unwrap_or_else(|_| "{}".to_owned())
}

pub fn native_import(json: &str) -> Result<Vec<(Collection, Vec<SavedRequest>)>, String> {
    let file: NativeCollectionFile =
        serde_json::from_str(json).map_err(|e| format!("Not a Rustman collection: {e}"))?;
    Ok(vec![(file.collection, file.requests)])
}
