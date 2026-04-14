# 8. Memory Zones (Normative)

## Zone Scope
- `zone` introduces a bounded allocation region.
- Values allocated in a zone MUST not escape that zone unless rules explicitly allow it.

Zone scope validity is a static safety property.

## Allocator Access
- Zone allocator accessors are only valid within active zone scope.
- Out-of-scope allocator access MUST be rejected.

Qualified forms such as `<zone_name>::allocator()` MUST resolve consistently with active zone
context rules.

## Cleanup
Implementations MUST ensure zone resources are reclaimed at scope end.

Cleanup insertion MUST be deterministic across all control-flow exits from the zone.

## Nesting
Nested zones are valid; inner-zone resources MUST be reclaimed no later than outer-zone cleanup.
