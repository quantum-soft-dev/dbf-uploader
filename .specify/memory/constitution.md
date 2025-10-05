<!--
SYNC IMPACT REPORT:
Version: 0.0.0 → 1.0.0
Change Type: Initial constitution creation
Modified Principles: N/A (all new)
Added Sections: Core Principles (5), Technology Standards, Development Workflow, Governance
Removed Sections: None
Templates Requiring Updates:
  ✅ plan-template.md - Constitution Check section already references this file
  ✅ spec-template.md - No changes needed (tech-agnostic)
  ✅ tasks-template.md - TDD and testing requirements align
Follow-up TODOs: None
Rationale: Creating initial constitution for DBF Uploader project with Rust 1.79.0,
test-first development, Windows service reliability, and latest stable dependencies.
-->

# DBF Uploader Constitution

## Core Principles

### I. Test-First Development (NON-NEGOTIABLE)

All code MUST be developed following strict TDD (Test-Driven Development):
- Contract tests written first and MUST fail before implementation
- Unit tests written before implementation code
- Integration tests written to validate end-to-end workflows
- Red-Green-Refactor cycle: Write failing test → Make it pass → Refactor
- No implementation code committed without corresponding tests
- Test coverage is not a metric goal, but all critical paths MUST be tested

**Rationale**: Prevents regressions, ensures testability, validates design before implementation, provides living documentation.

### II. Modular Architecture

Code MUST be organized into clear, single-purpose modules:
- Each module has a single, well-defined responsibility
- Public interfaces are minimal and explicit
- Internal implementation details remain private
- Modules are independently testable
- Circular dependencies are prohibited

**Rationale**: Maintainability, testability, clear boundaries, easier reasoning about code, supports parallel development.

### III. Windows Service Reliability

Service implementation MUST prioritize robustness and operational reliability:
- Graceful handling of all service lifecycle events (start, stop, pause, continue)
- Proper cleanup on shutdown (no resource leaks)
- Configuration changes applied safely (at batch boundaries, not mid-operation)
- All errors logged with sufficient context for diagnosis
- Service must recover from transient failures (network, disk, locked files)
- No data loss on unexpected shutdown

**Rationale**: Long-running services must be stable, debuggable, and resilient to production environment issues.

### IV. Security & Data Safety

Security and data integrity are mandatory:
- All network communication MUST use HTTPS/TLS
- Credentials stored with appropriate OS-level access controls
- Sensitive data (JWT tokens) stored in memory only, never persisted
- Source data (DBF files) MUST never be modified or deleted
- All file operations must handle errors (permissions, disk full, locks)
- Input validation on all external data (config files, API responses)

**Rationale**: Protect user data, prevent data loss, comply with security best practices, maintain trust.

### V. Observability & Debuggability

All components MUST provide clear operational visibility:
- Structured logging using the `tracing` ecosystem
- Error messages include full context (file, operation, error chain)
- Local fallback logging when remote logging unavailable
- Log levels: ERROR for failures, WARN for recoverable issues, INFO for operations, DEBUG for details
- Errors reported to server with client version for diagnostics
- No silent failures

**Rationale**: Production debugging, incident response, performance monitoring, proactive issue detection.

## Technology Standards

### Language & Runtime
- **Rust Version**: 1.79.0 (or latest stable if newer)
- **Edition**: 2021
- **Target**: Windows 10+ and Windows Server 2016+ (x86_64)
- **Update Policy**: Minor and patch updates allowed, major Rust version updates require testing

### Dependency Management
- **Policy**: Use latest stable versions of all crates at time of addition
- **Update Frequency**: Review and update dependencies quarterly for security patches
- **Pinning**: Lock file (`Cargo.lock`) MUST be committed for reproducible builds
- **Evaluation Criteria**: Maturity, maintenance status, community adoption, security track record
- **Required Crates**:
  - `windows-service` - Windows service integration
  - `tokio` - Async runtime
  - `tokio-cron-scheduler` - Cron scheduling
  - `reqwest` (with `rustls-tls`) - HTTP client
  - `dbase` (with `yore` feature) - DBF parsing
  - `serde` / `toml` - Configuration
  - `csv` - CSV writing
  - `flate2` - Gzip compression
  - `thiserror` / `anyhow` - Error handling
  - `tracing` / `tracing-subscriber` - Logging
  - `notify-debouncer-mini` - File watching

### Code Quality Standards
- All code MUST pass `cargo clippy` with zero warnings
- All code MUST be formatted with `cargo fmt`
- Documentation required for all public APIs
- No `unsafe` code without explicit justification and review
- Error handling: Use `Result` types, avoid `unwrap()` and `expect()` in production code
- Prefer explicit error types (`thiserror`) over generic errors for library code

## Development Workflow

### Before Writing Code
1. Understand requirements from feature spec
2. Design data model and contracts
3. Write contract/integration tests (MUST fail)
4. Write unit tests for components (MUST fail)
5. Get test approval before implementation

### Implementation Process
1. Implement minimum code to make one test pass
2. Run tests to verify (green)
3. Refactor for clarity and maintainability
4. Repeat for next test
5. Run full test suite before commit

### Testing Requirements
- **Contract Tests**: Validate API endpoint behavior (auth, upload, error reporting)
- **Unit Tests**: Test individual modules in isolation
- **Integration Tests**: Validate service lifecycle, batch processing, config reload
- **Test Organization**: Mirror source structure in `tests/` directory
- **CI/CD**: All tests MUST pass before merge

### Code Review Checklist
- [ ] All tests pass (`cargo test`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Code formatted (`cargo fmt`)
- [ ] TDD process followed (tests written first)
- [ ] Error handling complete (no unwrap/expect without justification)
- [ ] Logging added for operations and errors
- [ ] Security considerations addressed
- [ ] No unsafe code (or justified and reviewed)
- [ ] Documentation updated

## Governance

### Amendment Process
1. Propose changes via documented rationale
2. Review impact on existing code and templates
3. Update dependent templates (plan, spec, tasks)
4. Increment version (MAJOR for breaking, MINOR for additions, PATCH for clarifications)
5. Update `LAST_AMENDED_DATE` to amendment date
6. Document changes in sync impact report

### Compliance Verification
- Constitution compliance checked during `/plan` command (Constitution Check section)
- Violations must be documented in Complexity Tracking with justification
- Unjustified violations block feature planning

### Version Control
- **Semantic Versioning**: MAJOR.MINOR.PATCH
  - **MAJOR**: Backward-incompatible principle changes, removals, redefinitions
  - **MINOR**: New principles, sections, or material expansions
  - **PATCH**: Clarifications, wording fixes, non-semantic refinements
- **History**: All amendments tracked in sync impact reports

### Enforcement
- All planning phases MUST reference this constitution
- Design decisions MUST align with core principles
- Deviations require explicit justification in Complexity Tracking
- Continuous integration enforces code quality standards

**Version**: 1.0.0 | **Ratified**: 2025-10-05 | **Last Amended**: 2025-10-05
