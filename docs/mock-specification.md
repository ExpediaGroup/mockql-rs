# @mock Directive Specification

## 1. @mock

`@mock` is a directive that marks a field or operation for AI-generated mock data. When present, 
the annotated target's response data is produced by an AI model rather than resolved normally.

### Formal Definition

```graphql
directive @mock(hint: String) on QUERY | MUTATION | FIELD
```

## 2. Arguments

### hint

`hint` accepts an optional `String` value that provides guidance to the AI model for generating contextually appropriate mock data. 
If provided, the value must be of type `String`. A non-string value (e.g., `@mock(hint: 123)`) is invalid and must produce a validation error.

**Example:**

```graphql
{
  me @mock(hint: "return a user named Alice with 3 reviews") {
    name
    reviews {
      body
    }
  }
}
```

## 3. Valid Locations

`@mock` is valid on:

* **QUERY** -- Applied to a query operation definition. Indicates the entire query response should be AI-generated.
* **MUTATION** -- Applied to a mutation operation definition. Same semantics as QUERY.
* **FIELD** -- Applied to a field selection. Indicates that field and its entire subtree should be mocked.

`@mock` must not appear on:

* Fragment spreads
* Fragment definitions
* Inline fragments
* Introspection fields

## 4. Semantics

### 4.1 Operation-Level @mock

When `@mock` is applied to the operation definition, the entire response is AI-generated. Field-level `@mock` directives 
within the operation may still appear and provide additional hints for specific fields.

**Example:**

```graphql
query GetInfo @mock(hint: "test data for development") {
  me {
    id
    name
  }
  topProducts(first: 5) {
    name
    upc
  }
}
```

### 4.2 Field-Level @mock

When `@mock` is applied to a field, that field and its entire selection set subtree are AI-generated. 
Fields without `@mock` are resolved normally.

**Example:**

```graphql
query {
  me @mock(hint: "mock current user") {
    id
    name
    reviews {
      body
    }
  }
}
```

The field `me` and all of its nested selections (`id`, `name`, `reviews`, `reviews.body`) are AI-generated.

### 4.3 Partial Mocking

When some fields in an operation have `@mock` and others do not, the operation is partially mocked. 
Mocked fields and their entire subtrees are AI-generated, other fields are resolved normally. 
The two sets of results are merged into a single response.

**Example:**

```graphql
query {
  topProducts(first: 5) {
    upc
    name
    reviews @mock(hint: "positive reviews from verified buyers") {
      id
      body
    }
  }
}
```

`reviews` and its subtree are AI-generated; `upc` and `name` are resolved normally.

#### 4.3.1 Contextual mocking

When an operation is partially mocked, AI-generated data for mocked fields must be consistent with the values of 
non-mocked sibling and parent fields.

**Example:**

```graphql
query {
  topProducts(first: 5) {
    upc
    name
    reviews @mock(hint: "positive reviews from verified buyers") {
      id
      body
    }
  }
}
```

In this example, `upc` and `name` are resolved normally. The AI-generated `reviews` must be consistent with the actual 
`upc` and `name` values — e.g., if the product is `"Wireless Mouse"`, the reviews should reference that product.

### 4.4 Nested @mock Directives

`@mock` on a parent field and `@mock` on its descendant fields may coexist. Child-level `@mock` directives provide 
more specific hints for those fields. Both the parent and child are recorded as mocked fields.

**Example:**

```graphql
query {
  topProducts(first: 5) @mock(hint: "popular products") {
    upc
    name
    reviews @mock(hint: "positive reviews") {
      body
    }
  }
}
```

Both `topProducts` and its descendant `reviews` are recorded as mocked, each with its own hint.

The consistency guarantee from [4.3](#43-partial-mocking) does not apply to nested `@mock` fields, since the entire parent subtree 
is already AI-generated. There are no non-mocked sibling values to be consistent with.

## 5. Aliases and @mock

When a field has an alias, `@mock` applies to the aliased name in the response, not the underlying field name.

**Example:**

```graphql
query {
  first: topProducts @mock(hint: "first batch") {
    name
  }
  second: topProducts @mock(hint: "second batch") {
    name
  }
}
```

Each aliased field is mocked independently under its alias (`first` and `second`), not the underlying 
field name `topProducts`.

## 6. Fragments

Fields inside inline fragments or fragment definitions may have `@mock`. The fragment body is transparent — a mocked 
field inside a fragment is treated as if it appeared directly at the spread site.

**Example:**

```graphql
query GetProducts {
  topProducts(first: 5) {
    ...ProductInfo
  }
}

fragment ProductInfo on Product {
  name
  price
  reviews @mock(hint: "positive reviews") {
    id
    body
  }
}
```

The mocked field is `reviews`, spread under `topProducts`.

## 7. Validation Rules

### 7.1 Non-String Hint Argument

`hint` must be a `String`. Non-string values must produce a validation error.

**Counter-example (invalid):**

```graphql
query {
  me @mock(hint: 123) {
    name
  }
}
```

### 7.2 Introspection Fields

`@mock` must not be applied to introspection fields (fields whose name starts with `__`). Doing so must 
produce a validation error.

**Counter-example (invalid):**

```graphql
query {
  __schema @mock {
    types {
      name
    }
  }
}
```

### 7.3 Fragment Spreads

`@mock` must not be applied to fragment spreads. Doing so must produce a validation error.

**Counter-example (invalid):**

```graphql
query {
  topProducts(first: 5) {
    ...ProductFields @mock
  }
}
```

### 7.4 Fragment Definitions

`@mock` must not be applied to fragment definitions. Doing so must produce a validation error.

**Counter-example (invalid):**

```graphql
query {
  topProducts(first: 5) {
    ...ProductFields
  }
}

fragment ProductFields on Product @mock {
  name
}
```

### 7.5 Inline Fragments

`@mock` must not be applied to inline fragments. Doing so must produce a validation error.

**Counter-example (invalid):**

```graphql
query {
  topProducts(first: 5) {
    ... on Product @mock {
      name
    }
  }
}
```

### 7.6 Empty Selection Set

If all direct field children of a non-root selection set are individually mocked (and no ancestor is itself mocked), 
the result would be an empty selection set after field removal. This must produce a validation error.

However, this is permitted when an ancestor field is itself mocked, since the entire subtree is AI-generated.

**Counter-example (invalid):**

```graphql
query {
  me {
    reviews {
      id @mock(hint: "review id")
      body @mock(hint: "review body")
    }
  }
}
```

All fields under `me.reviews` have `@mock`, which would leave an empty selection set after removal.

**Valid (ancestor is mocked):**

```graphql
query {
  me @mock(hint: "mock user") {
    reviews {
      id @mock(hint: "review id")
      body @mock(hint: "review body")
    }
  }
}
```

This is allowed because `me` is mocked, so the entire subtree including `reviews` is AI-generated.

## 8. Mock Classification

An operation is classified into one of four scenarios based on `@mock` placement:

| Scenario                        | Condition                                                      |
|---------------------------------|----------------------------------------------------------------|
| **No Mocks**                    | No `@mock` directives present.                                 |
| **Operation Mocked**            | `@mock` on the operation definition.                           |
| **All Top-Level Fields Mocked** | Every top-level field has `@mock` (operation itself does not). |
| **Partial Mocked**              | Some (but not all) top-level fields have `@mock`.              |

Classification is determined as follows:

1. If validation errors are present, classification fails.
2. If `@mock` is present on the operation definition, the scenario is **Operation Mocked**.
3. If no mocked fields are found, the scenario is **No Mocks**.
4. If every top-level field in the operation has `@mock`, the scenario is **All Top-Level Fields Mocked**.
5. Otherwise, the scenario is **Partial Mocked**.

**Example -- No Mocks:**

```graphql
query {
  me {
    name
  }
}
```

**Example -- Operation Mocked:**

```graphql
query @mock(hint: "test data") {
  me {
    name
  }
  topProducts(first: 5) {
    name
  }
}
```

**Example -- All Top-Level Fields Mocked:**

```graphql
query {
  me @mock(hint: "mock user") {
    name
  }
  topProducts(first: 5) @mock(hint: "mock products") {
    name
  }
}
```

**Example -- Partial Mocked:**

```graphql
query {
  me {
    name
  }
  topProducts(first: 5) @mock(hint: "mock products") {
    name
  }
}
```

Only `topProducts` is mocked; `me` is resolved normally.

## 9. Schema Extension

`@mock` supports mocking fields and types that are not part of the current supergraph schema. A client may 
provide a GraphQL schema extension alongside the operation, as defined 
by the [GraphQL specification — Object Extensions](https://spec.graphql.org/September2025/#sec-Object-Extensions). 
The extended types and fields become available for use in operations with `@mock` directives.

### 9.1 Extending Existing Types

A schema extension may add new fields to existing types. The operation may then query these fields under `@mock`.

**Example:**

```graphql
# Schema extension
extend type Query {
  recommendedDestinations: [Destination]
}

type Destination {
  city: String
  country: String
}
```

```graphql
query {
  recommendedDestinations @mock(hint: "tropical vacation spots") {
    city
    country
  }
}
```

`recommendedDestinations` does not exist in the current schema. The schema extension makes it available, 
and `@mock` generates its data.

### 9.2 Defining New Types

A schema extension may introduce entirely new types, including object types, interfaces, and enums. 
These types may be referenced by extended fields.

**Example:**

```graphql
# Schema extension
interface Address {
  street: String
  city: String
}

type HomeAddress implements Address {
  street: String
  city: String
  isResidential: Boolean
}

extend type User {
  address: Address
}
```

```graphql
query {
  me @mock(hint: "user with a home address") {
    name
    address {
      street
      city
    }
  }
}
```

### 9.3 Interaction with Existing @mock Semantics

All existing `@mock` semantics apply to extended types and fields — including operation-level mocking, 
field-level mocking, partial mocking, nested directives, aliases, fragments and validation rules.

### 9.4 Validation

The schema extension must produce a valid schema when combined with the existing supergraph schema. 
If the extension is invalid — for example, it references undefined types or contains syntax errors — a validation error must be produced.
