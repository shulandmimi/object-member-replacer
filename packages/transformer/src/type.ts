import { TransformOption } from "./core/bridge";

export enum ModuleType {
    TypeScript = "typescript",
    JavaScript = "javascript",
}

export type Filter = string | RegExp | ((filename: string) => boolean);

export interface OOMPluginOptions
    extends Pick<
        TransformOption,
        "enableSourceMap" | "ignoreWords" | "preserveKeywords" | "optimize"
    > {
    /**
     * exclude files that match the filter
     */
    exclude?: Filter[];
    /**
     * include files that match the filter
     */
    include?: Filter[];
    /**
     * @default true
     */
    enableCache?: boolean;
}
