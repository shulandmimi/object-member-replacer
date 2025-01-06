const { OOMPlugin } = require("@oomm/transformer/webpack");
const TerserPlugin = require("terser-webpack-plugin");
const webpack = require("webpack");

/**
 * @type {import('webpack').Configuration}
 */
module.exports = {
    entry: "./src/index.js",
    module: {
        rules: [],
    },
    target: "node",
    mode: "production",
    plugins: [],
    devtool: "source-map",
    optimization: {
        minimizer: [
            new OOMPlugin({
                ignoreWords: ["process.env.GOGOGO", "require", "require.async"],
            }),
            new TerserPlugin(),
        ],
        splitChunks: {
            cacheGroups: {
                foo: {
                    test: /foo\.js$/,
                    name: "foo",
                    chunks: "all",
                    minSize: 0,
                },
            },
        },
    },
};
