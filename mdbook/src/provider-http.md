# HTTP Providers

`mockql` supports providers that expose an HTTP API for generating responses.

## Supported Providers

- `github-copilot`
- `gemini`

## Usage

The HTTP provider is selected after the flat `oneshot` or `proxy` options. The `schema` subcommand does not use an LLM provider.

```bash
mockql oneshot [flat options] http github-copilot --model <model>
mockql oneshot [flat options] http gemini --url <url> --auth-header <header-name> [--auth-value-env-var <env-var>]
```

## GitHub Copilot

`mockql` sends a chat-completions request to GitHub Copilot:

```text
POST https://api.githubcopilot.com/chat/completions
Authorization: Bearer $GITHUB_TOKEN
Content-Type: application/json
```

If `GITHUB_TOKEN` is not set, `mockql` fails before making a request.

## Gemini-compatible

`mockql` sends a Gemini `generateContent` request to the provided URL:

```text
POST <url>
<auth-header>: $<auth-value-env-var>
Content-Type: application/json
```

`--auth-value-env-var` defaults to `GEMINI_AUTH_VALUE`. For Google Gemini API, set the configured auth token environment variable to the API key and use `--auth-header x-goog-api-key`. For bearer-compatible endpoints, set the configured auth token environment variable to the full value, for example `Bearer <token>`, and use `--auth-header Authorization`.

## Security

HTTP provider credentials are sent over HTTPS. Treat them like any other API credential. Do not commit them to source control or expose them in CI logs.

For the full provider notes, see [`docs/provider-http.md`](https://github.com/ExpediaGroup/mockql-rs/blob/main/docs/provider-http.md).
