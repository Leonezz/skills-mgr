import { ChevronRight, Layers } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import type { LinkedProfileSummary } from "@/lib/schemas"

interface Props {
  profiles: LinkedProfileSummary[]
  onNavigate: (profileName: string) => void
}

export function ProjectLinkedProfilesList({ profiles, onNavigate }: Props) {
  if (profiles.length === 0) {
    return (
      <div className="space-y-2">
        <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
          Linked Profiles
        </p>
        <p className="text-xs text-muted-foreground">
          No profiles linked yet. Edit this project to attach profiles.
        </p>
      </div>
    )
  }

  return (
    <div className="space-y-2">
      <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
        Linked Profiles ({profiles.length})
      </p>
      <ul className="space-y-1.5">
        {profiles.map((profile) => (
          <li key={profile.name}>
            <button
              type="button"
              onClick={() => onNavigate(profile.name)}
              className="group flex w-full items-center gap-3 rounded-lg border border-border bg-muted/10 p-3 text-left transition-colors hover:border-primary/40 hover:bg-muted/30"
            >
              <Layers className="h-4 w-4 shrink-0 text-muted-foreground" />
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="truncate text-sm font-medium">
                    {profile.name}
                  </span>
                  <Badge
                    variant={profile.is_active ? "default" : "secondary"}
                    className="text-[10px]"
                  >
                    {profile.is_active ? "active" : "linked"}
                  </Badge>
                </div>
                <p className="text-xs text-muted-foreground">
                  {profile.skill_count} resolved skill
                  {profile.skill_count !== 1 ? "s" : ""}
                </p>
              </div>
              <ChevronRight className="h-4 w-4 shrink-0 text-muted-foreground transition-transform group-hover:translate-x-0.5" />
            </button>
          </li>
        ))}
      </ul>
    </div>
  )
}
