import { useState } from "react"
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Switch } from "@/components/ui/switch"
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog"
import { listAgents, addAgent, editAgent, removeAgent, toggleAgent } from "@/lib/api"
import { toast } from "sonner"
import { open } from "@tauri-apps/plugin-dialog"
import { Plus, MoreVertical, Bot, FolderOpen } from "lucide-react"
import type { Agent } from "@/lib/schemas"

// Cycle through a palette for agent icons
const AGENT_COLORS = [
  { icon: "#6366F1", bg: "#6366F11A" }, // indigo
  { icon: "#32D583", bg: "#32D58320" }, // green
  { icon: "#E85A4F", bg: "#E85A4F20" }, // red
  { icon: "#F59E0B", bg: "#F59E0B20" }, // amber
  { icon: "#3B82F6", bg: "#3B82F620" }, // blue
]

function getAgentColor(index: number) {
  return AGENT_COLORS[index % AGENT_COLORS.length]
}

function PathInput({
  value,
  onChange,
  placeholder,
}: {
  value: string
  onChange: (v: string) => void
  placeholder?: string
}) {
  async function handleBrowse() {
    const selected = await open({ directory: true, title: "Select folder" })
    if (selected) onChange(selected as string)
  }

  return (
    <div className="flex gap-2">
      <Input
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className="flex-1"
      />
      <Button type="button" variant="outline" size="sm" onClick={handleBrowse}>
        <FolderOpen className="h-4 w-4" />
      </Button>
    </div>
  )
}

export function Agents() {
  const queryClient = useQueryClient()
  const { data: agents, isLoading } = useQuery({
    queryKey: ["agents"],
    queryFn: listAgents,
  })

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
      closeAdd()
    },
    onError: (err) => toast.error(String(err)),
  })

  const editMutation = useMutation({
    mutationFn: () =>
      editAgent(showEdit!.name, editProjectPath, editGlobalPath),
    onSuccess: (msg) => {
      toast.success(msg)
      queryClient.invalidateQueries({ queryKey: ["agents"] })
      closeEdit()
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

  const toggleMutation = useMutation({
    mutationFn: ({ name, enabled }: { name: string; enabled: boolean }) =>
      toggleAgent(name, enabled),
    onSuccess: (msg) => {
      toast.success(msg)
      queryClient.invalidateQueries({ queryKey: ["agents"] })
    },
    onError: (err) => toast.error(String(err)),
  })

  function closeAdd() {
    setShowAdd(false)
    setNewName("")
    setNewProjectPath("")
    setNewGlobalPath("")
  }

  function openEdit(agent: Agent) {
    setEditProjectPath(agent.project_path)
    setEditGlobalPath(agent.global_path)
    setShowEdit(agent)
  }

  function closeEdit() {
    setShowEdit(null)
    setEditProjectPath("")
    setEditGlobalPath("")
  }

  return (
    <div className="flex flex-col flex-1 min-h-0">
      {/* Header — fixed */}
      <div className="shrink-0 flex items-center justify-between pb-6">
        <div>
          <h2 className="text-2xl font-bold tracking-tight">Agent Tools</h2>
          <p className="text-sm text-muted-foreground">
            Define where skills are placed for each AI agent tool
          </p>
        </div>
        <Button onClick={() => setShowAdd(true)}>
          <Plus className="h-4 w-4" />
          Add Agent
        </Button>
      </div>

      {/* Add Agent Dialog */}
      <Dialog open={showAdd} onOpenChange={(o) => { if (!o) closeAdd() }}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Add Agent</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-2">
              <Label>Agent Name</Label>
              <Input
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                placeholder="e.g. Claude Code"
              />
            </div>
            <div className="space-y-2">
              <Label>Relative Path (in project)</Label>
              <p className="text-xs text-muted-foreground">
                Relative to project root, e.g. .claude/skills/
              </p>
              <Input
                value={newProjectPath}
                onChange={(e) => setNewProjectPath(e.target.value)}
                placeholder=".claude/skills/"
              />
            </div>
            <div className="space-y-2">
              <Label>Global Path</Label>
              <p className="text-xs text-muted-foreground">
                Absolute path for global skill placement
              </p>
              <PathInput
                value={newGlobalPath}
                onChange={setNewGlobalPath}
                placeholder="~/.claude/skills/"
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={closeAdd}>
              Cancel
            </Button>
            <Button
              onClick={() => addMutation.mutate()}
              disabled={
                !newName || !newProjectPath || !newGlobalPath || addMutation.isPending
              }
            >
              {addMutation.isPending ? "Adding..." : "Add Agent"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Edit Agent Dialog */}
      <Dialog open={showEdit !== null} onOpenChange={(o) => { if (!o) closeEdit() }}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Edit Agent</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-2">
              <Label>Agent Name</Label>
              <Input value={showEdit?.name ?? ""} disabled />
            </div>
            <div className="space-y-2">
              <Label>Relative Path (in project)</Label>
              <p className="text-xs text-muted-foreground">
                Relative to project root, e.g. .claude/skills/
              </p>
              <Input
                value={editProjectPath}
                onChange={(e) => setEditProjectPath(e.target.value)}
                placeholder=".claude/skills/"
              />
            </div>
            <div className="space-y-2">
              <Label>Global Path</Label>
              <p className="text-xs text-muted-foreground">
                Absolute path for global skill placement
              </p>
              <PathInput
                value={editGlobalPath}
                onChange={setEditGlobalPath}
                placeholder="~/.claude/skills/"
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
              Delete Agent
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
            <h2 className="text-lg font-semibold">Remove Agent?</h2>
          </div>
          <p className="text-sm leading-relaxed text-muted-foreground">
            Are you sure you want to remove &quot;{showDelete}&quot;? This action cannot
            be undone. All agent configuration will be lost.
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
              {deleteMutation.isPending ? "Removing..." : "Remove"}
            </Button>
          </div>
        </DialogContent>
      </Dialog>

      {/* Agent List — scrollable */}
      <div className="flex-1 min-h-0 overflow-y-auto">
      {isLoading ? (
        <p className="text-muted-foreground">Loading...</p>
      ) : agents && agents.length > 0 ? (
        <div className="space-y-3 pb-4">
          {agents.map((agent: Agent, index: number) => {
            const color = getAgentColor(index)
            return (
              <div
                key={agent.name}
                className="animate-list-item group flex items-center gap-4 rounded-xl border border-border bg-card p-5 transition-colors hover:border-primary/30"
                style={{ animationDelay: `${index * 50}ms` }}
              >
                {/* Colored bot icon */}
                <div
                  className="flex h-10 w-10 shrink-0 items-center justify-center rounded-[10px]"
                  style={{ backgroundColor: color.bg }}
                >
                  <Bot className="h-5 w-5" style={{ color: color.icon }} />
                </div>

                {/* Info */}
                <div className="flex-1 space-y-1.5 min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="text-[15px] font-semibold">{agent.name}</span>
                    <span className={`h-2 w-2 rounded-full ${agent.enabled ? "bg-emerald-500" : "bg-muted-foreground/40"}`} />
                  </div>
                  <div className="flex items-center gap-6 text-xs text-muted-foreground">
                    <span>In project: {agent.project_path}</span>
                    <span>Global: {agent.global_path}</span>
                  </div>
                </div>

                {/* Toggle — global enable/disable */}
                <Switch
                  checked={agent.enabled}
                  onCheckedChange={(checked) => {
                    toggleMutation.mutate({ name: agent.name, enabled: checked })
                  }}
                />

                {/* Overflow menu */}
                <button
                  onClick={() => openEdit(agent)}
                  className="rounded p-1 text-muted-foreground transition-colors hover:text-foreground"
                >
                  <MoreVertical className="h-4 w-4" />
                </button>
              </div>
            )
          })}
        </div>
      ) : (
        <p className="text-muted-foreground">No agents configured.</p>
      )}
      </div>
    </div>
  )
}
