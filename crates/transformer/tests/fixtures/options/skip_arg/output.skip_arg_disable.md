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
var a = "./foo.js", b = "nest_arg";
console.log(a, foo(b));
console.log(a, foo(b));
console.log(a, foo(b));
console.log(a, foo(b));
console.log(a, foo(b));
console.log(a, foo(b));
console.log(a, foo(b));
console.log(a, foo(b));
console.log(a, foo(b));
console.log(a, foo(b));
console.log(a, foo(b));
console.debug(a, foo(b));
console.debug(a, foo(b));
console.debug(a, foo(b));
console.debug(a, foo(b));
console.debug(a, foo(b));
console.time(a, foo(b));
console.time(a, foo(b));
console.time(a, foo(b));
console.time(a, foo(b));
console.time(a, foo(b));
```