import { copyFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { resolve } from "node:path";

const root = fileURLToPath(new URL("..", import.meta.url));
const source = resolve(root, "../../crates/ri-api/assets/repo-explorer/index.html");
const target = resolve(root, "../../crates/ri-api/assets/repo_explorer.html");

await copyFile(source, target);
