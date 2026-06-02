use iced::Task;

use crate::app::AppState;
use crate::message::{GitMsg, Message};

pub(super) fn handle(state: &mut AppState, msg: GitMsg) -> Task<Message> {
    match msg {
        GitMsg::LogLoaded(log) => state.git_log = log,
        GitMsg::CommitAll => {
            let data_dir = state.data_dir.clone();
            let collections = state.collections.clone();
            let requests = state.requests.clone();
            return Task::perform(
                async move {
                    use crate::services::vcs;
                    match vcs::open_repo(&data_dir) {
                        Ok(repo) => {
                            for col in &collections {
                                let reqs = requests.get(&col.id).map(|v| v.as_slice()).unwrap_or(&[]);
                                let _ = vcs::save_collection(&repo, col, reqs);
                            }
                            let log = if let Some(col) = collections.first() {
                                vcs::collection_log(&repo, &col.id)
                            } else {
                                vec![]
                            };
                            Ok(log)
                        }
                        Err(e) => Err(e),
                    }
                },
                |result| match result {
                    Ok(log) => Message::Git(GitMsg::LogLoaded(log)),
                    Err(e) => Message::Git(GitMsg::Error(e)),
                },
            );
        }
        GitMsg::Committed(msg) => state.status_message = Some(format!("Committed: {msg}")),
        GitMsg::Error(e) => state.status_message = Some(format!("Git error: {e}")),
    }
    Task::none()
}
