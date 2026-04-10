import { z } from "zod"

export const SkillSchema = z.object({
  name: z.string(),
  description: z.string().nullable(),
  files: z.array(z.string()),
  source_type: z.string().nullable(),
  source_url: z.string().nullable(),
  source_ref: z.string().nullable(),
  is_builtin: z.boolean(),
  dir_path: z.string(),
  total_bytes: z.number(),
  token_estimate: z.number(),
})

export const ProfileSchema = z.object({
  name: z.string(),
  description: z.string().nullable(),
  skills: z.array(z.string()),
  includes: z.array(z.string()),
  active_projects: z.array(z.object({ path: z.string(), name: z.string() })),
})

export const GlobalSkillsSchema = z.object({
  skills: z.array(z.string()),
  placed_skills: z.array(z.string()),
  is_active: z.boolean(),
})

export const ProfilesResponseSchema = z.object({
  base: z.object({ skills: z.array(z.string()) }),
  global: GlobalSkillsSchema,
  profiles: z.array(ProfileSchema),
})

export const ProjectSchema = z.object({
  path: z.string(),
  name: z.string(),
  linked_profiles: z.array(z.string()),
  active_profiles: z.array(z.string()),
  placement_count: z.number(),
})

export const LinkedProfileSummarySchema = z.object({
  name: z.string(),
  is_active: z.boolean(),
  skill_count: z.number(),
})

export const PlacementSummarySchema = z.object({
  skill_name: z.string(),
  target_path: z.string(),
  placed_at: z.string(),
})

export const AgentPlacementsSchema = z.object({
  agent_name: z.string(),
  placements: z.array(PlacementSummarySchema),
})

export const ProjectLogEntrySchema = z.object({
  id: z.number(),
  timestamp: z.string(),
  source: z.string(),
  agent_name: z.string().nullable(),
  operation: z.string(),
  result: z.string(),
  details: z.string().nullable(),
})

export const ProjectDetailSchema = z.object({
  path: z.string(),
  name: z.string(),
  linked_profiles: z.array(LinkedProfileSummarySchema),
  placements_by_agent: z.array(AgentPlacementsSchema),
  recent_activity: z.array(ProjectLogEntrySchema),
})

export const AgentSchema = z.object({
  name: z.string(),
  project_path: z.string(),
  global_path: z.string(),
  enabled: z.boolean(),
})

export const LogEntrySchema = z.object({
  id: z.number(),
  timestamp: z.string(),
  source: z.string(),
  agent_name: z.string().nullable(),
  operation: z.string(),
  result: z.string(),
  details: z.string().nullable(),
})

export const StatusSchema = z.object({
  project_path: z.string(),
  base_skills: z.array(z.string()),
  active_profiles: z.array(z.string()),
  placement_count: z.number(),
})

export const DiscoveredSkillSchema = z.object({
  name: z.string(),
  description: z.string().nullable(),
  agent_name: z.string(),
  found_path: z.string(),
  scope: z.string(),
  files: z.array(z.string()),
  total_bytes: z.number(),
  token_estimate: z.number(),
  exists_in_registry: z.boolean(),
})

export type LinkedProfileSummary = z.infer<typeof LinkedProfileSummarySchema>
export type PlacementSummary = z.infer<typeof PlacementSummarySchema>
export type AgentPlacements = z.infer<typeof AgentPlacementsSchema>
export type ProjectLogEntry = z.infer<typeof ProjectLogEntrySchema>
export type ProjectDetail = z.infer<typeof ProjectDetailSchema>
export type GlobalSkills = z.infer<typeof GlobalSkillsSchema>
export type DiscoveredSkill = z.infer<typeof DiscoveredSkillSchema>
export type Skill = z.infer<typeof SkillSchema>
export type Profile = z.infer<typeof ProfileSchema>
export type ProfilesResponse = z.infer<typeof ProfilesResponseSchema>
export type Project = z.infer<typeof ProjectSchema>
export type Agent = z.infer<typeof AgentSchema>
export type LogEntry = z.infer<typeof LogEntrySchema>
export type Status = z.infer<typeof StatusSchema>
