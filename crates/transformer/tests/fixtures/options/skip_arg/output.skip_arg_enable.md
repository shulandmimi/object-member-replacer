## Config

```json
{
  "filename": null,
  "sourceMap": null,
  "enableSourceMap": false,
  "moduleType": null,
  "preserveKeywords": [],
  "ignoreWords": [
    {
      "type": "member",
      "path": "console",
      "subpath": false,
      "skipLitArg": false,
      "skipArg": true
    }
  ],
  "optimize": null
}
```

## Output

```js
console.log("./foo.js", foo("nest_arg"));
console.log("./foo.js", foo("nest_arg"));
console.log("./foo.js", foo("nest_arg"));
console.log("./foo.js", foo("nest_arg"));
console.log("./foo.js", foo("nest_arg"));
console.log("./foo.js", foo("nest_arg"));
console.log("./foo.js", foo("nest_arg"));
console.log("./foo.js", foo("nest_arg"));
console.log("./foo.js", foo("nest_arg"));
console.log("./foo.js", foo("nest_arg"));
console.log("./foo.js", foo("nest_arg"));
console.debug("./foo.js", foo("nest_arg"));
console.debug("./foo.js", foo("nest_arg"));
console.debug("./foo.js", foo("nest_arg"));
console.debug("./foo.js", foo("nest_arg"));
console.debug("./foo.js", foo("nest_arg"));
console.time("./foo.js", foo("nest_arg"));
console.time("./foo.js", foo("nest_arg"));
console.time("./foo.js", foo("nest_arg"));
console.time("./foo.js", foo("nest_arg"));
console.time("./foo.js", foo("nest_arg"));
```