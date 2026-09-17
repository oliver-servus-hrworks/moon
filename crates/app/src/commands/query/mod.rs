pub mod affected;
pub mod cache_status;
pub mod changed_files;
pub mod projects;
pub mod tasks;

use clap::Subcommand;

pub(super) const HEADING_AFFECTED: &str = "Affected by";
pub(super) const HEADING_FILTERS: &str = "Filters";

#[derive(Clone, Debug, Subcommand)]
pub enum QueryCommands {
    #[command(
        name = "affected",
        about = "Query affected status for projects and tasks."
    )]
    Affected(affected::QueryAffectedArgs),

    #[command(
        name = "cache-status",
        about = "Query whether task targets would be a cache hit, without running them."
    )]
    CacheStatus(cache_status::QueryCacheStatusArgs),

    #[command(
        name = "changed-files",
        about = "Query for changed files between revisions."
    )]
    ChangedFiles(changed_files::QueryChangedFilesArgs),

    #[command(
        name = "projects",
        about = "Query for projects within the project graph."
    )]
    Projects(projects::QueryProjectsArgs),

    #[command(
        name = "tasks",
        about = "Query for tasks within the task graph, grouped by project."
    )]
    Tasks(tasks::QueryTasksArgs),
}
