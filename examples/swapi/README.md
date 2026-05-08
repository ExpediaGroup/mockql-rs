# SWAPI Examples

These examples use the Star Wars API (SWAPI) hosted at `https://swapi-graphql.netlify.app/.netlify/functions/index`.

Run them from the workspace root with `cargo run -p mockql-cli -- ...` or with a previously built `./target/debug/mockql ...`.

## Partial List Items

This query demonstrates fetching real film data while mocking the `openingCrawl` and `director` fields.

### CLI provider

```bash
cargo run -p mockql-cli -- oneshot \           # Build and run the mockql binary, execute a single operation
  --operation ./examples/swapi/partial-list-items.graphql \   # GraphQL operation file to execute
  --variables ./examples/swapi/partial-list-items.json \      # JSON file with operation variables
  --graphql-url https://swapi-graphql.netlify.app/graphql \   # Target GraphQL endpoint for upstream requests and introspection
  cli \                                         # Use a local CLI agent as an LLM provider
    --provider opencode \                       # CLI backend: opencode
    --model github-copilot/gemini-3-flash-preview # Model: forwarded to the provider
```

### HTTP provider

```bash
cargo run -p mockql-cli -- oneshot \           # Build and run the mockql binary, execute a single operation
  --operation ./examples/swapi/partial-list-items.graphql \   # GraphQL operation file to execute
  --variables ./examples/swapi/partial-list-items.json \      # JSON file with operation variables
  --graphql-url https://swapi-graphql.netlify.app/graphql \   # Target GraphQL endpoint for upstream requests and introspection
  http \                                        # use an HTTP endpoint as an LLM provider
    --provider github-copilot \                 # HTTP backend: github-copilot (requires GITHUB_TOKEN)
    --model gemini-3-flash-preview              # Model: forwarded to the provider
```

## Nested Object Fields

This query demonstrates mocking fields deep within the graph, including nested species names for characters in a specific film.

### CLI provider

```bash
cargo run -p mockql-cli -- oneshot \           # Build and run the mockql binary, execute a single operation
  --operation ./examples/swapi/nested-object-fields.graphql \ # GraphQL operation file to execute
  --variables ./examples/swapi/nested-object-fields.json \    # JSON file with operation variables
  --graphql-url https://swapi-graphql.netlify.app/graphql \   # Target GraphQL endpoint for upstream requests and introspection
  cli \                                         # Use a local CLI agent as an LLM provider
    --provider claude \                         # CLI backend: claude
    --model sonnet                              # Model: forwarded to the provider
```

### HTTP provider

```bash
cargo run -p mockql-cli -- oneshot \           # Build and run the mockql binary, execute a single operation
  --operation ./examples/swapi/nested-object-fields.graphql \ # GraphQL operation file to execute
  --variables ./examples/swapi/nested-object-fields.json \    # JSON file with operation variables
  --graphql-url https://swapi-graphql.netlify.app/graphql \   # Target GraphQL endpoint for upstream requests and introspection
  http \                                        # use an HTTP endpoint as an LLM provider
    --provider github-copilot \                 # HTTP backend: github-copilot (requires GITHUB_TOKEN)
    --model gemini-3-flash-preview              # Model: forwarded to the provider
```
