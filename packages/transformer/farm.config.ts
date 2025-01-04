import { defineConfig } from "@farmfe/core";
import copy from "@farmfe/js-plugin-copy";
import isCI from "is-ci";
import dts from "@farmfe/js-plugin-dts";

export default defineConfig({
    compilation: {
        input: {
            webpack: "./src/core/webpack.ts",
        },
        output: {
            entryFilename: "[entryName]Entry.js",
            format: "cjs",
            targetEnv: "library-node",
        },
        external: ["^../binding/index.js$"],
        resolve: {
            autoExternalFailedResolve: true,
        },
        partialBundling: {
            enforceResources: [
                {
                    name: "index",
                    test: [".*"],
                },
            ],
        },
        mode: "development",
        minify: false,
    },
    plugins: [
        !isCI &&
            copy({
                targets: [
                    {
                        src: "./binding/*.node",
                        dest: "./dist/",
                    },
                    {
                        src: "./src/*.js",
                        dest: "./dist/",
                    },
                ],
            }),

        dts({
            include: ["./src/**/*.ts"],
        }),
    ],
});
