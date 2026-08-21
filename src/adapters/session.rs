#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderNativePreauthenticatedSession {
    Available,
    Unavailable,
    Ambiguous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PreauthenticatedSessionError {
    Unavailable,
    Ambiguous,
}

pub(crate) fn require_preauthenticated_session(
    session: ProviderNativePreauthenticatedSession,
) -> Result<(), PreauthenticatedSessionError> {
    match session {
        ProviderNativePreauthenticatedSession::Available => Ok(()),
        ProviderNativePreauthenticatedSession::Unavailable => {
            Err(PreauthenticatedSessionError::Unavailable)
        }
        ProviderNativePreauthenticatedSession::Ambiguous => {
            Err(PreauthenticatedSessionError::Ambiguous)
        }
    }
}
