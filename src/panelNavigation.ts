export type PanelDestination =
  | "codex"
  | "screenCapture"
  | "screenExplain"
  | "fill"
  | "activity"
  | "ask"
  | "resume"
  | "projects"
  | "shelf"
  | "timers"
  | "reminders"
  | "settings"
  | "music";
export type PanelRequest = { id: number; destination: PanelDestination };
export function destinationPage(destination: PanelDestination) {
  switch (destination) {
    case "codex":
    case "ask":
      return "Settings";
    case "screenCapture":
    case "screenExplain":
    case "fill":
      return "Fill";
    case "resume":
    case "projects":
      return "Settings";
    case "shelf":
      return "Shelf";
    case "timers":
    case "reminders":
      return "Tasks";
    case "settings":
      return "Settings";
    case "music":
      return "Music";
    default:
      return "Activity";
  }
}
