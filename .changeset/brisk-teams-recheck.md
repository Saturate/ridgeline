---
ridgeline: patch
---

#### Team changes show up without a restart

Ridgeline rechecks which Azure DevOps teams you're in every 15 minutes, so
joining a team or giving your token Project and Team read access shows up
on its own. Requests to Azure DevOps now time out after 30 seconds and
are retried, instead of waiting forever and stalling refreshes.
