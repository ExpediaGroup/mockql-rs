# HTTP Providers

`mockql` supports providers that expose an HTTP API for generating responses.

## Supported Providers

- `github-copilot`
- `gemini`

`github-copilot` sends an OpenAI-compatible chat-completions request. `gemini` sends a Gemini `generateContent` request to a user-provided Gemini or Gemini-compatible endpoint.

## Prerequisites

- A valid API token for the provider must be set as an environment variable.
- The machine running `mockql` must have outbound HTTPS access to the provider endpoint.

## Security Warning

HTTP provider credentials are sent over HTTPS. Treat these tokens with the same care as any other API credential. Do not commit them to source control or expose them in CI logs.

## Provider Selection

The HTTP provider is selected as a transport subcommand after the flat `oneshot` / `proxy` options. The `schema` subcommand does not use an LLM provider.

```bash
mockql oneshot [flat options] http github-copilot --model <model>
mockql oneshot [flat options] http gemini --url <url> --auth-header <header-name> [--auth-value-env-var <env-var>]
```

## How `mockql` Uses Each Provider

Verified against the current `mockql` source.

| Need             | GitHub Copilot                                   | Gemini-compatible                                           |
|------------------|--------------------------------------------------|-------------------------------------------------------------|
| Provider command | `http github-copilot --model <model>`            | `http gemini --url <url> --auth-header <header-name> [--auth-value-env-var <env-var>]` |
| API endpoint     | `https://api.githubcopilot.com/chat/completions` | User-provided `--url`                                       |
| Authentication   | `Authorization: Bearer $GITHUB_TOKEN`            | `<auth-header>: $<auth-value-env-var>`                      |
| Response text    | `choices[0].message.content`                     | `candidates[0].content.parts[0].text`                       |

## Exact Requests `mockql` Executes

### GitHub Copilot

```text
POST https://api.githubcopilot.com/chat/completions
Authorization: Bearer $GITHUB_TOKEN
Content-Type: application/json

{
  "model": "<model>",
  "messages": [{ "role": "user", "content": "<prompt_markdown>" }],
  "stream": false
}
```

Notes:

- If `GITHUB_TOKEN` is not set, `mockql` fails immediately with a `MissingEnv` error before making any request.

### Gemini-compatible

```text
POST <url>
<auth-header>: $<auth-value-env-var>
Content-Type: application/json

{
  "contents": [
    {
      "role": "user",
      "parts": [{ "text": "<prompt_markdown>" }]
    }
  ]
}
```

Notes:

- If the configured auth token environment variable is not set, `mockql` fails immediately with a `MissingEnv` error before making any request.
- `--auth-value-env-var` defaults to `GEMINI_AUTH_VALUE`.
- For Google Gemini API, set the configured auth token environment variable to the API key and use `--auth-header x-goog-api-key`.
- For bearer-token compatible endpoints, set the configured auth token environment variable to the full header value, for example `Bearer <token>`, and use `--auth-header Authorization`.
- The model is encoded in the Gemini endpoint URL, for example `/models/gemini-3.5-flash:generateContent`.

## Manual Provider Testing

For quick local experiments, use [`docs/prompt-example.md`](./prompt-example.md).

### GitHub Copilot

```bash
curl -s https://api.githubcopilot.com/chat/completions \
  -H "Authorization: Bearer $GITHUB_TOKEN" \
  -H "Content-Type: application/json" \
  -d "$(jq -n --arg prompt "$(cat ./docs/prompt-example.md)" --arg model "gemini-3-flash-preview" \
    '{model: $model, messages: [{role: "user", content: $prompt}], stream: false}')" \
  | jq
```

### Gemini-compatible

```bash
curl -s "https://generativelanguage.googleapis.com/v1beta/models/gemini-3.5-flash:generateContent" \
  -H "x-goog-api-key: $GEMINI_AUTH_VALUE" \
  -H "Content-Type: application/json" \
  -d "$(jq -n --arg prompt "$(cat ./docs/prompt-example.md)" \
    '{contents: [{role: "user", parts: [{text: $prompt}]}]}')" \
  | jq
```
