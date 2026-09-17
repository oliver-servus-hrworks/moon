mod utils;

use moon_app::commands::query::cache_status::*;
use moon_test_utils::MoonSandbox;
use starbase_utils::json::serde_json;
use utils::create_pipeline_sandbox;

const PROJECT_DIR: &str = if cfg!(windows) { "windows" } else { "unix" };

fn target(task: &str) -> String {
    format!("{PROJECT_DIR}:{task}")
}

fn query(sandbox: &MoonSandbox, targets: &[String]) -> QueryCacheStatusResult {
    let assert = sandbox.run_bin(|cmd| {
        cmd.arg("query").arg("cache-status").args(targets);
    });

    serde_json::from_str(assert.stdout().trim()).unwrap()
}

mod query_cache_status {
    use super::*;

    #[test]
    fn reports_a_miss_without_running_the_task() {
        let sandbox = create_pipeline_sandbox();
        let result = query(&sandbox, &[target("outputs")]);
        let status = result.targets.get(&target("outputs")).unwrap();

        assert!(matches!(status.status, CacheStatus::Miss));
        assert!(status.requested);
        assert!(!status.hash.is_empty());

        // The task itself never ran
        assert!(!sandbox.path().join(PROJECT_DIR).join("file.txt").exists());
    }

    #[test]
    fn reports_a_hit_after_the_task_has_run() {
        let sandbox = create_pipeline_sandbox();

        sandbox
            .run_bin(|cmd| {
                cmd.arg("exec").arg(target("outputs"));
            })
            .success();

        let result = query(&sandbox, &[target("outputs")]);
        let status = result.targets.get(&target("outputs")).unwrap();

        assert!(matches!(status.status, CacheStatus::Hit));
        assert!(matches!(
            status.source,
            Some(CacheSource::PreviousOutput | CacheSource::Local)
        ));
    }

    #[test]
    fn reports_uncacheable_when_caching_is_disabled() {
        let sandbox = create_pipeline_sandbox();
        let result = query(&sandbox, &["shared:notCached".into()]);
        let status = result.targets.get("shared:notCached").unwrap();

        assert!(matches!(status.status, CacheStatus::Uncacheable));
        assert!(status.source.is_none());
    }

    #[test]
    fn includes_dependencies_that_were_not_requested() {
        let sandbox = create_pipeline_sandbox();
        let result = query(&sandbox, &[target("hasFailingDep")]);

        assert!(
            result
                .targets
                .get(&target("hasFailingDep"))
                .unwrap()
                .requested
        );
        assert!(
            !result
                .targets
                .get(&target("exitNonZeroInline"))
                .unwrap()
                .requested
        );
    }
}
