import { invoke } from "@tauri-apps/api/core"
import { z } from "zod"
import {
  SkillSchema,
  DiscoveredSkillSchema,
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

// --- Discovery & Delegation ---

export async function scanSkills() {
  const data = await invoke("scan_skills")
  return z.array(DiscoveredSkillSchema).parse(data)
}

export interface DelegateRequest {
  found_path: string
}

export async function delegateSkills(
  skills: DelegateRequest[],
  profileName: string,
  createProfile: boolean,
  profileDescription?: string,
) {
  return await invoke("delegate_skills", {
    skills,
    profileName,
    createProfile,
    profileDescription,
  }) as string
}

export async function linkRemote(
  name: string,
  url: string,
  gitRef: string,
  subpath?: string,
) {
  return await invoke("link_remote", { name, url, subpath, gitRef }) as string
}

export async function unlinkRemote(name: string) {
  return await invoke("unlink_remote", { name }) as string
}

export async function syncSkill(name: string) {
  return await invoke("sync_skill", { name }) as string
}

export async function syncAllSkills() {
  return await invoke("sync_all_skills") as string
}

// --- Hubs ---

export interface HubInfo {
  name: string
  display_name: string
  hub_type: string
  base_url: string
  enabled: boolean
}

export async function listHubs() {
  return await invoke("list_hubs") as HubInfo[]
}

export async function browseHub(hubName: string) {
  const data = await invoke("browse_hub", { hubName })
  return data as RemoteSkillEntry[]
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

// --- Global Skills ---

export async function activateGlobal() {
  return await invoke("activate_global") as string
}

export async function deactivateGlobal() {
  return await invoke("deactivate_global") as string
}

export async function editGlobalSkills(skills: string[]) {
  return await invoke("edit_global_skills", { skills }) as string
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
  scan_auto_on_startup: boolean
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
