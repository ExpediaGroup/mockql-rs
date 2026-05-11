# Countries API Examples

These examples use the Countries API hosted at `https://countries.trevorblades.com/`.

Run them from the workspace root with `cargo run -p mockql-cli -- oneshot ...` or with a previously built `./target/debug/mockql oneshot ...`.

## Contextual Hint

This query uses specific `@mock(hint: "...")` directives to force custom values for country fields.

### CLI provider

```bash
cargo run -p mockql-cli -- oneshot \           # Build and run the mockql binary, execute a single operation
  --operation ./examples/countries/contextual-hint.graphql \  # GraphQL operation file to execute
  --variables ./examples/countries/contextual-hint.json \     # JSON file with operation variables
  --graphql-url https://countries.trevorblades.com/ \         # Target GraphQL endpoint for upstream requests and introspection
  cli \                                         # Use a local CLI agent as an LLM provider
    --provider claude \                         # CLI backend: claude
    --model sonnet                              # Model: forwarded to the provider
```

### HTTP provider

```bash
cargo run -p mockql-cli -- oneshot \           # Build and run the mockql binary, execute a single operation
  --operation ./examples/countries/contextual-hint.graphql \  # GraphQL operation file to execute
  --variables ./examples/countries/contextual-hint.json \     # JSON file with operation variables
  --graphql-url https://countries.trevorblades.com/ \         # Target GraphQL endpoint for upstream requests and introspection
  http \                                        # use an HTTP endpoint as an LLM provider
    --provider github-copilot \                 # HTTP backend: github-copilot (requires GITHUB_TOKEN)
    --model claude-sonnet-4                     # Model: forwarded to the provider
```

## Complete Interface Mocking

This query demonstrates mocking an entire `continent` object including its nested list of countries and languages.

### CLI provider

```bash
cargo run -p mockql-cli -- oneshot \           # Build and run the mockql binary, execute a single operation
  --operation ./examples/countries/complete-interface.graphql \  # GraphQL operation file to execute
  --variables ./examples/countries/complete-interface.json \     # JSON file with operation variables
  --graphql-url https://countries.trevorblades.com/ \           # Target GraphQL endpoint for upstream requests and introspection
  cli \                                         # Use a local CLI agent as an LLM provider
    --provider claude \                         # CLI backend: claude
    --model sonnet                              # Model: forwarded to the provider
```

### HTTP provider

```bash
cargo run -p mockql-cli -- oneshot \           # Build and run the mockql binary, execute a single operation
  --operation ./examples/countries/complete-interface.graphql \  # GraphQL operation file to execute
  --variables ./examples/countries/complete-interface.json \     # JSON file with operation variables
  --graphql-url https://countries.trevorblades.com/ \           # Target GraphQL endpoint for upstream requests and introspection
  http \                                        # use an HTTP endpoint as an LLM provider
    --provider github-copilot \                 # HTTP backend: github-copilot (requires GITHUB_TOKEN)
    --model claude-sonnet-4                     # Model: forwarded to the provider
```
