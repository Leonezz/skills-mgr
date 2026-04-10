import type { ProjectLogEntry } from "@/lib/schemas"

interface Props {
  activity: ProjectLogEntry[]
}

export function ProjectRecentActivity({ activity }: Props) {
  if (activity.length === 0) {
    return (
      <div className="space-y-2">
        <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
          Recent Activity
        </p>
        <p className="text-xs text-muted-foreground">
          No recorded activity for this project.
        </p>
      </div>
    )
  }

  return (
    <div className="space-y-2">
      <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
        Recent Activity ({activity.length})
      </p>
      <ul className="space-y-1.5">
        {activity.map((entry) => (
          <li
            key={entry.id}
            className="rounded-lg border border-border bg-muted/10 px-3 py-2"
          >
            <div className="flex items-center justify-between gap-2">
              <span className="text-xs font-medium">{entry.operation}</span>
              <span
                className={`text-[10px] font-semibold uppercase ${
                  entry.result === "success"
                    ? "text-emerald-600 dark:text-emerald-400"
                    : entry.result === "error"
                      ? "text-destructive"
                      : "text-muted-foreground"
                }`}
              >
                {entry.result}
              </span>
            </div>
            {entry.details && (
              <p className="mt-0.5 truncate text-[11px] text-muted-foreground">
                {entry.details}
              </p>
            )}
            <div className="mt-1 flex items-center gap-2 text-[10px] text-muted-foreground">
              <span>{entry.source}</span>
              {entry.agent_name && <span>via {entry.agent_name}</span>}
              <span className="ml-auto tabular-nums">
                {formatTimestamp(entry.timestamp)}
              </span>
            </div>
          </li>
        ))}
      </ul>
    </div>
  )
}

function formatTimestamp(timestamp: string): string {
  const date = new Date(timestamp)
  if (isNaN(date.getTime())) return timestamp
  return date.toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  })
}
