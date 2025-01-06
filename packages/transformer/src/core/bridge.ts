import { transform as t } from "../binding";
import { ModuleType } from "../type";

export interface TransformOption {
    filename?: string;
    sourceMap?: string;
    enableSourceMap?: boolean;
    moduleType?: ModuleType;
    ignore_words?: string[];
    preserve_keywords?: string[];
}

export interface TransformResult {
    code: string;
    map?: string;
}

export async function transform(
    code: string,
    options?: TransformOption
): Promise<TransformResult> {
    const result = t(code, options);

    return {
        code: result.content,
        map: result.map,
    };
}
