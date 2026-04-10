import { useState, useMemo, useEffect } from "react"
import { useSearchParams } from "react-router"
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
  deleteProfile,
  listSkills,
  activateGlobal,
  deactivateGlobal,
  editGlobalSkills,
} from "@/lib/api"
import { toast } from "sonner"
import { Plus, MoreVertical, Layers, Globe } from "lucide-react"
import type { Profile } from "@/lib/schemas"
import { formatTokens } from "@/lib/format"
import { ProfileDetailSheet } from "./profiles/ProfileDetailSheet"

export function Profiles() {
  const queryClient = useQueryClient()
  const { data, isLoading } = useQuery({ queryKey: ["profiles"], queryFn: listProfiles })
  const { data: skills } = useQuery({ queryKey: ["skills"], queryFn: listSkills })

  const profiles = data?.profiles ?? []

  const [showCreate, setShowCreate] = useState(false)
  const [showDelete, setShowDelete] = useState<string | null>(null)
  const [detailName, setDetailName] = useState<string | null>(null)
  const [detailMode, setDetailMode] = useState<"view" | "edit">("view")

  // Deep link: /profiles?detail=<name> auto-opens that profile's detail sheet.
  const [searchParams, setSearchParams] = useSearchParams()
  useEffect(() => {
    const target = searchParams.get("detail")
    if (!target || !profiles.length) return
    const match = profiles.find((p) => p.name === target)
    if (match) {
      setDetailMode("view")
      setDetailName(match.name)
    }
    const next = new URLSearchParams(searchParams)
    next.delete("detail")
    setSearchParams(next, { replace: true })
  }, [searchParams, profiles, setSearchParams])

  // Create form state
  const [newName, setNewName] = useState("")
  const [newDesc, setNewDesc] = useState("")
  const [newSkills, setNewSkills] = useState<string[]>([])
  const [newIncludes, setNewIncludes] = useState<string[]>([])
  const skillSuggestions = skills?.map((s) => s.name) ?? []
  const profileSuggestions = profiles.map((p) => p.name)

  // Build a lookup for skill token estimates
  const skillTokenMap = useMemo(() => {
    const map = new Map<string, number>()
    for (const s of skills ?? []) {
      map.set(s.name, s.token_estimate)
    }
    return map
  }, [skills])

  const profileTokenTotals = useMemo(() => {
    const totals = new Map<string, number>()

    function resolve(
      profile: { name?: string; skills: string[]; includes: string[] },
      visited: Set<string>,
    ): number {
      if (profile.name) visited.add(profile.name)
      const direct = profile.skills.reduce(
        (sum, name) => sum + (skillTokenMap.get(name) ?? 0),
        0,
      )
      const inherited = profile.includes.reduce(
        (sum, profName) => {
          if (visited.has(profName)) return sum
          visited.add(profName)
          const included = profiles.find((p) => p.name === profName)
          return sum + (included ? resolve(included, visited) : 0)
        },
        0,
      )
      return direct + inherited
    }

    for (const p of profiles) {
      totals.set(p.name, resolve(p, new Set<string>()))
    }
    return totals
  }, [profiles, skillTokenMap])

  function resolveProfileTokenTotal(
    profile: { name?: string; skills: string[]; includes: string[] },
  ): number {
    if (profile.name && profileTokenTotals.has(profile.name)) {
      return profileTokenTotals.get(profile.name)!
    }
    // Fallback for ad-hoc profiles (e.g. global skills)
    return profile.skills.reduce(
      (sum, name) => sum + (skillTokenMap.get(name) ?? 0),
      0,
    )
  }

  const createMutation = useMutation({
    mutationFn: () => createProfile(newName, newSkills, newIncludes, newDesc || undefined),
    onSuccess: (msg) => {
      toast.success(msg)
      queryClient.invalidateQueries({ queryKey: ["profiles"] })
      closeCreate()
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

  // Global skills state
  const [showGlobalEdit, setShowGlobalEdit] = useState(false)
  const [globalSkillsDraft, setGlobalSkillsDraft] = useState<string[]>([])
  const globalInfo = data?.global

  const activateGlobalMutation = useMutation({
    mutationFn: activateGlobal,
    onSuccess: (msg) => {
      toast.success(msg)
      queryClient.invalidateQueries({ queryKey: ["profiles"] })
    },
    onError: (err) => toast.error(String(err)),
  })

  const deactivateGlobalMutation = useMutation({
    mutationFn: deactivateGlobal,
    onSuccess: (msg) => {
      toast.success(msg)
      queryClient.invalidateQueries({ queryKey: ["profiles"] })
    },
    onError: (err) => toast.error(String(err)),
  })

  const editGlobalMutation = useMutation({
    mutationFn: (skills: string[]) => editGlobalSkills(skills),
    onSuccess: (msg) => {
      toast.success(msg)
      queryClient.invalidateQueries({ queryKey: ["profiles"] })
      setShowGlobalEdit(false)
    },
    onError: (err) => toast.error(String(err)),
  })

  function openGlobalEdit() {
    setGlobalSkillsDraft(globalInfo?.skills ?? [])
    setShowGlobalEdit(true)
  }

  function closeCreate() {
    setShowCreate(false)
    setNewName("")
    setNewDesc("")
    setNewSkills([])
    setNewIncludes([])
  }

  function openDetail(profile: Profile, mode: "view" | "edit" = "view") {
    setDetailMode(mode)
    setDetailName(profile.name)
  }

  return (
    <div className="flex flex-col flex-1 min-h-0">
      {/* Header — fixed */}
      <div className="shrink-0 flex items-center justify-between pb-6">
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

      {/* Profile Detail Sheet */}
      <ProfileDetailSheet
        profile={detailName ? profiles.find((p) => p.name === detailName) ?? null : null}
        allProfiles={profiles}
        skills={skills}
        initialMode={detailMode}
        onClose={() => setDetailName(null)}
        onDelete={(profile) => {
          setDetailName(null)
          setShowDelete(profile.name)
        }}
        onDuplicated={(newName) => {
          setDetailMode("view")
          setDetailName(newName)
        }}
      />

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

      {/* Global Skills Edit Dialog */}
      <Dialog open={showGlobalEdit} onOpenChange={(o) => { if (!o) setShowGlobalEdit(false) }}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Edit Global Skills</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <p className="text-sm text-muted-foreground">
              Global skills are placed in each agent&apos;s global path (machine-level, not per-project).
            </p>
            <div className="space-y-2">
              <Label>Skills</Label>
              <TagInput
                value={globalSkillsDraft}
                onChange={setGlobalSkillsDraft}
                suggestions={skillSuggestions}
                placeholder="Search and add skills..."
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowGlobalEdit(false)}>
              Cancel
            </Button>
            <Button
              onClick={() => editGlobalMutation.mutate(globalSkillsDraft)}
              disabled={editGlobalMutation.isPending}
            >
              {editGlobalMutation.isPending ? "Saving..." : "Save"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Global Skills Card */}
      {globalInfo && (
        <div className="shrink-0 mb-4">
          <div className="rounded-xl border border-border bg-card p-5">
            <div className="flex items-start gap-4">
              <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-[10px] bg-blue-500/10">
                <Globe className="h-5 w-5 text-blue-500" />
              </div>
              <div className="flex-1 space-y-2 min-w-0">
                <div className="flex items-center gap-2.5">
                  <span className="text-[15px] font-semibold">Global Skills</span>
                  <Badge variant={globalInfo.is_active ? "default" : "secondary"} className="text-[10px]">
                    {globalInfo.is_active ? "ACTIVE" : "INACTIVE"}
                  </Badge>
                </div>
                <p className="text-[13px] text-muted-foreground">
                  Skills placed in each agent&apos;s global path (machine-level)
                </p>
                {globalInfo.skills.length > 0 && (
                  <div className="flex flex-wrap gap-1">
                    {globalInfo.skills.map((s) => (
                      <span
                        key={s}
                        className="rounded bg-muted px-1.5 py-0.5 text-[11px] font-medium text-muted-foreground"
                      >
                        {s}
                      </span>
                    ))}
                  </div>
                )}
                <div className="flex items-center gap-4 text-xs text-muted-foreground">
                  <span>{globalInfo.skills.length} skill{globalInfo.skills.length !== 1 ? "s" : ""} configured</span>
                  <span>~{formatTokens(resolveProfileTokenTotal({ skills: globalInfo.skills, includes: [] }))} tokens</span>
                  {globalInfo.placed_skills.length > 0 && (
                    <span className="text-emerald-600 dark:text-emerald-400">
                      {globalInfo.placed_skills.length} placed
                    </span>
                  )}
                </div>
              </div>
              <div className="flex shrink-0 gap-2">
                {globalInfo.is_active ? (
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => deactivateGlobalMutation.mutate()}
                    disabled={deactivateGlobalMutation.isPending}
                  >
                    {deactivateGlobalMutation.isPending ? "..." : "Deactivate"}
                  </Button>
                ) : (
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => activateGlobalMutation.mutate()}
                    disabled={activateGlobalMutation.isPending || globalInfo.skills.length === 0}
                  >
                    {activateGlobalMutation.isPending ? "..." : "Activate"}
                  </Button>
                )}
                <button
                  onClick={openGlobalEdit}
                  className="shrink-0 rounded p-1.5 text-muted-foreground transition-colors hover:text-foreground hover:bg-muted"
                >
                  <MoreVertical className="h-4 w-4" />
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Profile List — scrollable */}
      <div className="flex-1 min-h-0 overflow-y-auto">
      {isLoading ? (
        <p className="text-muted-foreground">Loading...</p>
      ) : profiles.length > 0 ? (
        <div className="space-y-3 pb-4">
          {profiles.map((profile: Profile) => {
            const isBase = profile.name === "base" || profile.name === "base-layer"
            return (
              <div
                key={profile.name}
                role="button"
                tabIndex={0}
                onClick={() => openDetail(profile, "view")}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault()
                    openDetail(profile, "view")
                  }
                }}
                className="animate-list-item group cursor-pointer rounded-xl border border-border bg-card p-5 transition-colors hover:border-primary/30 focus:border-primary/50 focus:outline-none"
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
                      <span>~{formatTokens(resolveProfileTokenTotal(profile))} tokens</span>
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

                  {/* Quick edit: opens the sheet directly in edit mode */}
                  <button
                    onClick={(e) => {
                      e.stopPropagation()
                      openDetail(profile, "edit")
                    }}
                    className="shrink-0 rounded p-1.5 text-muted-foreground transition-colors hover:text-foreground hover:bg-muted"
                    title="Edit"
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
    </div>
  )
}
