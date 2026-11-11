---
ridgeline: patch
---

#### Azure DevOps PRs assigned to your teams show up

Reviewing only fetched PRs where you were a reviewer by name, so PRs that
Azure lists under "Assigned to my teams" never appeared, and the "Via team"
tab filter had nothing to show. Ridgeline now looks up your teams in each
organization and fetches the PRs where any of them is a reviewer.

These PRs land in Reviewing alongside your direct ones. To keep them apart,
set Reviewing's Assigned filter to "To me" under Settings → Tabs and add a
tab with it set to "Via team".

If your token can't list teams (it needs the Project and Team read scope),
Ridgeline logs it and carries on with your direct reviews.
