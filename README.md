# object-member-minify (oomm)

Try compressing object fields or string literals, which is suitable for some scenarios with strict size constraints.

## roadmap

- [ ] support other bundler
  - [x] webpack minify plugin
- compression at different stages
  - [ ] transform -> for module
  - [ ] before minify -> for chunk
- [ ] when mapping exceeds size, split into separate files
- [x] sourcemap
- [ ] better compression
