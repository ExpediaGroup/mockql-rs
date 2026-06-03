# mockql

CLI for GenAI-powered GraphQL response mocking via the `@mock` directive.

[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![crates.io](https://img.shields.io/crates/v/mockql-cli.svg)](https://crates.io/crates/mockql-cli)

Mock data rots. Manual mocking is boring. Fixtures drift the moment a schema changes, and frontends sit blocked on backends that aren't ready. LLMs are bad at inventing API shapes and great at filling in shapes that already exist — and GraphQL gives them a bounded shape for free.

**mockql is not a server.** It's a thin, composable, standalone CLI that sits between your GraphQL client and your GraphQL server. A CLI is the smallest possible integration surface: any language, agent, script, or demo environment can run a process. No SDK, no Apollo link, no Relay, no frameworks. You hand it an operation, a schema, an upstream URL, and a provider — it hands you back a GraphQL response.

## See it in action

```graphql
query LoyaltyRewards @mock {
  loyaltyRewards {
    heading @mock(hint: "Platinum member") {
      text
    }
    subtitles @mock(hint: "At least 4 items") {
      text
      theme
    }
  }
}
```

You get back schema-valid JSON — no handwritten fixtures:

```json
{
  "data": {
    "loyaltyRewards": {
      "heading": { "text": "Welcome back, Platinum Member" },
      "subtitles": [
        { "text": "2,450 points until your next reward", "theme": "HIGHLIGHT" },
        { "text": "3 nights earned this quarter", "theme": "STANDARD" },
        { "text": "Breakfast included on your next stay", "theme": "POSITIVE" },
        { "text": "Suite upgrade available at checkout", "theme": "POSITIVE" }
      ]
    }
  }
}
```

## Contextual mocking

The real power shows up when you need *most* of a response from your actual backend, but one field isn't ready yet. Mock just that field — the rest resolves for real:

```graphql
query TripDetails($id: ID!) {
  trip(id: $id) {
    property {
      name
      address
    }
    recommendations @mock(hint: "5 most popular nearby restaurants") {
      title
      description
      distance
    }
  }
}
```

```json
{
  "data": {
    "trip": {
      "property": {
        "name": "Hotel Palazzo Pischedda",
        "address": "Via Roma, 09089 Bosa OR, Italy"
      },
      "recommendations": [
        {
          "title": "Ristorante Sa Pischedda",
          "description": "Located directly at the hotel, famous for its authentic Sardinian seafood dishes and local Vermentino wine.",
          "distance": "0.0 miles"
        },
        {
          "title": "Locanda di Corte",
          "description": "Charming restaurant in the historic medieval center, serving traditional Bosa cuisine in an intimate courtyard.",
          "distance": "0.3 miles"
        }
      ]
    }
  }
}
```

The real backend resolves the trip property; the LLM resolves recommendations *for that specific property*. Cohesive, realistic data — without a single hardcoded fixture.

## How it works

![How mockql turns @mock into one GraphQL response](docs/assets/mockql-sequence.png)

`mockql` sits between your client and server as a thin layer. For every request it will:

- **Parse** — the operation is parsed and validated against your schema using [apollo-compiler](https://crates.io/crates/apollo-compiler).
- **Split** — `@mock`-annotated fields are separated from real fields. Real fields are forwarded upstream as normal. (If you know GraphQL Federation, this feels a lot like query planning.)
- **Prompt** — the operation, mocked fields, hints, and the relevant schema subset are assembled into a structured prompt. The operation and schema constrain the output shape: the LLM can't hallucinate fields that don't exist or return a string where an enum is expected.
- **Merge** — real upstream data and LLM-generated mock data are stitched back into a single response.

## Install

Install the `mockql` binary with Cargo:

```bash
cargo install mockql-cli
```

Or download a prebuilt binary from the [releases page](https://github.com/ExpediaGroup/mockql-rs/releases).

You also need **one** LLM provider:

- **CLI** on your `PATH`, already authenticated: `claude`, `codex`, or `opencode`.
- **HTTP** with credentials: `github-copilot` (`GITHUB_TOKEN`) or `gemini` (`AUTH_TOKEN`).

See [`docs/provider-cli.md`](docs/provider-cli.md) and [`docs/provider-http.md`](docs/provider-http.md) for full details.

## Quick start

Mock fields against a live public GraphQL endpoint and print the result to stdout:

```bash
mockql oneshot \
  --operation ./examples/swapi/partial-list-items.graphql \
  --variables ./examples/swapi/partial-list-items.json \
  --graphql-url https://swapi-graphql.netlify.app/graphql \
  cli --provider claude
```

## Documentation
- [Getting started](https://opensource.expediagroup.com/mockql-rs)
- [`@mock` directive specification](docs/mock-specification.md)

## Examples
- [`swapi`](examples/swapi/README.md)
- [`countries`](examples/countries/README.md)
- [`pokemon`](examples/pokemon/README.md)
