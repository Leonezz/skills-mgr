import { formatTokens } from "@/lib/format"
import type { ProfileResolution } from "./useProfileResolution"

interface Props {
  resolution: ProfileResolution
}

export function ProfileCompositionBlock({ resolution }: Props) {
  const { directSkills, inheritedSkills, tokens } = resolution
  const total = directSkills.length + inheritedSkills.length

  return (
    <div className="space-y-3">
      <div className="flex items-baseline justify-between">
        <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
          Composition
        </p>
        <span className="text-xs text-muted-foreground">
          ~{formatTokens(tokens.total)} tokens total
        </span>
      </div>
      <div className="grid grid-cols-2 gap-3">
        <CompositionRow
          label="Direct"
          count={directSkills.length}
          tokens={tokens.direct}
          accent="primary"
        />
        <CompositionRow
          label="Inherited"
          count={inheritedSkills.length}
          tokens={tokens.inherited}
          accent="muted"
        />
      </div>
      {total === 0 && (
        <p className="text-xs text-muted-foreground">
          No skills resolved — add skills directly or compose from another profile.
        </p>
      )}
    </div>
  )
}

function CompositionRow({
  label,
  count,
  tokens,
  accent,
}: {
  label: string
  count: number
  tokens: number
  accent: "primary" | "muted"
}) {
  const dotClass =
    accent === "primary" ? "bg-primary" : "bg-muted-foreground/40"
  return (
    <div className="rounded-lg border border-border bg-muted/20 p-3">
      <div className="flex items-center gap-2">
        <span className={`h-2 w-2 rounded-full ${dotClass}`} />
        <span className="text-xs font-medium text-muted-foreground">{label}</span>
      </div>
      <p className="mt-1.5 text-lg font-semibold tabular-nums">{count}</p>
      <p className="text-xs text-muted-foreground">
        ~{formatTokens(tokens)} tokens
      </p>
    </div>
  )
}
