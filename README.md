# component-slot-extractor

Greentic WASM component that extracts typed slot values from a user utterance using regex patterns.

Part of the M2 messaging-endpoints train:
- Consumes `RoutingDirective::Dispatch.utterance` from `greentic-fast2flow`.
- Feeds `prefill` on `component-adaptive-card` (M2.3).

## Slot types

`string`, `enum`, `number`, `date`, `boolean` — see `schemas/io/input.schema.json` for the input shape.

## Build

```bash
make wasm
```

## Status

- PR 1 — scaffold skeleton (this PR). `extract_slots` is a no-op stub.
- PR 2 — regex extraction for all 5 slot types.
- PR 3 — wiring with `component-adaptive-card` prefill (M2.3).

## License

MIT
