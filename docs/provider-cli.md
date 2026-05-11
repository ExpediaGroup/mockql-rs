# CLI Providers

`mockql` can use installed assistant CLIs as LLM backends.

## Supported Providers

- `claude`
- `codex`
- `opencode`

`mockql` executes these providers in non-interactive script mode. For Claude and Codex the generated prompt is piped to stdin; for OpenCode it is passed as a positional argument.

## Prerequisites

- The provider executable must be installed and available on `PATH`
- The provider must already be authenticated in whatever way that CLI expects
- The machine running `mockql` must be allowed to launch unattended provider sessions

## Security Warning

The current integration uses provider modes intended for unattended automation:

- Codex: `--yolo`
- Claude: `--dangerously-skip-permissions`
- OpenCode: none required (the `opencode run` subcommand is non-interactive by design)

That means the provider CLI is being invoked without interactive permission prompts. Do not treat this as a no-risk default. Use care when running `mockql` in repositories with sensitive source code, secrets, credentials, or production-adjacent configuration.

## Model Selection

The CLI provider is selected as a transport subcommand after the flat `oneshot` / `proxy` options. The `schema` subcommand does not use an LLM provider.

```
mockql oneshot [flat options] cli --provider <claude|codex|opencode> [--model <model>]
```

- `--provider` is **required**
- `--model` is optional and defaults based on `--provider`:
  - `cli --provider claude` => `sonnet`
  - `cli --provider codex` => `gpt-5.4-mini`
  - `cli --provider opencode` => `github-copilot/gemini-3-flash-preview`

## How `mockql` Uses Each Provider

Verified against the current `mockql` source and local CLI help output.

| Need                          | Codex                                                                                         | Claude                                                       | OpenCode                                           | Notes                                                                                                       |
|-------------------------------|-----------------------------------------------------------------------------------------------|--------------------------------------------------------------|----------------------------------------------------|-------------------------------------------------------------------------------------------------------------|
| Non-interactive mode          | `codex exec [PROMPT]`                                                                         | `claude --print [prompt]`                                    | `opencode run [PROMPT]`                            | `mockql` uses `codex exec`, `claude --print`, and `opencode run`.                                           |
| Read prompt from stdin        | `codex exec -`                                                                                | `claude --print -`                                           | N/A — prompt is a positional argument              | OpenCode receives the prompt as a CLI argument, not via stdin.                                              |
| Pass model                    | `-m, --model <MODEL>`                                                                         | `--model <MODEL>`                                            | `--model <MODEL>`                                  | All three support direct model selection.                                                                   |
| Reasoning setting             | `--config model_reasoning_effort="low"`                                                       | `--effort low`                                               | N/A                                                | OpenCode does not expose a reasoning-effort flag.                                                           |
| Output mode                   | `--json` plus `--output-last-message <path>`                                                  | `--output-format json`                                       | Raw JSON on stdout                                 | Codex reads a temp file; Claude and OpenCode read stdout.                                                   |
| Permission bypass mode used   | `--yolo`                                                                                      | `--dangerously-skip-permissions`                             | None needed                                        | `opencode run` is non-interactive by design.                                                                |

## Exact Commands `mockql` Executes

These are the commands currently constructed by the library. The prompt content is generated at runtime and piped to stdin (or passed as a positional argument for OpenCode).

### Codex

```bash
codex exec --skip-git-repo-check --yolo -m <model> --config 'model_reasoning_effort="low"' --json --output-last-message <temp>/last-message.json -
```

Notes:

- `--skip-git-repo-check` avoids requiring the current directory to be a git repository
- `--output-last-message` writes the final assistant message to a temp file, which `mockql` parses as the GraphQL response
- If no model is provided, `mockql` defaults to `gpt-5.4-mini`

### Claude

```bash
claude --print --dangerously-skip-permissions --model <model> --effort low --output-format json -
```

Notes:

- `--print` runs Claude in non-interactive mode
- `mockql` parses stdout as JSON and reads the `result` field from the response envelope
- If Claude returns an error envelope, `mockql` surfaces that as a request error
- If no model is provided, `mockql` defaults to `sonnet`

### OpenCode

```bash
opencode run <prompt> --model <model>
```

Notes:

- The prompt is passed as a positional argument to `opencode run`, not via stdin
- `mockql` reads JSON directly from stdout and parses it as a `GraphQLResponse`
- No permission-bypass or reasoning-effort flags are needed
- If no model is provided, `mockql` defaults to `github-copilot/gemini-3-flash-preview`

## Manual Provider Testing

For quick local experiments, use [`docs/prompt-example.md`](./prompt-example.md).

### Codex

```bash
codex exec --skip-git-repo-check --yolo -m gpt-5.4-mini --config 'model_reasoning_effort="low"' --json --output-last-message mock-response.codex.json - < ./docs/prompt-example.md
```

### Claude

```bash
claude --print --dangerously-skip-permissions --model opus --effort low --output-format json - < ./docs/prompt-example.md > mock-response.claude.json
```

### OpenCode

```bash
opencode run "$(cat ./docs/prompt-example.md)" --model github-copilot/gemini-3-flash-preview > mock-response.opencode.json
```
