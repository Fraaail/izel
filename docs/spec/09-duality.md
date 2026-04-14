# 9. Duality Types (Normative)

## Dual Declarations
`dual` declarations define paired encode/decode behavior over one structural source.

Dual declarations are elaborated into directional operations in lowering phases.

## Elaboration Rules
- Missing direction may be synthesized when derivation is valid.
- Unsupported derivation MUST produce diagnostics.

Synthesized forms MUST preserve declared generic and effect context where applicable.

## Round-Trip Law
Implementations MUST enforce or verify round-trip compatibility for dual representations.

For effectful paths, verification MAY be represented by generated tests or equivalent checks.

## Testing
Generated round-trip tests are part of conformance behavior for effectful dual declarations.

If generation is not possible due unsupported forms, implementations MUST emit diagnostics rather
than silently skipping safety checks.
