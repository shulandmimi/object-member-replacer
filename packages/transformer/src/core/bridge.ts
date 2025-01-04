import { transform as t } from "../binding";

export interface TransformOption {
    filename?: string;
    sourceMap?: string;
    enableSourceMap?: boolean;
    moduleType?: "javascript" | "typescript";
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
