import type { Compiler, LoaderDefinitionFunction } from "webpack";
import { transform, type TransformOption } from "./bridge";
import { createFilter, moduleTypeFromName } from "../util/module";
import { OOMPluginOptions } from "../type";

export const loader: LoaderDefinitionFunction = function (content) {
    const callback = this.async();

    transform(content, {}).then((result) => {
        callback(null, result.code);
    });
};

interface Output {
    name: string;
    source?: any;
}

const PLUGIN_NAME = "OOMPlugin";

export class OOMPlugin {
    filter?: ReturnType<typeof createFilter>;
    constructor(private options: OOMPluginOptions = {}) {}

    apply(compiler: Compiler) {
        compiler.hooks.compilation.tap(PLUGIN_NAME, (compilation) => {
            compilation.hooks.processAssets.tapPromise(
                {
                    stage: compiler.webpack.Compilation
                        .PROCESS_ASSETS_STAGE_OPTIMIZE_SIZE,
                    name: PLUGIN_NAME,
                    additionalAssets: true,
                },
                async (assets) => {
                    const {
                        enableSourceMap = Boolean(compiler.options.devtool),
                        ignoreWords,
                        preserveKeywords,
                        enableCache = true,
                        optimize,
                    } = this.options;

                    const cache = enableCache
                        ? compilation.getCache(PLUGIN_NAME)
                        : undefined;

                    const { SourceMapSource, RawSource } =
                        compiler.webpack.sources;

                    for (const [name, _asset] of Object.entries(assets)) {
                        const filter = (this.filter ??= createFilter(
                            this.options
                        ));

                        if (!filter(name)) {
                            continue;
                        }

                        const { source: inputSource, info } =
                            compilation.getAsset(name)!;

                        const eTag = cache?.getLazyHashedEtag(inputSource);
                        const cacheSource =
                            eTag && cache?.getItemCache(name, eTag);
                        let output: Output | undefined =
                            await cacheSource?.getPromise();

                        if (!output) {
                            const { source, map } = inputSource.sourceAndMap();
                            let inputCode = source.toString();

                            let formatSourceMap;

                            if (map) {
                                formatSourceMap =
                                    typeof map === "object" && map !== null
                                        ? JSON.stringify(map)
                                        : map;
                            }

                            let moduleType = moduleTypeFromName(name);

                            if (!moduleType) {
                                continue;
                            }

                            const options: TransformOption = {
                                moduleType,
                                filename: name,
                                sourceMap: formatSourceMap,
                                enableSourceMap,
                                ignoreWords,
                                preserveKeywords,
                                optimize,
                            };

                            const result = await transform(inputCode, options);

                            const code = result.code ?? inputCode;
                            const outputMap = result.map ?? map;

                            output = {
                                name,
                            };

                            if (outputMap) {
                                output.source = new SourceMapSource(
                                    code,
                                    name,
                                    outputMap,
                                    inputCode,
                                    formatSourceMap,
                                    true
                                );
                            } else {
                                output.source = new RawSource(code);
                            }

                            await cacheSource?.storePromise({
                                errors: [],
                                warning: [],
                                ...output,
                            });
                        }

                        if (output) {
                            let { source } = output;
                            compilation.updateAsset(name, source, info);
                        }
                    }

                    return Promise.resolve();
                }
            );
        });
    }
}

export default loader;
