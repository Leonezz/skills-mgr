import { useState } from "react"
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { listProfiles, createProfile, editProfile, deleteProfile, listSkills } from "@/lib/api"
import { toast } from "sonner"
import type { Profile } from "@/lib/schemas"

export function Profiles() {
  const queryClient = useQueryClient()
  const { data, isLoading } = useQuery({ queryKey: ["profiles"], queryFn: listProfiles })
  const { data: skills } = useQuery({ queryKey: ["skills"], queryFn: listSkills })
  const [showCreate, setShowCreate] = useState(false)
  const [showEdit, setShowEdit] = useState<Profile | null>(null)
  const [showDelete, setShowDelete] = useState<string | null>(null)

  // Create form state
  const [newName, setNewName] = useState("")
  const [newSkills, setNewSkills] = useState("")
  const [newIncludes, setNewIncludes] = useState("")

  // Edit form state
  const [editAddSkills, setEditAddSkills] = useState("")
  const [editRemoveSkills, setEditRemoveSkills] = useState<string[]>([])
  const [editAddIncludes, setEditAddIncludes] = useState("")

  const createMutation = useMutation({
    mutationFn: () => createProfile(
      newName,
      newSkills.split(",").map(s => s.trim()).filter(Boolean),
      newIncludes.split(",").map(s => s.trim()).filter(Boolean),
    ),
    onSuccess: (msg) => {
      toast.success(msg)
      queryClient.invalidateQueries({ queryKey: ["profiles"] })
      setShowCreate(false)
      setNewName("")
      setNewSkills("")
      setNewIncludes("")
    },
    onError: (err) => toast.error(String(err)),
  })

  const editMutation = useMutation({
    mutationFn: () => editProfile(
      showEdit!.name,
      editAddSkills.split(",").map(s => s.trim()).filter(Boolean),
      editRemoveSkills,
      editAddIncludes.split(",").map(s => s.trim()).filter(Boolean),
    ),
    onSuccess: (msg) => {
      toast.success(msg)
      queryClient.invalidateQueries({ queryKey: ["profiles"] })
      setShowEdit(null)
      setEditAddSkills("")
      setEditRemoveSkills([])
      setEditAddIncludes("")
    },
    onError: (err) => toast.error(String(err)),
  })

  const deleteMutation = useMutation({
    mutationFn: (name: string) => deleteProfile(name),
    onSuccess: (msg) => {
      toast.success(msg)
      queryClient.invalidateQueries({ queryKey: ["profiles"] })
      setShowDelete(null)
    },
    onError: (err) => toast.error(String(err)),
  })

  const profiles = data?.profiles ?? []
  const baseSkills = data?.base?.skills ?? []

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold">Profiles</h2>
        <button
          onClick={() => setShowCreate(true)}
          className="rounded-md bg-primary px-4 py-2 text-sm text-primary-foreground hover:bg-primary/90"
        >
          Create Profile
        </button>
      </div>

      {/* Create Modal */}
      {showCreate && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50" onClick={() => setShowCreate(false)}>
          <div className="w-full max-w-md rounded-lg bg-background p-6 shadow-lg" onClick={(e) => e.stopPropagation()}>
            <h3 className="mb-4 text-lg font-semibold">Create Profile</h3>
            <div className="space-y-3">
              <div>
                <label className="mb-1 block text-sm text-muted-foreground">Name</label>
                <input value={newName} onChange={(e) => setNewName(e.target.value)} className="w-full rounded-md border border-border bg-background px-3 py-2 text-sm" placeholder="my-profile" />
              </div>
              <div>
                <label className="mb-1 block text-sm text-muted-foreground">Skills (comma-separated)</label>
                <input value={newSkills} onChange={(e) => setNewSkills(e.target.value)} className="w-full rounded-md border border-border bg-background px-3 py-2 text-sm" placeholder="rust-patterns, code-review" />
                {skills && skills.length > 0 && (
                  <p className="mt-1 text-xs text-muted-foreground">Available: {skills.map(s => s.name).join(", ")}</p>
                )}
              </div>
              <div>
                <label className="mb-1 block text-sm text-muted-foreground">Includes (comma-separated profile names)</label>
                <input value={newIncludes} onChange={(e) => setNewIncludes(e.target.value)} className="w-full rounded-md border border-border bg-background px-3 py-2 text-sm" placeholder="base-profile" />
              </div>
            </div>
            <div className="mt-4 flex justify-end gap-2">
              <button onClick={() => setShowCreate(false)} className="rounded-md border border-border px-4 py-2 text-sm hover:bg-muted">Cancel</button>
              <button onClick={() => createMutation.mutate()} disabled={!newName || createMutation.isPending} className="rounded-md bg-primary px-4 py-2 text-sm text-primary-foreground hover:bg-primary/90 disabled:opacity-50">
                {createMutation.isPending ? "Creating..." : "Create"}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Edit Modal */}
      {showEdit && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50" onClick={() => setShowEdit(null)}>
          <div className="w-full max-w-md rounded-lg bg-background p-6 shadow-lg" onClick={(e) => e.stopPropagation()}>
            <h3 className="mb-4 text-lg font-semibold">Edit Profile: {showEdit.name}</h3>
            <div className="space-y-3">
              <div>
                <label className="mb-1 block text-sm text-muted-foreground">Current Skills</label>
                <div className="flex flex-wrap gap-1">
                  {showEdit.skills.map(s => (
                    <button
                      key={s}
                      onClick={() => setEditRemoveSkills(prev => prev.includes(s) ? prev.filter(x => x !== s) : [...prev, s])}
                      className={`rounded-md px-2 py-0.5 text-xs ${editRemoveSkills.includes(s) ? "bg-destructive/20 text-destructive line-through" : "bg-secondary text-secondary-foreground"}`}
                    >
                      {s}
                    </button>
                  ))}
                </div>
                <p className="mt-1 text-xs text-muted-foreground">Click to toggle removal</p>
              </div>
              <div>
                <label className="mb-1 block text-sm text-muted-foreground">Add Skills (comma-separated)</label>
                <input value={editAddSkills} onChange={(e) => setEditAddSkills(e.target.value)} className="w-full rounded-md border border-border bg-background px-3 py-2 text-sm" placeholder="new-skill" />
              </div>
              <div>
                <label className="mb-1 block text-sm text-muted-foreground">Add Includes (comma-separated)</label>
                <input value={editAddIncludes} onChange={(e) => setEditAddIncludes(e.target.value)} className="w-full rounded-md border border-border bg-background px-3 py-2 text-sm" />
                {showEdit.includes.length > 0 && (
                  <p className="mt-1 text-xs text-muted-foreground">Current includes: {showEdit.includes.join(", ")}</p>
                )}
              </div>
            </div>
            <div className="mt-4 flex justify-end gap-2">
              <button onClick={() => { setShowEdit(null); setEditRemoveSkills([]); setEditAddSkills(""); setEditAddIncludes(""); }} className="rounded-md border border-border px-4 py-2 text-sm hover:bg-muted">Cancel</button>
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
            <h3 className="mb-2 text-lg font-semibold">Delete Profile</h3>
            <p className="mb-4 text-sm text-muted-foreground">Are you sure you want to delete <strong>{showDelete}</strong>?</p>
            <div className="flex justify-end gap-2">
              <button onClick={() => setShowDelete(null)} className="rounded-md border border-border px-4 py-2 text-sm hover:bg-muted">Cancel</button>
              <button onClick={() => deleteMutation.mutate(showDelete)} disabled={deleteMutation.isPending} className="rounded-md bg-destructive px-4 py-2 text-sm text-destructive-foreground hover:bg-destructive/90 disabled:opacity-50">
                {deleteMutation.isPending ? "Deleting..." : "Delete"}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Base Skills */}
      {baseSkills.length > 0 && (
        <Card>
          <CardHeader><CardTitle>Base Skills (always active)</CardTitle></CardHeader>
          <CardContent>
            <div className="flex flex-wrap gap-2">
              {baseSkills.map((s: string) => (
                <span key={s} className="rounded-md bg-secondary px-2 py-1 text-sm">{s}</span>
              ))}
            </div>
          </CardContent>
        </Card>
      )}

      {/* Profile Cards */}
      {isLoading ? (
        <p className="text-muted-foreground">Loading...</p>
      ) : profiles.length > 0 ? (
        <div className="space-y-3">
          {profiles.map((profile: Profile) => (
            <Card key={profile.name} className="group relative">
              <div className="absolute right-3 top-3 flex gap-1 opacity-0 group-hover:opacity-100">
                <button
                  onClick={() => { setShowEdit(profile); setEditRemoveSkills([]); setEditAddSkills(""); setEditAddIncludes(""); }}
                  className="rounded-md p-1 text-muted-foreground hover:bg-muted hover:text-foreground"
                  title="Edit"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z"/></svg>
                </button>
                <button
                  onClick={() => setShowDelete(profile.name)}
                  className="rounded-md p-1 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
                  title="Delete"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/></svg>
                </button>
              </div>
              <CardHeader>
                <CardTitle className="text-base">{profile.name}</CardTitle>
              </CardHeader>
              <CardContent>
                <p className="text-sm text-muted-foreground">{profile.description ?? ""}</p>
                <div className="mt-2 flex flex-wrap gap-1">
                  {profile.skills.map((s: string) => (
                    <span key={s} className="rounded-md bg-secondary px-2 py-0.5 text-xs">{s}</span>
                  ))}
                </div>
                {profile.includes.length > 0 && (
                  <p className="mt-1 text-xs text-muted-foreground">Includes: {profile.includes.join(", ")}</p>
                )}
              </CardContent>
            </Card>
          ))}
        </div>
      ) : (
        <p className="text-muted-foreground">No profiles defined.</p>
      )}
    </div>
  )
}
