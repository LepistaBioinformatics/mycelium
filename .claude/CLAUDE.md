# Mycelium — Claude Code Instructions

## Skill: tlc-spec-driven

When running the `tlc-spec-driven` skill, use `.claude/specs/` as the root specs directory
**in place of** the default `.specs/` path referenced throughout the skill.

All spec documents live under:

```
.claude/specs/
├── project/        # PROJECT.md, ROADMAP.md, STATE.md
├── codebase/       # Brownfield analysis docs (STACK, ARCHITECTURE, CONVENTIONS, …)
├── features/       # Feature specs
└── quick/          # Quick-mode tasks
```

Whenever the skill instructs reading from or writing to `.specs/`, substitute `.claude/specs/` instead.
