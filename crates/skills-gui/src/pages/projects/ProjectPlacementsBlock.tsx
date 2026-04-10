import { useState } from "react"
import { ChevronDown, ChevronRight } from "lucide-react"
import type { AgentPlacements } from "@/lib/schemas"

interface Props {
  placementsByAgent: AgentPlacements[]
}

export function ProjectPlacementsBlock({ placementsByAgent }: Props) {
  const totalCount = placementsByAgent.reduce(
    (sum, a) => sum + a.placements.length,
    0,
  )

  if (totalCount === 0) {
    return (
      <div className="space-y-2">
        <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
          Placements
        </p>
        <p className="text-xs text-muted-foreground">
          No skills placed yet. Activate a linked profile to deploy skills.
        </p>
      </div>
    )
  }

  return (
    <div className="space-y-3">
      <div className="flex items-baseline justify-between">
        <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
          Placements ({totalCount})
        </p>
        <span className="text-xs text-muted-foreground">
          {placementsByAgent.length} agent
          {placementsByAgent.length !== 1 ? "s" : ""}
        </span>
      </div>
      <div className="space-y-2">
        {placementsByAgent.map((agent) => (
          <AgentGroup key={agent.agent_name} agent={agent} />
        ))}
      </div>
    </div>
  )
}

function AgentGroup({ agent }: { agent: AgentPlacements }) {
  const [expanded, setExpanded] = useState(false)

  return (
    <div className="rounded-lg border border-border bg-muted/10">
      <button
        type="button"
        onClick={() => setExpanded((v) => !v)}
        className="flex w-full items-center gap-2 px-3 py-2.5 text-left transition-colors hover:bg-muted/30"
      >
        {expanded ? (
          <ChevronDown className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
        ) : (
          <ChevronRight className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
        )}
        <span className="text-sm font-medium">{agent.agent_name}</span>
        <span className="ml-auto text-xs tabular-nums text-muted-foreground">
          {agent.placements.length} file
          {agent.placements.length !== 1 ? "s" : ""}
        </span>
      </button>
      {expanded && (
        <div className="max-h-48 overflow-y-auto border-t border-border px-3 py-2">
          <ul className="space-y-1">
            {agent.placements.map((p, i) => (
              <li
                key={`${p.skill_name}-${i}`}
                className="flex items-start justify-between gap-2"
              >
                <div className="min-w-0">
                  <p className="truncate text-xs font-medium">
                    {p.skill_name}
                  </p>
                  <p className="truncate font-mono text-[11px] text-muted-foreground">
                    {p.target_path}
                  </p>
                </div>
                <span className="shrink-0 text-[10px] tabular-nums text-muted-foreground">
                  {formatRelativeTime(p.placed_at)}
                </span>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  )
}

function formatRelativeTime(timestamp: string): string {
  const date = new Date(timestamp)
  if (isNaN(date.getTime())) return timestamp
  const now = Date.now()
  const diffMs = now - date.getTime()
  const diffMin = Math.floor(diffMs / 60_000)
  if (diffMin < 1) return "just now"
  if (diffMin < 60) return `${diffMin}m ago`
  const diffHrs = Math.floor(diffMin / 60)
  if (diffHrs < 24) return `${diffHrs}h ago`
  const diffDays = Math.floor(diffHrs / 24)
  return `${diffDays}d ago`
}
