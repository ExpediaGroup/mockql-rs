# AGENTS.md

`mockql-rs` — directive-driven GraphQL response mocking via `@mock`.

## Workspace Layout

- `crates/mockql-core/` — library: planning, prompts, upstream, providers, merge
  - `graphql/` — parse, extract `@mock`, filter ops, prompts, merge responses
  - `schema/` — SDL load + introspection
  - `llm_provider/` — model providers
  - `upstream/` — HTTP GraphQL client
  - `planner.rs`, `service.rs` — split + orchestration
- `crates/mockql-cli/` — `mockql` cli binary
- `xtask/` — dev tasks
- `docs/` — `@mock` spec, provider-cli docs

## Architecture

```mermaid
flowchart LR
    CLI[mockql-cli] --> SVC[MockService]
    SVC --> PLN[Planner<br/>split @mock]

    PLN -->|NoMocks| UP[Upstream GraphQLRequest<br/>without mocked fields]
    UP --> OUT[GraphQL Response]

    PLN -->|FullMock| PROV[LLM GraphQLRequest<br/>with mocked fields]
    PROV --> OUT

    PLN -->|PartialMock| UP
    UP -->|upstream data as context| PROV
    PROV --> MRG[Response Merger]
    MRG --> OUT

    linkStyle 2 stroke:#2196F3
    linkStyle 3 stroke:#2196F3
    linkStyle 4 stroke:#4CAF50
    linkStyle 5 stroke:#4CAF50
    linkStyle 6 stroke:#ff6600
    linkStyle 7 stroke:#ff6600
    linkStyle 8 stroke:#ff6600
    linkStyle 9 stroke:#ff6600
```
