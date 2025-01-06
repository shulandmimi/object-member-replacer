import { TransformOption } from "./core/bridge";

export enum ModuleType {
    TypeScript = "typescript",
    JavaScript = "javascript",
}

export interface OOMPluginOptions
    extends Pick<
        TransformOption,
        "enableSourceMap" | "ignore_words" | "preserve_keywords"
    > {}
