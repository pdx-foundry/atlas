//! Stable published failure text, independent of Native's Debug implementation.
use pdx_native::{Disposal, Error};

pub(super) fn error_reason(error: &Error) -> String {
    match error {
        Error::Unsupported { reason, .. } => format!("Operation unsupported: {reason}"),
        Error::BuildChanged => "The executable changed after it was opened".into(),
        Error::FixtureRequest { reason } => format!("Invalid fixture request: {reason}"),
        Error::UnknownRegistry { name } => format!("No registry is named {name}"),
        Error::UnknownCommand { name, .. } => format!("No command is named {name}"),
        Error::Method(reason) => format!("Method failed: {reason}"),
        Error::Observation { reason, .. } => format!("Observation not established: {reason}"),
        Error::Startup { reason, disposal } => format!(
            "Game startup failed: {reason}; {}",
            disposal_reason(disposal)
        ),
        Error::Cleanup { reason, disposal } => format!(
            "Session cleanup failed: {reason}; {}",
            disposal_reason(disposal)
        ),
        Error::Closed => "The game session is closed".into(),
        Error::NotRecorded { question } => format!("No answer is recorded for {question}"),
        Error::Recorded(reason) => format!("Recorded answer failed: {reason}"),
        Error::Supervisor(reason) => format!("Supervisor connection failed: {reason}"),
    }
}

fn disposal_reason(disposal: &Disposal) -> String {
    match disposal {
        Disposal::Confirmed => "process disposal confirmed".into(),
        Disposal::NotApplicable => "no game process was created".into(),
        Disposal::Unconfirmed(reason) => format!("process disposal unconfirmed: {reason}"),
    }
}

pub(super) fn session_reason(result: &Result<Disposal, Error>) -> String {
    match result {
        Ok(disposal) => disposal_reason(disposal),
        Err(error) => error_reason(error),
    }
}
