const { OOMPlugin } = require("@oomm/transformer/webpack");
const TerserPlugin = require("terser-webpack-plugin");
const webpack = require('webpack');

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
    plugins: [
        new webpack.DefinePlugin({
            'process.env.NODE_ENV': JSON.stringify('production'),
            '__target__': JSON.stringify('node'),
            'a.b.c': JSON.stringify('a.b.c'),
            'a.b': JSON.stringify('a.b'),
            'a(a.b.c).b.c': JSON.stringify('a.b.c'),
        })
    ],
    devtool: 'source-map',
    optimization: {
        minimizer: [
            // new OOMPlugin({
            //     ignoreWords: [
            //         "console",
            //         "require",
            //     ],
            //     stringLiteral: false,
            // }),
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
