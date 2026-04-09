Hard rules:
- Edit `crates/` only; treat other directories as reference-only unless explicitly requested.
- No placeholders (`todo!()`, `unimplemented!()`, placeholder panics, empty pipelines).
- Documentation describes current implementation behavior.
- `src/` is production-only; unit tests in `module/tests.rs`; benches in `benches/` using Criterion (`bench_` prefix).
- Naming: longform file names where it improves clarity; `Syntax*` reserved for syntax trees (not token stream); lexer nouns stay short (`Token`, `TokenKind`, `Trivia`, `Lexer`); trivia types live in `trivia.rs`.
- Imports: prefer top-level `use` imports and local names in signatures and implementation code. Do not write fully qualified module paths such as `crate::mod::Type` inside function bodies, match arms, or helper signatures just to name ordinary types or variants. Absolute/singleton uses are only OK for real singletons (`crate::CONST`, `crate::func()`, macros). Do not introduce new macros in this repo.
- Decomposition: do not create God objects, God modules, or catch-all files. If a type or file starts owning unrelated responsibilities such as collection, normalization, checking, diagnostics, effect handling, and declaration validation all at once, split it. Prefer small coordinating facades over monolithic structs; organize by responsibility.

