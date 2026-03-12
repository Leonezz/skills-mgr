import { invoke } from "@tauri-apps/api/core"
import { z } from "zod"
import {
  SkillSchema,
  ProfilesResponseSchema,
  ProjectSchema,
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

export async function importSkill(sourcePath: string) {
  return await invoke("import_skill", { sourcePath }) as string
}

export async function importRemoteSkill(url: string) {
  return await invoke("import_remote_skill", { url }) as string
}

export interface RemoteSkillEntry {
  name: string
  description: string | null
  subpath: string
}

export async function browseRemote(url: string) {
  const data = await invoke("browse_remote", { url })
  return data as RemoteSkillEntry[]
}

export async function importFromBrowse(subpaths: string[]) {
  return await invoke("import_from_browse", { subpaths }) as string
}

export async function openSkillDir(name: string) {
  await invoke("open_skill_dir", { name })
}

export async function removeSkill(name: string) {
  return await invoke("remove_skill", { name }) as string
}

export async function readSkillContent(name: string) {
  return await invoke("read_skill_content", { name }) as string
}

export async function updateSkill(name: string, description: string) {
  return await invoke("update_skill", { name, description }) as string
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

export async function toggleAgent(name: string, enabled: boolean) {
  return await invoke("toggle_agent", { name, enabled }) as string
}

// --- Projects ---

export async function listProjects() {
  const data = await invoke("list_projects")
  return z.array(ProjectSchema).parse(data)
}

export async function addProject(path: string, name?: string) {
  return await invoke("add_project", { path, name }) as string
}

export async function removeProject(path: string) {
  return await invoke("remove_project", { path }) as string
}

export async function linkProfileToProject(projectPath: string, profileName: string) {
  return await invoke("link_profile_to_project", { projectPath, profileName }) as string
}

export async function unlinkProfileFromProject(projectPath: string, profileName: string) {
  return await invoke("unlink_profile_from_project", { projectPath, profileName }) as string
}

export async function activateProject(projectPath: string) {
  return await invoke("activate_project", { projectPath }) as string
}

export async function deactivateProject(projectPath: string) {
  return await invoke("deactivate_project", { projectPath }) as string
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

// --- Settings ---

export interface SettingsPayload {
  mcp_enabled: boolean
  mcp_port: number
  mcp_transport: string
  git_sync_enabled: boolean
  git_sync_repo_url: string
}

export async function getSettings() {
  return await invoke("get_settings") as SettingsPayload
}

export async function saveSettings(payload: SettingsPayload) {
  return await invoke("save_settings", { payload }) as string
}

// --- Logs ---

export async function getRecentLogs(limit = 20) {
  const data = await invoke("get_recent_logs", { limit })
  return z.array(LogEntrySchema).parse(data)
}
