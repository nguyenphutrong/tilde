pub(crate) mod artifact_upload;
pub(crate) mod driver;
pub(crate) mod environment_snapshot;
pub(crate) mod retry;
pub(crate) mod setup_observability;

pub(crate) use driver::harness::{ClaudeHarness, task_env_vars, validate_cli_installed};

#[cfg(test)]
mod test_support;
