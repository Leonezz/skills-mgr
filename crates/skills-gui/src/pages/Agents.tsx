import { useState } from "react"
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { listAgents, addAgent, editAgent, removeAgent } from "@/lib/api"
import { toast } from "sonner"
import type { Agent } from "@/lib/schemas"

export function Agents() {
  const queryClient = useQueryClient()
  const { data: agents, isLoading } = useQuery({ queryKey: ["agents"], queryFn: listAgents })
  const [showAdd, setShowAdd] = useState(false)
  const [showEdit, setShowEdit] = useState<Agent | null>(null)
  const [showDelete, setShowDelete] = useState<string | null>(null)

  // Add form
  const [newName, setNewName] = useState("")
  const [newProjectPath, setNewProjectPath] = useState("")
  const [newGlobalPath, setNewGlobalPath] = useState("")

  // Edit form
  const [editProjectPath, setEditProjectPath] = useState("")
  const [editGlobalPath, setEditGlobalPath] = useState("")

  const addMutation = useMutation({
    mutationFn: () => addAgent(newName, newProjectPath, newGlobalPath),
    onSuccess: (msg) => {
      toast.success(msg)
      queryClient.invalidateQueries({ queryKey: ["agents"] })
      setShowAdd(false)
      setNewName("")
      setNewProjectPath("")
      setNewGlobalPath("")
    },
    onError: (err) => toast.error(String(err)),
  })

  const editMutation = useMutation({
    mutationFn: () => editAgent(showEdit!.name, editProjectPath, editGlobalPath),
    onSuccess: (msg) => {
      toast.success(msg)
      queryClient.invalidateQueries({ queryKey: ["agents"] })
      setShowEdit(null)
    },
    onError: (err) => toast.error(String(err)),
  })

  const deleteMutation = useMutation({
    mutationFn: (name: string) => removeAgent(name),
    onSuccess: (msg) => {
      toast.success(msg)
      queryClient.invalidateQueries({ queryKey: ["agents"] })
      setShowDelete(null)
    },
    onError: (err) => toast.error(String(err)),
  })

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold">Agents</h2>
        <button onClick={() => setShowAdd(true)} className="rounded-md bg-primary px-4 py-2 text-sm text-primary-foreground hover:bg-primary/90">
          Add Agent
        </button>
      </div>

      {/* Add Modal */}
      {showAdd && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50" onClick={() => setShowAdd(false)}>
          <div className="w-full max-w-md rounded-lg bg-background p-6 shadow-lg" onClick={(e) => e.stopPropagation()}>
            <h3 className="mb-4 text-lg font-semibold">Add Agent</h3>
            <div className="space-y-3">
              <div>
                <label className="mb-1 block text-sm text-muted-foreground">Name</label>
                <input value={newName} onChange={(e) => setNewName(e.target.value)} className="w-full rounded-md border border-border bg-background px-3 py-2 text-sm" placeholder="claude-code" />
              </div>
              <div>
                <label className="mb-1 block text-sm text-muted-foreground">Project Path</label>
                <input value={newProjectPath} onChange={(e) => setNewProjectPath(e.target.value)} className="w-full rounded-md border border-border bg-background px-3 py-2 text-sm" placeholder=".claude/skills" />
              </div>
              <div>
                <label className="mb-1 block text-sm text-muted-foreground">Global Path</label>
                <input value={newGlobalPath} onChange={(e) => setNewGlobalPath(e.target.value)} className="w-full rounded-md border border-border bg-background px-3 py-2 text-sm" placeholder="~/.claude/skills" />
              </div>
            </div>
            <div className="mt-4 flex justify-end gap-2">
              <button onClick={() => setShowAdd(false)} className="rounded-md border border-border px-4 py-2 text-sm hover:bg-muted">Cancel</button>
              <button onClick={() => addMutation.mutate()} disabled={!newName || !newProjectPath || !newGlobalPath || addMutation.isPending} className="rounded-md bg-primary px-4 py-2 text-sm text-primary-foreground hover:bg-primary/90 disabled:opacity-50">
                {addMutation.isPending ? "Adding..." : "Add"}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Edit Modal */}
      {showEdit && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50" onClick={() => setShowEdit(null)}>
          <div className="w-full max-w-md rounded-lg bg-background p-6 shadow-lg" onClick={(e) => e.stopPropagation()}>
            <h3 className="mb-4 text-lg font-semibold">Edit Agent: {showEdit.name}</h3>
            <div className="space-y-3">
              <div>
                <label className="mb-1 block text-sm text-muted-foreground">Project Path</label>
                <input value={editProjectPath} onChange={(e) => setEditProjectPath(e.target.value)} className="w-full rounded-md border border-border bg-background px-3 py-2 text-sm" />
              </div>
              <div>
                <label className="mb-1 block text-sm text-muted-foreground">Global Path</label>
                <input value={editGlobalPath} onChange={(e) => setEditGlobalPath(e.target.value)} className="w-full rounded-md border border-border bg-background px-3 py-2 text-sm" />
              </div>
            </div>
            <div className="mt-4 flex justify-end gap-2">
              <button onClick={() => setShowEdit(null)} className="rounded-md border border-border px-4 py-2 text-sm hover:bg-muted">Cancel</button>
              <button onClick={() => editMutation.mutate()} disabled={editMutation.isPending} className="rounded-md bg-primary px-4 py-2 text-sm text-primary-foreground hover:bg-primary/90 disabled:opacity-50">
                {editMutation.isPending ? "Saving..." : "Save"}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Delete Confirmation */}
      {showDelete && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50" onClick={() => setShowDelete(null)}>
          <div className="w-full max-w-sm rounded-lg bg-background p-6 shadow-lg" onClick={(e) => e.stopPropagation()}>
            <h3 className="mb-2 text-lg font-semibold">Remove Agent</h3>
            <p className="mb-4 text-sm text-muted-foreground">Remove agent <strong>{showDelete}</strong>?</p>
            <div className="flex justify-end gap-2">
              <button onClick={() => setShowDelete(null)} className="rounded-md border border-border px-4 py-2 text-sm hover:bg-muted">Cancel</button>
              <button onClick={() => deleteMutation.mutate(showDelete)} disabled={deleteMutation.isPending} className="rounded-md bg-destructive px-4 py-2 text-sm text-destructive-foreground hover:bg-destructive/90 disabled:opacity-50">
                {deleteMutation.isPending ? "Removing..." : "Remove"}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Agent Cards */}
      {isLoading ? (
        <p className="text-muted-foreground">Loading...</p>
      ) : agents && agents.length > 0 ? (
        <div className="space-y-3">
          {agents.map((agent: Agent) => (
            <Card key={agent.name} className="group relative">
              <div className="absolute right-3 top-3 flex gap-1 opacity-0 group-hover:opacity-100">
                <button
                  onClick={() => { setShowEdit(agent); setEditProjectPath(agent.project_path); setEditGlobalPath(agent.global_path); }}
                  className="rounded-md p-1 text-muted-foreground hover:bg-muted hover:text-foreground"
                  title="Edit"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z"/></svg>
                </button>
                <button
                  onClick={() => setShowDelete(agent.name)}
                  className="rounded-md p-1 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
                  title="Remove"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/></svg>
                </button>
              </div>
              <CardHeader>
                <CardTitle className="text-base">{agent.name}</CardTitle>
              </CardHeader>
              <CardContent>
                <p className="text-sm text-muted-foreground">Project: {agent.project_path}</p>
                <p className="text-sm text-muted-foreground">Global: {agent.global_path}</p>
              </CardContent>
            </Card>
          ))}
        </div>
      ) : (
        <p className="text-muted-foreground">No agents configured.</p>
      )}
    </div>
  )
}
