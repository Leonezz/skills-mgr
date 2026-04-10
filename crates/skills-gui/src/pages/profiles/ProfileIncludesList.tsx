import { ChevronRight, Layers } from "lucide-react"
import { formatTokens } from "@/lib/format"
import type { Profile, Skill } from "@/lib/schemas"

interface Props {
  includes: string[]
  allProfiles: Profile[]
  skills: Skill[] | undefined
  onNavigate: (profileName: string) => void
}

export function ProfileIncludesList({
  includes,
  allProfiles,
  skills,
  onNavigate,
}: Props) {
  if (includes.length === 0) return null

  const profilesByName = new Map(allProfiles.map((p) => [p.name, p]))
  const skillTokenMap = new Map(
    (skills ?? []).map((s) => [s.name, s.token_estimate]),
  )

  return (
    <div className="space-y-2">
      <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
        Includes ({includes.length})
      </p>
      <ul className="space-y-1.5">
        {includes.map((name) => {
          const included = profilesByName.get(name)
          const skillCount = included?.skills.length ?? 0
          const tokenEstimate = included
            ? included.skills.reduce(
                (sum, s) => sum + (skillTokenMap.get(s) ?? 0),
                0,
              )
            : 0
          const exists = included !== undefined
          return (
            <li key={name}>
              <button
                type="button"
                disabled={!exists}
                onClick={() => exists && onNavigate(name)}
                className="group flex w-full items-center gap-3 rounded-lg border border-border bg-muted/10 p-3 text-left transition-colors hover:border-primary/40 hover:bg-muted/30 disabled:cursor-not-allowed disabled:opacity-60"
              >
                <Layers className="h-4 w-4 shrink-0 text-muted-foreground" />
                <div className="flex-1 min-w-0">
                  <p className="truncate text-sm font-medium">{name}</p>
                  {exists ? (
                    <p className="text-xs text-muted-foreground">
                      {skillCount} direct skill{skillCount !== 1 ? "s" : ""} · ~{formatTokens(tokenEstimate)} tok
                    </p>
                  ) : (
                    <p className="text-xs text-destructive">
                      Missing profile
                    </p>
                  )}
                </div>
                {exists && (
                  <ChevronRight className="h-4 w-4 shrink-0 text-muted-foreground transition-transform group-hover:translate-x-0.5" />
                )}
              </button>
            </li>
          )
        })}
      </ul>
    </div>
  )
}
