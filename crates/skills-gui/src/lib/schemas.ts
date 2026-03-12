import { z } from "zod"

export const SkillSchema = z.object({
  name: z.string(),
  description: z.string().nullable(),
  files: z.array(z.string()),
  source_type: z.string().nullable(),
  is_builtin: z.boolean(),
  dir_path: z.string(),
})

export const ProfileSchema = z.object({
  name: z.string(),
  description: z.string().nullable(),
  skills: z.array(z.string()),
  includes: z.array(z.string()),
  active_projects: z.array(z.object({ path: z.string(), name: z.string() })),
})

export const ProfilesResponseSchema = z.object({
  base: z.object({ skills: z.array(z.string()) }),
  profiles: z.array(ProfileSchema),
})

export const ProjectSchema = z.object({
  path: z.string(),
  name: z.string(),
  linked_profiles: z.array(z.string()),
  active_profiles: z.array(z.string()),
  placement_count: z.number(),
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

export type Skill = z.infer<typeof SkillSchema>
export type Profile = z.infer<typeof ProfileSchema>
export type ProfilesResponse = z.infer<typeof ProfilesResponseSchema>
export type Project = z.infer<typeof ProjectSchema>
export type Agent = z.infer<typeof AgentSchema>
export type LogEntry = z.infer<typeof LogEntrySchema>
export type Status = z.infer<typeof StatusSchema>
