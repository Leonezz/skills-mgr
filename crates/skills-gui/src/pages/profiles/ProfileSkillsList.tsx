import { useState } from "react"
import { ExternalLink } from "lucide-react"
import { formatTokens } from "@/lib/format"
import type { Skill } from "@/lib/schemas"

interface Props {
  /** Skills listed directly on this profile. */
  directSkills: string[]
  /** Skills contributed by included profiles only. */
  inheritedSkills: string[]
  /** For each inherited skill, which profile it came from (best-effort). */
  inheritedFrom: Map<string, string>
  skills: Skill[] | undefined
  /** If provided, skill rows become clickable and fire this callback. */
  onSkillClick?: (name: string) => void
}

export function ProfileSkillsList({
  directSkills,
  inheritedSkills,
  inheritedFrom,
  skills,
  onSkillClick,
}: Props) {
  const [showResolved, setShowResolved] = useState(false)

  const skillLookup = new Map((skills ?? []).map((s) => [s.name, s]))

  return (
    <div className="space-y-4">
      {/* Direct skills */}
      <div className="space-y-2">
        <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
          Direct Skills ({directSkills.length})
        </p>
        {directSkills.length > 0 ? (
          <ul className="max-h-64 space-y-1 overflow-y-auto rounded-lg border border-border bg-muted/10 p-2">
            {directSkills.map((name) => {
              const meta = skillLookup.get(name)
              return (
                <SkillRow
                  key={name}
                  name={name}
                  tokens={meta?.token_estimate}
                  description={meta?.description ?? null}
                  missing={!meta}
                  onClick={onSkillClick ? () => onSkillClick(name) : undefined}
                />
              )
            })}
          </ul>
        ) : (
          <p className="text-xs text-muted-foreground">
            No direct skills on this profile.
          </p>
        )}
      </div>

      {/* Resolved (collapsible) */}
      {inheritedSkills.length > 0 && (
        <div className="space-y-2">
          <button
            type="button"
            onClick={() => setShowResolved((v) => !v)}
            className="flex w-full items-center justify-between text-xs font-semibold uppercase tracking-wide text-muted-foreground transition-colors hover:text-foreground"
          >
            <span>Inherited Skills ({inheritedSkills.length})</span>
            <span>{showResolved ? "hide" : "show"}</span>
          </button>
          {showResolved && (
            <ul className="max-h-64 space-y-1 overflow-y-auto rounded-lg border border-border bg-muted/10 p-2">
              {inheritedSkills.map((name) => {
                const meta = skillLookup.get(name)
                return (
                  <SkillRow
                    key={name}
                    name={name}
                    tokens={meta?.token_estimate}
                    description={meta?.description ?? null}
                    missing={!meta}
                    source={inheritedFrom.get(name)}
                    onClick={onSkillClick ? () => onSkillClick(name) : undefined}
                  />
                )
              })}
            </ul>
          )}
        </div>
      )}
    </div>
  )
}

function SkillRow({
  name,
  tokens,
  description,
  missing,
  source,
  onClick,
}: {
  name: string
  tokens: number | undefined
  description: string | null
  missing: boolean
  source?: string
  onClick?: () => void
}) {
  const inner = (
    <>
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="truncate text-sm font-medium">{name}</span>
          {missing && (
            <span className="text-[10px] font-semibold uppercase text-destructive">
              missing
            </span>
          )}
          {source && (
            <span className="truncate text-[10px] text-muted-foreground">
              via {source}
            </span>
          )}
        </div>
        {description && (
          <p className="truncate text-xs text-muted-foreground">
            {description}
          </p>
        )}
      </div>
      {tokens !== undefined && (
        <span className="shrink-0 text-xs tabular-nums text-muted-foreground">
          ~{formatTokens(tokens)}
        </span>
      )}
      {onClick && !missing && (
        <ExternalLink className="h-3.5 w-3.5 shrink-0 text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100" />
      )}
    </>
  )

  if (onClick && !missing) {
    return (
      <li>
        <button
          type="button"
          onClick={onClick}
          className="group flex w-full items-start gap-3 rounded px-2 py-1.5 text-left transition-colors hover:bg-muted/60"
        >
          {inner}
        </button>
      </li>
    )
  }

  return (
    <li className="flex items-start gap-3 rounded px-2 py-1.5">{inner}</li>
  )
}
