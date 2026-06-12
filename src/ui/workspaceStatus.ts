import path from "node:path";

export function getWorkspaceDisplayName(workspacePath: string): string {
  const normalized = path.normalize(workspacePath.trim());
  if (!normalized) return "";

  const baseName = path.basename(normalized);
  if (baseName) return baseName;

  return path.parse(normalized).root || normalized;
}

