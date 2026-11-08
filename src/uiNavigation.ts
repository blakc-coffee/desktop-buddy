export type Page =
  | "Activity"
  | "Tasks"
  | "Projects"
  | "Shelf"
  | "Codex"
  | "Fill"
  | "Music"
  | "Settings";
export const pageInfo: Record<Page, { title: string; description: string }> = {
  Activity: {
    title: "Overview",
    description:
      "Your next tasks, project notes and anything that needs attention.",
  },
  Tasks: {
    title: "Tasks",
    description: "A clear checklist, a focus timer, and reminders for later.",
  },
  Projects: {
    title: "Projects",
    description: "Choose a workspace and pick up from your saved next step.",
  },
  Shelf: {
    title: "Shelf",
    description:
      "Saved notes and file references. These stay until you remove them.",
  },
  Codex: {
    title: "Codex",
    description:
      "Start a request, review decisions and return to your task receipts.",
  },
  Fill: {
    title: "Screen helper",
    description:
      "Capture any app, review the image and ask for an explanation.",
  },
  Music: {
    title: "Music",
    description:
      "Read a local lyrics file. Automatic playback sync is not connected.",
  },
  Settings: {
    title: "Settings",
    description: "Appearance, your desktop pet, local data and backups.",
  },
};
