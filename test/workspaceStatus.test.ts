import assert from "node:assert/strict";
import path from "node:path";
import { getWorkspaceDisplayName } from "../src/ui/workspaceStatus.js";

const workspacePath = path.join("D:", "Projects-star", "Xuflow");
assert.equal(getWorkspaceDisplayName(workspacePath), "Xuflow");

const rootPath = path.parse(process.cwd()).root;
assert.equal(getWorkspaceDisplayName(rootPath), rootPath);

