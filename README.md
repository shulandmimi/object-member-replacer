# object-member-minify (oomm)

Try compressing object fields or string literals, which is suitable for some scenarios with strict size constraints.

## roadmap

- [ ] support other bundler
- compression at different stages
  - [ ] transform -> for module
  - [ ] before minify -> for chunk
- [ ] when mapping exceeds size, split into separate files
- [ ] sourcemap
- [ ] better compression
