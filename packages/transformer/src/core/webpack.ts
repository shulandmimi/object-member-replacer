import type { Compiler, LoaderDefinitionFunction } from "webpack";
import { transform, type TransformOption } from "./bridge";
import { moduleTypeFromName } from "../util/module";

export const loader: LoaderDefinitionFunction = function (content) {
    const callback = this.async();

    transform(content, {}).then((result) => {
        callback(null, result.code);
    });
};

export interface OOMPluginOptions {}

interface Output {
    name: string;
    code: string;
    map: any;
    source?: any;
}

const PLUGIN_NAME = "OOMPlugin";

export class OOMPlugin {
    constructor(options: OOMPluginOptions = {}) {}

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

                        if (!output) {
                            const { source, map } = inputSource.sourceAndMap();
                            let inputCode = source.toString();

                            let inputMap;

                            if (map) {
                                inputMap =
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
                                sourceMap: inputMap,
                            };

                            const result = await transform(inputCode, options);

                            output = {
                                name,
                                code: result.code,
                                map: result.map ?? map,
                            };

                            if (output.map) {
                                output.source = new SourceMapSource(
                                    output.code,
                                    name,
                                    output.map,
                                    inputCode,
                                    inputMap,
                                    true
                                );
                            } else {
                                output.source = new RawSource(output.code);
                            }

                            await cacheSource.storePromise({
                                source: output.source,
                                errors: [],
                                warning: [],
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
