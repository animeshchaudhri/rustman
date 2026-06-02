
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JobKind {
    Request,
    Parse,
    Search,
    Format,
}

impl JobKind {
    pub(crate) const COUNT: usize = 4;

    pub(crate) const fn index(self) -> usize {
        match self {
            JobKind::Request => 0,
            JobKind::Parse => 1,
            JobKind::Search => 2,
            JobKind::Format => 3,
        }
    }
}
