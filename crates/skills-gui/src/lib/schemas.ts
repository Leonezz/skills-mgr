import { z } from "zod"

export const SkillSchema = z.object({
  name: z.string(),
  description: z.string().nullable(),
  files: z.array(z.string()),
  source_type: z.string().nullable(),
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

export type Skill = z.infer<typeof SkillSchema>
export type LogEntry = z.infer<typeof LogEntrySchema>
