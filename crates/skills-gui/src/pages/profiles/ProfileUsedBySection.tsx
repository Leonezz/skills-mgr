import { FolderKanban, Layers } from "lucide-react"
import type { Profile } from "@/lib/schemas"

interface Props {
  profile: Profile
  allProfiles: Profile[]
  onNavigateProfile: (profileName: string) => void
}

export function ProfileUsedBySection({
  profile,
  allProfiles,
  onNavigateProfile,
}: Props) {
  const reverseIncludes = allProfiles
    .filter((p) => p.name !== profile.name && p.includes.includes(profile.name))
    .map((p) => p.name)

  const activeProjects = profile.active_projects

  if (activeProjects.length === 0 && reverseIncludes.length === 0) {
    return (
      <div className="space-y-2">
        <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
          Used By
        </p>
        <p className="text-xs text-muted-foreground">
          Not referenced by any project or profile yet.
        </p>
      </div>
    )
  }

  return (
    <div className="space-y-3">
      <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
        Used By
      </p>

      {activeProjects.length > 0 && (
        <div className="space-y-1.5">
          <p className="text-[11px] font-medium text-muted-foreground">
            Active in {activeProjects.length} project
            {activeProjects.length !== 1 ? "s" : ""}
          </p>
          <ul className="space-y-1">
            {activeProjects.map((proj) => (
              <li
                key={proj.path}
                className="flex items-start gap-2.5 rounded-lg border border-border bg-muted/10 p-2.5"
              >
                <FolderKanban className="mt-0.5 h-4 w-4 shrink-0 text-emerald-600 dark:text-emerald-400" />
                <div className="flex-1 min-w-0">
                  <p className="truncate text-sm font-medium">{proj.name}</p>
                  <p className="truncate text-[11px] font-mono text-muted-foreground">
                    {proj.path}
                  </p>
                </div>
              </li>
            ))}
          </ul>
        </div>
      )}

      {reverseIncludes.length > 0 && (
        <div className="space-y-1.5">
          <p className="text-[11px] font-medium text-muted-foreground">
            Included by {reverseIncludes.length} profile
            {reverseIncludes.length !== 1 ? "s" : ""}
          </p>
          <ul className="space-y-1">
            {reverseIncludes.map((name) => (
              <li key={name}>
                <button
                  type="button"
                  onClick={() => onNavigateProfile(name)}
                  className="flex w-full items-center gap-2.5 rounded-lg border border-border bg-muted/10 p-2.5 text-left transition-colors hover:border-primary/40 hover:bg-muted/30"
                >
                  <Layers className="h-4 w-4 shrink-0 text-muted-foreground" />
                  <span className="truncate text-sm font-medium">{name}</span>
                </button>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  )
}
