import { transform as t } from "../binding";
import { ModuleType } from "../type";

export interface IgnoreWordObject {
    /**
     * expr `require.async("m1")`
     *
     * - path: `"require"` => match `"require.async"`
     * - subpath: `true` => ignore collect `"async"`
     * - skipLitArg: `false` => ignore collect (`"m1"`)
     *
     */
    type: "member";
    /**
     * The path of the word to ignore.
     *
     *
     *
     * try match
     * ```unknown
     *
     * // path: "_require"
     * _require.async("./foo")
     * ^^^^^^^^
     *
     * // path: "foo.bar.foo1.bar1"
     * foo.bar.foo1.bar1("./foo")
     * ^^^ ^^^ ^^^^ ^^^^
     * ```
     */
    path: string;
    /**
     *
     * ignore the subpath of the word. eg. `async`
     *
     * ```unknown
     * _require.async("./foo")
     *          ^^^^^
     * ```
     *
     * @default true
     */
    subpath?: boolean;
    /**
     *
     * ignore the literal argument of the word. eg. `"./foo"`
     *
     * ```unknown
     * _require.async("./foo")
     *                 ^^^^^
     * ```
     * @default false
     */
    skipLitArg?: boolean;

    /**
     *
     * ```unknown
     * `require.async("namespace", "m1")`
     *                ^^^^^^^^^^^^^^^^^
     * ```
     * @default false
     */
    skipArg?: boolean;
}

export interface StringLitOption {
    /**
     * match string literal
     *
     * - `content`: `"use strict"` => ignore collect `"use strict"`
     **/
    type: "stringLit";
    content: string;
}

export type IgnoreWord = string | IgnoreWordObject | StringLitOption;

export interface TransformOption {
    filename?: string;
    /**
     * source map file path
     */
    sourceMap?: string;
    /**
     * enable source map, default `false` or webpack sourcemap config
     * @default false
     */
    enableSourceMap?: boolean;
    moduleType?: ModuleType;
    /**
     * ignore words in the code
     */
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
