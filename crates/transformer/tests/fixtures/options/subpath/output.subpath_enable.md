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
      "skipLitArg": false,
      "skipArg": false
    }
  ]
}
```

## Output

```js
var a = "log", b = "./foo.js", c = "debug", d = "time";
console[a](b);
console[a](b);
console[a](b);
console[a](b);
console[a](b);
console[a](b);
console[a](b);
console[a](b);
console[a](b);
console[a](b);
console[a](b);
console[c](b);
console[c](b);
console[c](b);
console[c](b);
console[c](b);
console[d](b);
console[d](b);
console[d](b);
console[d](b);
console[d](b);
```