# HTTP Providers

`mockql` supports providers that expose an HTTP API for generating responses.

## Supported Providers

- `github-copilot`

## Usage

The HTTP provider is selected after the flat `oneshot` or `proxy` options. The `schema` subcommand does not use an LLM provider.

```bash
mockql oneshot [flat options] http --provider <github-copilot> --model <model>
```

## GitHub Copilot

`mockql` sends a chat-completions request to GitHub Copilot:

```text
POST https://api.githubcopilot.com/chat/completions
Authorization: Bearer $GITHUB_TOKEN
Content-Type: application/json
```

If `GITHUB_TOKEN` is not set, `mockql` fails before making a request.

## Security

The `GITHUB_TOKEN` is sent as a Bearer token over HTTPS. Treat it like any other API credential. Do not commit it to source control or expose it in CI logs.

For the full provider notes, see [`docs/provider-http.md`](https://github.com/ExpediaGroup/mockql-rs/blob/main/docs/provider-http.md).
