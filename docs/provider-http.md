# HTTP Providers

`mockql` supports providers that expose an HTTP API for generating responses.

## Supported Providers

- `github-copilot`

`mockql` sends an POST request to the provider's chat-completions endpoint and parses the response JSON.

## Prerequisites

- A valid API token for the provider must be set as an environment variable
- The machine running `mockql` must have outbound HTTPS access to the provider endpoint

## Security Warning

The `GITHUB_TOKEN` is sent as a Bearer token in the `Authorization` header over HTTPS. Treat this token with the same care as any other API credential. Do not commit it to source control or expose it in CI logs.

## Model Selection

The HTTP provider is selected as a transport subcommand after the flat `oneshot` / `proxy` options. The `schema` subcommand does not use an LLM provider.

```
mockql oneshot [flat options] http --provider <github-copilot> --model <model>
```

- `--provider` is **required**
- `--model` is **required**

## How `mockql` Uses Each Provider

Verified against the current `mockql` source.

| Need                          | GitHub Copilot                                                         | Notes                                                                             |
|-------------------------------|------------------------------------------------------------------------|-----------------------------------------------------------------------------------|
| API endpoint                  | `https://api.githubcopilot.com/chat/completions`                       | OpenAI-compatible chat completions API.                                           |
| Authentication                | `Authorization: Bearer $GITHUB_TOKEN`                                  | Token read from the `GITHUB_TOKEN` env var.                                       |

## Exact Requests `mockql` Executes

### GitHub Copilot

```
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

- If `GITHUB_TOKEN` is not set, `mockql` fails immediately with a `MissingEnv` error before making any request

## Manual Provider Testing

For quick local experiments, use [`docs/prompt-example.md`](/Users/samvazquez/www/opensource/mockql-rs/docs/prompt-example.md).

### GitHub Copilot

```bash
curl -s https://api.githubcopilot.com/chat/completions \
  -H "Authorization: Bearer $GITHUB_TOKEN" \
  -H "Content-Type: application/json" \
  -d "$(jq -n --arg prompt "$(cat ./docs/prompt-example.md)" --arg model "gemini-3-flash-preview" \
    '{model: $model, messages: [{role: "user", content: $prompt}], stream: false}')" \
  | jq
```
