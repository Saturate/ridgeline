export interface PrId {
  provider: string;
  project: string;
  repository: string;
  number: number;
}

export interface UserId {
  provider: string;
  id: string;
  displayName: string;
}

export interface PullRequest {
  id: PrId;
  title: string;
  description: string | null;
  status: PrStatus;
  isDraft: boolean;
  createdAt: string;
  author: User;
  sourceBranch: string;
  targetBranch: string;
  reviewers: Reviewer[];
  repository: Repository;
  labels: string[];
  mergeStatus: MergeStatus | null;
  buildStatus: BuildStatus | null;
  sourceCommitId: string | null;
  webUrl: string;
}

export type PrStatus = "Active" | "Completed" | "Abandoned";

export interface User {
  id: string;
  displayName: string;
  uniqueName: string | null;
}

export interface Reviewer {
  user: User;
  vote: Vote;
  isRequired: boolean;
}

export type Vote =
  | "Approved"
  | "ApprovedWithSuggestions"
  | "NoVote"
  | "WaitingForAuthor"
  | "Rejected";

export interface Repository {
  name: string;
  project: string;
}

export type MergeStatus =
  | "Succeeded"
  | "Conflicts"
  | "RejectedByPolicy"
  | "NotSet"
  | "Queued";

export interface DiffStats {
  filesChanged: number;
  additions: number;
  deletions: number;
}

export type BuildStatus = "Succeeded" | "Failed" | "InProgress" | "NotStarted";

export interface PolicyStatus {
  name: string;
  isBlocking: boolean;
  status: PolicyEvaluation;
}

export type PolicyEvaluation =
  | "Approved"
  | "Rejected"
  | "Running"
  | "Queued"
  | "NotApplicable";

export interface PrDetail {
  pr: PullRequest;
  diffStats: DiffStats | null;
  buildStatus: BuildStatus | null;
  policies: PolicyStatus[];
}

export interface PollResult {
  reviewing: PullRequest[];
  authored: PullRequest[];
  errors: PollError[];
}

export type PollErrorKind = "network" | "auth" | "server" | "parse" | "unknown";

export interface PollError {
  provider: string;
  kind: PollErrorKind;
  message: string;
}

export interface Config {
  general: GeneralConfig;
  providers: ProviderConfig[];
}

export type ProviderIndicator = "off" | "border" | "badge";

export interface GeneralConfig {
  refresh_interval_secs: number;
  stale_threshold_hours: number;
  notifications_enabled: boolean;
  notifications: NotificationConfig;
  provider_indicator: ProviderIndicator;
  age_warning_hours: number;
  age_danger_hours: number;
  show_project_name: boolean;
  parse_conventional_commits: boolean;
  tabs: TabConfig[];
}

export type TabSource = "reviewing" | "authored" | "all";
export type TabDisplay = "reviewing" | "authored";

export interface TabConfig {
  label: string;
  source: TabSource;
  display: TabDisplay;
  enabled: boolean;
  filter: TabFilter;
}

export interface TabFilter {
  max_reviewers: number | null;
}

export interface NotificationConfig {
  new_pr: boolean;
  vote_changed: boolean;
  waiting_for_author: boolean;
  build_failed: boolean;
  completed: boolean;
}

export type ProviderConfig = {
  type: "azure-devops";
} & AzureDevOpsConfig;

export interface AzureDevOpsConfig {
  name: string;
  url: string;
  color?: string;
  projects: ProjectFilter[];
}

export interface ProjectFilter {
  name: string;
  repos: string[];
}

export type Change =
  | { type: "newPr"; title: string; author: string; repo: string }
  | {
      type: "voteChanged";
      prTitle: string;
      reviewer: string;
      newVote: Vote;
    }
  | { type: "completed"; title: string; repo: string };
