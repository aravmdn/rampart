# Blocked File Read Verification

Goal: prove early Phase 1 trust loop for one blocked filesystem read.

Loop:
1. Launch session through daemon contract with selected project and profile.
2. Feed adapter raw blocked read for path outside project roots.
3. Normalize raw engine event into:
   - audit event with blocked outcome
   - violation event with stable rule metadata
4. Show violation explanation in desktop shell.

Automated artifacts:
- `crates/rampartd/tests/blocked_action_loop.rs`
- `apps/desktop/src/App.test.tsx`

Evidence expected:
- daemon event stream contains one launch audit, one blocked audit, one violation
- violation rule id is `fs.read.project-root-only`
- violation reason says access outside allowed project roots
- desktop shell renders blocked read explanation for target path

Current gap:
- real engine execution still stubbed
- Rust compile and test run blocked on missing local `cargo`

Claim boundary:
- This artifact proves contract shape and user-visible explanation path.
- It does not claim real OS enforcement until engine execution is wired and verified on target platform.
