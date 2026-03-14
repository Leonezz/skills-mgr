import { useState, useMemo } from "react"
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { Card, CardContent } from "@/components/ui/card"
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
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetBody,
  SheetFooter,
} from "@/components/ui/sheet"
import { listSkills, createSkill, removeSkill, importSkill, importRemoteSkill, browseRemote, importFromBrowse, openSkillDir, updateSkill, scanSkills, delegateSkills, linkRemote, unlinkRemote, listProfiles } from "@/lib/api"
import type { RemoteSkillEntry, DelegateRequest } from "@/lib/api"
import { open } from "@tauri-apps/plugin-dialog"
import { toast } from "sonner"
import {
  Plus,
  Search,
  Filter,
  MoreVertical,
  FileCode,
  FolderOpen,
  Globe,
  Loader2,
  Check,
  ExternalLink,
  AlertTriangle,
} from "lucide-react"
import type { Skill, DiscoveredSkill } from "@/lib/schemas"
import { formatTokens, formatBytes } from "@/lib/format"

export function Skills() {
  const queryClient = useQueryClient()
  const { data: skills, isLoading } = useQuery({
    queryKey: ["skills"],
    queryFn: listSkills,
  })

  const [search, setSearch] = useState("")
  const [showAdd, setShowAdd] = useState(false)
  const [showDelete, setShowDelete] = useState<string | null>(null)
  const [detail, setDetail] = useState<Skill | null>(null)
  const [showEdit, setShowEdit] = useState<Skill | null>(null)
  const [editDesc, setEditDesc] = useState("")

  // Tab state
  const [tab, setTab] = useState<"registry" | "discover">("registry")

  // Discovery query (manual trigger)
  const {
    data: discovered,
    isLoading: isScanning,
    refetch: runScan,
  } = useQuery({
    queryKey: ["discoveredSkills"],
    queryFn: scanSkills,
    enabled: false,
  })

  // Profiles query (for delegation dialog)
  const { data: profilesData } = useQuery({
    queryKey: ["profiles"],
    queryFn: listProfiles,
  })

  // Delegation state
  const [showDelegate, setShowDelegate] = useState(false)
  const [delegateSelected, setDelegateSelected] = useState<Set<string>>(new Set())
  const [delegateMode, setDelegateMode] = useState<"new" | "existing">("existing")
  const [delegateProfileName, setDelegateProfileName] = useState("")
  const [delegateProfileDesc, setDelegateProfileDesc] = useState("")
  const [delegateExistingProfile, setDelegateExistingProfile] = useState("")

  // Link remote state
  const [showLinkRemote, setShowLinkRemote] = useState(false)
  const [linkUrl, setLinkUrl] = useState("")
  const [linkRef, setLinkRef] = useState("main")
  const [linkSubpath, setLinkSubpath] = useState("")

  // Add form state
  const [addMode, setAddMode] = useState<"create" | "local" | "remote">("create")
  const [newName, setNewName] = useState("")
  const [newDesc, setNewDesc] = useState("")
  const [newSourcePath, setNewSourcePath] = useState("")
  const [remoteUrl, setRemoteUrl] = useState("")
  const [remoteSkills, setRemoteSkills] = useState<RemoteSkillEntry[]>([])
  const [selectedRemote, setSelectedRemote] = useState<Set<string>>(new Set())
  const [browseLoading, setBrowseLoading] = useState(false)
  const [browseError, setBrowseError] = useState("")

  // Conflict resolution state
  const [conflictAction, setConflictAction] = useState<"overwrite" | "skip">("overwrite")

  const existingNames = useMemo(
    () => new Set(skills?.map((s) => s.name) ?? []),
    [skills],
  )

  const filteredSkills = useMemo(() => {
    if (!skills) return []
    const q = search.trim().toLowerCase()
    if (!q) return skills
    return skills.filter(
      (s) =>
        s.name.toLowerCase().includes(q) ||
        (s.description ?? "").toLowerCase().includes(q) ||
        (s.source_type ?? "").toLowerCase().includes(q)
    )
  }, [skills, search])

  function isUrlLike(s: string): boolean {
    return s.startsWith("http://") || s.startsWith("https://") || /^[a-zA-Z0-9_-]+\/[a-zA-Z0-9_-]+/.test(s)
  }

  const conflictingRemote = useMemo(
    () => remoteSkills.filter((s) => existingNames.has(s.name)),
    [remoteSkills, existingNames],
  )

  const createMutation = useMutation({
    mutationFn: async () => {
      // Auto-detect: if user typed a URL/shorthand in create mode, redirect to remote import
      if (addMode === "create" && isUrlLike(newName)) {
        return importRemoteSkill(newName)
      }
      if (addMode === "remote") {
        // If we have browsed skills selected, import from cached staging
        if (remoteSkills.length > 0 && selectedRemote.size > 0) {
          let selected = [...selectedRemote]
          // Apply conflict resolution — skip conflicting ones if action is "skip"
          if (conflictAction === "skip") {
            selected = selected.filter((subpath) => {
              const entry = remoteSkills.find((s) => s.subpath === subpath)
              return !entry || !existingNames.has(entry.name)
            })
          }
          if (selected.length === 0) {
            return "No new skills to import (all skipped due to conflicts)"
          }
          return importFromBrowse(selected)
        }
        return importRemoteSkill(remoteUrl)
      }
      if (addMode === "local") {
        return importSkill(newSourcePath)
      }
      if (newName.includes("/") || newName.includes("\\")) {
        return Promise.reject(new Error("Skill name cannot contain '/' or '\\'. Did you mean to use GitHub Import?"))
      }
      return createSkill(newName, newDesc || "No description")
    },
    onSuccess: (msg) => {
      toast.success(msg)
      queryClient.invalidateQueries({ queryKey: ["skills"] })
      closeAdd()
    },
    onError: (err) => toast.error(String(err)),
  })

  const deleteMutation = useMutation({
    mutationFn: (name: string) => removeSkill(name),
    onSuccess: (msg) => {
      toast.success(msg)
      queryClient.invalidateQueries({ queryKey: ["skills"] })
      setShowDelete(null)
      if (detail?.name === showDelete) setDetail(null)
    },
    onError: (err) => toast.error(String(err)),
  })

  const editMutation = useMutation({
    mutationFn: () => updateSkill(showEdit!.name, editDesc),
    onSuccess: (msg) => {
      toast.success(msg)
      queryClient.invalidateQueries({ queryKey: ["skills"] })
      closeEdit()
    },
    onError: (err) => toast.error(String(err)),
  })

  const delegateMutation = useMutation({
    mutationFn: () => {
      if (!discovered) return Promise.reject(new Error("No discovered skills"))
      const selected = discovered.filter((s) => delegateSelected.has(s.found_path))
      const requests: DelegateRequest[] = selected.map((s) => ({
        name: s.name,
        agent_name: s.agent_name,
        found_path: s.found_path,
      }))
      const isNew = delegateMode === "new"
      const profileName = isNew ? delegateProfileName : delegateExistingProfile
      return delegateSkills(requests, profileName, isNew, isNew ? delegateProfileDesc || undefined : undefined)
    },
    onSuccess: (msg) => {
      toast.success(msg)
      queryClient.invalidateQueries({ queryKey: ["skills"] })
      queryClient.invalidateQueries({ queryKey: ["profiles"] })
      closeDelegateDialog()
    },
    onError: (err) => toast.error(String(err)),
  })

  const linkRemoteMutation = useMutation({
    mutationFn: () => {
      if (!detail) return Promise.reject(new Error("No skill selected"))
      return linkRemote(detail.name, linkUrl, linkRef, linkSubpath || undefined)
    },
    onSuccess: (msg) => {
      toast.success(msg)
      queryClient.invalidateQueries({ queryKey: ["skills"] })
      closeLinkRemoteDialog()
    },
    onError: (err) => toast.error(String(err)),
  })

  const unlinkRemoteMutation = useMutation({
    mutationFn: (name: string) => unlinkRemote(name),
    onSuccess: (msg) => {
      toast.success(msg)
      queryClient.invalidateQueries({ queryKey: ["skills"] })
      setDetail(null)
    },
    onError: (err) => toast.error(String(err)),
  })

  function closeDelegateDialog() {
    setShowDelegate(false)
    setDelegateSelected(new Set())
    setDelegateMode("existing")
    setDelegateProfileName("")
    setDelegateProfileDesc("")
    setDelegateExistingProfile("")
  }

  function closeLinkRemoteDialog() {
    setShowLinkRemote(false)
    setLinkUrl("")
    setLinkRef("main")
    setLinkSubpath("")
  }

  function openEdit(skill: Skill) {
    setEditDesc(skill.description ?? "")
    setShowEdit(skill)
    setDetail(null)
  }

  function closeEdit() {
    setShowEdit(null)
    setEditDesc("")
  }

  function closeAdd() {
    setShowAdd(false)
    setAddMode("create")
    setNewName("")
    setNewDesc("")
    setNewSourcePath("")
    setRemoteUrl("")
    setRemoteSkills([])
    setSelectedRemote(new Set())
    setBrowseLoading(false)
    setBrowseError("")
    setConflictAction("overwrite")
  }

  async function handleBrowseRemote() {
    if (!remoteUrl.trim()) return
    setBrowseLoading(true)
    setBrowseError("")
    setRemoteSkills([])
    setSelectedRemote(new Set())
    try {
      const skills = await browseRemote(remoteUrl.trim())
      if (skills.length === 0) {
        setBrowseError("No skills found in this repository.")
      } else if (skills.length === 1) {
        // Single skill — check conflict
        if (existingNames.has(skills[0].name)) {
          // Show as selectable so user can decide
          setRemoteSkills(skills)
          setSelectedRemote(new Set(skills.map((s) => s.subpath)))
        } else {
          // No conflict — import directly from staging
          const msg = await importFromBrowse([skills[0].subpath])
          toast.success(msg)
          queryClient.invalidateQueries({ queryKey: ["skills"] })
          closeAdd()
        }
      } else {
        setRemoteSkills(skills)
        setSelectedRemote(new Set(skills.map((s) => s.subpath)))
      }
    } catch (e) {
      setBrowseError(String(e))
    } finally {
      setBrowseLoading(false)
    }
  }


  async function handleBrowseSource() {
    const selected = await open({
      directory: true,
      title: "Select skill folder",
    })
    if (selected) setNewSourcePath(selected as string)
  }


  function formatSourceDisplay(skill: Skill): string {
    if (!skill.source_type) return "Local file"
    const parts: string[] = [skill.source_type]
    if (skill.source_url) {
      // Shorten GitHub URLs: https://github.com/owner/repo → owner/repo
      const gh = skill.source_url.replace(/^https?:\/\/github\.com\//, "")
      parts.push(gh)
    }
    if (skill.source_ref && skill.source_ref !== "main" && skill.source_ref !== "master") {
      parts.push(`@${skill.source_ref}`)
    }
    return parts.join(" \u00b7 ")
  }

  return (
    <div className="flex flex-col flex-1 min-h-0">
      {/* Header — fixed */}
      <div className="shrink-0 space-y-6 pb-6">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-2xl font-bold tracking-tight">Skills Registry</h2>
            <p className="text-sm text-muted-foreground">
              Manage your skill collection
            </p>
          </div>
          <Button onClick={() => setShowAdd(true)}>
            <Plus className="h-4 w-4" />
            Add Skill
          </Button>
        </div>

        {/* Search */}
        <div className="flex items-center gap-3">
          <div className="relative flex-1">
            <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Search skills by name, tag, or source..."
              className="pl-9"
            />
          </div>
          <Button variant="outline" size="sm">
            <Filter className="h-4 w-4" />
            Filter
          </Button>
        </div>

        {/* Tab Switcher */}
        <div className="flex gap-1 rounded-lg bg-muted p-1">
          <button
            onClick={() => setTab("registry")}
            className={`flex items-center gap-1.5 rounded-md px-3 py-1.5 text-sm font-medium transition-colors ${
              tab === "registry"
                ? "bg-background text-foreground shadow-sm"
                : "text-muted-foreground hover:text-foreground"
            }`}
          >
            Registry
            {skills && skills.length > 0 && (
              <Badge variant="secondary" className="ml-1 text-[10px]">
                {skills.length}
              </Badge>
            )}
          </button>
          <button
            onClick={() => setTab("discover")}
            className={`flex items-center gap-1.5 rounded-md px-3 py-1.5 text-sm font-medium transition-colors ${
              tab === "discover"
                ? "bg-background text-foreground shadow-sm"
                : "text-muted-foreground hover:text-foreground"
            }`}
          >
            Discover
            {discovered && discovered.length > 0 && (
              <Badge variant="secondary" className="ml-1 text-[10px]">
                {discovered.length}
              </Badge>
            )}
          </button>
        </div>
      </div>

      {/* Add Skill Dialog */}
      <Dialog open={showAdd} onOpenChange={(o) => { if (!o) closeAdd() }}>
        <DialogContent className={remoteSkills.length > 0 ? "max-w-lg" : undefined}>
          <DialogHeader>
            <DialogTitle>Add New Skill</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            {/* Source mode tabs */}
            <div className="flex gap-1 rounded-lg bg-muted p-1">
              {([
                { key: "create" as const, label: "Create", icon: Plus },
                { key: "local" as const, label: "Local Import", icon: FolderOpen },
                { key: "remote" as const, label: "GitHub Import", icon: Globe },
              ]).map(({ key, label, icon: Icon }) => (
                <button
                  key={key}
                  onClick={() => setAddMode(key)}
                  className={`flex flex-1 items-center justify-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-medium transition-colors ${
                    addMode === key
                      ? "bg-background text-foreground shadow-sm"
                      : "text-muted-foreground hover:text-foreground"
                  }`}
                >
                  <Icon className="h-3.5 w-3.5" />
                  {label}
                </button>
              ))}
            </div>

            {/* Create mode */}
            {addMode === "create" && (
              <>
                <div className="space-y-2">
                  <Label>Skill Name</Label>
                  <Input
                    value={newName}
                    onChange={(e) => setNewName(e.target.value)}
                    placeholder="e.g. eslint-config"
                  />
                </div>
                <div className="space-y-2">
                  <Label>Description</Label>
                  <Input
                    value={newDesc}
                    onChange={(e) => setNewDesc(e.target.value)}
                    placeholder="Brief description of this skill"
                  />
                </div>
              </>
            )}

            {/* Local import mode */}
            {addMode === "local" && (
              <div className="space-y-2">
                <Label>Source Folder</Label>
                <div className="flex gap-2">
                  <Input
                    value={newSourcePath}
                    onChange={(e) => setNewSourcePath(e.target.value)}
                    placeholder="~/skills/my-skill/"
                    className="flex-1"
                  />
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    onClick={handleBrowseSource}
                  >
                    <FolderOpen className="h-4 w-4" />
                  </Button>
                </div>
                <p className="text-xs text-muted-foreground">
                  Select a folder containing SKILL.md. Name and description will be read automatically.
                </p>
              </div>
            )}

            {/* Remote import mode */}
            {addMode === "remote" && (
              <div className="space-y-3">
                <div className="space-y-2">
                  <Label>GitHub URL or Shorthand</Label>
                  <Input
                    value={remoteUrl}
                    onChange={(e) => {
                      setRemoteUrl(e.target.value)
                      setRemoteSkills([])
                      setSelectedRemote(new Set())
                      setBrowseError("")
                    }}
                    placeholder="owner/repo or https://github.com/owner/repo"
                    onKeyDown={(e) => {
                      if (e.key === "Enter") {
                        e.preventDefault()
                        handleBrowseRemote()
                      }
                    }}
                  />
                  {!remoteSkills.length && !browseLoading && !browseError && (
                    <p className="text-xs text-muted-foreground">
                      Enter a repository URL and click Import. Single-skill repos are imported directly.
                      Multi-skill collections let you pick which skills to import.
                    </p>
                  )}
                </div>

                {/* Loading state */}
                {browseLoading && (
                  <div className="flex items-center gap-2 text-sm text-muted-foreground">
                    <Loader2 className="h-4 w-4 animate-spin" />
                    Downloading and scanning repository...
                  </div>
                )}

                {/* Browse error */}
                {browseError && (
                  <p className="text-xs text-destructive">{browseError}</p>
                )}

                {/* Discovered skills — selectable card grid */}
                {remoteSkills.length > 0 && (
                  <div className="space-y-2">
                    <div className="flex items-center justify-between">
                      <Label>
                        Found {remoteSkills.length} skills — select to import
                      </Label>
                      <button
                        type="button"
                        className="text-xs text-primary hover:underline"
                        onClick={() => {
                          if (selectedRemote.size === remoteSkills.length) {
                            setSelectedRemote(new Set())
                          } else {
                            setSelectedRemote(new Set(remoteSkills.map((s) => s.subpath)))
                          }
                        }}
                      >
                        {selectedRemote.size === remoteSkills.length ? "Deselect All" : "Select All"}
                      </button>
                    </div>
                    <div className="grid grid-cols-2 gap-2 max-h-56 overflow-y-auto">
                      {remoteSkills.map((entry) => {
                        const isSelected = selectedRemote.has(entry.subpath)
                        const hasConflict = existingNames.has(entry.name)
                        return (
                          <button
                            key={entry.subpath}
                            type="button"
                            onClick={() => {
                              setSelectedRemote((prev) => {
                                const next = new Set(prev)
                                if (next.has(entry.subpath)) next.delete(entry.subpath)
                                else next.add(entry.subpath)
                                return next
                              })
                            }}
                            className={`flex items-start gap-2.5 rounded-lg border p-3 text-left transition-colors ${
                              isSelected
                                ? hasConflict
                                  ? "border-amber-500 bg-amber-500/5"
                                  : "border-primary bg-primary/5"
                                : "border-border hover:border-muted-foreground/30"
                            }`}
                          >
                            <div
                              className={`mt-0.5 flex h-4 w-4 shrink-0 items-center justify-center rounded border transition-colors ${
                                isSelected
                                  ? "border-primary bg-primary text-primary-foreground"
                                  : "border-muted-foreground/30"
                              }`}
                            >
                              {isSelected && <Check className="h-3 w-3" />}
                            </div>
                            <div className="min-w-0 flex-1">
                              <div className="flex items-center gap-1.5">
                                <p className="text-sm font-medium truncate">{entry.name}</p>
                                {hasConflict && (
                                  <AlertTriangle className="h-3.5 w-3.5 shrink-0 text-amber-500" />
                                )}
                              </div>
                              {hasConflict && (
                                <p className="text-[10px] font-medium text-amber-500 mt-0.5">
                                  Already exists
                                </p>
                              )}
                              {entry.description && (
                                <p className="text-xs text-muted-foreground line-clamp-2 mt-0.5">
                                  {entry.description}
                                </p>
                              )}
                            </div>
                          </button>
                        )
                      })}
                    </div>

                    {/* Conflict resolution options */}
                    {conflictingRemote.length > 0 && (
                      <div className="flex items-center gap-3 rounded-md border border-amber-500/30 bg-amber-500/5 p-2.5">
                        <AlertTriangle className="h-4 w-4 shrink-0 text-amber-500" />
                        <span className="text-xs text-amber-700 dark:text-amber-400">
                          {conflictingRemote.length} skill{conflictingRemote.length > 1 ? "s" : ""} already exist{conflictingRemote.length === 1 ? "s" : ""}
                        </span>
                        <div className="ml-auto flex gap-1">
                          <button
                            type="button"
                            onClick={() => setConflictAction("overwrite")}
                            className={`rounded px-2 py-1 text-[11px] font-medium transition-colors ${
                              conflictAction === "overwrite"
                                ? "bg-amber-500 text-white"
                                : "bg-muted text-muted-foreground hover:text-foreground"
                            }`}
                          >
                            Overwrite
                          </button>
                          <button
                            type="button"
                            onClick={() => setConflictAction("skip")}
                            className={`rounded px-2 py-1 text-[11px] font-medium transition-colors ${
                              conflictAction === "skip"
                                ? "bg-amber-500 text-white"
                                : "bg-muted text-muted-foreground hover:text-foreground"
                            }`}
                          >
                            Skip
                          </button>
                        </div>
                      </div>
                    )}
                  </div>
                )}
              </div>
            )}
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={closeAdd}>
              Cancel
            </Button>
            <Button
              onClick={() => {
                if (addMode === "remote" && remoteSkills.length === 0) {
                  handleBrowseRemote()
                } else {
                  createMutation.mutate()
                }
              }}
              disabled={
                createMutation.isPending ||
                browseLoading ||
                (addMode === "create" && !newName) ||
                (addMode === "local" && !newSourcePath) ||
                (addMode === "remote" && !remoteUrl) ||
                (addMode === "remote" && remoteSkills.length > 0 && selectedRemote.size === 0)
              }
            >
              {browseLoading
                ? "Scanning..."
                : createMutation.isPending
                  ? "Importing..."
                  : addMode === "remote"
                    ? remoteSkills.length > 0
                      ? `Import ${selectedRemote.size} Skill${selectedRemote.size !== 1 ? "s" : ""}`
                      : "Import from GitHub"
                  : addMode === "local" ? "Import Skill"
                  : "Create Skill"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation */}
      <Dialog
        open={showDelete !== null}
        onOpenChange={(o) => { if (!o) setShowDelete(null) }}
      >
        <DialogContent className="max-w-[420px] space-y-4 p-7">
          <div className="flex items-center gap-3">
            <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-destructive/15 text-lg font-bold text-destructive">
              !
            </div>
            <h2 className="text-lg font-semibold">Delete Skill?</h2>
          </div>
          <p className="text-sm leading-relaxed text-muted-foreground">
            Are you sure you want to delete &quot;{showDelete}&quot;? This action cannot
            be undone. The skill will be removed from all profiles.
          </p>
          <hr className="border-border" />
          <div className="flex gap-3">
            <Button variant="outline" className="flex-1" onClick={() => setShowDelete(null)}>
              Cancel
            </Button>
            <Button
              variant="destructive"
              className="flex-1"
              onClick={() => {
                if (showDelete) deleteMutation.mutate(showDelete)
              }}
              disabled={deleteMutation.isPending}
            >
              {deleteMutation.isPending ? "Deleting..." : "Delete"}
            </Button>
          </div>
        </DialogContent>
      </Dialog>

      {/* Edit Skill Dialog */}
      <Dialog open={showEdit !== null} onOpenChange={(o) => { if (!o) closeEdit() }}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Edit Skill</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-2">
              <Label>Skill Name</Label>
              <Input value={showEdit?.name ?? ""} disabled />
            </div>
            <div className="space-y-2">
              <Label>Description</Label>
              <Input
                value={editDesc}
                onChange={(e) => setEditDesc(e.target.value)}
                placeholder="Skill description..."
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={closeEdit}>
              Cancel
            </Button>
            <Button
              onClick={() => editMutation.mutate()}
              disabled={editMutation.isPending}
            >
              {editMutation.isPending ? "Saving..." : "Save Changes"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Skill Detail Side Panel */}
      <Sheet open={detail !== null} onOpenChange={(o) => { if (!o) setDetail(null) }}>
        <SheetContent>
          <SheetHeader onClose={() => setDetail(null)}>
            <h3 className="text-lg font-semibold">{detail?.name}</h3>
          </SheetHeader>
          <SheetBody className="space-y-5">
            {/* Tags */}
            {(detail?.source_type || detail?.is_builtin) && (
              <div className="flex flex-wrap gap-2">
                {detail?.is_builtin && <Badge variant="secondary">Built-in</Badge>}
                {detail?.source_type && <Badge variant="accent">{detail.source_type}</Badge>}
              </div>
            )}

            <hr className="border-border" />

            {/* Description */}
            <div className="space-y-2">
              <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                Description
              </p>
              <p className="text-sm leading-relaxed">
                {detail?.description ?? "No description"}
              </p>
            </div>

            <hr className="border-border" />

            {/* Details */}
            <div className="space-y-3">
              <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                Details
              </p>
              <div className="space-y-1 text-sm">
                <span className="text-muted-foreground">Source</span>
                <p className="break-all">
                  {detail ? formatSourceDisplay(detail) : "Local file"}
                </p>
              </div>
              {detail?.source_url && (
                <div className="space-y-1 text-sm">
                  <span className="text-muted-foreground">URL</span>
                  <p className="break-all text-xs font-mono text-muted-foreground">
                    {detail.source_url}
                  </p>
                </div>
              )}
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Files</span>
                <span>{detail?.files.length ?? 0} files</span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Size</span>
                <span>{detail ? formatBytes(detail.total_bytes) : "—"}</span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Token Estimate</span>
                <span>{detail ? `~${formatTokens(detail.token_estimate)} tokens` : "—"}</span>
              </div>
              {detail && detail.files.length > 0 && (
                <div className="mt-2 max-h-32 overflow-y-auto rounded-md border border-border p-2">
                  {detail.files.map((f) => (
                    <p
                      key={f}
                      className="truncate font-mono text-xs text-muted-foreground"
                    >
                      {f}
                    </p>
                  ))}
                </div>
              )}
            </div>
          </SheetBody>
          <SheetFooter>
            <Button
              variant="outline"
              className="shrink-0"
              onClick={() => {
                if (detail) {
                  openSkillDir(detail.name).catch((e) => toast.error(String(e)))
                }
              }}
            >
              <ExternalLink className="h-4 w-4" />
              Open in Finder
            </Button>
            {detail?.source_type === "git" ? (
              <Button
                variant="outline"
                className="shrink-0"
                onClick={() => {
                  if (detail) {
                    unlinkRemoteMutation.mutate(detail.name)
                  }
                }}
                disabled={unlinkRemoteMutation.isPending}
              >
                <Globe className="h-4 w-4" />
                {unlinkRemoteMutation.isPending ? "Unlinking..." : "Unlink Remote"}
              </Button>
            ) : (
              <Button
                variant="outline"
                className="shrink-0"
                onClick={() => setShowLinkRemote(true)}
              >
                <Globe className="h-4 w-4" />
                Link to Remote
              </Button>
            )}
            <Button className="flex-1" onClick={() => { if (detail) openEdit(detail) }}>
              Edit Skill
            </Button>
            {!detail?.is_builtin && (
              <Button
                variant="outline"
                className="flex-1 border-destructive/40 text-destructive hover:bg-destructive/10"
                onClick={() => {
                  if (detail) {
                    setShowDelete(detail.name)
                    setDetail(null)
                  }
                }}
              >
                Remove
              </Button>
            )}
          </SheetFooter>
        </SheetContent>
      </Sheet>

      {/* Skill Cards Grid — scrollable */}
      <div className="flex-1 min-h-0 overflow-y-auto">
        {tab === "registry" && (
          <>
            {isLoading ? (
              <p className="text-muted-foreground">Loading...</p>
            ) : filteredSkills.length > 0 ? (
              <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3 pb-4">
                {filteredSkills.map((skill: Skill) => (
                  <SkillCard
                    key={skill.name}
                    name={skill.name}
                    description={skill.description}
                    files={skill.files}
                    token_estimate={skill.token_estimate}
                    source_type={skill.source_type}
                    is_builtin={skill.is_builtin}
                    onClick={() => setDetail(skill)}
                  />
                ))}
              </div>
            ) : skills && skills.length > 0 ? (
              <p className="text-muted-foreground">
                No skills matching &ldquo;{search}&rdquo;
              </p>
            ) : (
              <p className="text-muted-foreground">No skills in registry.</p>
            )}
          </>
        )}

        {tab === "discover" && (
          <div className="space-y-4 pb-4">
            <div className="flex items-center gap-3">
              <Button
                onClick={() => runScan()}
                disabled={isScanning}
              >
                {isScanning ? (
                  <Loader2 className="h-4 w-4 animate-spin" />
                ) : (
                  <Search className="h-4 w-4" />
                )}
                {isScanning ? "Scanning..." : "Scan for Skills"}
              </Button>
              {discovered && discovered.length > 0 && delegateSelected.size > 0 && (
                <Button
                  variant="outline"
                  onClick={() => setShowDelegate(true)}
                >
                  Delegate {delegateSelected.size} Skill{delegateSelected.size !== 1 ? "s" : ""}
                </Button>
              )}
            </div>

            {discovered && discovered.length > 0 && (
              <DiscoverResults
                discovered={discovered}
                selected={delegateSelected}
                onToggle={(path) => {
                  setDelegateSelected((prev) => {
                    const next = new Set(prev)
                    if (next.has(path)) next.delete(path)
                    else next.add(path)
                    return next
                  })
                }}
                onToggleAll={(paths) => {
                  setDelegateSelected((prev) => {
                    const allSelected = paths.every((p) => prev.has(p))
                    if (allSelected) {
                      const next = new Set(prev)
                      for (const p of paths) next.delete(p)
                      return next
                    }
                    return new Set([...prev, ...paths])
                  })
                }}
              />
            )}

            {discovered && discovered.length === 0 && !isScanning && (
              <p className="text-muted-foreground">
                No undiscovered skills found. All skills from your agents are already in the registry.
              </p>
            )}
          </div>
        )}
      </div>

      {/* Delegate Dialog */}
      <Dialog open={showDelegate} onOpenChange={(o) => { if (!o) closeDelegateDialog() }}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delegate Skills to Profile</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <p className="text-sm text-muted-foreground">
              Import {delegateSelected.size} skill{delegateSelected.size !== 1 ? "s" : ""} into the registry and add to a profile.
            </p>

            {/* Mode toggle */}
            <div className="flex gap-1 rounded-lg bg-muted p-1">
              <button
                onClick={() => setDelegateMode("existing")}
                className={`flex flex-1 items-center justify-center rounded-md px-3 py-1.5 text-xs font-medium transition-colors ${
                  delegateMode === "existing"
                    ? "bg-background text-foreground shadow-sm"
                    : "text-muted-foreground hover:text-foreground"
                }`}
              >
                Add to Existing
              </button>
              <button
                onClick={() => setDelegateMode("new")}
                className={`flex flex-1 items-center justify-center rounded-md px-3 py-1.5 text-xs font-medium transition-colors ${
                  delegateMode === "new"
                    ? "bg-background text-foreground shadow-sm"
                    : "text-muted-foreground hover:text-foreground"
                }`}
              >
                Create New Profile
              </button>
            </div>

            {delegateMode === "existing" && (
              <div className="space-y-2">
                <Label>Profile</Label>
                <select
                  value={delegateExistingProfile}
                  onChange={(e) => setDelegateExistingProfile(e.target.value)}
                  className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                >
                  <option value="">Select a profile...</option>
                  {profilesData?.profiles.map((p) => (
                    <option key={p.name} value={p.name}>
                      {p.name}
                    </option>
                  ))}
                </select>
              </div>
            )}

            {delegateMode === "new" && (
              <>
                <div className="space-y-2">
                  <Label>Profile Name</Label>
                  <Input
                    value={delegateProfileName}
                    onChange={(e) => setDelegateProfileName(e.target.value)}
                    placeholder="e.g. web-dev"
                  />
                </div>
                <div className="space-y-2">
                  <Label>Description (optional)</Label>
                  <Input
                    value={delegateProfileDesc}
                    onChange={(e) => setDelegateProfileDesc(e.target.value)}
                    placeholder="Profile description"
                  />
                </div>
              </>
            )}
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={closeDelegateDialog}>
              Cancel
            </Button>
            <Button
              onClick={() => delegateMutation.mutate()}
              disabled={
                delegateMutation.isPending ||
                delegateSelected.size === 0 ||
                (delegateMode === "new" && !delegateProfileName) ||
                (delegateMode === "existing" && !delegateExistingProfile)
              }
            >
              {delegateMutation.isPending ? "Delegating..." : "Delegate"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Link to Remote Dialog */}
      <Dialog open={showLinkRemote} onOpenChange={(o) => { if (!o) closeLinkRemoteDialog() }}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Link to Remote</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <p className="text-sm text-muted-foreground">
              Link &quot;{detail?.name}&quot; to a remote Git repository for sync.
            </p>
            <div className="space-y-2">
              <Label>URL</Label>
              <Input
                value={linkUrl}
                onChange={(e) => setLinkUrl(e.target.value)}
                placeholder="https://github.com/owner/repo or owner/repo"
              />
            </div>
            <div className="space-y-2">
              <Label>Git Ref</Label>
              <Input
                value={linkRef}
                onChange={(e) => setLinkRef(e.target.value)}
                placeholder="main"
              />
            </div>
            <div className="space-y-2">
              <Label>Subpath (optional)</Label>
              <Input
                value={linkSubpath}
                onChange={(e) => setLinkSubpath(e.target.value)}
                placeholder="e.g. skills/my-skill"
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={closeLinkRemoteDialog}>
              Cancel
            </Button>
            <Button
              onClick={() => linkRemoteMutation.mutate()}
              disabled={linkRemoteMutation.isPending || !linkUrl}
            >
              {linkRemoteMutation.isPending ? "Linking..." : "Link Remote"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}

/* ------------------------------------------------------------------ */
/*  Shared SkillCard component                                        */
/* ------------------------------------------------------------------ */

function fileExts(files: string[]): string[] {
  const exts = new Set<string>()
  for (const f of files) {
    const dot = f.lastIndexOf(".")
    if (dot > 0) exts.add(f.slice(dot))
  }
  return [...exts].sort()
}

interface SkillCardProps {
  name: string
  description: string | null
  files: string[]
  token_estimate: number
  /** Registry-mode props */
  source_type?: string | null
  is_builtin?: boolean
  onClick?: () => void
  /** Discover-mode props */
  selected?: boolean
  onToggle?: () => void
  exists_in_registry?: boolean
  agent_name?: string
}

function SkillCard({
  name,
  description,
  files,
  token_estimate,
  source_type,
  is_builtin,
  onClick,
  selected,
  onToggle,
  exists_in_registry,
  agent_name,
}: SkillCardProps) {
  const isDiscover = onToggle !== undefined

  const borderClass = isDiscover
    ? selected
      ? exists_in_registry
        ? "border-amber-500 bg-amber-500/5"
        : "border-primary bg-primary/5"
      : "border-border hover:border-muted-foreground/30"
    : "hover:border-primary/30"

  return (
    <Card
      className={`animate-list-item group cursor-pointer transition-colors ${borderClass}`}
      onClick={isDiscover ? onToggle : onClick}
    >
      <CardContent className="p-5">
        <div className="flex items-start gap-3.5">
          {/* Checkbox (discover) or icon (registry) */}
          {isDiscover ? (
            <div
              className={`mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded border transition-colors ${
                selected
                  ? "border-primary bg-primary text-primary-foreground"
                  : "border-muted-foreground/30"
              }`}
            >
              {selected && <Check className="h-3.5 w-3.5" />}
            </div>
          ) : (
            <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-primary/10">
              <FileCode className="h-[18px] w-[18px] text-primary" />
            </div>
          )}

          {/* Content */}
          <div className="flex-1 min-w-0 space-y-2">
            {/* Name + overflow */}
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-1.5 min-w-0">
                <span className="text-[15px] font-semibold truncate">{name}</span>
                {exists_in_registry && (
                  <AlertTriangle className="h-3.5 w-3.5 shrink-0 text-amber-500" />
                )}
              </div>
              {!isDiscover && (
                <button
                  onClick={(e) => {
                    e.stopPropagation()
                    onClick?.()
                  }}
                  className="shrink-0 rounded p-0.5 text-muted-foreground opacity-0 transition-opacity hover:text-foreground group-hover:opacity-100"
                >
                  <MoreVertical className="h-4 w-4" />
                </button>
              )}
            </div>

            {/* Conflict warning */}
            {exists_in_registry && (
              <p className="text-[10px] font-medium text-amber-500 -mt-1">
                Already in registry
              </p>
            )}

            {/* Description */}
            <p className="line-clamp-2 text-[13px] leading-relaxed text-muted-foreground">
              {description ?? "No description"}
            </p>

            {/* Tags + file info row */}
            <div className="flex items-center gap-1.5 flex-wrap pt-0.5">
              {is_builtin && (
                <Badge variant="secondary" className="text-[10px]">
                  Built-in
                </Badge>
              )}
              {source_type && (
                <Badge variant="accent" className="text-[10px]">
                  {source_type}
                </Badge>
              )}
              {agent_name && (
                <Badge variant="outline" className="text-[10px]">
                  {agent_name}
                </Badge>
              )}
              <span className="text-[11px] text-muted-foreground">
                {files.length} file{files.length !== 1 ? "s" : ""}
              </span>
              <span className="text-[11px] text-muted-foreground">
                ~{formatTokens(token_estimate)} tokens
              </span>
              {fileExts(files).map((ext) => (
                <span
                  key={ext}
                  className="rounded bg-muted px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground"
                >
                  {ext}
                </span>
              ))}
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  )
}

/* ------------------------------------------------------------------ */
/*  DiscoverResults sub-component                                     */
/* ------------------------------------------------------------------ */

interface DiscoverResultsProps {
  discovered: DiscoveredSkill[]
  selected: Set<string>
  onToggle: (path: string) => void
  onToggleAll: (paths: string[]) => void
}

function DiscoverResults({ discovered, selected, onToggle, onToggleAll }: DiscoverResultsProps) {
  const grouped = useMemo(() => {
    const map = new Map<string, DiscoveredSkill[]>()
    for (const s of discovered) {
      const existing = map.get(s.scope) ?? []
      map.set(s.scope, [...existing, s])
    }
    return map
  }, [discovered])

  return (
    <div className="space-y-4">
      {[...grouped.entries()].map(([scope, items]) => {
        const paths = items.map((s) => s.found_path)
        const allSelected = paths.every((p) => selected.has(p))
        return (
          <div key={scope} className="space-y-2">
            <div className="flex items-center justify-between">
              <h4 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                {scope}
              </h4>
              <button
                type="button"
                className="text-xs text-primary hover:underline"
                onClick={() => onToggleAll(paths)}
              >
                {allSelected ? "Deselect All" : "Select All"}
              </button>
            </div>
            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
              {items.map((skill) => (
                <SkillCard
                  key={skill.found_path}
                  name={skill.name}
                  description={skill.description}
                  files={skill.files}
                  token_estimate={skill.token_estimate}
                  agent_name={skill.agent_name}
                  exists_in_registry={skill.exists_in_registry}
                  selected={selected.has(skill.found_path)}
                  onToggle={() => onToggle(skill.found_path)}
                />
              ))}
            </div>
          </div>
        )
      })}
    </div>
  )
}
