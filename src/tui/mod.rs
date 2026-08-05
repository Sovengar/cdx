pub mod actions;
pub mod app;
pub mod events;
pub mod grep;
pub mod score;
pub mod ui;

pub fn run(initial_query: Option<String>) -> anyhow::Result<Option<std::path::PathBuf>> {
    events::run(initial_query)
}
