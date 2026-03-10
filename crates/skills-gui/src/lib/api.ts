import { invoke } from "@tauri-apps/api/core"
import { z } from "zod"
import { SkillSchema, LogEntrySchema } from "./schemas"

export async function listSkills() {
  const data = await invoke("list_skills")
  return z.array(SkillSchema).parse(data)
}

export async function listProfiles() {
  return await invoke("list_profiles")
}

export async function listAgents() {
  return await invoke("list_agents")
}

export async function getStatus(projectPath: string) {
  return await invoke("get_status", { projectPath })
}

export async function activateProfile(profileName: string, projectPath: string, force = false) {
  return await invoke("activate_profile", { profileName, projectPath, force })
}

export async function deactivateProfile(profileName: string, projectPath: string) {
  return await invoke("deactivate_profile", { profileName, projectPath })
}

export async function getRecentLogs(limit = 20) {
  const data = await invoke("get_recent_logs", { limit })
  return z.array(LogEntrySchema).parse(data)
}
