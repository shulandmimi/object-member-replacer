# object-member-minify (oomm)

Try compressing object fields or string literals, which is suitable for some scenarios with strict size constraints.

## theory

When string literals are used with a certain frequency & the cost (source code length) of using the literal after extracting it as a variable reference is less than the cost of using the original literal, extract it as a variable for dynamic reference

```js
const obj = {};
obj.namespace.namespace1.namespace2.namespace3 = 10;
obj.namespace.namespace1.namespace2.namespace3 = 10;

// =>

var a = "namespace",
  b = "namespace1",
  c = "namespace2",
  d = "namespace3";
const obj = {};
obj[a][b][c][d] = 10;
obj[a][b][c][d] = 10;
```

## When do you need it?

- extreme size reduction is required
- the overhead of file compression (e.g. `gzip`) is higher than using `oomm`

## start

```shell
# pnpm
pnpm i @oomm/transformer -D

# npm
npm i @oomm/transformer -D

# yarn
yarn add @oomm/transformer -D
```

## usage

```js
const { OOMPlugin } = require("@oomm/transformer/webpack");

module.exports = {
  optimization: {
    minimizer: [
      new OOMPlugin({
        ignoreWords: [
          "process.env.GOGOGO",
          // ignore collect
          // ```unknown
          // function foo() { "use strict" }
          //                   ^^^^^^^^^^
          // ```
          { type: "stringLit", content: "use strict" },
          // _require.async("./foo")
          {
            type: "member",
            // try match
            // ```unknown
            // _require.async("./foo")
            // ^^^^^^^^
            // ```
            path: "_require",
            // ignore collect
            // ```unknown
            // _require.async("./foo")
            //          ^^^^^
            // ```
            subpath: false,
            // ingore collect
            // ```unknown
            //_require.async("./foo")
            //                ^^^^^
            // ```
            skipLitArg: true,
          },
          // console.log("hello world");
          // console.error("error!!!");
          {
            type: "member",
            // match
            // ```unknown
            //  console.log("hello world");
            //  ^^^^^^^
            // ```
            path: "console",
            // ignore collect
            // ```unknown
            //  console.log("hello world");
            //          ^^^
            // ```
            subpath: false,
            // Will ignore all expressions, and related data will no longer be collected
            //
            // ```unknown
            // require.async("namespace", "m1")
            //               ^^^^^^^^^^^^^^^^^
            // ```
            // @default false
            skipLitArg: true,
          },
        ],
        // match after output file
        // support RegExp | string
        exclude: ["exclude"],
      }),
      new TerserPlugin({
        terserOptions: {
          compress: {
            drop_console: true,
          },
        },
      }),
    ],
  },
};
````

## roadmap

- [ ] support other bundler
  - [x] webpack minify plugin
- compression at different stages
  - [ ] transform -> for module
  - [x] before minify -> for chunk
- [ ] when mapping exceeds size, split into separate files
- [x] sourcemap
- [ ] better compression
  - [ ] composition string
  - [ ] more precise calculation of the cost
