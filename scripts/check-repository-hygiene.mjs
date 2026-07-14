import { execFileSync } from "node:child_process";
import { lstatSync, readFileSync } from "node:fs";
import { basename, extname, resolve } from "node:path";

const allowedMarkdown = new Set([
  "readme.md",
  "contributing.md",
  "security.md",
  "code_of_conduct.md",
]);
const forbiddenDirectories = [
  ".agents/",
  ".codex/",
  "app-data/",
  "blob-report/",
  "coverage/",
  "dist/",
  "docs/",
  "logs/",
  "node_modules/",
  "playwright-report/",
  "quarantine/",
  "src-tauri/gen/",
  "src-tauri/target",
  "test-results/",
];
const forbiddenExtensions = new Set([
  ".appimage",
  ".appx",
  ".appxbundle",
  ".db",
  ".deb",
  ".dll",
  ".dmg",
  ".exe",
  ".key",
  ".log",
  ".msi",
  ".msix",
  ".msixbundle",
  ".p12",
  ".pdb",
  ".pem",
  ".pfx",
  ".rpm",
  ".shm",
  ".sqlite",
  ".sqlite3",
  ".wal",
  ".zip",
]);
const binaryExtensions = new Set([".gif", ".icns", ".ico", ".jpeg", ".jpg", ".png", ".webp"]);
const maxFileBytes = 5 * 1024 * 1024;

function candidateFiles() {
  const output = execFileSync(
    "git",
    ["ls-files", "-z", "--cached", "--others", "--exclude-standard"],
    { encoding: "utf8" },
  );

  return output
    .split("\0")
    .filter(Boolean)
    .map((file) => file.replaceAll("\\", "/"));
}

function environmentPaths() {
  const values = [process.cwd(), process.env.USERPROFILE, process.env.HOME].filter(Boolean);
  return [...new Set(values.flatMap((value) => [value, value.replaceAll("\\", "/")]))];
}

const failures = [];
const files = candidateFiles();
const privatePaths = environmentPaths();

for (const file of files) {
  const lower = file.toLowerCase();
  const extension = extname(lower);
  const name = basename(lower);
  const absolute = resolve(file);
  const metadata = lstatSync(absolute);

  if (extension === ".md" && !allowedMarkdown.has(lower)) {
    failures.push(`${file}: Markdown is not in the public allowlist`);
  }

  if (forbiddenDirectories.some((directory) => lower.startsWith(directory))) {
    failures.push(`${file}: generated, private, or internal directory`);
  }

  if (forbiddenExtensions.has(extension)) {
    failures.push(`${file}: generated or sensitive file type`);
  }

  if ((name === ".env" || name.startsWith(".env.")) && !name.endsWith(".example")) {
    failures.push(`${file}: environment values must not be committed`);
  }

  if (metadata.size > maxFileBytes) {
    failures.push(`${file}: exceeds the ${maxFileBytes / 1024 / 1024} MiB public-file limit`);
  }

  if (!binaryExtensions.has(extension) && metadata.size <= 2 * 1024 * 1024) {
    const contents = readFileSync(absolute, "utf8");
    const exposedPath = privatePaths.find((privatePath) =>
      privatePath.length >= 4 ? contents.includes(privatePath) : false,
    );
    if (exposedPath) {
      failures.push(`${file}: contains a machine-specific home or repository path`);
    }
  }
}

if (failures.length > 0) {
  console.error("Repository hygiene check failed:\n");
  for (const failure of [...new Set(failures)].sort()) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}

console.log(`Repository hygiene check passed for ${files.length} public files.`);
