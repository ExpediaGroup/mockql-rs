# Countries API Examples

These examples use the Countries API hosted at `https://countries.trevorblades.com/`.

Run them from the workspace root with `cargo run -p mockql-cli -- oneshot ...` or with a previously built `./target/debug/mockql oneshot ...`.

## Contextual Hint

This query uses specific `@mock(hint: "...")` directives to force custom values for country fields.

### CLI provider

```bash
cargo run -p mockql-cli -- oneshot \
  --operation ./examples/countries/contextual-hint.graphql \
  --variables ./examples/countries/contextual-hint.json \
  --graphql-url https://countries.trevorblades.com/ \
  cli \
    --provider claude \
    --model sonnet
```

### HTTP provider

```bash
cargo run -p mockql-cli -- oneshot \
  --operation ./examples/countries/contextual-hint.graphql \
  --variables ./examples/countries/contextual-hint.json \
  --graphql-url https://countries.trevorblades.com/ \
  http \
    --provider github-copilot \
    --model claude-sonnet-4
```

## Complete Interface Mocking

This query demonstrates mocking an entire `continent` object including its nested list of countries and languages.

### CLI provider

```bash
cargo run -p mockql-cli -- oneshot \
  --operation ./examples/countries/complete-interface.graphql \
  --variables ./examples/countries/complete-interface.json \
  --graphql-url https://countries.trevorblades.com/ \
  cli --provider claude --model sonnet
```

### HTTP provider

```bash
cargo run -p mockql-cli -- oneshot \
  --operation ./examples/countries/complete-interface.graphql \
  --variables ./examples/countries/complete-interface.json \
  --graphql-url https://countries.trevorblades.com/ \
  http --provider github-copilot --model claude-sonnet-4
```

## Schema Extension

This query fetches real country data while mocking a `travelAdvisory` field that is absent from the base Countries API schema. The `--schema-extension <SCHEMA_EXTENSION>` option points to a GraphQL schema extension file for mocking fields or types absent from the base schema.

### HTTP provider

```bash
cargo run -p mockql-cli -- oneshot \
  --operation ./examples/countries/schema-extension-example.graphql \
  --variables ./examples/countries/schema-extension-example.json \
  --schema-extension ./examples/countries/schema-extension.graphql \
  --graphql-url https://countries.trevorblades.com/ \
  http --provider github-copilot --model gemini-3.5-flash
```
