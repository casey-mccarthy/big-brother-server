---
description: Run linter and tests, fix issues, then push to PR (creates one if needed)
---

Run the following workflow to lint, test, and push changes to a PR:

## Step 1: Run Linter
Run `cargo clippy --all-targets --all-features -- -D warnings` and fix any issues found. Make the minimal changes necessary to resolve clippy warnings.

## Step 2: Check Formatting
Run `cargo fmt --all -- --check`. If formatting issues exist, run `cargo fmt --all` to fix them.

## Step 3: Run Tests
Run `cargo test --all-features`. If any tests fail, investigate and fix the issues. Re-run linter and formatter after making fixes.

## Step 4: Commit Changes
If there are any changes (from fixes or formatting):
1. Stage all modified files
2. Create a commit with message: "fix: address linter warnings and formatting issues"
3. Use conventional commit format

## Step 5: Push to PR
1. Check if current branch has an open PR using `gh pr view --json state,url`
2. If PR exists and is open: push changes to the branch
3. If no PR exists:
   - Push the branch to origin
   - Create a new PR with `gh pr create` including a summary of changes

Report the final status and PR URL when complete.
