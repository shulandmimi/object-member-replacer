# @oomm/transformer

## features

- support minify object member (Computed(`["field"]`), Ident(`.field`)) and string literal(`"field"`)

```ts
let obj = {};

obj.foo_bar_field_1 = "1";

console.log(obj.foo_bar_field_1);

const v = obj.foo_bar_field_1;

// ===>

var a = "foo_bar_field_1";

console.log(obj.foo_bar_field_1);
// =>
console.log(obj[a]);

const v = obj.foo_bar_field_1;
// =>
const v = obj[a];
```

## usage

**install**

```shell
npm i @oomm/transformer
```

### webpack

`@oomm/transformer` is only suitable for specific compression, general compression still requires other compressors to complete (e.g., `terser-webpack-plugin`).

```js
// webpack.config.js
const { OOMPlugin } = require("@oomm/transformer");

module.exports = {
  plugins: [new OOMPlugin()],
};
```
