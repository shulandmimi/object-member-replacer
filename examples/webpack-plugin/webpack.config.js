const { OOMPlugin } = require("@oomm/transformer/webpack");
const TerserPlugin = require("terser-webpack-plugin");

/**
 * @type {import('webpack').Configuration}
 */
module.exports = {
    entry: "./src/index.js",
    module: {
        rules: [],
    },
    target: 'node',
    mode: "production",
    plugins: [],
    devtool: 'source-map',
    optimization: {
        minimizer: [
            new OOMPlugin({
                ignoreWords: [
                    "console",
                    "require",
                ],
                stringLiteral: false,
            }),
            // new TerserPlugin()
        ],
        splitChunks: {
            cacheGroups: {
                foo: {
                    test: /foo\.js$/,
                    name: 'foo',
                    chunks: 'all',
                    minSize: 0,
                }
            }
        }
    },
};
