import { useQuery } from "@tanstack/react-query"
import { getRecentLogs } from "@/lib/api"

export function ActivityLog() {
  const { data: logs, isLoading } = useQuery({ queryKey: ["logs"], queryFn: () => getRecentLogs(50) })

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold">Activity Log</h2>
      {isLoading ? (
        <p className="text-muted-foreground">Loading...</p>
      ) : logs && logs.length > 0 ? (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-border text-left text-muted-foreground">
                <th className="pb-2 pr-4">Time</th>
                <th className="pb-2 pr-4">Operation</th>
                <th className="pb-2 pr-4">Source</th>
                <th className="pb-2 pr-4">Agent</th>
                <th className="pb-2 pr-4">Result</th>
                <th className="pb-2">Details</th>
              </tr>
            </thead>
            <tbody>
              {logs.map((log) => (
                <tr key={log.id} className="border-b border-border">
                  <td className="py-2 pr-4 text-muted-foreground">{log.timestamp}</td>
                  <td className="py-2 pr-4">{log.operation}</td>
                  <td className="py-2 pr-4">{log.source}</td>
                  <td className="py-2 pr-4">{log.agent_name ?? "-"}</td>
                  <td className="py-2 pr-4">{log.result}</td>
                  <td className="py-2">{log.details ?? ""}</td>
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
