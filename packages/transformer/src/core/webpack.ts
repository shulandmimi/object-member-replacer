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
                    const cache = compilation.getCache(PLUGIN_NAME);
                    const assetsShouldMinify = await Promise.all(
                        Object.keys(assets).map(async (name) => {
                            const { source, info } =
                                compilation.getAsset(name)!;

                            const eTag = cache.getLazyHashedEtag(source);
                            const cacheSource = cache.getItemCache(name, eTag);
                            const cacheOutput: Output =
                                await cacheSource.getPromise();

                            return {
                                name,
                                info,
                                source,
                                output: cacheOutput,
                                cacheSource,
                            };
                        })
                    );

                    const { SourceMapSource, RawSource } =
                        compiler.webpack.sources;

                    for (const asset of assetsShouldMinify) {
                        const {
                            name,
                            info,
                            source: inputSource,
                            cacheSource,
                        } = asset;

                        let output: Output = asset.output;

                        const filter = (this.filter ??= createFilter(
                            this.options
                        ));

                        if (!filter(name)) {
                            continue;
                        }

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

                            const {
                                enableSourceMap,
                                ignoreWords,
                                preserveKeywords,
                            } = this.options;
                            const options: TransformOption = {
                                moduleType,
                                filename: name,
                                sourceMap: formatSourceMap,
                                enableSourceMap,
                                ignoreWords,
                                preserveKeywords,
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

                            await cacheSource.storePromise({
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
