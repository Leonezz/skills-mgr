import { useState } from "react"
import { useQuery } from "@tanstack/react-query"
import { getRecentLogs } from "@/lib/api"

export function ActivityLog() {
  const [sourceFilter, setSourceFilter] = useState<string>("all")
  const { data: logs, isLoading } = useQuery({ queryKey: ["logs", 100], queryFn: () => getRecentLogs(100) })

  const filteredLogs = logs?.filter(log =>
    sourceFilter === "all" || log.source === sourceFilter
  )

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold">Activity Log</h2>
        <div className="flex gap-1">
          {["all", "cli", "gui", "mcp"].map(s => (
            <button
              key={s}
              onClick={() => setSourceFilter(s)}
              className={`rounded-md px-3 py-1 text-sm ${sourceFilter === s ? "bg-primary text-primary-foreground" : "bg-secondary text-secondary-foreground hover:bg-secondary/80"}`}
            >
              {s === "all" ? "All" : s.toUpperCase()}
            </button>
          ))}
        </div>
      </div>
      {isLoading ? (
        <p className="text-muted-foreground">Loading...</p>
      ) : filteredLogs && filteredLogs.length > 0 ? (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-border text-left text-muted-foreground">
                <th className="pb-2 pr-4">Time</th>
                <th className="pb-2 pr-4">Operation</th>
                <th className="pb-2 pr-4">Source</th>
                <th className="pb-2 pr-4">Result</th>
                <th className="pb-2">Details</th>
              </tr>
            </thead>
            <tbody>
              {filteredLogs.map((log) => (
                <tr key={log.id} className="border-b border-border">
                  <td className="py-2 pr-4 text-muted-foreground">{log.timestamp}</td>
                  <td className="py-2 pr-4 font-medium">{log.operation}</td>
                  <td className="py-2 pr-4">
                    <span className={`inline-block rounded px-1.5 py-0.5 text-xs ${
                      log.source === "cli" ? "bg-blue-500/10 text-blue-500" :
                      log.source === "gui" ? "bg-green-500/10 text-green-500" :
                      "bg-purple-500/10 text-purple-500"
                    }`}>
                      {log.source.toUpperCase()}
                    </span>
                  </td>
                  <td className="py-2 pr-4">
                    <span className={`inline-block rounded px-1.5 py-0.5 text-xs ${
                      log.result === "success" ? "bg-green-500/10 text-green-500" : "bg-red-500/10 text-red-500"
                    }`}>
                      {log.result}
                    </span>
                  </td>
                  <td className="py-2 text-muted-foreground">{log.details ?? ""}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : (
        <p className="text-muted-foreground">No operations logged yet.</p>
      )}
    </div>
  )
}
