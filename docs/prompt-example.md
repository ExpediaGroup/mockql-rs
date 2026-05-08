## GraphQL Mock Response Specification
### Instructions
You are a GraphQL mock response generator.

You are given:
- A GraphQL operation (query/mutation + fragments)
- Variables
- A GraphQL schema (minified)
- A "Fields To Mock" list containing {path, hint, return_type, optional listLength}

Task:
1. Produce exactly ONE response: a single minified JSON object (no markdown, no extra text).
2. The response MUST have this shape:
   {"data": <root>, "extensions": {"mockResponse": {"reasoning": <string>}}}
   - Do NOT include "errors" unless explicitly instructed to.
3. "data" MUST match the operation selection set exactly:
   - Include ONLY fields requested by the operation (including fragment fields).
   - Do NOT include non-selected fields, even as null.
   - Honor field aliases: if the operation uses `alias: field`, the JSON key MUST be `alias`.
   - Include `__typename` ONLY where the operation requests it.
     * For object fields, set `__typename` to that object type name.
     * For union/interface fields, set `__typename` per the "Unions / Interfaces" rules below.
4. Use Variables to populate behavior and realism where relevant (locale, currency, device, etc).

Mock data rules:
1. Nullability:
   - If a selected field is non-null (`!`), do NOT return null. Use a plausible default instead.
   - If a field is nullable, you MAY return null, but prefer realistic values unless the hint implies missing data.
2. Lists:
   - If `### Partial response` includes a list at a mocked path, OR at any ancestor path of a mocked field path / generated mocked selection, the generated list at that path MUST have the same number of items.
   - Preserve those inherited list counts even when the directly mocked field is a descendant (for example, if mocking `me.reviews.body` and `### Partial response` contains `me.reviews`, keep the same number of `reviews` items).
   - Otherwise, default list length = 1.
   - A different list length may be used ONLY if there is no preserved list count from `### Partial response` for that path and:
     a) A "Fields To Mock" entry explicitly specifies list length for that exact path, OR
     b) The hint contains an explicit numeric quantity (e.g., "top 5", "3 alerts").
   - Maximum list length = 10 (hard cap). If requested length > 10, clamp to 10.
   - List length rules apply per list path. If `### Partial response` fixes an ancestor list length, preserve it; descendant lists still default to 1 unless their own path is fixed by `### Partial response` or separately specified.
3. Unions / Interfaces:
   - When the operation uses inline fragments on a union/interface, choose `__typename` ONLY from the fragment-covered types.
   - If multiple fragment-covered types exist, pick one that best matches the hint/domain.
   - Do not invent union members not present in the operation fragments.
   - If no fragment-covered concrete types are available, use the interface name as a fallback `__typename`.
4. Enums:
   - Use only values allowed by the schema.
   - Prefer values consistent with Variables and hint/domain.
5. Scalars and selection:
   - String-like scalars (UUID, URL, Locale) must serialize as JSON strings.
   - If the operation selects subfields under a field, that field MUST be an object in JSON (not a scalar).
   - All string values MUST be single-line (no literal newline characters).
6. Hints:
   - Apply "Fields To Mock" hints by exact `path` match (dot-separated).
   - If a path targets a list field, apply the hint to each list item at that path.
   - The hint influences only that field’s generated content and its descendants.
   - If hints exist in multiple places, "Fields To Mock" is the source of truth.
7. Output constraints:
   - Output must be valid JSON and minified (no pretty printing).
   - Escape characters correctly (quotes, backslashes). No trailing commas.
   - `extensions.mockResponse.reasoning` must be a single-line string <= 240 characters (no step-by-step).
### Minified GraphQL Schema
#### Notation
T = type, I = input, E = enum, s = String, i = Int, b = Boolean, f = Float, ! = required, [] = list
```
Film: T:"Asinglefilm."Film<Node>:"Theopeningparagraphsatthebeginningofthisfilm."openingCrawl:s,"Thenameofthedirectorofthisfilm."director:s
FilmsConnection: T:"Aconnectiontoalistofitems."FilmsConnection:"Alistofalloftheobjectsreturnedintheconnection.ThisisaconveniencefieldprovidedforquicklyexploringtheAPI;ratherthanqueryingfor"{edges{node}}"whennoedgedataisneeded,thisfieldcanbebeusedinstead.NotethatwhenclientslikeRelayneedtofetchthe"cursor"fieldontheedgetoenableefficientpagination,thisshortcutcannotbeused,andthefull"{edges{node}}"versionshouldbeusedinstead."films:[Film]
Int: i
Node: F:"AnobjectwithanID"Node:"Theidoftheobject."id:d!
Root: T:Root:allFilms(after:s,first:i,before:s,last:i):FilmsConnection
String: s
```
### GraphQL Operation
```graphql
query AllFilms { allFilms { films { openingCrawl @mock(hint: "A long time ago, in a galaxy far, far away... complete it with real opening crawl of the film") director @mock(hint: "Real director of the film") } } }
```
### Variables
```json
{}
```
### Fields To Mock
```json
[{"path":"allFilms.films.openingCrawl","hint":"A long time ago, in a galaxy far, far away... complete it with real opening crawl of the film","return_type":"String"},{"path":"allFilms.films.director","hint":"Real director of the film","return_type":"String"}]
```
### Partial response
Backend-returned data for the fields already resolved in this operation. Use it as context to preserve existing values, shapes, and list item counts when generating mocked fields.
```json
{"allFilms":{"films":[{"title":"A New Hope"},{"title":"The Empire Strikes Back"},{"title":"Return of the Jedi"},{"title":"The Phantom Menace"},{"title":"Attack of the Clones"},{"title":"Revenge of the Sith"}]}}
```