use iced::Task;

use crate::app::{AppState, UpdateState};
use crate::message::{Message, UpdateMsg};
use crate::services::update;

pub(super) fn handle(state: &mut AppState, msg: UpdateMsg) -> Task<Message> {
    match msg {
        UpdateMsg::Check => {
            // Don't clobber an install that's already in flight or finished.
            if matches!(state.update, UpdateState::Installing | UpdateState::Ready(_)) {
                return Task::none();
            }
            state.update = UpdateState::Checking;
            return Task::perform(update::check(), |res| {
                Message::Update(UpdateMsg::Checked(res))
            });
        }
        UpdateMsg::Checked(Ok(Some(info))) => state.update = UpdateState::Available(info),
        UpdateMsg::Checked(Ok(None)) => state.update = UpdateState::UpToDate,
        UpdateMsg::Checked(Err(e)) => state.update = UpdateState::Failed(e),
        UpdateMsg::Install => {
            state.update = UpdateState::Installing;
            return Task::perform(update::install(), |res| {
                Message::Update(UpdateMsg::Installed(res))
            });
        }
        UpdateMsg::Installed(Ok(version)) => state.update = UpdateState::Ready(version),
        UpdateMsg::Installed(Err(e)) => state.update = UpdateState::Failed(e),
        UpdateMsg::Dismiss => state.update = UpdateState::Idle,
        UpdateMsg::Restart => {
            // On success this never returns (the process is replaced); only the
            // error path falls through.
            let Err(e) = update::restart();
            state.update = UpdateState::Failed(e);
        }
    }
    Task::none()
}
