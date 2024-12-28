import { defineConfig } from '@farmfe/core';
import copy from '@farmfe/js-plugin-copy';
import isCI from 'is-ci';
import dts from '@farmfe/js-plugin-dts';

export default defineConfig({
    compilation: {
        input: {
            webpack: './src/webpack.ts',
        },
        output: {
            format: 'cjs',
            targetEnv: 'library-node',
        },
        external: ['.node$'],
        resolve: {
            autoExternalFailedResolve: true,
        },
        partialBundling: {
            enforceResources: [
                {
                    name: 'index',
                    test: ['.*'],
                },
            ],
        },
        mode: 'development',
        minify: false,
    },
    plugins: [
        !isCI &&
            copy({
                targets: [
                    {
                        src: './binding/node.linux-x64-gnu.node',
                        dest: './dist/',
                    },
                ],
            }),

        dts(),
    ],
});
