use tracing::subscriber;

pub fn init() -> anyhow::Result<Initialization> {
    subscriber::set_global_default(subscriber::NoSubscriber::new())?;
    Ok(Initialization)
}

pub struct Initialization;

impl Initialization {
    pub fn log_initialization_warning(&mut self) {}

    pub(crate) fn shutdown(&mut self) {}
}
