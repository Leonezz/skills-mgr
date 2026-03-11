import { useState } from "react"
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Badge } from "@/components/ui/badge"
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog"
import { TagInput } from "@/components/ui/tag-input"
import {
  listProfiles,
  createProfile,
  editProfile,
  deleteProfile,
  listSkills,
} from "@/lib/api"
import { toast } from "sonner"
import { Plus, MoreVertical, Layers } from "lucide-react"
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
  const [newDesc, setNewDesc] = useState("")
  const [newSkills, setNewSkills] = useState<string[]>([])
  const [newIncludes, setNewIncludes] = useState<string[]>([])

  // Edit form state
  const [editDesc, setEditDesc] = useState("")
  const [editAddSkills, setEditAddSkills] = useState<string[]>([])
  const [editRemoveSkills, setEditRemoveSkills] = useState<string[]>([])
  const [editAddIncludes, setEditAddIncludes] = useState<string[]>([])

  const profiles = data?.profiles ?? []
  const skillSuggestions = skills?.map((s) => s.name) ?? []
  const profileSuggestions = profiles.map((p) => p.name)

  const createMutation = useMutation({
    mutationFn: () => createProfile(newName, newSkills, newIncludes, newDesc || undefined),
    onSuccess: (msg) => {
      toast.success(msg)
      queryClient.invalidateQueries({ queryKey: ["profiles"] })
      closeCreate()
    },
    onError: (err) => toast.error(String(err)),
  })

  const editMutation = useMutation({
    mutationFn: () =>
      editProfile(
        showEdit!.name,
        editAddSkills,
        editRemoveSkills,
        editAddIncludes,
        editDesc || undefined,
      ),
    onSuccess: (msg) => {
      toast.success(msg)
      queryClient.invalidateQueries({ queryKey: ["profiles"] })
      closeEdit()
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

  function closeCreate() {
    setShowCreate(false)
    setNewName("")
    setNewDesc("")
    setNewSkills([])
    setNewIncludes([])
  }

  function openEdit(profile: Profile) {
    setEditAddSkills([])
    setEditRemoveSkills([])
    setEditAddIncludes([])
    setEditDesc(profile.description ?? "")
    setShowEdit(profile)
  }

  function closeEdit() {
    setShowEdit(null)
    setEditAddSkills([])
    setEditRemoveSkills([])
    setEditAddIncludes([])
    setEditDesc("")
  }

  function getEditCurrentSkills(): string[] {
    if (!showEdit) return []
    const remaining = showEdit.skills.filter((s) => !editRemoveSkills.includes(s))
    return [...remaining, ...editAddSkills]
  }

  function handleEditSkillsChange(tags: string[]) {
    if (!showEdit) return
    const originalSkills = showEdit.skills
    const removals: string[] = []
    const additions: string[] = []
    for (const skill of originalSkills) {
      if (!tags.includes(skill)) removals.push(skill)
    }
    for (const tag of tags) {
      if (!originalSkills.includes(tag)) additions.push(tag)
    }
    setEditRemoveSkills(removals)
    setEditAddSkills(additions)
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold tracking-tight">Profiles</h2>
          <p className="text-sm text-muted-foreground">
            Compose and manage skill profiles
          </p>
        </div>
        <Button onClick={() => setShowCreate(true)}>
          <Plus className="h-4 w-4" />
          Create Profile
        </Button>
      </div>

      {/* Create Dialog */}
      <Dialog open={showCreate} onOpenChange={(o) => { if (!o) closeCreate() }}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Create Profile</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-2">
              <Label>Profile Name</Label>
              <Input
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                placeholder="e.g. typescript-dev"
              />
            </div>
            <div className="space-y-2">
              <Label>Description</Label>
              <Input
                value={newDesc}
                onChange={(e) => setNewDesc(e.target.value)}
                placeholder="TypeScript development profile with strict typing..."
              />
            </div>
            <div className="space-y-2">
              <Label>Compose from Profiles</Label>
              <p className="text-xs text-muted-foreground">Inherit skills from existing profiles</p>
              <TagInput
                value={newIncludes}
                onChange={setNewIncludes}
                suggestions={profileSuggestions}
                placeholder="+ Add profile"
              />
            </div>
            <div className="space-y-2">
              <Label>Include Skills</Label>
              <TagInput
                value={newSkills}
                onChange={setNewSkills}
                suggestions={skillSuggestions}
                placeholder="Search and add skills..."
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={closeCreate}>
              Cancel
            </Button>
            <Button
              onClick={() => createMutation.mutate()}
              disabled={!newName || createMutation.isPending}
            >
              {createMutation.isPending ? "Creating..." : "Create Profile"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Edit Dialog */}
      <Dialog open={showEdit !== null} onOpenChange={(o) => { if (!o) closeEdit() }}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Edit Profile</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-2">
              <Label>Profile Name</Label>
              <Input value={showEdit?.name ?? ""} disabled />
            </div>
            <div className="space-y-2">
              <Label>Description</Label>
              <Input
                value={editDesc}
                onChange={(e) => setEditDesc(e.target.value)}
                placeholder="Profile description..."
              />
            </div>
            <div className="space-y-2">
              <Label>Compose from Profiles</Label>
              <TagInput
                value={[...(showEdit?.includes ?? []), ...editAddIncludes]}
                onChange={(tags) => {
                  const existing = showEdit?.includes ?? []
                  setEditAddIncludes(tags.filter((t) => !existing.includes(t)))
                }}
                suggestions={profileSuggestions.filter(
                  (p) => p !== showEdit?.name && !showEdit?.includes.includes(p),
                )}
                placeholder="+ Add profile"
              />
            </div>
            <div className="space-y-2">
              <Label>Direct Skills ({getEditCurrentSkills().length})</Label>
              <TagInput
                value={getEditCurrentSkills()}
                onChange={handleEditSkillsChange}
                suggestions={skillSuggestions}
                placeholder="+ Add skill"
              />
            </div>
          </div>
          <DialogFooter className="justify-between">
            <button
              onClick={() => {
                if (showEdit) {
                  setShowDelete(showEdit.name)
                  closeEdit()
                }
              }}
              className="text-sm text-destructive hover:underline"
            >
              Delete Profile
            </button>
            <div className="flex gap-2">
              <Button variant="outline" onClick={closeEdit}>
                Cancel
              </Button>
              <Button
                onClick={() => editMutation.mutate()}
                disabled={editMutation.isPending}
              >
                {editMutation.isPending ? "Saving..." : "Save Changes"}
              </Button>
            </div>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation */}
      <Dialog open={showDelete !== null} onOpenChange={(o) => { if (!o) setShowDelete(null) }}>
        <DialogContent className="max-w-[420px] space-y-4 p-7">
          <div className="flex items-center gap-3">
            <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-destructive/15 text-lg font-bold text-destructive">
              !
            </div>
            <h2 className="text-lg font-semibold">Delete Profile?</h2>
          </div>
          <p className="text-sm leading-relaxed text-muted-foreground">
            Are you sure you want to delete &quot;{showDelete}&quot;? This action cannot
            be undone. All skill associations will be removed.
          </p>
          <hr className="border-border" />
          <div className="flex gap-3">
            <Button variant="outline" className="flex-1" onClick={() => setShowDelete(null)}>
              Cancel
            </Button>
            <Button
              variant="destructive"
              className="flex-1"
              onClick={() => { if (showDelete) deleteMutation.mutate(showDelete) }}
              disabled={deleteMutation.isPending}
            >
              {deleteMutation.isPending ? "Deleting..." : "Delete"}
            </Button>
          </div>
        </DialogContent>
      </Dialog>

      {/* Profile List */}
      {isLoading ? (
        <p className="text-muted-foreground">Loading...</p>
      ) : profiles.length > 0 ? (
        <div className="space-y-3">
          {profiles.map((profile: Profile) => {
            const isBase = profile.name === "base" || profile.name === "base-layer"
            return (
              <div
                key={profile.name}
                className="animate-list-item group rounded-xl border border-border bg-card p-5 transition-colors hover:border-primary/30"
              >
                <div className="flex items-start gap-4">
                  {/* Icon */}
                  <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-[10px] bg-primary/10">
                    <Layers className="h-5 w-5 text-primary" />
                  </div>

                  {/* Content */}
                  <div className="flex-1 space-y-2 min-w-0">
                    {/* Name + badge */}
                    <div className="flex items-center gap-2.5">
                      <span className="text-[15px] font-semibold">{profile.name}</span>
                      {isBase && (
                        <Badge variant="secondary" className="text-[10px]">
                          BASE
                        </Badge>
                      )}
                    </div>
                    {/* Description */}
                    {profile.description && (
                      <p className="text-[13px] text-muted-foreground line-clamp-2">
                        {profile.description}
                      </p>
                    )}
                    {/* Skills as mini badges */}
                    {profile.skills.length > 0 && (
                      <div className="flex flex-wrap gap-1">
                        {profile.skills.slice(0, 5).map((s) => (
                          <span
                            key={s}
                            className="rounded bg-muted px-1.5 py-0.5 text-[11px] font-medium text-muted-foreground"
                          >
                            {s}
                          </span>
                        ))}
                        {profile.skills.length > 5 && (
                          <span className="rounded bg-muted px-1.5 py-0.5 text-[11px] text-muted-foreground">
                            +{profile.skills.length - 5} more
                          </span>
                        )}
                      </div>
                    )}

                    {/* Meta row */}
                    <div className="flex items-center gap-4 text-xs text-muted-foreground">
                      <span>{profile.skills.length} skill{profile.skills.length !== 1 ? "s" : ""}</span>
                      {profile.includes.length > 0 && (
                        <span className="text-primary">
                          Includes: {profile.includes.join(", ")}
                        </span>
                      )}
                      {profile.active_projects.length > 0 && (
                        <span className="text-emerald-600 dark:text-emerald-400">
                          Active in {profile.active_projects.length} project{profile.active_projects.length > 1 ? "s" : ""}
                        </span>
                      )}
                    </div>
                  </div>

                  {/* Edit button */}
                  <button
                    onClick={() => openEdit(profile)}
                    className="shrink-0 rounded p-1.5 text-muted-foreground transition-colors hover:text-foreground hover:bg-muted"
                  >
                    <MoreVertical className="h-4 w-4" />
                  </button>
                </div>
              </div>
            )
          })}
        </div>
      ) : (
        <p className="text-muted-foreground">No profiles defined.</p>
      )}
    </div>
  )
}
