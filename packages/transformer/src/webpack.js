const webpack = require("./webpackEntry");

module.exports = webpack.default || webpack;
Object.assign(module.exports, webpack)
