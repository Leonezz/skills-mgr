import { useMemo } from "react"
import type { Profile, Skill } from "@/lib/schemas"

export interface ProfileResolution {
  /** Skills listed directly on the profile (deduped, order preserved). */
  directSkills: string[]
  /** Skills contributed only by transitively included profiles (not already direct). */
  inheritedSkills: string[]
  /** Full flattened skill set after resolving includes (deduped). */
  resolvedSkills: string[]
  /** Which included profile contributed each inherited skill (best-effort: first source wins). */
  inheritedFrom: Map<string, string>
  tokens: {
    direct: number
    inherited: number
    total: number
  }
}

interface UseProfileResolutionArgs {
  profile: { name?: string; skills: string[]; includes: string[] } | null
  allProfiles: Profile[]
  skills: Skill[] | undefined
}

/**
 * Resolves a profile's effective skill set by walking its includes graph.
 * Safe against cycles. Pure — memoized on inputs.
 */
export function useProfileResolution({
  profile,
  allProfiles,
  skills,
}: UseProfileResolutionArgs): ProfileResolution {
  const skillTokenMap = useMemo(() => {
    const map = new Map<string, number>()
    for (const s of skills ?? []) map.set(s.name, s.token_estimate)
    return map
  }, [skills])

  const profilesByName = useMemo(() => {
    const map = new Map<string, Profile>()
    for (const p of allProfiles) map.set(p.name, p)
    return map
  }, [allProfiles])

  return useMemo(() => {
    const empty: ProfileResolution = {
      directSkills: [],
      inheritedSkills: [],
      resolvedSkills: [],
      inheritedFrom: new Map(),
      tokens: { direct: 0, inherited: 0, total: 0 },
    }
    if (!profile) return empty

    const directSet = new Set(profile.skills)
    const directSkills = Array.from(directSet)

    const inheritedFrom = new Map<string, string>()
    const visited = new Set<string>()
    if (profile.name) visited.add(profile.name)

    function walk(includeNames: string[], sourceChain: string) {
      for (const name of includeNames) {
        if (visited.has(name)) continue
        visited.add(name)
        const included = profilesByName.get(name)
        if (!included) continue
        for (const skill of included.skills) {
          if (!directSet.has(skill) && !inheritedFrom.has(skill)) {
            inheritedFrom.set(skill, sourceChain || name)
          }
        }
        walk(included.includes, sourceChain || name)
      }
    }

    walk(profile.includes, "")

    const inheritedSkills = Array.from(inheritedFrom.keys())
    const resolvedSkills = [...directSkills, ...inheritedSkills]

    const directTokens = directSkills.reduce(
      (sum, name) => sum + (skillTokenMap.get(name) ?? 0),
      0,
    )
    const inheritedTokens = inheritedSkills.reduce(
      (sum, name) => sum + (skillTokenMap.get(name) ?? 0),
      0,
    )

    return {
      directSkills,
      inheritedSkills,
      resolvedSkills,
      inheritedFrom,
      tokens: {
        direct: directTokens,
        inherited: inheritedTokens,
        total: directTokens + inheritedTokens,
      },
    }
  }, [profile, profilesByName, skillTokenMap])
}
