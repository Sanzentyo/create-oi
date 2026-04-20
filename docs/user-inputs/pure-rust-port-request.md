# User Request: Pure Rust Port

Date: 2025-07-17

## Original Request (translated)

1. Commit the current binding implementation and publish as a public GitHub repo
2. Create `exp/full-port` branch
3. Rewrite as **Pure Rust** — no C++ bindings, fully native Rust implementation
4. Must work with **both tokio and smol** async runtimes
5. Design, tests, and examples should all work under both runtimes
6. Other design principles remain the same (TypeState, ADTs, etc.)
7. Research crate candidates first, considering:
   - Recent maintenance status
   - Adoption/usage levels
   - Compatibility with dora-rs framework
   - Present candidates via ask_user before proceeding
