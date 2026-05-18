# CLI Providers

`mockql` can use installed assistant CLIs as LLM backends.

## Supported Providers

- `claude`
- `codex`
- `opencode`

The provider executable must be installed, available on `PATH`, and already authenticated.

## Usage

The CLI provider is selected after the flat `oneshot` or `proxy` options. The `schema` subcommand does not use an LLM provider.

```bash
mockql oneshot [flat options] cli --provider <claude|codex|opencode> [--model <model>]
```

## Defaults

- `cli --provider claude` defaults to `sonnet`
- `cli --provider codex` defaults to `gpt-5.4-mini`
- `cli --provider opencode` defaults to `github-copilot/gemini-3-flash-preview`

## Security

The CLI integration runs providers in unattended automation modes. Use care when running `mockql` in repositories with sensitive source code, secrets, credentials, or production-adjacent configuration.

For the full provider notes, see [`docs/provider-cli.md`](https://github.com/ExpediaGroup/mockql-rs/blob/main/docs/provider-cli.md).
