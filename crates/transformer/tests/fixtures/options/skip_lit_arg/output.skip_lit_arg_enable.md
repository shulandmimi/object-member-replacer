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
      "subpath": true,
      "skipLitArg": true,
      "skipArg": false
    }
  ],
  "optimize": null
}
```

## Output

```js
var a = "log", b = "debug", c = "time";
console[a]("./foo.js");
console[a]("./foo.js");
console[a]("./foo.js");
console[a]("./foo.js");
console[a]("./foo.js");
console[a]("./foo.js");
console[a]("./foo.js");
console[a]("./foo.js");
console[a]("./foo.js");
console[a]("./foo.js");
console[a]("./foo.js");
console[b]("./foo.js");
console[b]("./foo.js");
console[b]("./foo.js");
console[b]("./foo.js");
console[b]("./foo.js");
console[c]("./foo.js");
console[c]("./foo.js");
console[c]("./foo.js");
console[c]("./foo.js");
console[c]("./foo.js");
```