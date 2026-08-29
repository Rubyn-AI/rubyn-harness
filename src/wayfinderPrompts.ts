import type { WayfinderMapData, WayfinderTicket } from "./bridge";

export const WAYFINDER_SKILL_PATH = "wayfinder/wayfinder.md";
export const GRILLING_SKILL_PATH = "wayfinder/grilling.md";

function skillBody(content: string) {
  return content.replace(/^---\s*\n[\s\S]*?\n---\s*\n/, "").trim();
}

export function composeWayfinderLaunchPrompt(
  ticket: WayfinderTicket,
  data: WayfinderMapData,
  wayfinderSkill: string,
  grillingSkill?: string,
) {
  const skills = [
    `<skill name="wayfinder" source="mattpocock/skills">\n${skillBody(wayfinderSkill)}\n</skill>`,
    grillingSkill
      ? `<skill name="grilling" source="mattpocock/skills">\n${skillBody(grillingSkill)}\n</skill>`
      : undefined,
  ].filter(Boolean);
  const dependencies = ticket.dependsOn
    .map((id) => data.tickets.find((candidate) => candidate.id === id))
    .filter((candidate): candidate is WayfinderTicket => Boolean(candidate))
    .map((dependency) => `${dependency.title}: ${dependency.resolution || dependency.resultNote || dependency.status}`);

  return [
    ...skills,
    `<wayfinder-harness-adapter>
Rubyn Harness is the tracker adapter described by the Wayfinder skill. The map and ticket already exist in the Harness, so do not create, assign, close, or edit issues or local tracker files. Work only this ticket. Use the dedicated wayfinder tool for app-native map changes; never substitute prose, private task tools, or files for a Harness node.

For a grilling ticket, follow the Grilling skill as a live HITL exchange. Never answer the human's decisions yourself. Ask only the current decision frontier, give a recommended answer for every question, and wait for the human before advancing. Once the human explicitly settles the decision, use wayfinder create_node for newly visible frontier questions. The human approves the current node's final resolution in the Harness.

For other ticket types, preserve Wayfinder's distinction between decisions and deliverables. Use wayfinder create_node for newly surfaced decision tickets, in dependency order. A node_type of "code" becomes a real Task automatically only after the map is activated and its dependencies are settled. Return a concise proposed resolution with evidence, remaining uncertainty, and anything that stayed in or moved out of fog.

Every materialized code Task belongs in the Harness workflow column with key "${data.map.codeTaskStatus}". Do not choose or change that destination yourself; the human selected it when this map was started.
</wayfinder-harness-adapter>`,
    `# Active Wayfinder map

Map: ${data.map.title}
Destination: ${data.map.destination || data.map.idea}
Notes: ${data.map.notes || "None"}

# Active ticket

Type: ${ticket.ticketType}
Title: ${ticket.title}
Question: ${ticket.question || "Not provided"}
Information: ${ticket.information || "Not provided"}
Required outcome: ${ticket.outcome || "Not provided"}
Dependencies already settled: ${dependencies.length ? dependencies.join("; ") : "None"}`,
  ].join("\n\n");
}

export function composeWayfinderChartPrompt(
  data: WayfinderMapData,
  wayfinderSkill: string,
  grillingSkill: string,
) {
  const bootstrap = data.tickets.find((ticket) => ticket.title === "Name the destination") || data.tickets[0];
  return [
    `<skill name="wayfinder" source="mattpocock/skills">\n${skillBody(wayfinderSkill)}\n</skill>`,
    `<skill name="grilling" source="mattpocock/skills">\n${skillBody(grillingSkill)}\n</skill>`,
    `<wayfinder-harness-adapter>
You are charting a new map inside Rubyn Harness, which is the tracker adapter for the Wayfinder skill. Do not create issues, markdown tracker files, or private tasks.

First run Matt Pocock's Grilling process as a live human conversation. Ask only the current decision frontier, include a recommended answer for each question, and wait. Never decide for the human and never create nodes from unanswered assumptions.

After the human explicitly confirms a clear destination and first frontier:
1. Call wayfinder with action "update_map", map_id "${data.map.id}", and the settled destination and notes.
2. Call wayfinder once per visible frontier node with action "create_node". Supply map_id, title, question, description, outcome, node_type, model_role, effort, and blocked_by. Create nodes in dependency order; blocked_by may contain exact titles of nodes created earlier.
3. Use grill for human decisions, research for facts, prototype for behavior or appearance, code for executable work, and user_action only for work a person must do. Code nodes become real Harness Tasks after the human activates the map and their dependencies are settled.
4. Do not resolve the bootstrap node or activate the map. The human reviews the graph and activates it in the Harness.

The human chose the Harness workflow column with key "${data.map.codeTaskStatus}" for every code Task created by this map. Preserve that choice; never infer a different column.

Avoid duplicate nodes. Fog that cannot yet be phrased as a precise question stays in your summary rather than becoming a node.
</wayfinder-harness-adapter>`,
    `# New map\n\nMap ID: ${data.map.id}\nIdea: ${data.map.idea}\nCode task column: ${data.map.codeTaskStatus}\nBootstrap node ID: ${bootstrap?.id ?? "unknown"}`,
  ].join("\n\n");
}
