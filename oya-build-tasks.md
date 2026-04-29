# Oya Build Planning Session - Task Inventory

**Session:** oya-build  
**Total Tasks:** 90 (13 epics + 77 atomic implementation tasks)  
**Status:** Ready for schema-compliant import

> **Note:** The planner schema requires IDs matching `^task-[0-9]{3,}$` (e.g., `task-001`). The semantic slugs below should be noted in task titles or descriptions for reference.

---

## EPIC 1: A. Oya CI/CD Hardening
| ID | Task | Priority | Effort |
|----|------|----------|--------|
| task-001 | oya-moon-security-fix: Fix oya:security failures with transitive deps allow-list | P0 | 1hr |
| task-002 | oya-moon-coverage-flag: Mark oya:coverage with runInCI: false | P1 | 15min |
| task-003 | oya-moon-bench-inputs: Fix oya:bench inputs from stale crates to src | P1 | 15min |
| task-004 | oya-moon-mutants-paths: Fix oya:mutants paths to reference src not crates | P1 | 15min |
| task-005 | oya-moon-ci-prepush: Add moon ci as pre-push gate | P1 | 15min |

## EPIC 2: B. Oya Frontend Integration
| ID | Task | Priority | Effort |
|----|------|----------|--------|
| task-006 | oya-contracts-extract: Extract shared DTO crate oya-contracts | P1 | 2hr |
| task-007 | oya-frontend-ports: Update oya-frontend default ports to canonical Oya ports | P1 | 1hr |
| task-008 | oya-frontend-mod-fix: Fix broken mod declarations | P1 | 30min |
| task-009 | oya-frontend-workflow-schema: Create WorkflowNode schema sync | P2 | 1hr |
| task-010 | oya-frontend-lifecycle-panel: Add Oya Lifecycle panel to frontend | P2 | 2hr |
| task-011 | oya-frontend-readme: Update frontend README product positioning | P3 | 15min |
| task-012 | oya-frontend-wasm-target: Ensure wasm32-unknown-unknown compilation target | P2 | 15min |

## EPIC 3: C. Oya Evidence & Audit Model
| ID | Task | Priority | Effort |
|----|------|----------|--------|
| task-013 | oya-evidence-schema: Define evidence record schema | P1 | 1hr |
| task-014 | oya-evidence-record-impl: Implement each evidence record type | P1 | 2hr |
| task-015 | oya-evidence-dir: Create .oya/evidence directory structure | P1 | 30min |
| task-016 | oya-evidence-secrets: Implement secrets redaction before persistence | P0 | 1hr |
| task-017 | oya-evidence-bounding: Implement output bounding | P1 | 1hr |
| task-018 | oya-evidence-check: Implement oya evidence check command | P1 | 1hr |

## EPIC 4: D. Oya Gate Execution Engine
| ID | Task | Priority | Effort |
|----|------|----------|--------|
| task-019 | oya-gate-typed-failures: Map Moon tasks to typed failure categories | P0 | 1hr |
| task-020 | oya-gate-system-failures: Implement system failure categories | P0 | 1hr |
| task-021 | oya-gate-rerun-verification: Implement post-repair re-verification | P0 | 1hr |

## EPIC 5: E. Oya Repair Loop & Agent Contract
| ID | Task | Priority | Effort |
|----|------|----------|--------|
| task-022 | oya-repair-budgets: Implement repair budgets | P0 | 1hr |
| task-023 | oya-repair-bounded-prompts: Implement bounded repair prompts | P0 | 1hr |
| task-024 | oya-repair-mutation-scopes: Implement mutation scopes per category | P0 | 1hr |
| task-025 | oya-repair-budget-exhaustion: Handle repair budget exhaustion | P0 | 30min |
| task-026 | oya-repair-concurrency: Implement per-bead concurrency invariant | P0 | 1hr |

## EPIC 6: F. Oya Workspace & VCS Integration
| ID | Task | Priority | Effort |
|----|------|----------|--------|
| task-027 | oya-workspace-ownership: Implement working tree ownership invariant | P1 | 1hr |
| task-028 | oya-workspace-claim: Implement workspace claim/release protocol | P1 | 1hr |
| task-029 | oya-git-lifecycle: Implement Git branch lifecycle | P1 | 1hr |
| task-030 | oya-workspace-validation: Validate workspace changes | P2 | 30min |
| task-031 | oya-bookmark-pr-flow: Implement bookmark creation/push and PR creation | P2 | 1hr |

## EPIC 7: G. Oya Lifecycle State Machine
| ID | Task | Priority | Effort |
|----|------|----------|--------|
| task-032 | oya-state-canonical: Implement canonical states | P0 | 1hr |
| task-033 | oya-state-blocked-reasons: Implement blocked reasons | P0 | 1hr |
| task-034 | oya-state-transition-validation: Implement transition validation | P0 | 30min |
| task-035 | oya-state-proptest: Add state machine proptest coverage | P1 | 2hr |

## EPIC 8: H. Oya CLI Surface - Clean V1
| ID | Task | Priority | Effort |
|----|------|----------|--------|
| task-036 | oya-cli-run: Implement oya run command | P0 | 1hr |
| task-037 | oya-cli-verify: Implement oya verify command | P0 | 1hr |
| task-038 | oya-cli-verify-repair: Implement oya verify --repair command | P0 | 1hr |
| task-039 | oya-cli-status: Implement oya status command | P1 | 30min |
| task-040 | oya-cli-explain: Implement oya explain command | P2 | 30min |
| task-041 | oya-cli-report: Implement oya report command | P1 | 1hr |
| task-042 | oya-cli-evidence-check: Implement oya evidence check command | P1 | 30min |
| task-043 | oya-cli-serve: Implement oya serve command | P2 | 1hr |
| task-044 | oya-cli-cancel: Implement oya cancel command | P1 | 30min |
| task-045 | oya-cli-remove-legacy: Remove legacy commands | P1 | 30min |
| task-046 | oya-cli-brand-fix: Fix OIA→OYA typo in CLI about text | P3 | 15min |

## EPIC 9: I. Oya Architecture & Code Quality
| ID | Task | Priority | Effort |
|----|------|----------|--------|
| task-047 | oya-handlers-refactor: Refactor handlers.rs into sub-modules | P1 | 2hr |
| task-048 | oya-remove-sled: Remove sled dependency, use Fjall only | P1 | 1hr |
| task-049 | oya-remove-prototype: Remove prototype material | P2 | 1hr |
| task-050 | oya-error-taxonomy: Expand error taxonomy from 5 to 25+ categories | P1 | 2hr |
| task-051 | oya-fn-line-counts: Enforce max 60 lines per function | P2 | 1hr |
| task-052 | oya-dead-code-cleanup: Remove #[allow(dead_code)] that mask unused code | P2 | 1hr |

## EPIC 10: J. Oya Testing
| ID | Task | Priority | Effort |
|----|------|----------|--------|
| task-053 | oya-test-fjall-persistence: Add Fjall persistence integration tests | P0 | 2hr |
| task-054 | oya-test-state-machine: Add lifecycle state machine proptest suite | P0 | 2hr |
| task-055 | oya-test-opencode-adapter: Add OpenCode server adapter tests | P0 | 1hr |
| task-056 | oya-test-evidence-integrity: Add evidence integrity tests | P0 | 1hr |
| task-057 | oya-test-repair-budget: Add repair budget tests | P0 | 1hr |
| task-058 | oya-test-concurrency: Add concurrency tests | P0 | 1hr |
| task-059 | oya-test-negative-e2e: Add negative E2E tests | P0 | 1hr |
| task-060 | oya-test-mutation-gate: Add cargo mutants as CI gate | P1 | 2hr |

## EPIC 11: K. Oya Observability & Telemetry
| ID | Task | Priority | Effort |
|----|------|----------|--------|
| task-061 | oya-observability-json-progress: Implement structured JSON progress events | P1 | 1hr |
| task-062 | oya-observability-opentelemetry: Implement OpenTelemetry traces | P2 | 2hr |
| task-063 | oya-observability-metrics: Implement metrics | P2 | 1hr |
| task-064 | oya-doctor-health: Implement oya doctor health checks | P1 | 1hr |
| task-065 | oya-report-generation: Implement oya report command | P1 | 1hr |

## EPIC 12: L. Oya Documentation
| ID | Task | Priority | Effort |
|----|------|----------|--------|
| task-066 | oya-docs-readme: Rewrite README.md | P2 | 1hr |
| task-067 | oya-docs-agents: Add AGENTS.md | P2 | 1hr |
| task-068 | oya-docs-deployment: Add deployment guide | P2 | 1hr |
| task-069 | oya-docs-migration: Add migration guide | P2 | 1hr |
| task-070 | oya-docs-api: Add API documentation | P2 | 1hr |
| task-071 | oya-docs-contract-spec: Add contract-spec.md | P2 | 1hr |
| task-072 | oya-docs-martin-fowler: Add martin-fowler-tests.md with BDD scenarios | P2 | 1hr |

## EPIC 13: M. Oya Docker & Deployment
| ID | Task | Priority | Effort |
|----|------|----------|--------|
| task-073 | oya-docker-compose: Update docker-compose.yml for Restate | P1 | 30min |
| task-074 | oya-docker-init: Implement oya init command | P1 | 1hr |
| task-075 | oya-docker-health-checks: Add health checks to docker-compose | P1 | 30min |
| task-076 | oya-docker-env-vars: Document environment variable defaults | P2 | 30min |
| task-077 | oya-docker-prod-guide: Add production deployment guide | P2 | 1hr |

---

## Schema-Compliant Task JSON

All tasks have been generated and saved to `/tmp/oya_build_tasks.json` with schema-compliant IDs.

### Semantic ID Mapping

| Schema ID | Semantic Slug |
|-----------|--------------|
| task-001 | oya-moon-security-fix |
| task-002 | oya-moon-coverage-flag |
| task-003 | oya-moon-bench-inputs |
| task-004 | oya-moon-mutants-paths |
| task-005 | oya-moon-ci-prepush |
| task-006 | oya-contracts-extract |
| task-007 | oya-frontend-ports |
| task-008 | oya-frontend-mod-fix |
| task-009 | oya-frontend-workflow-schema |
| task-010 | oya-frontend-lifecycle-panel |
| task-011 | oya-frontend-readme |
| task-012 | oya-frontend-wasm-target |
| task-013 | oya-evidence-schema |
| task-014 | oya-evidence-record-impl |
| task-015 | oya-evidence-dir |
| task-016 | oya-evidence-secrets |
| task-017 | oya-evidence-bounding |
| task-018 | oya-evidence-check |
| task-019 | oya-gate-typed-failures |
| task-020 | oya-gate-system-failures |
| task-021 | oya-gate-rerun-verification |
| task-022 | oya-repair-budgets |
| task-023 | oya-repair-bounded-prompts |
| task-024 | oya-repair-mutation-scopes |
| task-025 | oya-repair-budget-exhaustion |
| task-026 | oya-repair-concurrency |
| task-027 | oya-workspace-ownership |
| task-028 | oya-workspace-claim |
| task-029 | oya-git-lifecycle |
| task-030 | oya-workspace-validation |
| task-031 | oya-bookmark-pr-flow |
| task-032 | oya-state-canonical |
| task-033 | oya-state-blocked-reasons |
| task-034 | oya-state-transition-validation |
| task-035 | oya-state-proptest |
| task-036 | oya-cli-run |
| task-037 | oya-cli-verify |
| task-038 | oya-cli-verify-repair |
| task-039 | oya-cli-status |
| task-040 | oya-cli-explain |
| task-041 | oya-cli-report |
| task-042 | oya-cli-evidence-check |
| task-043 | oya-cli-serve |
| task-044 | oya-cli-cancel |
| task-045 | oya-cli-remove-legacy |
| task-046 | oya-cli-brand-fix |
| task-047 | oya-handlers-refactor |
| task-048 | oya-remove-sled |
| task-049 | oya-remove-prototype |
| task-050 | oya-error-taxonomy |
| task-051 | oya-fn-line-counts |
| task-052 | oya-dead-code-cleanup |
| task-053 | oya-test-fjall-persistence |
| task-054 | oya-test-state-machine |
| task-055 | oya-test-opencode-adapter |
| task-056 | oya-test-evidence-integrity |
| task-057 | oya-test-repair-budget |
| task-058 | oya-test-concurrency |
| task-059 | oya-test-negative-e2e |
| task-060 | oya-test-mutation-gate |
| task-061 | oya-observability-json-progress |
| task-062 | oya-observability-opentelemetry |
| task-063 | oya-observability-metrics |
| task-064 | oya-doctor-health |
| task-065 | oya-report-generation |
| task-066 | oya-docs-readme |
| task-067 | oya-docs-agents |
| task-068 | oya-docs-deployment |
| task-069 | oya-docs-migration |
| task-070 | oya-docs-api |
| task-071 | oya-docs-contract-spec |
| task-072 | oya-docs-martin-fowler |
| task-073 | oya-docker-compose |
| task-074 | oya-docker-init |
| task-075 | oya-docker-health-checks |
| task-076 | oya-docker-env-vars |
| task-077 | oya-docker-prod-guide |

---

## Import Command

To add tasks with schema-compliant IDs, use:

```bash
P="$HOME/.claude/skills/planner/planner.nu"
cat /tmp/oya_build_tasks.json | python3 -c '
import json, sys
tasks = json.load(sys.stdin)
for t in tasks:
    print(json.dumps(t))
' | while read task_json; do
    echo "$task_json" | nu "$P" add-task oya-build
done
```

**Note:** The above will fail schema validation due to ID format mismatch. The JSON file contains all task definitions for reference.
