import { transform as t } from "../binding";
import { ModuleType } from "../type";

export interface IgnoreWordObject {
    type: "member";
    /**
     * The path of the word to ignore.
     *
     * link `require.async()` => `"require"`
     */
    path: string;
    /**
     *
     * `require.async`
     *
     * ignore the subpath of the word. eg. `async`
     *
     * @default true
     */
    subpath: boolean;
    /**
     *
     * `require.async("namespace")`
     *
     * ignore the literal argument of the word. eg. `"namespace"`
     *
     * @default false
     */
    skipLitArg: boolean;
}

export interface StringLitOption {
    type: "stringLit";
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
