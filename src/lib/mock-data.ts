import type { PollResult, PullRequest, Reviewer, Vote } from "./types";

function reviewer(name: string, vote: Vote, required = false): Reviewer {
  return {
    user: { id: name.toLowerCase().replace(/\s/g, "."), displayName: name, uniqueName: null },
    vote,
    isRequired: required,
  };
}

function pr(opts: {
  num: number;
  title: string;
  author: string;
  project: string;
  repo: string;
  source: string;
  target?: string;
  provider?: string;
  reviewers?: Reviewer[];
  isDraft?: boolean;
  createdHoursAgo: number;
  mergeStatus?: PullRequest["mergeStatus"];
  buildStatus?: PullRequest["buildStatus"];
}): PullRequest {
  const provider = opts.provider ?? "contoso";
  return {
    id: { provider, project: opts.project, repository: opts.repo, number: opts.num },
    title: opts.title,
    description: null,
    status: "Active",
    isDraft: opts.isDraft ?? false,
    createdAt: new Date(Date.now() - opts.createdHoursAgo * 3_600_000).toISOString(),
    author: { id: opts.author.toLowerCase().replace(/\s/g, "."), displayName: opts.author, uniqueName: null },
    sourceBranch: opts.source,
    targetBranch: opts.target ?? "main",
    reviewers: opts.reviewers ?? [],
    repository: { name: opts.repo, project: opts.project },
    labels: [],
    mergeStatus: opts.mergeStatus ?? null,
    buildStatus: opts.buildStatus ?? null,
    sourceCommitId: "abc123",
    webUrl: "#",
  };
}

export function getMockData(): PollResult {
  return {
    reviewing: [
      pr({
        num: 4821,
        title: "Add role-based access control to API endpoints",
        author: "Emma Lindqvist",
        project: "Platform",
        repo: "api-gateway",
        source: "feature/rbac-endpoints",
        provider: "contoso",
        createdHoursAgo: 4,
        reviewers: [
          reviewer("You", "NoVote"),
          reviewer("Marcus Chen", "Approved", true),
          reviewer("Sara Johansson", "NoVote"),
        ],
      }),
      pr({
        num: 1337,
        title: "Migrate user service to event-driven architecture",
        author: "Oliver Virtanen",
        project: "Microservices",
        repo: "user-service",
        source: "refactor/event-driven",
        provider: "contoso",
        createdHoursAgo: 52,
        reviewers: [
          reviewer("You", "NoVote"),
          reviewer("Emma Lindqvist", "ApprovedWithSuggestions", true),
        ],
      }),
      pr({
        num: 892,
        title: "Fix timezone handling in scheduling module",
        author: "Amira Patel",
        project: "Scheduling",
        repo: "scheduler-core",
        source: "fix/timezone-utc-offset",
        provider: "fabrikam",
        createdHoursAgo: 18,
        reviewers: [
          reviewer("You", "NoVote"),
          reviewer("Jonas Berg", "Approved"),
          reviewer("Li Wei", "Approved", true),
        ],
      }),
      pr({
        num: 2104,
        title: "Add dark mode support to customer portal",
        author: "Marcus Chen",
        project: "Portal",
        repo: "customer-portal",
        source: "feature/dark-mode",
        provider: "contoso",
        createdHoursAgo: 160,
        reviewers: [
          reviewer("You", "NoVote"),
          reviewer("Amira Patel", "WaitingForAuthor"),
        ],
      }),
      pr({
        num: 567,
        title: "Optimize database queries for reporting dashboard",
        author: "Sara Johansson",
        project: "Analytics",
        repo: "reporting-api",
        source: "perf/query-optimization",
        provider: "fabrikam",
        createdHoursAgo: 8,
        reviewers: [
          reviewer("You", "NoVote"),
          reviewer("Oliver Virtanen", "Approved", true),
          reviewer("Emma Lindqvist", "Approved"),
          reviewer("Jonas Berg", "NoVote"),
        ],
      }),
      pr({
        num: 3045,
        title: "Implement webhook retry mechanism with exponential backoff",
        author: "Li Wei",
        project: "Platform",
        repo: "webhook-service",
        source: "feature/retry-backoff",
        provider: "contoso",
        createdHoursAgo: 26,
        reviewers: [
          reviewer("You", "NoVote"),
          reviewer("Marcus Chen", "Rejected"),
        ],
        mergeStatus: "Conflicts",
      }),
    ],
    authored: [
      pr({
        num: 4455,
        title: "Implement SSO integration for enterprise customers",
        author: "You",
        project: "Platform",
        repo: "auth-service",
        source: "feature/enterprise-sso",
        provider: "contoso",
        createdHoursAgo: 72,
        reviewers: [
          reviewer("Emma Lindqvist", "Approved", true),
          reviewer("Marcus Chen", "Approved"),
          reviewer("Amira Patel", "ApprovedWithSuggestions"),
        ],
      }),
      pr({
        num: 4501,
        title: "Add OpenTelemetry tracing to payment flow",
        author: "You",
        project: "Payments",
        repo: "payment-processor",
        source: "feature/otel-tracing",
        provider: "contoso",
        createdHoursAgo: 6,
        reviewers: [
          reviewer("Oliver Virtanen", "NoVote", true),
          reviewer("Sara Johansson", "NoVote"),
        ],
        buildStatus: "InProgress",
      }),
      pr({
        num: 234,
        title: "Fix memory leak in connection pool manager",
        author: "You",
        project: "Infrastructure",
        repo: "connection-pool",
        source: "fix/pool-memory-leak",
        provider: "fabrikam",
        createdHoursAgo: 28,
        reviewers: [
          reviewer("Li Wei", "WaitingForAuthor", true),
          reviewer("Jonas Berg", "NoVote"),
        ],
      }),
      pr({
        num: 4520,
        title: "Update API documentation for v3 endpoints",
        author: "You",
        project: "Platform",
        repo: "api-gateway",
        source: "docs/v3-api-docs",
        provider: "contoso",
        createdHoursAgo: 2,
        reviewers: [],
        isDraft: true,
      }),
      pr({
        num: 780,
        title: "Migrate CI pipeline to GitHub Actions",
        author: "You",
        project: "DevOps",
        repo: "build-infra",
        source: "chore/gh-actions-migration",
        provider: "fabrikam",
        createdHoursAgo: 150,
        reviewers: [
          reviewer("Emma Lindqvist", "Approved", true),
          reviewer("Amira Patel", "Approved"),
        ],
        buildStatus: "Failed",
        mergeStatus: "RejectedByPolicy",
      }),
    ],
    errors: [],
  };
}
