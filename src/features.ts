// Archived implementations and saved data stay available for a future release.
export const features = { codexChats: false, workspaces: false };
export function pageEnabled(page: string) {
  return page === "Codex" ? features.codexChats : page === "Projects" ? features.workspaces : true;
}
