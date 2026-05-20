# Pokemon API Examples

These examples use the PokeAPI GraphQL server hosted at `https://graphql.pokeapi.co/v1beta2`.

Run them from the workspace root with `cargo run -p mockql-cli -- oneshot ...` or with a previously built `./target/debug/mockql oneshot ...`.

## List Pokemon

This query fetches real Pokedex data for the first Pokemon while mocking their display names.

### CLI provider

```bash
cargo run -p mockql-cli -- oneshot \
  --operation ./examples/pokemon/list-pokemon.graphql \
  --variables ./examples/pokemon/list-pokemon.json \
  --graphql-url https://graphql.pokeapi.co/v1beta2 \
  cli --provider claude --model sonnet
```

### HTTP provider

```bash
cargo run -p mockql-cli -- oneshot \
  --operation ./examples/pokemon/list-pokemon.graphql \
  --variables ./examples/pokemon/list-pokemon.json \
  --graphql-url https://graphql.pokeapi.co/v1beta2 \
  http --provider github-copilot --model claude-sonnet-4
```

## Kanto Pokedex

This query fetches real Kanto Pokedex entries while mocking the nested Pokemon species names.

### CLI provider

```bash
cargo run -p mockql-cli -- oneshot \
  --operation ./examples/pokemon/kanto-pokedex.graphql \
  --variables ./examples/pokemon/kanto-pokedex.json \
  --graphql-url https://graphql.pokeapi.co/v1beta2 \
  cli --provider claude --model sonnet
```

### HTTP provider

```bash
cargo run -p mockql-cli -- oneshot \
  --operation ./examples/pokemon/kanto-pokedex.graphql \
  --variables ./examples/pokemon/kanto-pokedex.json \
  --graphql-url https://graphql.pokeapi.co/v1beta2 \
  http --provider github-copilot --model gemini-3.5-flash
```

## Schema Extension

This query fetches real Pokemon data while mocking a `trainerNotes` field that is absent from the base PokeAPI schema. The `--schema-extension <SCHEMA_EXTENSION>` option points to a GraphQL schema extension file for mocking fields or types absent from the base schema.

### HTTP provider

```bash
cargo run -p mockql-cli -- oneshot \
  --operation ./examples/pokemon/schema-extension-example.graphql \
  --variables ./examples/pokemon/schema-extension-example.json \
  --schema-extension ./examples/pokemon/schema-extension.graphql \
  --graphql-url https://graphql.pokeapi.co/v1beta2 \
  http --provider github-copilot --model gemini-3.5-flash
```
