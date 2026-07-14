import { execFileSync } from "node:child_process";

function currentBranch() {
  if (process.env.BRANCH_NAME) {
    return process.env.BRANCH_NAME;
  }
  if (process.argv[2]) {
    return process.argv[2];
  }
  return execFileSync("git", ["branch", "--show-current"], { encoding: "utf8" }).trim();
}

const branch = currentBranch();
const topicPattern = /^(feature|fix|security|refactor|test|docs|chore)\/[a-z0-9][a-z0-9._-]*$/;
const allowed = branch === "main" || branch.startsWith("dependabot/") || topicPattern.test(branch);

if (!allowed) {
  console.error(
    `Invalid branch name: ${branch || "(detached HEAD)"}. ` +
      "Use main, dependabot/*, or a short-lived feature|fix|security|refactor|test|docs|chore branch.",
  );
  process.exit(1);
}

console.log(`Branch name accepted: ${branch}`);
