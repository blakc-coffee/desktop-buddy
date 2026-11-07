export type PanelDestination = "fill" | "activity" | "ask" | "resume" | "projects" | "shelf" | "timers" | "reminders" | "settings" | "music";
export type PanelRequest = { id: number; destination: PanelDestination };
export function destinationPage(destination: PanelDestination) {
  switch (destination) {
    case "fill": return "Fill";
    case "projects": return "Projects";
    case "shelf": return "Shelf";
    case "timers": case "reminders": return "Tasks";
    case "settings": return "Settings";
    case "music": return "Music";
    default: return "Activity";
  }
}
