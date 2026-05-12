# MockQL Rust

Gen AI GraphQL response mocking via `@mock` directive.

## 📋 Specification

The `@mock` directive specification lives in [`docs/mock-specification.md`](./docs/mock-specification.md).

## 🎯 What It Does

`mockql` lets you annotate a GraphQL operation with `@mock` and generate only the mocked portions with an LLM provider.
The upstream GraphQL server does not need to define the `@mock` directive. `mockql` decorates the schema with it automatically while loading it (locally or via introspection).

Depending on where `@mock` appears, `mockql` will:

- Pass the operation straight through to the upstream GraphQL server
- Generate the full response with an LLM provider
- Fetch real upstream data for non-mocked fields, then merge in generated mock data for mocked fields

## 📦 Workspace Layout

- [`crates/mockql-core`](./crates/mockql-core): Core crate
- [`crates/mockql-cli`](./crates/mockql-cli): CLI crate and `mockql` binary
- [`docs/mock-specification.md`](./docs/mock-specification.md): `@mock` directive semantics
- [`docs/provider-cli.md`](./docs/provider-cli.md): provider prerequisites and exact CLI integration details
- [`docs/provider-http.md`](./docs/provider-http.md): provider prerequisites and exact CLI integration details
- [`examples/swapi`](./examples/swapi): SWAPI GraphQL examples
- [`examples/countries`](./examples/countries): Countries GraphQL examples

## 📜 Prerequisites

- One supported provider CLI installed and available on `PATH`
  - `claude`
  - `codex`
  - `opencode`
- Or an HTTP-backed provider with credentials.
  - `github-copilot` (requires `GITHUB_TOKEN` env variable)

## 🔨 Build

```bash
cargo build -p mockql
```

Run the binary with:

```bash
cargo run -p mockql-cli -- --help
```

Or directly after building:

```bash
./target/debug/mockql --help
```

## ⌨️ Usage


### oneshot
Execute a single GraphQL operation and print the response to stdout.
```bash
mockql oneshot --operation <file.graphql> --graphql-url <https://example.com/graphql> [options] cli|http
```

### proxy
start an HTTP proxy server that listens for GraphQL requests, exposing a graphiql interface at `http://localhost:<port>/graphiql`.
```bash
mockql proxy --port <port> --graphql-url <https://example.com/graphql> [options] cli|http
```

### schema
Load and print the decorated GraphQL schema with `@mock` directive
```bash
mockql schema (--schema <schema.graphql> | --graphql-url <https://example.com/graphql>) [options]
```

## Quick Start Examples

### SWAPI

```bash
cargo run -p mockql-cli -- oneshot \
  --operation ./examples/swapi/partial-list-items.graphql \
  --variables ./examples/swapi/partial-list-items.json \
  --graphql-url https://swapi-graphql.netlify.app/graphql \
  cli --provider codex --model gpt-5.4-mini
```

```bash
cargo run -p mockql-cli -- oneshot \
  --operation ./examples/swapi/partial-list-items.graphql \
  --variables ./examples/swapi/partial-list-items.json \
  --graphql-url https://swapi-graphql.netlify.app/graphql \
  cli --provider opencode --model github-copilot/gemini-3-flash-preview
```

### Countries API

```bash
cargo run -p mockql-cli -- oneshot \
  --operation ./examples/countries/contextual-hint.graphql \
  --variables ./examples/countries/contextual-hint.json \
  --graphql-url https://countries.trevorblades.com/ \
  cli --provider codex --model gpt-5.4-mini
```

For more example commands, see:

- [`examples/swapi/README.md`](./examples/swapi/README.md)
- [`examples/countries/README.md`](./examples/countries/README.md)
