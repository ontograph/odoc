use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;

use ontocode_protocol::user_input::UserInput;

/// Host-owned turn environment summary visible to turn-input contributors.
#[derive(Debug, Clone)]
pub struct TurnInputEnvironment {
    /// Stable host environment id used to route executor-scoped capabilities.
    pub environment_id: String,
    /// Effective working directory for this turn in the environment.
    pub cwd: PathBuf,
    /// Whether this is the primary environment for the turn.
    pub is_primary: bool,
}

/// Exact host-authorized local paths that extensions may read for one turn.
#[derive(Debug, Clone, Default)]
pub struct TurnReadPathGrants {
    paths: Vec<PathBuf>,
}

impl TurnReadPathGrants {
    pub fn new(paths: Vec<PathBuf>) -> Self {
        Self { paths }
    }

    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }
}

/// Host-owned wall-clock bound for one turn's extension work.
///
/// Published into turn-scoped `ExtensionData` rather than added to
/// [`TurnInputContext`], so contributors that do not need time keep their
/// current construction sites. The instant is monotonic, so
/// [`Self::remaining_ms`] shrinks as the turn progresses and reaches zero once
/// the deadline passes. Absence of this value is meaningful: a contributor
/// must keep its own fail-closed allowance rather than assume unlimited time.
#[derive(Debug, Clone, Copy)]
pub struct TurnDeadline {
    deadline: Instant,
}

impl TurnDeadline {
    /// Builds a deadline `budget` after the turn's monotonic start instant.
    pub fn from_start(started_at: Instant, budget: Duration) -> Self {
        Self {
            deadline: started_at + budget,
        }
    }

    /// Time left before the deadline, saturating at zero once it has passed.
    pub fn remaining(&self) -> Duration {
        self.deadline.saturating_duration_since(Instant::now())
    }

    /// Milliseconds left before the deadline, clamped into `u64`.
    pub fn remaining_ms(&self) -> u64 {
        u64::try_from(self.remaining().as_millis()).unwrap_or(u64::MAX)
    }
}

/// Turn facts supplied before the host records turn-local model input items.
#[derive(Debug, Clone)]
pub struct TurnInputContext {
    /// Stable host-owned turn identifier.
    pub turn_id: String,
    /// User input submitted for this turn.
    pub user_input: Vec<UserInput>,
    /// Resolved turn environments, in host priority order.
    pub environments: Vec<TurnInputEnvironment>,
}
