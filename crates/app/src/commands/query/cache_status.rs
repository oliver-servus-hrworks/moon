use crate::session::{MoonSession, SessionResult};
use clap::Args;
use moon_action::ActionNode;
use moon_action_context::TargetState;
use moon_action_graph::{ActionGraphBuilderOptions, RunRequirements};
use moon_task::TargetLocator;
use moon_task_runner::TaskRunner;
use moon_task_runner::output_hydrater::HydrateFrom;
use serde::{Deserialize, Serialize};
use starbase_utils::json;
use std::collections::BTreeMap;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct QueryCacheStatusArgs {
    #[arg(required = true, help = "List of task targets to check")]
    targets: Vec<TargetLocator>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CacheStatus {
    Hit,
    Miss,
    Uncacheable,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CacheSource {
    PreviousOutput,
    Local,
    Remote,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetCacheStatus {
    pub hash: String,
    pub status: CacheStatus,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<CacheSource>,

    /// Whether the target was requested, or pulled in as a dependency.
    pub requested: bool,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryCacheStatusResult {
    pub targets: BTreeMap<String, TargetCacheStatus>,
}

#[instrument(skip(session))]
pub async fn cache_status(session: MoonSession, args: QueryCacheStatusArgs) -> SessionResult {
    let app_context = session.get_app_context().await?;
    let workspace_graph = session.get_workspace_graph().await?;

    // Hashing only reads the workspace, so avoid the actions that would mutate it
    let mut action_graph_builder = session
        .build_action_graph_with_options(ActionGraphBuilderOptions::new(false))
        .await?;

    let partition = action_graph_builder
        .run_tasks(&args.targets, RunRequirements::default())
        .await?;

    let requested = partition.targets.values().cloned().collect::<Vec<_>>();
    let (action_context, action_graph) = action_graph_builder.build();

    let mut result = QueryCacheStatusResult {
        targets: BTreeMap::default(),
    };

    // Dependency hashes are part of a task's hash, so the graph must be
    // walked in the same order the pipeline would run it
    for node_index in action_graph.sort_topological()? {
        let Some(node) = action_graph
            .get_inner_graph()
            .node_weight(node_index)
            .and_then(|index| action_graph.get_node_from_index(index))
        else {
            continue;
        };

        let ActionNode::RunTask(inner) = node else {
            continue;
        };

        let project = workspace_graph.get_project(inner.target.get_project_id()?)?;
        let task = workspace_graph.get_task(&inner.target)?;
        let mut runner = TaskRunner::new(&app_context, &project, &task, None)?;

        let hash = runner.hash(&action_context, node).await?;

        // Record the state so that dependents hash the same as they would in a real run
        action_context.set_target_state(&task.target, TargetState::Passed(hash.to_string()));

        let (status, source) = if runner.is_cache_enabled() {
            match runner.is_cached(&hash).await? {
                Some(HydrateFrom::PreviousOutput) => {
                    (CacheStatus::Hit, Some(CacheSource::PreviousOutput))
                }
                Some(HydrateFrom::LocalArchive) => (CacheStatus::Hit, Some(CacheSource::Local)),
                Some(HydrateFrom::Storage(manifest)) => (
                    CacheStatus::Hit,
                    Some(if manifest.remote {
                        CacheSource::Remote
                    } else {
                        CacheSource::Local
                    }),
                ),
                None => (CacheStatus::Miss, None),
            }
        } else {
            (CacheStatus::Uncacheable, None)
        };

        result.targets.insert(
            task.target.to_string(),
            TargetCacheStatus {
                hash: hash.to_string(),
                status,
                source,
                requested: requested.contains(&task.target),
            },
        );
    }

    session
        .console
        .out
        .write_line(json::format(&result, true)?)?;

    Ok(None)
}
