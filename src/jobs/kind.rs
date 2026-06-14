
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JobKind {
    Request,
    Parse,
    Format,
}

impl JobKind {
    pub(crate) const COUNT: usize = 3;

    pub(crate) const fn index(self) -> usize {
        match self {
            JobKind::Request => 0,
            JobKind::Parse => 1,
            JobKind::Format => 2,
        }
    }
}
