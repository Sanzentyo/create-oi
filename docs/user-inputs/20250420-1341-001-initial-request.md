# Initial Request

## Date
2026-04-20

## User Request (Original)
justのskillsもloadして、実行しなさい。これは、 https://github.com/AutonomyLab/libcreate.git のRustラッパーを作成するクレートです。TypeStateパターンとか、ADTみたいなRustらしいクレートになるようにしなさい。こちらのC/C++のbuild自体はzigbuildとかを使ってやって欲しい。こちらの入出力は、docs/user-inputsに保存し、それ以外の進捗などもdocsに構造化して書き込んでいきなさい

## Summary
- Create a Rust wrapper crate for [libcreate](https://github.com/AutonomyLab/libcreate.git)
- Use TypeState pattern and ADT (Algebraic Data Types) for idiomatic Rust design
- Use zigbuild (zig as C/C++ compiler) for building the C/C++ code
- Save user inputs/outputs to `docs/user-inputs/`
- Track progress and other documentation in `docs/`
- Use `just` command runner for build automation

## Constraints
- libcreate depends on Boost (system, thread, asio) and C++11
- The C++ API uses classes (not C-compatible) - requires C wrapper for FFI
- Robot models: ROOMBA_400 (V_1), CREATE_1 (V_2), CREATE_2 (V_3)
- OI modes: OFF → PASSIVE → SAFE → FULL (state machine, perfect for TypeState)
