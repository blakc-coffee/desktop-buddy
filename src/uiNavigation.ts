export type Page =
  | "Activity"
  | "Projects"
  | "Shelf"
  | "Codex"
  | "Fill"
  | "Music"
  | "Settings";
export const pageInfo: Record<Page, { title: string }> = {
  Activity: { title: "Overview" },
  Projects: { title: "Projects" },
  Shelf: { title: "Shelf" },
  Codex: { title: "Codex" },
  Fill: { title: "Screen helper" },
  Music: { title: "Music" },
  Settings: { title: "Settings" },
};
