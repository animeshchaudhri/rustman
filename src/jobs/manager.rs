use tokio_util::sync::CancellationToken;

use super::kind::JobKind;

#[derive(Debug, Default)]
pub struct JobManager {
    slots: [Slot; JobKind::COUNT],
}

#[derive(Debug, Default)]
struct Slot {
    generation: u64,
    token: Option<CancellationToken>,
}

impl JobManager {

    pub fn start(&mut self, kind: JobKind) -> (u64, CancellationToken) {
        let slot = &mut self.slots[kind.index()];
        if let Some(previous) = slot.token.take() {
            previous.cancel();
        }
        slot.generation = slot.generation.wrapping_add(1);
        let token = CancellationToken::new();
        slot.token = Some(token.clone());
        (slot.generation, token)
    }

    /// True if no newer job of `kind` has started since `generation` was issued.
    pub fn is_current(&self, kind: JobKind, generation: u64) -> bool {
        self.slots[kind.index()].generation == generation
    }

    /// Cancel the running job of `kind` without bumping the generation (e.g. Abort).
    pub fn cancel(&mut self, kind: JobKind) {
        if let Some(token) = self.slots[kind.index()].token.take() {
            token.cancel();
        }
    }

    pub fn cancel_all(&mut self) {
        for slot in &mut self.slots {
            if let Some(token) = slot.token.take() {
                token.cancel();
            }
        }
    }
}

impl Drop for JobManager {
    fn drop(&mut self) {
        self.cancel_all();
    }
}
