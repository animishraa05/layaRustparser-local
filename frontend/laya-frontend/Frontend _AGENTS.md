# AGENTS.md

## Project: ULPF Analyst Frontend

This document contains the complete instructions for implementing **Issue #13** of the ULPF project.

---

# 1. PRIMARY GOAL

Implement the frontend for:

**[frontend/M] Analyst dashboard (live EPS, alerts, mix) #13**

The goal is to build a functional analyst-facing security/telemetry dashboard that connects the existing Next.js frontend to the existing ULPF Rust backend.

This is NOT a static UI-only task.

The implementation must cover:

**Design → Frontend → Components → API integration → Mock/fixture mode → Live backend mode → Loading/Error/Empty states → Testing**

The final result should be a usable analyst dashboard for the demonstration.

---

# 2. CURRENT TECHNOLOGY STACK

The frontend application has ALREADY been initialized.

Do NOT initialize another Next.js project.

Do NOT run `create-next-app` again.

Current stack:

* Next.js
* React
* TypeScript
* Tailwind CSS
* shadcn/ui
* App Router
* ESLint

The frontend application is located at:

```text
frontend/laya-frontend/
```

The existing Next.js setup is the source of truth.

Before making changes:

1. Inspect `package.json`.
2. Inspect the existing `src/` structure.
3. Inspect `components.json`.
4. Inspect `tailwind` configuration.
5. Inspect existing global CSS.
6. Inspect existing layout files.

Reuse the existing setup.

Do not replace or recreate it unless there is a genuine technical requirement.

---

# 3. IMPORTANT DIRECTORY STRUCTURE

The repository has this general structure:

```text
layaRustparser/
│
├── AGENTS.md
│
├── crates/
│   └── ulpf-cli/
│       └── src/
│           └── serve/
│               ├── handlers.rs
│               ├── mod.rs
│               └── state.rs
│
├── data/
│   └── fixtures/
│       └── api/
│           ├── alerts.json
│           ├── blocks.json
│           ├── metrics.json
│           ├── onboard_preview.json
│           ├── parsers.json
│           ├── parsers_test.json
│           ├── prove_501.json
│           ├── prove_live.json
│           ├── records_block_1.json
│           └── system.json
│
├── docs/
│   ├── CONTRACTS.md
│   └── openapi.yaml
│
└── frontend/
    └── laya-frontend/
        ├── design_page1.md
        ├── design_page2.md
        ├── design_page4.md
        ├── src/
        ├── public/
        ├── package.json
        ├── components.json
        ├── tsconfig.json
        └── ...
```

---

# 4. FRONTEND LOCATION

ALL Issue #13 frontend code must live inside:

```text
frontend/laya-frontend/
```

Do not create another frontend application.

Do NOT create:

```text
frontend-2/
frontend-app/
web/
dashboard/
ui/
next-app/
```

outside the existing application.

The three pages in this issue must belong to the SAME Next.js application.

---

# 5. DESIGN FILES

Three design specification files have already been placed inside the frontend:

```text
frontend/laya-frontend/design_page1.md
frontend/laya-frontend/design_page2.md
frontend/laya-frontend/design_page4.md
```

These files describe the intended UI.

They are extremely important.

Before implementing the pages, READ all three files.

Do not guess the design.

Use the design files to understand:

* page layout
* navigation
* theme
* typography
* fonts
* colors
* spacing
* cards
* borders
* shadows
* tables
* charts
* badges
* icons
* status indicators
* responsive behavior
* visual hierarchy
* component placement
* page relationships

The design files are the primary visual reference for Issue #13.

Do not replace the design with a generic admin dashboard template.

---

# 6. ISSUE #13 PAGE SCOPE

Issue #13 contains exactly THREE frontend pages.

Do not implement Issue #14 or #15.

---

## PAGE 1 — ANALYST DASHBOARD & LIVE OVERVIEW

Page purpose:

Provide the analyst with an immediate overview of system activity and security telemetry.

The page contains:

### KPI Telemetry Cards

Display the important live telemetry metrics from the backend.

Examples include:

* EPS / events per second
* latency
* ingestion/system status
* relevant security/processing counters

Do not invent metrics.

Use the metrics exposed by the backend contracts.

---

### Live SVG Performance Chart

Display the live performance/telemetry trend.

The chart should update from backend data.

Do not create a fake random-number generator and label the output as live data.

If the backend provides time-series points, use those.

If the backend only exposes a current metric, design the frontend so that polling creates a time-series history from actual responses.

Clearly handle:

* initial loading
* active live state
* backend unavailable
* empty data

The chart should visually follow `design_page1.md`.

---

### Alert Feed

Display security/system alerts.

The alert feed must support the alert categories required by Issue #13, including:

* Drain new-template drift
* Rare-cluster surge
* Tamper alarms

Use severity indicators consistent with the design.

Do not hardcode alerts as permanent production data.

Mock alerts may be used in fixture mode.

---

### Real-Time OCSF Stream Table

Display incoming/available OCSF records in a table.

Use the backend API and contracts to determine:

* available fields
* record structure
* identifiers
* timestamps
* vendor information
* disposition
* relevant security fields

Do not invent API fields.

---

# 7. PAGE 2 — THREAT INTELLIGENCE & VENDOR MIX

Page purpose:

Give the analyst visibility into vendor ingestion, action disposition, and IOC-correlated attack activity.

The page contains:

---

## Multi-Vendor Ingestion Stream Mix

Show the distribution of incoming records/events across vendors.

The UI must visually match:

```text
design_page2.md
```

Use actual backend data when available.

If fixture mode is active, use:

```text
data/fixtures/api/
```

Do not fabricate real backend results.

---

## Action Disposition Split

Display the action/disposition breakdown:

```text
Allowed
Blocked
Dropped
```

Use the backend data/contract to determine the actual counts.

The UI should clearly communicate the proportions and/or counts.

---

## Live IOC-Correlated Attack Vectors Table

Display the attack-vector/IOC information available from the backend.

The table should support the fields defined by the backend contract.

Do not create unsupported fields merely to make the table look populated.

Handle:

* no results
* loading
* API error
* backend unavailable

---

# 8. PAGE 4 — SIEM ALERTING & INCIDENT TRIAGE

Page purpose:

Provide the analyst with alert triage and forensic inspection capabilities.

The page contains:

---

## Critical / Medium Incident Queue

Display the incident/alert queue.

At minimum, support the severity levels required by the design and backend.

The design should clearly distinguish:

* Critical
* Medium

Do not use arbitrary severity levels unless they exist in the backend/design.

---

## Triage Indicators

Display useful triage metadata available from the backend.

Examples may include:

* severity
* status
* source
* vendor
* disposition
* timestamps
* identifiers

Use the actual API contract as the authority.

---

## Live Forensic Workspace Inspection Panel

When an alert/record is selected, display the available forensic information.

Use backend-supported data.

The panel must not pretend to have information that the API does not provide.

Handle:

* no selected record
* selected record
* loading
* failed inspection
* unavailable backend

---

# 9. BACKEND/API INFORMATION

The backend has already been implemented upstream.

The frontend MUST use the existing backend rather than creating a new backend.

Backend source:

```text
crates/ulpf-cli/src/serve/
```

Important files include:

```text
crates/ulpf-cli/src/serve/handlers.rs
crates/ulpf-cli/src/serve/mod.rs
crates/ulpf-cli/src/serve/state.rs
```

---

# 10. API CONTRACT — READ THIS FIRST

Before implementing API calls, read:

```text
docs/CONTRACTS.md
```

and:

```text
docs/openapi.yaml
```

These documents describe the backend API.

Also inspect:

```text
crates/ulpf-cli/src/serve/
```

to understand implementation behavior.

Do not infer endpoint names or response schemas from the visual design.

The API contract is authoritative for:

* endpoint paths
* HTTP methods
* request bodies
* query parameters
* response structures
* error behavior
* status codes
* available data

---

# 11. API FIXTURES

Mock API responses already exist here:

```text
data/fixtures/api/
```

Relevant files include:

```text
metrics.json
alerts.json
blocks.json
records_block_1.json
system.json
```

Inspect these fixtures before implementing mock mode.

The fixtures represent the backend's expected API data structures.

Do not invent a completely different mock schema.

---

# 12. WHERE FRONTEND API CODE GOES

API communication belongs in:

```text
frontend/laya-frontend/src/lib/api.ts
```

Create this file if it does not already exist.

Do NOT scatter raw `fetch()` calls throughout every component.

Bad:

```text
Dashboard → fetch()
AlertFeed → fetch()
VendorMix → fetch()
Chart → fetch()
```

Prefer:

```text
                 ┌── Analyst Dashboard
                 │
                 ├── Alert Feed
Frontend ────────┤
                 ├── Vendor Mix
                 │
                 └── SIEM
                       │
                       ▼
                 src/lib/api.ts
                       │
                       ▼
                  ULPF backend
```

The API layer should provide typed functions for the required backend operations.

For example, conceptually:

```ts
getMetrics()
getAlerts()
getRecords()
```

Use the ACTUAL endpoint names and response types from:

```text
docs/CONTRACTS.md
docs/openapi.yaml
```

Do not blindly copy these example function names if the backend contract uses different operations.

---

# 13. FRONTEND TYPES

Create:

```text
src/lib/types.ts
```

if needed.

Define TypeScript types for backend responses.

Avoid:

```ts
any
```

unless genuinely unavoidable.

The frontend should have a clear boundary between:

```text
Backend response
      ↓
Typed API client
      ↓
Frontend data model
      ↓
UI components
```

---

# 14. MOCK DATA

Create:

```text
src/lib/mock-data.ts
```

if needed.

Mock mode must be based on the repository fixtures.

Do not generate random fake security data.

Mock mode exists so the UI can be demonstrated without a running backend.

The frontend should make it clear whether it is displaying:

```text
LIVE
```

or

```text
MOCK / FIXTURE
```

according to the design.

---

# 15. REAL-TIME METRICS

Issue #13 specifically requires live metrics.

The metrics endpoint should be polled according to the issue requirement:

```text
Every 1 second
```

Use a proper React mechanism for polling.

Avoid creating multiple uncontrolled intervals.

The implementation must:

* start polling when the relevant page/component is mounted
* clean up the interval when unmounted
* avoid duplicate polling
* handle request failures
* avoid memory leaks
* update the UI with new data
* preserve enough history for the live chart where required

Do not use random values to simulate live EPS.

---

# 16. COMPONENT STRUCTURE

Use reusable components.

Suggested structure:

```text
src/
├── app/
│   ├── page.tsx
│   ├── threat-intelligence/
│   │   └── page.tsx
│   └── siem-alerting/
│       └── page.tsx
│
├── components/
│   ├── ui/
│   │   └── shadcn components
│   │
│   ├── layout/
│   │   ├── Sidebar.tsx
│   │   ├── Header.tsx
│   │   └── PageContainer.tsx
│   │
│   ├── dashboard/
│   │   ├── KpiTelemetryCards.tsx
│   │   ├── PerformanceChart.tsx
│   │   ├── AlertFeed.tsx
│   │   └── OcsfStreamTable.tsx
│   │
│   ├── threat-intelligence/
│   │   ├── VendorMix.tsx
│   │   ├── DispositionSplit.tsx
│   │   └── AttackVectorsTable.tsx
│   │
│   ├── siem/
│   │   ├── IncidentQueue.tsx
│   │   ├── TriageIndicators.tsx
│   │   └── ForensicWorkspace.tsx
│   │
│   └── common/
│       ├── LoadingState.tsx
│       ├── EmptyState.tsx
│       └── ErrorState.tsx
│
└── lib/
    ├── api.ts
    ├── mock-data.ts
    ├── types.ts
    └── utils.ts
```

This is a recommended organization, not a requirement to create every file immediately.

Only create components that are actually needed.

---

# 17. SHADCN/UI

shadcn/ui is ALREADY configured.

Do not initialize shadcn again.

Existing configuration:

```text
components.json
```

Reusable UI primitives should come from shadcn/ui where appropriate.

If a required component is missing, install it using:

```bash
npx shadcn@latest add <component>
```

Examples:

```bash
npx shadcn@latest add button
npx shadcn@latest add card
npx shadcn@latest add badge
npx shadcn@latest add table
npx shadcn@latest add tabs
npx shadcn@latest add separator
npx shadcn@latest add tooltip
npx shadcn@latest add dropdown-menu
npx shadcn@latest add scroll-area
npx shadcn@latest add progress
```

Install ONLY the components actually needed.

Do not install the entire shadcn component library unnecessarily.

If the environment has issues executing `npx` on Windows PowerShell, use:

```bash
npx.cmd shadcn@latest add <component>
```

Do not replace shadcn with another component library.

---

# 18. CHARTS

For the live performance chart, use an appropriate locally installed React charting solution if one is already present.

First inspect:

```text
package.json
```

If a chart library is not present, choose a lightweight dependency that works with the existing Next.js application and can be installed/bundled locally.

Do not use an externally hosted chart.

The chart must follow the visual design in:

```text
design_page1.md
```

The chart must consume real/mock API data rather than arbitrary hardcoded values.

---

# 19. ICONS

Use the icon solution already present in the project if available.

Do not use:

```text
Google Material Symbols CDN
external icon websites
remote icon fonts
```

If an icon library is required, install it as a local npm dependency.

Icons should follow the design files.

---

# 20. FONTS

The application must work in an air-gapped environment.

Do NOT use:

```text
Google Fonts
fonts.googleapis.com
fonts.gstatic.com
remote font URLs
```

Do not add:

```html
<link href="https://fonts.googleapis.com/...">
```

If the design requires a specific font:

1. Check whether it is already installed/bundled.
2. Check whether a local font exists in `public/fonts`.
3. Otherwise use an appropriate local/system fallback.

Do not introduce runtime network dependency for typography.

---

# 21. OFFLINE / AIR-GAPPED REQUIREMENT

This project must support an offline/air-gapped environment.

Therefore:

DO NOT use:

```text
https://cdn.tailwindcss.com
Google Fonts
Google-hosted images
external JavaScript
external CSS
external icon CDNs
remote runtime APIs other than the configured local ULPF backend
```

All frontend dependencies must be npm-installed and bundled.

All static assets must be local.

The only runtime backend communication should be with the ULPF backend configured for this application.

---

# 22. NO HARD-CODED LIVE TELEMETRY

Do not do this:

```ts
const eps = 142500;
const latency = 1.28;
```

and label it:

```text
LIVE
```

If the value is mock data, explicitly treat it as fixture/mock data.

If the value is live, obtain it from the API.

Hardcoded values are acceptable only for:

* static labels
* UI configuration
* design constants
* fallback placeholders
* clearly marked fixture/demo data

---

# 23. LOADING STATES

Every API-backed section needs an appropriate loading state.

Examples:

```text
Loading metrics...
Loading alerts...
Loading OCSF records...
Loading vendor data...
Loading incidents...
```

Prefer skeletons/placeholders consistent with the Stitch design where appropriate.

Do not leave a blank screen while waiting for the API.

---

# 24. EMPTY STATES

Handle valid empty responses.

Examples:

```text
No alerts detected.
No OCSF records available.
No IOC correlations found.
No incidents require triage.
No vendor data available.
```

Empty state must be visually intentional.

Do not display broken tables or `undefined`.

---

# 25. ERROR STATES

Handle API errors.

Examples:

```text
Unable to load metrics.
Backend unavailable.
Failed to load alerts.
Unable to retrieve records.
```

Where appropriate provide a retry mechanism.

Do not silently swallow API failures.

Do not show stale/mock data as live without clearly indicating the mode.

---

# 26. BACKEND UNAVAILABLE

The frontend must remain usable when `ulpf serve` is not running.

Development/demo should be able to use fixture data.

The UI should clearly communicate whether it is:

```text
LIVE
```

or:

```text
MOCK
```

Do not pretend the backend is live when it is unavailable.

---

# 27. RESPONSIVE DESIGN

Implement the layouts described by the design files.

The frontend should work on:

* desktop
* smaller laptop screens
* reasonable tablet widths

Do not destroy the visual hierarchy on smaller screens.

Tables may require horizontal scrolling where appropriate.

Do not unnecessarily redesign the desktop layout.

---

# 28. VISUAL IMPLEMENTATION RULE

The design files are the source of truth for visual appearance.

Before writing UI code:

```text
READ
design_page1.md
design_page2.md
design_page4.md
```

Extract:

* colors
* typography
* spacing
* dimensions
* card styling
* borders
* radius
* shadows
* chart appearance
* table appearance
* sidebar/header structure
* status colors
* page backgrounds

Then implement the design with Tailwind CSS and shadcn/ui.

Do not replace the design with a generic SaaS dashboard.

---

# 29. SHARED LAYOUT

The three Issue #13 pages belong to the same application.

If the design contains a common:

* sidebar
* header
* navigation
* theme
* status area

implement it once and reuse it.

Do not duplicate the entire layout inside each page.

For example:

```text
App Layout
│
├── Sidebar
├── Header
│
└── Page Content
    ├── Analyst Dashboard
    ├── Threat Intelligence
    └── SIEM Alerting
```

---

# 30. ROUTING

Use Next.js App Router.

Suggested routes:

```text
/
```

for the Analyst Dashboard.

For the other pages:

```text
/threat-intelligence
/siem-alerting
```

Follow the design files if they specify different route names.

Do not create unnecessary routing layers.

---

# 31. SERVER VS CLIENT COMPONENTS

Use Next.js Server Components by default.

Use `"use client"` only where client-side behavior is actually required.

Client components are expected for things such as:

* live polling
* interactive charts
* client-side filters
* table selection
* interactive forensic workspace
* UI state

Do not mark the entire application as `"use client"` unnecessarily.

---

# 32. STATE MANAGEMENT

Do not introduce Redux/Zustand/etc. unless there is a demonstrated requirement.

Prefer:

* React state
* component state
* server components
* small custom hooks
* centralized API functions

Keep the implementation simple.

---

# 33. DATA FLOW

The intended architecture is:

```text
                  ULPF Rust Backend
                         │
                         │ HTTP API
                         ▼
                  src/lib/api.ts
                         │
                         ▼
                  Typed API models
                         │
            ┌────────────┼────────────┐
            ▼            ▼            ▼
       Dashboard    Threat Intel     SIEM
            │            │            │
            ▼            ▼            ▼
       Components    Components    Components
```

For mock mode:

```text
data/fixtures/api/
          │
          ▼
   mock-data.ts
          │
          ▼
       UI
```

Keep this separation clean.

---

# 34. DO NOT MODIFY THE RUST BACKEND UNLESS NECESSARY

Issue #13 is primarily a frontend issue.

Do not modify backend behavior merely to make the frontend easier to implement.

First use the existing API contract.

If an API requirement genuinely cannot be fulfilled because of a backend limitation:

1. Identify the exact missing endpoint/data.
2. Check `docs/CONTRACTS.md`.
3. Check `docs/openapi.yaml`.
4. Check backend implementation.
5. Document the blocker.

Do not silently invent a backend response.

---

# 35. TESTING REQUIREMENTS

Before declaring Issue #13 complete, test:

### Page 1

* dashboard renders
* KPI cards render
* metrics load
* 1-second polling works
* chart updates
* alerts load
* OCSF table loads
* loading state
* empty state
* API failure state
* backend unavailable state

### Page 2

* vendor mix loads
* disposition split loads
* Allowed / Blocked / Dropped are correctly represented
* attack vector table loads
* loading state
* empty state
* error state

### Page 4

* incident queue loads
* Critical / Medium display correctly
* triage information displays
* selecting an incident updates forensic workspace
* no-selection state
* loading state
* error state

---

# 36. BUILD VERIFICATION

From:

```text
frontend/laya-frontend/
```

run:

```bash
npm run lint
```

and:

```bash
npm run build
```

Both should succeed before Issue #13 is considered complete.

If the project has additional package scripts, inspect:

```bash
npm run
```

and use the relevant checks.

---

# 37. GIT SAFETY

You are working on:

```text
feat/13-analyst-dashboard
```

Do NOT work directly on:

```text
master
```

Do not reset or overwrite unrelated repository work.

Before modifying files:

```bash
git status
```

Before completion:

```bash
git status
git diff
```

Keep changes scoped to Issue #13.

Do not modify Issues #14/#15 implementation.

---

# 38. DO NOT CREATE UNRELATED FEATURES

Do NOT implement:

* Query Explorer
* Crypto Vault
* Parser management
* Integrity management
* System Health
* Evidence bundle export
* Onboarding wizard
* Tamper drill
* Benchmark management

Those belong to later issues.

Only implement the three Issue #13 pages:

```text
1. Analyst Dashboard & Live Overview
2. Threat Intelligence & Vendor Mix
4. SIEM Alerting & Incident Triage
```

---

# 39. IMPLEMENTATION ORDER

Follow this order.

## Step 1 — Inspect

Read:

```text
design_page1.md
design_page2.md
design_page4.md

docs/CONTRACTS.md
docs/openapi.yaml

data/fixtures/api/*
```

Then inspect:

```text
package.json
components.json
src/
```

and the existing backend under:

```text
crates/ulpf-cli/src/serve/
```

---

## Step 2 — Understand the design

Determine:

* global layout
* navigation
* colors
* fonts
* spacing
* reusable UI
* page-specific components
* responsive behavior

Do not start coding before understanding the three designs.

---

## Step 3 — Establish API layer

Implement/complete:

```text
src/lib/api.ts
src/lib/types.ts
src/lib/mock-data.ts
```

based on the actual API contract.

---

## Step 4 — Build shared layout

Implement shared navigation/layout components if required.

---

## Step 5 — Implement Page 1

Build:

* KPI cards
* live chart
* alert feed
* OCSF stream

Connect them to the backend.

---

## Step 6 — Implement Page 2

Build:

* vendor mix
* disposition split
* IOC/attack vector table

Connect them to backend/fixtures.

---

## Step 7 — Implement Page 4

Build:

* incident queue
* triage indicators
* forensic workspace

Connect them to backend/fixtures.

---

## Step 8 — Handle states

Verify:

```text
Loading
Success
Empty
Error
Backend unavailable
```

---

## Step 9 — Verify design

Compare implementation against:

```text
design_page1.md
design_page2.md
design_page4.md
```

Correct visual differences.

---

## Step 10 — Test

Run:

```bash
npm run lint
npm run build
```

Also manually test the application.

---

# 40. IMPORTANT: DO NOT STOP AT A STATIC MOCKUP

The following is NOT considered complete:

```text
Beautiful UI
+
Hardcoded numbers
+
Hardcoded alerts
```

The issue specifically requires live EPS, alerts, and mix.

The implementation must connect the UI to the actual backend/fixtures.

---

# 41. IMPORTANT: DO NOT OVER-ENGINEER

Do not introduce unnecessary:

* state-management frameworks
* abstraction layers
* API frameworks
* component libraries
* backend changes
* databases
* authentication systems
* analytics systems

Use the existing stack.

Keep the architecture understandable for a student team.

---

# 42. FINAL ACCEPTANCE CRITERIA

Issue #13 is complete only when:

### Design

* [ ] Page 1 matches `design_page1.md`
* [ ] Page 2 matches `design_page2.md`
* [ ] Page 4 matches `design_page4.md`

### Page 1

* [ ] KPI telemetry cards
* [ ] Live EPS
* [ ] Live SVG performance chart
* [ ] Alert feed
* [ ] OCSF stream table

### Page 2

* [ ] Vendor ingestion mix
* [ ] Allowed / Blocked / Dropped disposition split
* [ ] IOC-correlated attack vectors table

### Page 4

* [ ] Critical/Medium incident queue
* [ ] Triage indicators
* [ ] Forensic workspace

### Backend

* [ ] API contracts inspected
* [ ] API client implemented
* [ ] Fixture mode implemented
* [ ] Live backend mode implemented
* [ ] No fake live telemetry

### UX

* [ ] Loading states
* [ ] Empty states
* [ ] Error states
* [ ] Backend unavailable state
* [ ] Responsive layout

### Offline

* [ ] No Google Fonts
* [ ] No CDN Tailwind
* [ ] No external icon CDN
* [ ] No external runtime assets
* [ ] Dependencies are locally bundled

### Quality

* [ ] `npm run lint` passes
* [ ] `npm run build` passes
* [ ] No unnecessary `any`
* [ ] No obvious console errors
* [ ] No broken routes
* [ ] No unrelated Issue #14/#15 work

---

# 43. COMPLETION REPORT

When everything above is complete, report the result in this format:

```text
ISSUE #13 COMPLETE

Pages:
- Analyst Dashboard & Live Overview
- Threat Intelligence & Vendor Mix
- SIEM Alerting & Incident Triage

API:
- Endpoints connected:
  - <actual endpoints used>

Mock/Fixture Mode:
- <description>

Live Mode:
- <description>

Components:
- <list>

Testing:
- npm run lint: PASS/FAIL
- npm run build: PASS/FAIL
- Manual testing: PASS/FAIL

Offline Compatibility:
- PASS/FAIL

Known Issues:
- <none or list>

Files Changed:
- <list>

Issue #13 is ready for review.
```

After giving this report:

**STOP.**

Do NOT begin Issue #14.

Do NOT begin Issue #15.

Wait for explicit instructions before continuing.

---

# 44. MOST IMPORTANT RULE

The definition of done is:

```text
Stitch/design specification
        ↓
Correct Next.js implementation
        ↓
Reusable components
        ↓
Tailwind + shadcn styling
        ↓
API integration
        ↓
Mock/fixture support
        ↓
Live backend support
        ↓
Loading / Empty / Error states
        ↓
Offline compatibility
        ↓
Lint + Build + Manual testing
        ↓
Issue #13 COMPLETE
        ↓
STOP
```

A page that only looks correct is NOT complete.

A page that works but does not match the provided design is NOT complete.

A page that uses hardcoded live-looking values is NOT complete.

A page that requires external CDN resources is NOT complete.

The goal is a **production-quality, offline-compatible, API-connected implementation of Issue #13 within the existing Next.js application.**
