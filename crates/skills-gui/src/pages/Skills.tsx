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
import { listSkills, createSkill, removeSkill, importSkill, importRemoteSkill, updateSkill } from "@/lib/api"
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
} from "lucide-react"
import type { Skill } from "@/lib/schemas"

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

  // Add form state
  const [addMode, setAddMode] = useState<"create" | "local" | "remote">("create")
  const [newName, setNewName] = useState("")
  const [newDesc, setNewDesc] = useState("")
  const [newSourcePath, setNewSourcePath] = useState("")
  const [remoteUrl, setRemoteUrl] = useState("")

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

  const createMutation = useMutation({
    mutationFn: () => {
      // Auto-detect: if user typed a URL/shorthand in create mode, redirect to remote import
      if (addMode === "create" && isUrlLike(newName)) {
        return importRemoteSkill(newName)
      }
      if (addMode === "remote") {
        return importRemoteSkill(remoteUrl)
      }
      if (addMode === "local") {
        return importSkill(newSourcePath)
      }
      // Validate skill name doesn't contain path separators
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
  }

  async function handleBrowseSource() {
    const selected = await open({
      directory: true,
      title: "Select skill folder",
    })
    if (selected) setNewSourcePath(selected as string)
  }

  function fileExtensions(files: string[]): string[] {
    const exts = new Set<string>()
    for (const f of files) {
      const dot = f.lastIndexOf(".")
      if (dot > 0) exts.add(f.slice(dot))
    }
    return [...exts].sort()
  }

  return (
    <div className="space-y-6">
      {/* Header */}
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

      {/* Add Skill Dialog */}
      <Dialog open={showAdd} onOpenChange={(o) => { if (!o) closeAdd() }}>
        <DialogContent>
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
              <div className="space-y-2">
                <Label>GitHub URL or Shorthand</Label>
                <Input
                  value={remoteUrl}
                  onChange={(e) => setRemoteUrl(e.target.value)}
                  placeholder="owner/repo/path/to/skill"
                />
                <p className="text-xs text-muted-foreground">
                  Supported formats:
                </p>
                <ul className="space-y-0.5 text-xs text-muted-foreground list-disc pl-4">
                  <li>https://github.com/owner/repo/tree/main/path</li>
                  <li>owner/repo (imports entire repo)</li>
                  <li>owner/repo/path/to/skill (imports subdirectory)</li>
                </ul>
              </div>
            )}
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={closeAdd}>
              Cancel
            </Button>
            <Button
              onClick={() => createMutation.mutate()}
              disabled={
                createMutation.isPending ||
                (addMode === "create" && !newName) ||
                (addMode === "local" && !newSourcePath) ||
                (addMode === "remote" && !remoteUrl)
              }
            >
              {createMutation.isPending
                ? addMode === "remote" ? "Downloading..." : "Adding..."
                : addMode === "remote" ? "Import from GitHub"
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
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Source</span>
                <span>{detail?.source_type ?? "Local file"}</span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Files</span>
                <span>{detail?.files.length ?? 0} files</span>
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

      {/* Skill Cards Grid */}
      {isLoading ? (
        <p className="text-muted-foreground">Loading...</p>
      ) : filteredSkills.length > 0 ? (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {filteredSkills.map((skill: Skill, index: number) => (
            <Card
              key={skill.name}
              className="animate-list-item group cursor-pointer transition-colors hover:border-primary/30"
              style={{ animationDelay: `${index * 40}ms` }}
              onClick={() => setDetail(skill)}
            >
              <CardContent className="p-5">
                <div className="flex items-start gap-3.5">
                  {/* Icon */}
                  <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-primary/10">
                    <FileCode className="h-[18px] w-[18px] text-primary" />
                  </div>

                  {/* Content */}
                  <div className="flex-1 min-w-0 space-y-2">
                    {/* Name + overflow */}
                    <div className="flex items-center justify-between">
                      <span className="text-[15px] font-semibold truncate">
                        {skill.name}
                      </span>
                      <button
                        onClick={(e) => {
                          e.stopPropagation()
                          setDetail(skill)
                        }}
                        className="shrink-0 rounded p-0.5 text-muted-foreground opacity-0 transition-opacity hover:text-foreground group-hover:opacity-100"
                      >
                        <MoreVertical className="h-4 w-4" />
                      </button>
                    </div>

                    {/* Description */}
                    <p className="line-clamp-2 text-[13px] leading-relaxed text-muted-foreground">
                      {skill.description ?? "No description"}
                    </p>

                    {/* Tags + file info row */}
                    <div className="flex items-center gap-1.5 flex-wrap pt-0.5">
                      {skill.is_builtin && (
                        <Badge variant="secondary" className="text-[10px]">
                          Built-in
                        </Badge>
                      )}
                      {skill.source_type && (
                        <Badge variant="accent" className="text-[10px]">
                          {skill.source_type}
                        </Badge>
                      )}
                      <span className="text-[11px] text-muted-foreground">
                        {skill.files.length} file{skill.files.length !== 1 ? "s" : ""}
                      </span>
                      {fileExtensions(skill.files).map((ext) => (
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
          ))}
        </div>
      ) : skills && skills.length > 0 ? (
        <p className="text-muted-foreground">
          No skills matching &ldquo;{search}&rdquo;
        </p>
      ) : (
        <p className="text-muted-foreground">No skills in registry.</p>
      )}
    </div>
  )
}
