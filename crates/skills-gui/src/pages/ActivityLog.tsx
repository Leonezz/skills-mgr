import { useState } from "react"
import { useQuery } from "@tanstack/react-query"
import { Button } from "@/components/ui/button"
import {
  Table,
  TableHeader,
  TableBody,
  TableRow,
  TableHead,
  TableCell,
} from "@/components/ui/table"
import { getRecentLogs } from "@/lib/api"
import { Monitor, Bot, Calendar } from "lucide-react"
import type { LogEntry } from "@/lib/schemas"

const sourceStyles: Record<string, { text: string; bg: string }> = {
  gui: { text: "text-indigo-400", bg: "bg-indigo-500/15" },
  cli: { text: "text-orange-400", bg: "bg-orange-500/15" },
  mcp: { text: "text-emerald-400", bg: "bg-emerald-500/15" },
}

function formatTimestamp(ts: string): string {
  try {
    const d = new Date(ts)
    return d.toLocaleDateString("en-US", { month: "short", day: "numeric" }) +
      ", " +
      d.toLocaleTimeString("en-US", { hour: "numeric", minute: "2-digit" })
  } catch {
    return ts
  }
}

function SourceBadge({ source }: { source: string }) {
  const style = sourceStyles[source] ?? { text: "text-muted-foreground", bg: "bg-muted" }
  return (
    <span className={`inline-flex items-center rounded-md px-2 py-0.5 text-[11px] font-medium ${style.text} ${style.bg}`}>
      {source.toUpperCase()}
    </span>
  )
}

export function ActivityLog() {
  const [sourceFilter, setSourceFilter] = useState<string>("all")
  const { data: logs, isLoading } = useQuery({
    queryKey: ["logs", 100],
    queryFn: () => getRecentLogs(100),
  })

  const filteredLogs = logs?.filter(
    (log) => sourceFilter === "all" || log.source === sourceFilter
  )

  return (
    <div className="flex flex-col" style={{ height: "calc(100vh - 4rem)" }}>
      {/* Header */}
      <div className="shrink-0">
        <h2 className="text-2xl font-bold tracking-tight">Activity Log</h2>
        <p className="text-sm text-muted-foreground">
          Operation history and audit trail
        </p>
      </div>

      {/* Filters */}
      <div className="shrink-0 flex items-center gap-2.5 py-6">
        <Button
          variant={sourceFilter === "all" ? "default" : "outline"}
          size="sm"
          onClick={() => setSourceFilter("all")}
        >
          <Monitor className="h-3.5 w-3.5" />
          All Sources
        </Button>
        {["cli", "gui", "mcp"].map((s) => (
          <Button
            key={s}
            variant={sourceFilter === s ? "default" : "outline"}
            size="sm"
            onClick={() => setSourceFilter(s)}
          >
            {s === "cli" && <Bot className="h-3.5 w-3.5" />}
            {s === "gui" && <Monitor className="h-3.5 w-3.5" />}
            {s === "mcp" && <Calendar className="h-3.5 w-3.5" />}
            {s.toUpperCase()}
          </Button>
        ))}
      </div>

      {/* Table */}
      {isLoading ? (
        <p className="text-muted-foreground">Loading...</p>
      ) : filteredLogs && filteredLogs.length > 0 ? (
        <div className="flex-1 min-h-0 flex flex-col overflow-hidden rounded-xl border border-border bg-card">
          {/* Scrollable area with sticky header */}
          <div className="flex-1 overflow-y-auto">
            <Table>
              <TableHeader className="sticky top-0 z-10 bg-muted/80 backdrop-blur-sm">
                <TableRow className="hover:bg-transparent">
                  <TableHead className="w-40">Timestamp</TableHead>
                  <TableHead className="w-20">Source</TableHead>
                  <TableHead className="w-28">Agent</TableHead>
                  <TableHead>Operation</TableHead>
                  <TableHead className="w-20 text-right">Result</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {filteredLogs.map((log: LogEntry) => (
                  <TableRow key={log.id}>
                    <TableCell className="w-40 text-foreground">
                      {formatTimestamp(log.timestamp)}
                    </TableCell>
                    <TableCell className="w-20">
                      <SourceBadge source={log.source} />
                    </TableCell>
                    <TableCell className="w-28 truncate text-foreground">
                      {log.agent_name ?? "\u2014"}
                    </TableCell>
                    <TableCell className="truncate text-foreground">
                      {log.operation}
                      {log.details && (
                        <span className="ml-1 text-muted-foreground">{log.details}</span>
                      )}
                    </TableCell>
                    <TableCell
                      className={`w-20 text-right font-medium ${
                        log.result === "success"
                          ? "text-emerald-500"
                          : log.result === "error"
                            ? "text-red-500"
                            : "text-muted-foreground"
                      }`}
                    >
                      {log.result === "success" ? "Success" : log.result === "error" ? "Error" : log.result}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        </div>
      ) : (
        <p className="text-muted-foreground">No operations logged yet.</p>
      )}
    </div>
  )
}
