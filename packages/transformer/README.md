# @oomm/transformer

[documents](https://github.com/shulandmimi/object-member-replacer)

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
