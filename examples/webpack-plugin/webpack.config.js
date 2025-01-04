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
    mode: "production",
    plugins: [],
    devtool: 'source-map',
    optimization: {
        minimizer: [
            new OOMPlugin(),
            new TerserPlugin()
        ],
    },
};
