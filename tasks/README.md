Parent slice(s): [sidecar-format](../slices/sidecar-format.md), [substitution-engine](../slices/substitution-engine.md), [runner](../slices/runner.md), [build-pipeline](../slices/build-pipeline.md), [cli-surface](../slices/cli-surface.md), [shim-management](../slices/shim-management.md), [distribution](../slices/distribution.md), [testing-harness](../slices/testing-harness.md)

# shrt — Tasks

| #  | Task | Path | Depends on | Slice(s) |
|----|------|------|------------|----------|
| 01 | workspace skeleton + argv-stub crate | [01-workspace-and-argv-stub](01-workspace-and-argv-stub.md) | — | distribution, testing-harness, build-pipeline |
| 02 | runner sidecar parser | [02-runner-sidecar-parser](02-runner-sidecar-parser.md) | 01 | sidecar-format, runner |
| 03 | runner substitution engine + CRT argv tokenizer | [03-runner-substitute-argv](03-runner-substitute-argv.md) | 01 | substitution-engine |
| 04 | runner path module (PATH+PATHEXT + cwd) | [04-runner-path-module](04-runner-path-module.md) | 01 | runner |
| 05 | runner main orchestration | [05-runner-main-orchestration](05-runner-main-orchestration.md) | 02, 03, 04 | runner |
| 06 | shrt CLI skeleton + build.rs embedding | [06-shrt-skeleton-buildrs](06-shrt-skeleton-buildrs.md) | 01, 05 | build-pipeline, cli-surface, distribution |
| 07 | shrt config module (writer + reader) | [07-shrt-config-module](07-shrt-config-module.md) | 06 | shim-management, sidecar-format |
| 08 | shrt paths module (shim_dir + is_on_path) | [08-shrt-paths-module](08-shrt-paths-module.md) | 06 | shim-management, cli-surface |
| 09 | shrt init + path commands | [09-shrt-init-and-path](09-shrt-init-and-path.md) | 01, 07, 08 | cli-surface, shim-management |
| 10 | shrt add + remove commands + test helpers | [10-shrt-add-remove](10-shrt-add-remove.md) | 01, 07, 08 | cli-surface, shim-management |
| 11 | shrt list + show commands | [11-shrt-list-show](11-shrt-list-show.md) | 10 | cli-surface, shim-management |
| 12 | shrt sync command | [12-shrt-sync](12-shrt-sync.md) | 10 | cli-surface, shim-management |
| 13 | shrt doctor command | [13-shrt-doctor](13-shrt-doctor.md) | 10, 12 | cli-surface, shim-management |
| 14 | integration tests: invocation/exit-codes/round-trip/name-validation/perf | [14-integration-tests-roundtrip-perf](14-integration-tests-roundtrip-perf.md) | 10, 13 | testing-harness |
| 15 | CI workflow | [15-ci-workflow](15-ci-workflow.md) | 01 | build-pipeline |
| 16 | release workflow + bundle script + Scoop job | [16-release-workflow](16-release-workflow.md) | 15 | distribution |
| 17 | README + manual smoke checklist | [17-readme-and-smoke-doc](17-readme-and-smoke-doc.md) | 13 | distribution, testing-harness |

## Dependency graph

```
01 ─┬─> 02 ─┐
    ├─> 03 ─┼─> 05 ─> 06 ─┬─> 07 ─┐
    ├─> 04 ─┘             │       ├─> 09
    │                     └─> 08 ─┤
    │                             ├─> 10 ─┬─> 11
    │                             │       ├─> 12 ─> 13 ─> 14
    │                             │       │              └─> 17
    │                             │       └─────────────────┘
    │
    ├─> 15 ─> 16
    │
    └────────────── (17 also depends on 13)
```

## Notes

- Tasks 02, 03, 04 can ship in parallel after 01 lands. They reconverge at 05.
- Tasks 11, 12 can run in parallel after 10. 13 needs both 10 and 12.
- Task 15 (CI) depends only on 01 and can be merged early to gate subsequent PRs.
- Task 17 (README) depends on 13 because the README cross-checks every documented command against `shrt --help`, which requires every command implemented.
- The original 18-task proposal merged tasks 01 and 02 (workspace + argv-stub) into a single task because the standalone workspace skeleton has no testable surface and would have violated the create-tasks "at least one AC item runs tests" rule.
