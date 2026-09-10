# Task metrics journal

Policy: [`METRICS_POLICY.md`](./METRICS_POLICY.md)

Append new entries below (newest at bottom). Only `Kind: PRODUCT` counts in 10-task reports.

---

<!-- Entries start below this line -->

## TASK-2026-09-10-001

Kind: PRODUCT

Date: 2026-09-10
Project: codenotch
Task: Windows notch click-through — transparent area was blocking hover/clicks on apps underneath

Routing:
- Complexity: SMALL
- Type: BUG
- Budget profile: SMALL
- Budget reason: Known UI overlay bug; fix path mirrors macOS ignoresMouseEvents
- Workflow selected: senior → reviewer → fix → validation
- Architect used: NO
- Architect model: n/a
- Parallel workers: 0
- Initial tier: ROUTINE
- Premium used: 0
- Escalation: NO
- Reason for escalation: none

Budget:
- Agents spawned: 3
- Models used: composer-2.5 (senior×2), kimi-k2.7-code (reviewer×2)
- Expensive model used: NO
- Full repo scan: NO
- Full repo scan reason: none
- Repeated context load suspected: NO
- Context scope: windows/codenotch main.rs + notch.html; macOS NotchWindowController for reference
- Audit scope: N/A
- Findings count: N/A
- Actual tokens: UNAVAILABLE
- Actual cost: UNAVAILABLE

Architect:
- model: n/a (orchestrator-only)
- effort: low

Workers:
- role: senior
- model: composer-2.5
- effort: medium

Reviewer:
- model: kimi-k2.7-code
- effort: medium

Result:
- STATUS: DONE
- Reviewer first verdict: CHANGES REQUIRED
- Review cycles: 2
- Validation: PASS (cargo check)
- User intervention required: NO
- Scope creep detected: NO
- Post-DONE regression: UNKNOWN
- Agents used: senior, reviewer
- Files changed: windows/codenotch/src/main.rs, windows/codenotch/ui/notch.html
- Approx complexity: SMALL

Review findings:
- Critical: 1 (lock-order deadlock — fixed)
- Important: 1 (welcome bbox click-through — fixed)
- Minor: 3 (deferred: CSS zoom/DPR, fillet hit, invoke spam)

Notes:
- Commit d5731c7 on publish-windows → publish/main; BUILD r33

## TASK-2026-09-10-002

Kind: PRODUCT

Date: 2026-09-10
Project: codenotch
Task: Scroll blocked near right-edge notch (YouTube) — harden click-through for WebView2 + install r35

Routing:
- Complexity: SMALL
- Type: BUG
- Budget profile: SMALL
- Budget reason: Follow-up to click-through; WebView2 child HWND + stale install
- Workflow selected: senior → reviewer → fix → validation → local install
- Architect used: NO
- Architect model: n/a
- Parallel workers: 0
- Initial tier: ROUTINE
- Premium used: 0
- Escalation: NO
- Reason for escalation: none

Budget:
- Agents spawned: 4
- Models used: composer-2.5 (senior×2), kimi-k2.7-code (reviewer×2)
- Expensive model used: NO
- Full repo scan: NO
- Full repo scan reason: none
- Repeated context load suspected: NO
- Context scope: windows/codenotch/src/main.rs click-through path
- Audit scope: N/A
- Findings count: N/A
- Actual tokens: UNAVAILABLE
- Actual cost: UNAVAILABLE

Architect:
- model: n/a
- effort: low

Workers:
- role: senior
- model: composer-2.5
- effort: medium

Reviewer:
- model: kimi-k2.7-code
- effort: medium

Result:
- STATUS: DONE
- Reviewer first verdict: CHANGES REQUIRED
- Review cycles: 2
- Validation: PASS (cargo release + run.log build=r35 ignore=true)
- User intervention required: NO
- Scope creep detected: NO
- Post-DONE regression: UNKNOWN
- Agents used: senior, reviewer
- Files changed: windows/codenotch/src/main.rs
- Approx complexity: SMALL

Review findings:
- Critical: 0
- Important: 2 (UI-thread marshal + ignoring-on-success — fixed)
- Minor: 1

Notes:
- Root cause of user report: still running r31 from Local\Programs; also WebView2 children needed WS_EX_TRANSPARENT. Installed r35 locally. Commit cd0ad5d / BUILD r35.
