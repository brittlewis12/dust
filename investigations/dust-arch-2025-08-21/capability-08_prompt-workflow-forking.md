Title: Prompt/Workflow Forking

Scope
- Create new assistants/apps by duplicating existing configurations or restoring prior versions.

Assistants (Front)
- Duplicate via UI query param `duplicate` in builder routes: `front/pages/w/[wId]/builder/assistants/new.tsx` and agents counterpart.
- Versioning: per-assistant version, author, timestamps; lists/history endpoints in API and poke tools.

Dust Spec Apps (Core)
- Forks correspond to new app hashes derived from altered block sequences/configs.
- Admin/poke views allow restoring old versions to re-run or compare.

Operational Notes
- Assistant duplication is explicitly supported in product UI; Dust Spec forks follow standard Core version semantics.

