import { invoke } from "@tauri-apps/api/core"
import { z } from "zod"
import {
  SkillSchema,
  ProfilesResponseSchema,
  AgentSchema,
  LogEntrySchema,
} from "./schemas"

// --- Skills ---

export async function listSkills() {
  const data = await invoke("list_skills")
  return z.array(SkillSchema).parse(data)
}

export async function createSkill(name: string, description: string) {
  return await invoke("create_skill", { name, description }) as string
}

export async function removeSkill(name: string) {
  return await invoke("remove_skill", { name }) as string
}

// --- Profiles ---

export async function listProfiles() {
  const data = await invoke("list_profiles")
  return ProfilesResponseSchema.parse(data)
}

export async function createProfile(
  name: string,
  skills: string[],
  includes: string[],
  description?: string,
) {
  return await invoke("create_profile", { name, skills, includes, description }) as string
}

export async function editProfile(
  name: string,
  addSkills: string[],
  removeSkills: string[],
  addIncludes: string[],
  description?: string,
) {
  return await invoke("edit_profile", { name, addSkills, removeSkills, addIncludes, description }) as string
}

export async function deleteProfile(name: string) {
  return await invoke("delete_profile", { name }) as string
}

// --- Agents ---

export async function listAgents() {
  const data = await invoke("list_agents")
  return z.array(AgentSchema).parse(data)
}

export async function addAgent(name: string, projectPath: string, globalPath: string) {
  return await invoke("add_agent", { name, projectPath, globalPath }) as string
}

export async function editAgent(name: string, projectPath: string, globalPath: string) {
  return await invoke("edit_agent", { name, projectPath, globalPath }) as string
}

export async function removeAgent(name: string) {
  return await invoke("remove_agent", { name }) as string
}

// --- Status & Placements ---

export async function getStatus(projectPath: string) {
  return await invoke("get_status", { projectPath })
}

export async function activateProfile(profileName: string, projectPath: string, force = false) {
  return await invoke("activate_profile", { profileName, projectPath, force }) as string
}

export async function deactivateProfile(profileName: string, projectPath: string) {
  return await invoke("deactivate_profile", { profileName, projectPath }) as string
}

// --- Logs ---

export async function getRecentLogs(limit = 20) {
  const data = await invoke("get_recent_logs", { limit })
  return z.array(LogEntrySchema).parse(data)
}
