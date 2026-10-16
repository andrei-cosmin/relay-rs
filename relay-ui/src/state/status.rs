#[derive(Clone, PartialEq)]
pub(crate) enum Status {
    Loading,
    Restarting,
    Ready,
    Edited,
    Saving,
    Saved,
    Failed(String),
}

impl Status {
    pub(crate) fn label(&self) -> &str {
        match self {
            Self::Loading => "Loading…",
            Self::Restarting => "Restarting…",
            Self::Ready => "Up to date",
            Self::Edited => "Unsaved changes",
            Self::Saving => "Saving…",
            Self::Saved => "Saved",
            Self::Failed(error) => error,
        }
    }

    pub(crate) fn tone(&self) -> &'static str {
        match self {
            Self::Saved => "ok",
            Self::Failed(_) => "bad",
            Self::Edited => "pending",
            Self::Restarting => "pending",
            Self::Loading | Self::Ready | Self::Saving => "neutral",
        }
    }
}
