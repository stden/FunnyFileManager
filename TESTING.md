<!-- Purpose: Documents how the project test suite is organized and run. -->

# Testing Strategy

## Test pyramid

1. **Unit tests**
   - fastest
   - pure logic
   - no network or external services

2. **Integration tests**
   - filesystem, S3 adapters, external boundaries
   - use mocks or controlled fixtures when possible

3. **E2E / smoke tests**
   - only critical user flows
   - keep count small and stable

## Coverage policy

- Keep high coverage on business logic and core UI state handling.
- Exclude generated/runtime artifacts from coverage.
- Prevent regressions in critical modules.

## CI gates

- format must pass
- lint must pass
- tests must pass
- coverage must not regress on critical areas
