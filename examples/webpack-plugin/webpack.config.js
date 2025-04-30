const { OOMPlugin } = require("@oomm/transformer/webpack");
const { readdirSync } = require("node:fs");
const path = require("node:path");
const TerserPlugin = require("terser-webpack-plugin");
const webpack = require("webpack");

function srcFiles(dir) {
    return readdirSync(dir)
        .map((item) => path.join(dir, item))
        .map((item) => {
            return {
                test: new RegExp(`${path.relative(process.cwd(), item)}`),
                name: path.basename(item, ".js"),
            };
        });
}

/**
 * @type {import('webpack').Configuration}
 */
module.exports = {
    entry: "./src/index.js",
    plugins: [],
    cache: false,
    mode: "production",
    devtool: "source-map",
    optimization: {
        minimizer: [
            new OOMPlugin({
                ignoreWords: [
                    "process.env.GOGOGO",
                    { type: "stringLit", content: "use strict" },
                    {
                        type: "member",
                        path: "_require",
                        subpath: false,
                        skipLitArg: true,
                    },
                    {
                        type: "member",
                        path: "console",
                        subpath: false,
                        skipArg: true,
                    },
                ],
                exclude: ["exclude"],
            }),
            new TerserPlugin({
                terserOptions: {
                    compress: {
                        drop_console: true,
                    },
                },
            }),
        ],
        splitChunks: {
            cacheGroups: {
                ...srcFiles(path.join(process.cwd(), "./src")).reduce(
                    (acc, item) => ({
                        ...acc,
                        [item.name]: {
                            ...item,
                            chunks: "all",
                            minSize: 0,
                        },
                    }),
                    {}
                ),
            },
        },
    },
};
