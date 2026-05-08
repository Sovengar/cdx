pub mod app;
pub mod events;
pub mod ui;

pub fn run(initial_query: Option<String>) -> anyhow::Result<Option<std::path::PathBuf>> {
    events::run(initial_query)
}
