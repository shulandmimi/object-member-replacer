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
      "skipArg": false
    }
  ],
  "optimize": null
}
```

## Output

```js
var a = "./foo.js";
console.log(a);
console.log(a);
console.log(a);
console.log(a);
console.log(a);
console.log(a);
console.log(a);
console.log(a);
console.log(a);
console.log(a);
console.log(a);
console.debug(a);
console.debug(a);
console.debug(a);
console.debug(a);
console.debug(a);
console.time(a);
console.time(a);
console.time(a);
console.time(a);
console.time(a);
```