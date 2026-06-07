# mockql-cli

The `mockql` command-line binary. A thin, composable layer that decorates any GraphQL
server with GenAI-powered response mocking via the `@mock` directive.

This crate is deliberately small: it parses arguments, loads a schema, builds a
provider configuration, and hands everything to [`mockql-core`](../mockql-core), which
does the real work (planning, prompting, upstream calls, merging). If you want to
understand *how* mocking happens, read that crate's README. This one is about the
**surface a user touches**.

A CLI is the smallest possible integration surface: any language, agent, script, or
demo environment can run a process.

## The three subcommands

`main.rs` defines one `clap` command with three subcommands. Each has its own module
under `commands/` with an `args.rs` (the flags) and a `mod.rs` (the `exec_*` function).

| Subcommand    | What it does                                                                     |
|---------------|----------------------------------------------------------------------------------|
| **`oneshot`** | Execute a single operation and print the result to stdout                        |
| **`proxy`**   | Run an HTTP server that mocks GraphQL responses per request (with a GraphiQL UI) |
| **`schema`**  | Print a schema decorated with the `@mock` directive (optionally minified)        |

`oneshot` and `proxy` are the same engine wrapped two ways — single-shot vs. long-running. 
`schema` doesn't execute anything, it's a schema-inspection helper.

## Glossary

The argument-level vocabulary a user encounters:

| Term                     | What it means                                                                                |
|--------------------------|----------------------------------------------------------------------------------------------|
| **Transport**            | How to reach the LLM: a local `cli` provider or an `http` provider                           |
| **Provider**             | The specific backend — `claude`/`codex`/`opencode` (CLI) or `github-copilot`/`gemini` (HTTP) |
| **Serialization format** | `json` or `toon` — how generated data is requested and parsed                                |
| **Schema source**        | A local SDL file *or* a URL to introspect                                                    |
| **Schema extension**     | Extra SDL applied before planning, so you can `@mock` not-yet-real fields                    |
| **Variables**            | Operation variables, read from a JSON file                                                   |
| **Headers**              | `name:value` pairs forwarded to the upstream GraphQL server                                  |

The shared argument plumbing is the glue worth knowing: it's where CLI flags become the
`mockql-core` types (`ProviderConfig`, `Header`, `SerializationFormat`, the variables
map, and the schema-extension payload) — translating user-facing strings into the
strongly-typed inputs `MockService` expects.

## How a command runs

1. `main.rs` parses args and dispatches to one of `exec_oneshot` / `exec_proxy` /
   `exec_schema`.
2. The command loads a schema (`load_schema`), builds a `reqwest::Client`, and — for
   `oneshot`/`proxy` — turns the chosen transport into a `ProviderConfig` via
   `TransportArg::into_provider_config`.
3. It constructs a `MockService` and either calls `execute` once (`oneshot`), serves it
   over HTTP (`proxy`), or just prints the schema (`schema`).
4. Errors bubble up as `CliError`; `main` prints them to stderr and returns a non-zero
   exit code.

## Install & usage

```bash
cargo install mockql-cli   # installs the `mockql` binary
```

Prerequisites, provider setup, end-to-end examples, and the directive spec live in the
top-level docs rather than here, to avoid duplication:

- [Project README](../../README.md) — install, prerequisites, demos
- [`@mock` directive specification](../../docs/mock-specification.md)
- [CLI providers](../../docs/provider-cli.md) · [HTTP providers](../../docs/provider-http.md)
- Examples: [`swapi`](../../examples/swapi/README.md), [`countries`](../../examples/countries/README.md), [`pokemon`](../../examples/pokemon/README.md)
