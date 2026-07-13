// Keep in sync with the `--strict` types listed in .pre-commit-config.yaml's
// conventional-pre-commit hook and the commit_parsers in release-plz.toml.
export default {
  extends: ["@commitlint/config-conventional"],
  rules: {
    "type-enum": [
      2,
      "always",
      ["feat", "fix", "perf", "docs", "refactor", "revert", "style", "test", "chore", "ci", "build"],
    ],
  },
};
