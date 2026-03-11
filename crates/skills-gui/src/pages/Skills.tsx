import { useState } from "react"
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { listSkills, createSkill, removeSkill } from "@/lib/api"
import { toast } from "sonner"
import type { Skill } from "@/lib/schemas"

export function Skills() {
  const queryClient = useQueryClient()
  const { data: skills, isLoading } = useQuery({ queryKey: ["skills"], queryFn: listSkills })
  const [showCreate, setShowCreate] = useState(false)
  const [showDelete, setShowDelete] = useState<string | null>(null)
  const [newName, setNewName] = useState("")
  const [newDesc, setNewDesc] = useState("")

  const createMutation = useMutation({
    mutationFn: () => createSkill(newName, newDesc || "No description"),
    onSuccess: (msg) => {
      toast.success(msg)
      queryClient.invalidateQueries({ queryKey: ["skills"] })
      setShowCreate(false)
      setNewName("")
      setNewDesc("")
    },
    onError: (err) => toast.error(String(err)),
  })

  const deleteMutation = useMutation({
    mutationFn: (name: string) => removeSkill(name),
    onSuccess: (msg) => {
      toast.success(msg)
      queryClient.invalidateQueries({ queryKey: ["skills"] })
      setShowDelete(null)
    },
    onError: (err) => toast.error(String(err)),
  })

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold">Skills Registry</h2>
        <button
          onClick={() => setShowCreate(true)}
          className="rounded-md bg-primary px-4 py-2 text-sm text-primary-foreground hover:bg-primary/90"
        >
          Create Skill
        </button>
      </div>

      {/* Create Modal */}
      {showCreate && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50" onClick={() => setShowCreate(false)}>
          <div className="w-full max-w-md rounded-lg bg-background p-6 shadow-lg" onClick={(e) => e.stopPropagation()}>
            <h3 className="mb-4 text-lg font-semibold">Create Skill</h3>
            <div className="space-y-3">
              <div>
                <label className="mb-1 block text-sm text-muted-foreground">Name</label>
                <input
                  value={newName}
                  onChange={(e) => setNewName(e.target.value)}
                  className="w-full rounded-md border border-border bg-background px-3 py-2 text-sm"
                  placeholder="my-skill"
                />
              </div>
              <div>
                <label className="mb-1 block text-sm text-muted-foreground">Description</label>
                <input
                  value={newDesc}
                  onChange={(e) => setNewDesc(e.target.value)}
                  className="w-full rounded-md border border-border bg-background px-3 py-2 text-sm"
                  placeholder="What this skill does"
                />
              </div>
            </div>
            <div className="mt-4 flex justify-end gap-2">
              <button onClick={() => setShowCreate(false)} className="rounded-md border border-border px-4 py-2 text-sm hover:bg-muted">Cancel</button>
              <button
                onClick={() => createMutation.mutate()}
                disabled={!newName || createMutation.isPending}
                className="rounded-md bg-primary px-4 py-2 text-sm text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
              >
                {createMutation.isPending ? "Creating..." : "Create"}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Delete Confirmation */}
      {showDelete && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50" onClick={() => setShowDelete(null)}>
          <div className="w-full max-w-sm rounded-lg bg-background p-6 shadow-lg" onClick={(e) => e.stopPropagation()}>
            <h3 className="mb-2 text-lg font-semibold">Delete Skill</h3>
            <p className="mb-4 text-sm text-muted-foreground">
              Are you sure you want to delete <strong>{showDelete}</strong>? This cannot be undone.
            </p>
            <div className="flex justify-end gap-2">
              <button onClick={() => setShowDelete(null)} className="rounded-md border border-border px-4 py-2 text-sm hover:bg-muted">Cancel</button>
              <button
                onClick={() => deleteMutation.mutate(showDelete)}
                disabled={deleteMutation.isPending}
                className="rounded-md bg-destructive px-4 py-2 text-sm text-destructive-foreground hover:bg-destructive/90 disabled:opacity-50"
              >
                {deleteMutation.isPending ? "Deleting..." : "Delete"}
              </button>
            </div>
          </div>
        </div>
      )}

      {isLoading ? (
        <p className="text-muted-foreground">Loading...</p>
      ) : skills && skills.length > 0 ? (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {skills.map((skill: Skill) => (
            <Card key={skill.name} className="group relative">
              <button
                onClick={() => setShowDelete(skill.name)}
                className="absolute right-3 top-3 rounded-md p-1 text-muted-foreground opacity-0 hover:bg-destructive/10 hover:text-destructive group-hover:opacity-100"
                title="Delete skill"
              >
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/></svg>
              </button>
              <CardHeader>
                <CardTitle className="text-base">{skill.name}</CardTitle>
              </CardHeader>
              <CardContent>
                <p className="text-sm text-muted-foreground">
                  {skill.description ?? "No description"}
                </p>
                <div className="mt-2 flex gap-1">
                  {skill.source_type && (
                    <span className="inline-block rounded-md bg-secondary px-2 py-0.5 text-xs text-secondary-foreground">
                      {skill.source_type}
                    </span>
                  )}
                  <span className="inline-block rounded-md bg-secondary px-2 py-0.5 text-xs text-secondary-foreground">
                    {skill.files.length} files
                  </span>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      ) : (
        <p className="text-muted-foreground">No skills in registry.</p>
      )}
    </div>
  )
}
