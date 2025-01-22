import { transform as t } from "../binding";
import { ModuleType } from "../type";

export interface IgnoreWordObject {
    /**
     * The path of the word to ignore.
     *
     * link `require.async()` => `"require"`
     */
    path: string;
    /**
     * Whether to ignore the subpath of the word.
     */
    subpath: boolean;
    /**
     * Whether to ignore the literal argument of the word.
     */
    skipLitArg: boolean;
}

export interface StringLitOption {
    content: string;
}

export type IgnoreWord = string | IgnoreWordObject | StringLitOption;

export interface TransformOption {
    filename?: string;
    sourceMap?: string;
    enableSourceMap?: boolean;
    moduleType?: ModuleType;
    ignoreWords?: IgnoreWord[];
    preserveKeywords?: string[];
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
