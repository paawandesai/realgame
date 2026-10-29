// Pre-commit checks on staged files only. lint-staged hides unstaged edits
// while these run, so they see exactly what is being committed.
// Budget: under 10 s (measured; see the CI PR for numbers).
export default {
  // rustfmt in check mode: never rewrites the index, just blocks the commit.
  // Fix with `cargo fmt --all`.
  "*.rs": "rustfmt --check --edition 2021",
  // tsc cannot check single files against tsconfig, so any staged web TS
  // triggers one project-wide type check (about 1.5 s).
  "web/**/*.ts": () => "sh scripts/hooks/typecheck-web.sh",
};
