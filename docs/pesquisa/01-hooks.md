# Hooks do Claude Code

> Pesquisa automática de 2026-10-02 (somente leitura), em inglês. Pode envelhecer: confira antes de confiar.

## Relatório

## Claude Code Hooks & Pet Project Integration Report (v2.1.288)

### 1. Complete Hook Events Reference for v2.1.288

Claude Code fires hooks at three cadences: per-session, per-turn, and per-tool-call. Below is the complete list of events available in version 2.1.288 with exact stdin structures.

#### **Session Lifecycle Events**

**SessionStart** - Fires once per session
- stdin fields: session_id (string), prompt_id (UUID), transcript_path (string), cwd (string), scratchpad_dir (string), permission_mode (string), effort.level, hook_event_name="SessionStart"
- Optional fields: source (startup|resume|clear|compact|fork), model, agent_type, session_title, seconds_since_last_response, context_tokens, prompt_cache_likely_expired (boolean), estimated_cache_write_usd (float)

**SessionEnd** - When session terminates
- Same common fields as SessionStart, hook_event_name="SessionEnd"
- CRITICAL: All hooks at SessionEnd share 1.5s total budget (60s max per single hook)

**Setup** - Initialization with --init-only or -p --init/--maintenance
- Common fields + hook_event_name="Setup"
- Note: MCP tool hooks NOT available; MCP servers haven't connected yet

#### **Per-Turn Events (Can Block)**

**UserPromptSubmit** - Before Claude processes a prompt
- stdin: hook_event_name="UserPromptSubmit", prompt_text, plus common fields
- Timeout default: 30s (shorter than typical 600s)
- Exit code 2 blocks submission

**UserPromptExpansion** - Before slash command expands
- stdin: hook_event_name="UserPromptExpansion", skill_name, skill_args, common fields

**Stop** - Claude finishes responding (fires after assistant message completes)
- stdin: hook_event_name="Stop", permission_mode, last_assistant_message (text of Claude's response), common fields
- KEY: This fires ONCE per main-agent turn. Does NOT fire for subagent Stop
- Exit code 2 blocks continuation
- **For pet detection**: last_assistant_message field tells you what Claude produced

**StopFailure** - When turn ends due to API error
- stdin: hook_event_name="StopFailure", error_type (rate_limit|overloaded|authentication_failed|etc), error_message, common fields

#### **Tool Execution Loop Events**

**PreToolUse** - Before tool call executes (can block or modify input)
- stdin: hook_event_name="PreToolUse", tool_name (string), tool_input (object), tool_use_id (string), permission_mode, common fields
- Timeout: 600s default
- Exit code 2 blocks execution
- **For pet detection**: Can count PostToolUse events to measure work done in a turn

**PostToolUse** - After tool succeeds (can modify result)
- stdin: hook_event_name="PostToolUse", tool_name, tool_input, tool_output (execution result), tool_use_id, permission_mode, common fields
- Cannot block; can only modify context
- **For pet**: Count these to measure "work completed" between Stop events

**PostToolUseFailure** - After tool fails
- stdin: hook_event_name="PostToolUseFailure", tool_name, tool_input, error (error message), tool_use_id, common fields

**PostToolBatch** - After batch of parallel tool calls (rare)
- stdin: hook_event_name="PostToolBatch", tool_calls (array), common fields

**PermissionRequest** - Tool needs permission decision (can auto-approve/deny)
- stdin: hook_event_name="PermissionRequest", tool_name, tool_input, tool_use_id, permission_mode, classifier_result (verdict|confidence|reasoning), common fields
- Exit code 0 with decision field in output: {hookSpecificOutput: {permissionDecision: "allow"|"deny"|"ask"|"defer"}}

**PermissionDenied** - Auto mode denied tool call
- stdin: hook_event_name="PermissionDenied", tool_name, tool_input, reason (string), common fields

#### **Subagent Events**

**SubagentStart** - Subagent spawned
- stdin: hook_event_name="SubagentStart", agent_type, agent_id, agent_name, common fields

**SubagentStop** - Subagent finishes
- stdin: hook_event_name="SubagentStop", agent_type, agent_id, last_message, common fields
- KEY: SubagentStop fires SEPARATELY from Stop. Use to track background/parallel work
- Matcher: match by agent_type name

**TeammateIdle** - Agent team teammate idle
- stdin: hook_event_name="TeammateIdle", teammate_id, idle_duration, common fields

**TaskCreated** - Task creation via TaskCreate
- stdin: hook_event_name="TaskCreated", task_id, task_description, agent_id, common fields

**TaskCompleted** - Task marked completed
- stdin: hook_event_name="TaskCompleted", task_id, result_summary, agent_id, common fields

#### **Configuration & Context Events**

**ConfigChange** - Configuration file changes
- stdin: hook_event_name="ConfigChange", changed_file, change_type (create|modify|delete), common fields

**CwdChanged** - Working directory changes
- stdin: hook_event_name="CwdChanged", new_cwd, old_cwd, common fields

**DirectoryAdded** - Directory added via /add-dir
- stdin: hook_event_name="DirectoryAdded", directory, common fields

**FileChanged** - Watched file changes on disk (matcher: literal filenames)
- stdin: hook_event_name="FileChanged", file_path, change_type (create|modify|delete), common fields

**InstructionsLoaded** - CLAUDE.md or rules loaded
- stdin: hook_event_name="InstructionsLoaded", source (file path), common fields

#### **Model & Compaction Events**

**PreModelSwitch** - Before model switch (can block)
- stdin: hook_event_name="PreModelSwitch", new_model, old_model, reason, common fields
- Timeout default: 30s
- Exit code 2 blocks switch

**PostModelSwitch** - After model changes
- stdin: hook_event_name="PostModelSwitch", new_model, old_model, common fields
- Timeout default: 30s

**PreCompact** - Before context compaction
- stdin: hook_event_name="PreCompact", current_tokens, target_tokens, common fields
- Can inject additional context via additionalContext in output

**PostCompact** - After compaction completes
- stdin: hook_event_name="PostCompact", tokens_removed, new_total_tokens, common fields

#### **MCP & Notifications**

**Elicitation** - MCP server requests user input
- stdin: hook_event_name="Elicitation", server_name, content (JSON form schema), common fields

**ElicitationResult** - User responds to elicitation
- stdin: hook_event_name="ElicitationResult", server_name, form_values (user input), common fields

**Notification** - Notification sent
- stdin: hook_event_name="Notification", notification_type (permission_prompt|idle_prompt|auth_success|elicitation_dialog|skill_completion|etc), notification_details, common fields
- Matcher: match by notification_type name

#### **Display Events**

**MessageDisplay** - While assistant message streams (display-only, cannot block)
- stdin: hook_event_name="MessageDisplay", message_chunk (text), message_position, common fields
- Timeout default: 10s (very short)

**WorktreeCreate** - Git worktree created
- stdin: hook_event_name="WorktreeCreate", worktree_path, branch, common fields

**WorktreeRemove** - Git worktree removed
- stdin: hook_event_name="WorktreeRemove", worktree_path, common fields

---

### 2. Hook Handler Types & Configuration (All Exact Fields)

#### **Command Hook** (`type: "command"`)
```
- command: string (shell command or path to executable)
- args: array (optional; presence switches to exec-form, no shell parsing)
- shell: "bash" or "powershell" (optional; only on Windows if exec-form)
- async: boolean (optional; run in background, default false)
- asyncRewake: boolean (optional; background + wake on exit code 2, implies async=true)
- timeout: integer seconds (default varies by event: 600s typical, 30s for UserPromptSubmit/PreModelSwitch/PostModelSwitch, 10s for MessageDisplay, 1.5s shared at SessionEnd)
- statusMessage: string (optional; shown while running)
- if: string (optional; permission rule filter, e.g., "Bash(rm *)")
- once: boolean (optional; remove after first success)
```

**Behavior when pet container is down:**
- Exit code 0: Success; reads JSON from stdout
- Exit code 2: Blocking error (if event supports blocking); message from stderr
- Other exit codes: Non-blocking error; execution continues
- **FIRE-AND-FORGET without slowdown:** Use `async: true` with `timeout: 5` and empty exit handling (always returns exit code 1)

#### **HTTP Hook** (`type: "http"`)
```
- url: string (http://localhost:PORT/path; NOT https required for localhost)
- method: "POST" (default; only POST documented)
- headers: object (optional; {Authorization: "Bearer TOKEN"})
- allowedEnvVars: array of strings (optional; which env vars can be interpolated in headers)
- timeout: integer seconds (default 600s)
- if: string (optional; permission rule filter)
```

**Behavior:**
- Sends hook stdin JSON as POST body
- Returns on timeout or connection refused (silent; doesn't block Claude)
- No response body parsing; fire-and-forget by design
- **Perfect for pet**: POST to http://127.0.0.1:8888/events with timeout 2 and Docker container may or may not be running

#### **MCP Tool Hook** (`type: "mcp_tool"`)
```
- server: string (name or "plugin:plugin-name:server-name" for plugin-bundled servers)
- tool: string (tool name on the server)
- input: object (template with ${tool_input.FIELD} interpolation)
- timeout: integer seconds (default 600s)
```

**Behavior:**
- NOT available at SessionStart or Setup (MCP servers haven't connected yet)
- Can access plugin-provided MCP servers directly
- Slower than command/HTTP; use for complex integrations

#### **Prompt Hook** (`type: "prompt"`)
```
- prompt: string (the prompt text; supports $ARGUMENTS and other interpolations)
- model: string (optional; defaults to background model, e.g., "claude-opus-5")
- timeout: integer seconds (default 30s)
```

**Behavior:**
- Single-turn LLM evaluation (no tool use)
- Slow; rarely used for real-time events

#### **Agent Hook** (`type: "agent"`) - EXPERIMENTAL
```
- Unspecified in current docs; likely not stable for production use
```

---

### 3. Distinguishing Main-Agent from Subagent Completion

This is critical for your pet to know when the human's requested work is done vs. background tasks.

**Main-Agent Stop Signal:**
- Hook event: `Stop` (not `SubagentStop`)
- stdin field: `hook_event_name === "Stop"`
- Fires ONCE at the end of each main-agent turn, after all tool calls in that turn complete
- In stdin: `last_assistant_message` contains the assistant's final response text
- **Measurement**: Count `PostToolUse` events between UserPromptSubmit and Stop to measure work done in that turn

**Subagent Stop Signal:**
- Hook event: `SubagentStop` (fires independently)
- stdin field: `hook_event_name === "SubagentStop"`, `agent_id`, `agent_type`
- Fires for each subagent completion (background tasks, parallel agents, teammates)
- Does NOT indicate main-agent is done; main-agent may still be running

**How Headless Sessions Differ:**
- In `claude -p` (non-interactive): Stop fires after the completion message is generated
- In SDK runs (Agent SDK): Stop fires after the session's agentic loop ends per "turn"
- Subagent events fire in both cases; SubagentStop signals background work completion

**Session ID & Project Detection:**
- stdin always includes: `session_id` (unique per session), `cwd` (current working directory)
- Derive project name from `cwd` or look for `.claude/` marker
- Use `session_id` in pet's display (optional; nice for debugging)

---

### 4. Packaging & Installation Options (Ranked by Friction & Uninstall Ease)

#### **OPTION A: Claude Code Plugin + Marketplace (RECOMMENDED)**

**Easiest friction; cleanest uninstall.**

**Directory Layout:**
```
your-docker-pet-repo/
  .claude-plugin/
    plugin.json              (manifest: name, version, description, author)
    marketplace.json         (ONLY if distributing via marketplace)
  docker-compose.yml
  bin/                       (executables Claude Code's Bash tool can call)
    notify-pet.sh            (wrapper to POST to docker container)
  hooks/
    hooks.json               (hook configuration)
  skills/
    (optional: setup skills)
  .mcp.json                  (optional: MCP servers your plugin provides)
  Dockerfile
  Makefile                   (or scripts for building docker image)
  README.md
  LICENSE
```

**Complete plugin.json Example:**
```json
{
  "name": "desktop-pet-notifier",
  "displayName": "Desktop Pet Notifier",
  "version": "1.0.0",
  "description": "Cute pixel-art character that animates when Claude Code finishes development tasks",
  "author": {
    "name": "Your Name",
    "email": "you@example.com"
  },
  "homepage": "https://github.com/yourname/desktop-pet",
  "repository": "https://github.com/yourname/desktop-pet",
  "license": "MIT",
  "keywords": ["pet", "notifications", "wayland", "desktop"],
  "hooks": "./hooks/hooks.json"
}
```

**Complete hooks/hooks.json Example (fire-and-forget HTTP):**
```json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "http",
            "url": "http://127.0.0.1:8888/events",
            "timeout": 2,
            "headers": {}
          }
        ]
      }
    ],
    "Stop": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "http",
            "url": "http://127.0.0.1:8888/events",
            "timeout": 2,
            "headers": {}
          }
        ]
      }
    ],
    "SubagentStop": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "http",
            "url": "http://127.0.0.1:8888/events",
            "timeout": 2,
            "headers": {}
          }
        ]
      }
    ],
    "UserPromptSubmit": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "http",
            "url": "http://127.0.0.1:8888/events",
            "timeout": 2,
            "headers": {}
          }
        ]
      }
    ]
  }
}
```

**Installation for Users (Private GitHub Repo):**

1. User runs in Claude Code session:
   ```
   /plugin marketplace add github-username/desktop-pet
   ```
   (Claude Code builds the marketplace source from this shorthand for a public repo)

2. For PRIVATE repo, user needs git SSH key or GitHub credential:
   - SSH: `ssh -T git@github.com` must work (key in ssh-agent)
   - HTTPS: User runs `gh auth login && gh auth setup-git` once
   - User then: `/plugin marketplace add github-username/desktop-pet`

3. Or user installs from local path during development:
   ```
   claude --plugin-dir /path/to/desktop-pet-repo
   ```

4. User installs the plugin from the marketplace:
   ```
   /plugin install desktop-pet-notifier@<marketplace-name>
   ```

5. Docker service and pet start as part of plugin lifecycle

**Uninstall:**
```
/plugin uninstall desktop-pet-notifier
```
- Removes plugin, hooks, and docker cleanup can run in SessionEnd hook

**Update:**
- User runs `/plugin marketplace update <name>` or `claude plugin update desktop-pet-notifier`
- Or: automatic in background if admin enables `autoUpdate` in managed settings

**Advantages:**
- No settings.json merge needed; plugins are isolated
- Hooks are versioned with the plugin
- User can enable/disable from `/plugin` UI
- Multi-project; install once, use everywhere
- Marketplace distribution scales (GitHub releases, private repos)

**Constraints:**
- Plugin name must be kebab-case (desktop-pet-notifier)
- Manifest `name` must not start with claude-, anthropic-, or contain them as whole word
- Plugin validation with `claude plugin validate ./` must pass

---

#### **OPTION B: Merge Hooks into settings.json (Simpler, Minimal)**

**Lower friction for single-project use; requires jq + commit discipline.**

**Setup:**
```bash
cd your-project
mkdir -p .claude
```

**Merge using jq (non-destructive):**
```bash
jq '.hooks += {
  "Stop": [{"matcher": "*", "hooks": [{"type": "http", "url": "http://127.0.0.1:8888/events", "timeout": 2}]}],
  "SubagentStop": [{"matcher": "*", "hooks": [{"type": "http", "url": "http://127.0.0.1:8888/events", "timeout": 2}]}]
}' .claude/settings.json > .claude/settings.json.tmp && mv .claude/settings.json.tmp .claude/settings.json
```

Or manually add to `.claude/settings.json`:
```json
{
  "hooks": {
    "Stop": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "http",
            "url": "http://127.0.0.1:8888/events",
            "timeout": 2
          }
        ]
      }
    ]
  }
}
```

**Commit to .claude/settings.json (shared with team)**
- `.claude/settings.local.json` is gitignored (for secrets)

**Uninstall:**
- Remove the `hooks` block from `.claude/settings.json`
- Or just delete the file and reconfigure manually

**Advantages:**
- No separate plugin structure
- Hooks live in the project
- One-file config

**Disadvantages:**
- Repeated across projects
- Must merge carefully with jq (risks overwriting other hooks)
- No versioning; no update mechanism

---

#### **OPTION C: Combined: Plugin + Docker Compose Setup**

**The complete production setup for your pet:**

**.claude-plugin/plugin.json** declares the plugin but NOT the hooks initially:
```json
{
  "name": "desktop-pet",
  "description": "Pet character for Claude Code",
  "version": "1.0.0",
  "author": { "name": "you" },
  "hooks": "./hooks/hooks.json"
}
```

**hooks/hooks.json** (subset: only the essential events that trigger pet animations):
```json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "startup|resume",
        "hooks": [
          {
            "type": "command",
            "command": "docker-compose -f ~/.claude/plugins/data/desktop-pet/docker-compose.yml up -d",
            "timeout": 10,
            "async": true
          }
        ]
      }
    ],
    "Stop": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "http",
            "url": "http://127.0.0.1:8888/stop",
            "timeout": 2
          }
        ]
      }
    ],
    "SessionEnd": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "docker-compose -f ~/.claude/plugins/data/desktop-pet/docker-compose.yml down",
            "timeout": 5,
            "async": true
          }
        ]
      }
    ]
  }
}
```

**Dockerfile** (Alpine + wlroots for Wayland layer-shell):
```dockerfile
FROM alpine:latest
RUN apk add --no-cache \
    nodejs npm \
    libxkbcommon libxkbcommon-dev \
    wayland wayland-dev \
    wayland-protocols \
    weston \
    grim slurp
COPY . /app
WORKDIR /app
RUN npm install
EXPOSE 8888
CMD ["node", "server.mjs"]
```

**docker-compose.yml:**
```yaml
version: '3.8'
services:
  pet:
    build: .
    image: desktop-pet:local
    container_name: claude-desktop-pet
    ports:
      - "8888:8888"
    environment:
      TZ: America/Sao_Paulo
      WAYLAND_DISPLAY: ${WAYLAND_DISPLAY}
      XDG_RUNTIME_DIR: ${XDG_RUNTIME_DIR}
      HYPRLAND_INSTANCE_SIGNATURE: ${HYPRLAND_INSTANCE_SIGNATURE}
    volumes:
      - ${XDG_RUNTIME_DIR}:${XDG_RUNTIME_DIR}:ro
      - /tmp/.X11-unix:/tmp/.X11-unix:ro
    restart: unless-stopped
    logging:
      driver: json-file
      options:
        max-size: "10m"
        max-file: "5"
```

**server.mjs (Node.js + Quickshell for layer-shell):**
- HTTP server on 8888 listening for `/events` POST from hooks
- Parses hook JSON (event type, session_id, last_message, etc.)
- Spawns animation via Quickshell layer-shell surface
- Follows active monitor (via `hyprctl -j monitors`)
- Draggable via mouse (mouseDown/mouseUp handlers)

---

### 5. Alternative Signals Beyond Hooks

**Hooks are the primary & recommended signal for "Claude finished" but alternatives exist:**

| Signal | Pros | Cons | Use When |
|--------|------|------|----------|
| `Stop` hook (HTTP) | Real-time, direct, low-latency | Requires daemon | Primary choice ✓ |
| `Notification` hook | Catches permission prompts, auth, etc. | Noisy (many non-work events) | Pet reacts to user prompts too |
| OpenTelemetry export (CLAUDE_OTEL_ENABLED) | Structured metrics, spans | No current public docs; experimental | Internal instrumentation |
| Transcript JSONL poll (read transcript_path) | Works without hooks | Polling latency, permission issues | Fallback only |
| Message stream token count | Indicates response end | No direct API | Analysis post-hoc |
| MCP server tool invocation | Custom integration | Complex; requires MCP | Advanced scenarios |

**Best strategy for pet:** `Stop` hook (primary) + optional `UserPromptSubmit` (idle detection).

---

### 6. Gotchas & Configuration Edge Cases

**When Hook Config Changes Take Effect:**
- `~/.claude/settings.json` (global): Next session
- `.claude/settings.json` (project): Next session OR `/reload-plugins` in current session if plugin
- `.claude/settings.local.json` (project local): Next session (never hot-reloaded)
- Plugin `hooks/hooks.json` (in marketplace plugin): Next `/reload-plugins` or new session
- **No automatic reload:** Editing hooks requires manual `/reload-plugins` in interactive mode

**Trust & Approval Prompts:**
- First load of a private git marketplace: User must approve git clone on first install
- Private plugin from private repo: SSH key or GitHub credentials required (no token field in manifest)
- **No approval for HTTP hooks** (fire and forget, never blocks)
- Command hooks that reference `${user_config.*}`: Not allowed in shell form (use exec form with `args` instead)

**Rate of Hook Events:**
- `PreToolUse` fires for EVERY tool call (Bash, Write, Edit, MCP tools, etc.)
- In a busy session: Could be 50+ PreToolUse/PostToolUse pairs per turn
- **Don't use PreToolUse for rate-limited notifications** (use Stop instead)
- `PostToolUse` fires even for read-only tools (Bash read, Read, Glob, etc.)

**Environment Variables Available to Hooks:**
- `CLAUDE_PROJECT_DIR`: Project root (from `cwd`)
- `CLAUDE_PLUGIN_ROOT`: Plugin install directory (if running in plugin)
- `CLAUDE_PLUGIN_DATA`: Plugin-specific data dir (useful for caching)
- `CLAUDE_EFFORT`: Current effort level
- `CLAUDE_CODE_SIMPLE=1`: Set if `--bare` was used
- Plugin `userConfig` options: Exported as `CLAUDE_PLUGIN_OPTION_KEY` (upercased)
- **NOT available:** User's env vars (security: hooks don't inherit shell env); credentials

**Exit Codes & Non-Blocking Behavior:**
- Exit 0: Hook succeeded; reads JSON output
- Exit 2: Blocking error (only for events that support blocking: UserPromptSubmit, Stop, PreToolUse, PermissionRequest, PreModelSwitch, PreCompact)
- Exit 1, 127, etc.: Non-blocking error; execution continues silently
- **Fire-and-forget HTTP**: Timeout or connection-refused is silent; pet can be down without breaking Claude

**The `/hooks` Command:**
- In interactive session: `/hooks` opens the hooks inspector
- Shows all loaded hooks (from settings, plugins, skills)
- Lists each matcher and handler
- Useful for debugging why a hook didn't fire
- Also shows disabled hooks (if `disableAllHooks: true` is set)

**Headless & SDK Behavior:**
- `claude -p` (print mode): SessionStart + Stop fire normally; no interactive prompt approval
- `claude --bg` (background): Same as -p; no approval prompts
- Agent SDK runs: Stop fires at each agent loop iteration (similar to -p)
- **All hook events fire in headless mode** (none are terminal-only)

**Critical Timing & Concurrency:**
- SessionEnd hook timeout: ALL hooks at SessionEnd share 1.5s total budget (max 60s single hook)
- Parallel tool calls: PostToolUse fires for EACH, not once for all
- If your HTTP hook is slow: Each PostToolUse pauses for timeout (even with async=true on command)
- **Recommendation: Keep HTTP timeout ≤ 2s**

**Docker & Host Networking:**
- `localhost` and `127.0.0.1` from Claude Code process → pet container (if local Docker)
- `WAYLAND_DISPLAY`, `XDG_RUNTIME_DIR`: Must pass through to container via volume mount
- `HYPRLAND_INSTANCE_SIGNATURE`: Environment variable; pass to container for monitor tracking
- Docker network mode: Host mode allows direct localhost access; bridge mode requires explicit port map

**Git Marketplace Access for Private Repos:**
- HTTPS: User must run `gh auth login && gh auth setup-git` (stores credential in keychain)
- SSH: User must have `~/.ssh/id_rsa` loaded in ssh-agent (no passphrase prompts allowed)
- Private repo marketplace: User's git credentials apply; plugin market place have no token field
- **Alternative:** Use `archive` source with `headersHelper` command if distributing private plugin as zip

**Plugin Version & Updates:**
- If you set `version: "1.0.0"` in plugin.json: Users stay on that version until you bump it
- If you omit `version`: Users track commits (fetch latest on background auto-update)
- Marketplace entry `version` overrides `plugin.json` version (pick one)
- Updates only occur when version changes or user runs `/plugin install --update`

**Known Limitations:**
- Hooks cannot receive or set Claude's internal state (memory, context window usage, token budgets)
- HTTP hook response body is ignored (fire-and-forget only)
- MCP tool hooks not available at SessionStart/Setup (servers haven't connected)
- Hooks cannot directly modify AI model behavior (only context/blocks/retries)
- No hook for \"user started typing\" (detect via transcript polling or channels)

---

## Recommended Architecture for Your Desktop Pet

Based on the above research, here's the optimal setup:

1. **Plugin-based distribution** (.claude-plugin/plugin.json + hooks/hooks.json)
2. **HTTP hooks** (POST to localhost:8888) for Stop, SessionStart, SubagentStop, UserPromptSubmit
3. **Docker Compose locally** with Wayland socket mount
4. **Quickshell layer-shell** for wlr-layer-shell on Hyprland (follows active monitor, draggable)
5. **Pixel-art animation** triggered by hook event type in JSON stdin
6. **Fire-and-forget design** (HTTP timeout 2s; pet down ≠ Claude blocked)

The pet reactor should:
- Parse `hook_event_name` to determine animation type
- Count `PostToolUse` events between `UserPromptSubmit` and `Stop` to gauge work done
- Use `last_assistant_message` (from Stop hook) to display tooltip/status
- Track session_id in UI for debugging
- Always stay on the active Hyprland monitor (check `hyprctl -j cursorpos`)

GitHub distribution: Publish as plugin via private repo marketplace; users add with `/plugin marketplace add butkeraites/desktop-pet`.

## Fatos-chave

- **[high]** Claude Code 2.1.288 fires the Stop hook ONCE per main-agent turn after all tools complete, not for subagents  
  _Evidência:_ Hooks reference documentation from code.claude.com/docs/en/hooks.md specifies hook_event_name=Stop with no agent_id field, vs SubagentStop which includes agent_id
- **[high]** HTTP hooks in Claude Code are fire-and-forget: timeout or connection refused is silent and never blocks Claude execution  
  _Evidência:_ Hooks guide states HTTP hooks have no response-body parsing and cannot block; they're designed for notifications
- **[high]** SessionEnd hooks share a 1.5s total budget for ALL hooks combined (60s max per single hook)  
  _Evidência:_ Complete hooks reference explicitly states: 'SessionEnd timeout: Shared 1.5s budget for all hooks (max 60s)'
- **[high]** Plugin hooks take effect after /reload-plugins or next session; settings.json hooks take effect at next session only  
  _Evidência:_ Plugin components documentation specifies /reload-plugins loads component changes; settings changes require session restart
- **[high]** The stdin JSON passed to hooks always includes session_id, cwd, permission_mode, and hook_event_name  
  _Evidência:_ Hooks reference lists 'Common Input Fields (All Events)' with these exact fields for every event type
- **[high]** MCP tool hooks are NOT available at SessionStart or Setup (MCP servers haven't connected yet)  
  _Evidência:_ Hooks reference explicitly notes: 'MCP hooks not available: On SessionStart at launch and all Setup events (before MCP servers connect)'
- **[high]** Claude Code plugins for private GitHub repos require user's git credentials (SSH key or gh auth); no token field exists in plugin.json  
  _Evidência:_ Plugin marketplace documentation: 'Claude Code runs git with interactive prompts off and relies on whatever credentials that machine already holds. Claude Code has no git token of its own.'
- **[high]** SubagentStop fires independently of Stop and signals background/parallel agent completion, not main-agent completion  
  _Evidência:_ Hooks reference distinguishes: 'Stop: Claude finishes responding' vs 'SubagentStop: Subagent finishes' with separate event_name values
- **[high]** Environment variables like ANTHROPIC_API_KEY are removed from hook process environments for security; user_config options are exported as CLAUDE_PLUGIN_OPTION_KEYNAME  
  _Evidência:_ Manifest reference states: 'Credentials removed from environment... user_config options exported as CLAUDE_PLUGIN_OPTION_<KEY>'
- **[high]** Hooks cannot reference ${user_config.*} in shell-form commands (only in exec-form args or MCP/LSP config)  
  _Evidência:_ Manifest reference: 'Shell-form hook commands, monitor commands, and MCP headersHelper reject ${user_config.*}. A component that references it fails with an error.'

## Recomendações

- Use the Stop hook as the primary signal for pet animation ('Claude finished work'). Post hook stdin JSON to http://127.0.0.1:8888/events with timeout=2s for fire-and-forget behavior.
- Package the pet as a Claude Code plugin in a .claude-plugin/ directory with plugin.json manifest and hooks/hooks.json configuration, not as merged settings.json. Plugins scale to multiple projects and users can install/uninstall cleanly.
- For private GitHub repo distribution, document that users need git SSH key or GitHub credentials (gh auth setup-git) before running /plugin marketplace add. No authentication token field exists in the plugin manifest.
- Listen to both Stop (main-agent work done) and SubagentStop (background work done) hooks separately. Count PostToolUse events between UserPromptSubmit and Stop to measure 'work completed' in each turn.
- Use HTTP hooks (type: http) rather than command hooks for pet notifications. HTTP hooks never block Claude, survive container downtime silently, and are fire-and-forget by design.
- Implement the pet as a Docker container spawned at SessionStart (command hook, async=true) and torn down at SessionEnd. Mount WAYLAND_DISPLAY and XDG_RUNTIME_DIR volumes and pass HYPRLAND_INSTANCE_SIGNATURE env var for monitor detection.
- Build the pet container's Wayland integration with Quickshell (available on the host at /usr/share/omarchy/...) to create a wlr-layer-shell surface. This allows top-of-screen positioning, monitor tracking, and mouse dragging.
- Keep the HTTP hook timeout to 2 seconds maximum. If the pet container is offline or slow, Claude Code continues without waiting; the pet will batch animations on next container startup (from SessionStart hook).
- Set PostToolUse hook timeout to 600s default but never block (exit code 0 only). This allows the pet to observe every tool call without slowing Claude; use the events to count work but not to gate execution.
- Document that hook configuration changes require /reload-plugins in the current session (for plugin hooks) or a new session (for settings.json hooks). There is no hot-reload of .claude/settings.json.
- For private plugin marketplace, host on GitHub and users add with /plugin marketplace add owner/repo#branch. Document the requirement: 'User must have git SSH key or gh auth setup-git credential stored before adding.'
- Use Notification hook (matcher: permission_prompt, idle_prompt, skill_completion) optionally to animate pet for user-interaction events (not just 'work done'), but recognize this is high-frequency (50+ PreToolUse events per turn).
- Store session_id, hook_event_name, and last_assistant_message from hook stdin in the pet's state for debugging and display. These fields let the pet correlate animations with Claude's actual work and provide context UI.

## Riscos

- If hook HTTP timeout is too long (> 10s), PostToolUse hooks will block Claude's tool execution, defeating the 'fire-and-forget' goal and slowing down interactive development. → **Set timeout: 2 on all HTTP hooks in hooks.json. Document that pet container latency must stay under 2s. Use async: true on command hooks that manage the container.**
- Private GitHub plugin marketplace requires user's git credentials; if user has no GitHub SSH key or credential helper, /plugin marketplace add fails silently. → **In README, document: 'Before adding this marketplace, run: gh auth login && gh auth setup-git' (for HTTPS) or 'Ensure SSH key is loaded in ssh-agent' (for SSH). Offer fallback: clone locally and use --plugin-dir.**
- SessionEnd hook has only 1.5s total budget for all hooks (shared across all plugins + settings). If pet's cleanup takes too long, sessionEnd is skipped and Docker container stays running. → **Use async: true, timeout: 5 on the SessionEnd docker-compose down hook. Ensure docker-compose down completes in < 5s (set container stop-timeout: 2). Alternatively, don't clean up; let Docker restart policy handle it (restart: unless-stopped).**
- MCP tool hooks cannot be used at SessionStart to initialize the pet (because MCP servers haven't connected yet). If the pet relies on MCP tools for animation, it will fail to initialize. → **Use command hooks (shell script calling docker-compose) or HTTP hooks for initialization, not MCP tool hooks. MCP tools can be used for PostToolUse/Stop hooks (after servers connect), but not SessionStart/Setup.**
- If the pet Docker container is down or unresponsive, HTTP hooks will timeout silently. Claude Code will continue working, but users will see no animation feedback and may not notice the pet is offline. → **Log HTTP timeouts to stderr or a file (~/.claude/plugins/data/desktop-pet/pet.log) so user can diagnose. Add a /pet:status skill to check container health. Consider logging each hook event even if HTTP fails.**
- Hook configuration in plugin.json hooks/ is versioned with the plugin. If user updates the plugin and hooks change, the old hooks still fire until /reload-plugins or next session. → **Document 'After plugin update, run /reload-plugins to apply hook changes.' Avoid breaking hook changes in minor versions; use major version bumps for hook API changes.**
- If plugin name doesn't follow Claude Code rules (no spaces, not starting with claude-/anthropic-, no reserved words), validation fails silently and plugin won't load. → **Run `claude plugin validate ./desktop-pet-repo` before publishing. Keep name kebab-case (desktop-pet-notifier). Test with `--plugin-dir` first.**
- The Wayland socket and XDG_RUNTIME_DIR are in /run/user/1000 which is cleaned on logout. If the pet container reads stale WAYLAND_DISPLAY or XDG_RUNTIME_DIR after host suspend/resume, it will fail to connect to the compositor. → **Re-read WAYLAND_DISPLAY and XDG_RUNTIME_DIR at each SessionStart (pass fresh env vars via docker-compose). Or spawn the container only once and restart it on SessionStart (check if already running).**
- Hyprland monitor focus changes frequently (workspace switch, monitor hotplug). If the pet caches monitor info at SessionStart and doesn't poll, it will appear on the wrong monitor when user switches workspaces. → **Poll `hyprctl -j monitors` and `hyprctl -j activeworkspace` every 100ms in the pet's animation loop to track active monitor in real time. Update layer-shell output dynamically.**
- If the pet's HTTP server on 127.0.0.1:8888 crashes but the Docker container is still running, hooks will timeout and Claude will stall momentarily. If hooks is blocking (Stop), this could block user input. → **Never use exit code 2 (blocking) in Stop hooks. Use exit code 1 (non-blocking) or timeout. This ensures Claude continues even if pet is unresponsive. Monitor pet process health separately.**

## Perguntas em aberto

- Does Claude Code support dynamic hook reloading (e.g., when .claude/settings.json is edited during a session)? Or must the user always run /reload-plugins or restart?
- When Stop fires in `claude -p` (non-interactive mode), does last_assistant_message contain the full response or just a summary?
- Is there a way to pass the `project` name or `.claude/` metadata to hooks, or must the pet derive it from `cwd`?
- Can a hook HTTP endpoint receive binary data (e.g., image of the assistant's message) or only JSON?
- For private GitHub plugin marketplaces, does the plugin version field in plugin.json override the marketplace entry's version, and does that trigger a re-fetch?
- What happens if a plugin's hooks/hooks.json is invalid JSON? Does the plugin fail to load, or are hooks silently skipped?
- Does Claude Code support hot-reloading of plugin hooks in the current session (like /reload-plugins for skills), or must the plugin be uninstalled/reinstalled to pick up hook changes?
- If a user has multiple plugins with hooks, and two hooks match the same event with different matchers, does Claude Code run both or only the first?
- Can a hook's JSON output (e.g., additionalContext) inject instructions that persist across the rest of the session, or are they consumed per-turn?
- For Docker Compose in a plugin, should the compose file be inside CLAUDE_PLUGIN_ROOT (versioned) or in CLAUDE_PLUGIN_DATA (persistent across updates)?
