import { writeFileSync, mkdirSync } from "node:fs";
import path, { join } from "node:path";

interface GenerateContextOptions {
    topLevelFieldCount: number;
    nestedLevel: number;
    /** 多级访问 */
    nestedFiledCount?: number;
    nestedLevelFieldCount?: number;
    generateFactory?: ((context: GenerateContext) => string)[];
}

const outDir = join(process.cwd(), "output");

class GenerateContext {
    decls: string[] = [];
    topLevelFields: Set<string> = new Set();
    totalIdentList: Set<string> = new Set();
    levelIdentList: Record<number, Set<string>> = {};
    levelObjectLitIdentList: Record<number, Set<string>> = {};
    shouldVisitLevelObject: Set<number> = new Set();
    levelNestedFieldRecord: Record<number, string[]> = {};

    constructor(public options: GenerateContextOptions) {
        this.generateNestedDecls();
        this.generateToplevelFields();
        this.generateNestedIdentList();
    }

    generateNestedDecls() {
        const times = Math.floor(Math.random() * 100) + 1;

        const obj = new Set();
        const decls: string[] = [];

        for (let i = 0; i < times; i++) {
            let ident = allocateIdent({ excludeSets: [this.totalIdentList] });

            obj.add(ident);

            decls.push(ident);

            this.totalIdentList.add(ident);
        }
    }

    generateToplevelFields() {
        this.topLevelFields.clear();

        for (let i = 0; i < this.options.topLevelFieldCount; i++) {
            const ident = allocateIdent({ excludeSets: [this.totalIdentList] });
            this.topLevelFields.add(ident);
            this.totalIdentList.add(ident);
        }

        this.topLevelFields.forEach((item) => {
            this.totalIdentList.add(item);
        });
    }

    generateNestedIdentList() {
        this.totalIdentList.add("i");
        this.totalIdentList.add("_nest");

        for (let i = 0; i < this.options.nestedLevel; i++) {
            const levelIdentSet = (this.levelIdentList[i] ??= new Set());
            const levelObjectLitIdentSet = (this.levelObjectLitIdentList[i] ??=
                new Set());
            const levelNestedFieldArr = (this.levelNestedFieldRecord[i] ??= []);

            let visitIdent = false;
            for (
                let j = 0;
                j < (this.options.nestedLevelFieldCount ?? 5);
                j++
            ) {
                const newIdent = allocateIdent({
                    excludeSets: [this.totalIdentList],
                });

                this.totalIdentList.add(newIdent);

                levelIdentSet.add(newIdent);

                if (
                    (Math.random() > 0.5 || this.options.nestedLevel === 1) &&
                    !visitIdent
                ) {
                    visitIdent = true;
                    levelObjectLitIdentSet.add(newIdent);
                    levelNestedFieldArr.push(newIdent);
                    for (
                        let o = 0;
                        o < (this.options.nestedFiledCount ?? 5);
                        o++
                    ) {
                        const nestedIdent = allocateIdent({
                            excludeSets: [this.totalIdentList],
                        });

                        this.totalIdentList.add(nestedIdent);

                        levelNestedFieldArr.push(nestedIdent);
                    }
                }
            }

            if (
                levelObjectLitIdentSet.size &&
                (Math.random() > 0.5 || this.options.nestedLevel === 1)
            ) {
                this.shouldVisitLevelObject.add(i);
            }
        }
    }
}

interface GenerateCodeOptions {
    generateBenchCode: {
        name: string;
        handler: (context: GenerateContext, code: GenerateCode) => string;
    }[];
}

class GenerateCode {
    object_lit_decl?: string;
    currentGenerateBenchCodeIndex: number = 0;

    constructor(
        public context: GenerateContext,
        public options: GenerateCodeOptions
    ) {}

    generate(name: string): string {
        const code = this.generateCode();
        const _name = `generated-${name}.js`;
        const outfile = join(outDir, _name);
        mkdirSync(outDir, { recursive: true });
        writeFileSync(outfile, code, "utf-8");
        console.log(outfile);
        return outfile;
    }

    randomValue() {
        const v = Math.random();
        switch (true) {
            case v < 0.1:
                return "null";
            case v < 0.2:
                return "undefined";
            case v < 0.3:
                return "true";
            case v < 0.4:
                return "false";
            case v < 0.5:
                return "NaN";
            case v < 0.6:
                return "Infinity";
            default:
                return `${Math.floor(Math.random() * 100)}`;
        }
    }

    generateObjectLit() {
        return (this.object_lit_decl ??= `{ ${[
            ...this.context.topLevelFields,
        ].reduce((r, i) => r + `${i}: ${this.randomValue()},`, "")}}`);
    }

    generateTopLevelFieldDecl() {
        return `var ${Array.from(this.context.topLevelFields)
            .map((item) => `${item} = "${item}"`)
            .join(", ")};`;
    }

    generateCode() {
        const { context } = this;

        const topLevelFields = Array.from(context.topLevelFields)
            .map((item) => `${item} = "${item}"`)
            .join(", ");

        return `
var ${topLevelFields};
function _assert(v) {
    Boolean(v === 123)
}
${this.generateNestedCode(0) ?? ""}
`;
    }

    generateNestedCode(level = 0): string | undefined {
        const { context } = this;

        const curLevel = level;

        if (curLevel >= context.options.nestedLevel) {
            return this.generateBenchCode();
        }

        const levelObjectLitIdentSet =
            context.levelObjectLitIdentList[curLevel];
        const [ident, ...fields] = context.levelNestedFieldRecord[curLevel];
        const nestedIdentList = Array.from(context.levelIdentList[curLevel])
            .map(
                (ident) =>
                    `${ident} = ${
                        levelObjectLitIdentSet.has(ident)
                            ? this.generateObjectLit()
                            : this.randomValue()
                    }`
            )
            .join(", ");

        return `
        function nested${curLevel}() {
            var ${nestedIdentList};
            ${fields.map((item) => `${ident}['${item}'] = ${ident}`)}

            ${this.generateNestedCode(curLevel + 1) ?? ""}
        }
        nested${curLevel}();
        `;
    }

    generateBenchCode() {
        const generator =
            this.options.generateBenchCode[this.currentGenerateBenchCodeIndex];

        return generator.handler(this.context, this) ?? "";
    }
}
// const loopTimes = 100000000;
const loopTimes = 10000000;
const TopLevelFieldCount = 50;
const runTimes = 20;

interface GenerateBenchCodeOptions {
    isDynamic?: boolean;
    isHybrid?: boolean;
    isNestedVisit?: boolean;
}
function generateBenchCode(
    context: GenerateContext,
    options: GenerateBenchCodeOptions = {}
) {
    const {
        isDynamic = true,
        isHybrid = false,
        isNestedVisit = false,
    } = options;
    const arr = [...context.shouldVisitLevelObject];
    const objectLitIdentList = arr.flatMap((item) => [
        ...context.levelObjectLitIdentList[item],
    ]);
    const topLevelFields = [...context.topLevelFields];

    function generateCode(variable: string, field: string) {
        if (isHybrid) {
            const isDynamic = Math.random() > 0.5;
            if (isDynamic) {
                return `${variable}[${field}]`;
            }

            return `${variable}.${field}`;
        }
        return isDynamic ? `${variable}[${field}]` : `${variable}.${field}`;
    }

    function generateNormalVisit() {
        return objectLitIdentList
            .flatMap((variable) =>
                topLevelFields.map(
                    (field) => `_assert(${generateCode(variable, field)})`
                )
            )
            .join(";");
    }

    function generateNestedVisit() {
        return arr
            .map((item) => context.levelNestedFieldRecord[item])
            .map((item) => {
                const [ident, ...fields] = item;
                return Array.from({ length: fields.length }, (_, index) => [
                    ...fields.slice(index),
                    ...fields.slice(0, index),
                ])
                    .map((item) => {
                        if (!isDynamic)
                            return `_assert(${ident}.${item.join(".")});`;
                        return `_assert(${ident}["${item.join('"]["')}"]);`;
                    })
                    .join(";");
            })
            .join(";");
    }

    return `
const start = performance.now();
for (let i = 0; i < ${loopTimes}; i++) {
${isNestedVisit ? generateNestedVisit() : generateNormalVisit()}
}
console.log(performance.now() - start);
    `;
}

const files = ["member_dynamic_field", "member_static_field"];

const context = new GenerateContext({
    nestedLevel: 1,
    topLevelFieldCount: TopLevelFieldCount,
    nestedFiledCount: 10,
});

type IGenerator = {
    name: string;
    generateFactor: GenerateCodeOptions["generateBenchCode"];
    benchmarkFormatter: (output: string) => {
        staticTotalTime?: number;
        dynamicTotalTime?: number;
        diff?: number;
    };
};

const commands = ["node", "bun", "deno"];

import { spawn, spawnSync } from "node:child_process";
import Table from "cli-table3";
import { allocateIdent } from "./allocateIdent.js";

const run = (command: string, file: string) => {
    return new Promise((resolve) => {
        const result = spawn(command, [file], {
            stdio: ["inherit", "pipe", "inherit"],
            shell: true,
        });

        let _stdout = "";
        result.stdout.on("data", (data) => {
            _stdout += data;
        });

        result.on("exit", () => {
            resolve(_stdout);
        });
    });
};

const runSync = (command: string, file: string) => {
    const result = spawnSync(command, [file], {
        stdio: ["inherit", "pipe", "inherit"],
        shell: true,
    });

    return result.stdout.toString();
};

const generatedFiles = files.slice(0, 1).map((name) => [
    name,
    join(outDir, `generated.js`),
    // join(outDir, `generated-${name}.js`),
]);

const table = new Table({
    head: ["Command", "Name", "Static", "Dynamic", "Diff"],
});

// Promise.all(
//     commands.flatMap((command) => {
//         return generatedFiles.map(async ([name, file]) => {
//             const output = await run(command, file);
//             table.push([command, name, Number(output).toFixed(2)]);
//         });
//     })
// ).finally(() => {
//     console.log("\n", table.toString());
// });
// const result: Record<string, { name: string; datas: number[] }[]> =
//     commands.reduce((r, i) => ({ ...r, [i]: [] }), {});

// for (let i = 0; i < 10; i++) {
//     console.log(`Loop ${i + 1}...`);
//     commands.forEach((command) => {
//         return generatedFiles.forEach(([name, file]) => {
//             const output = runSync(command, file);

//             const [_static, _dynamic] = output
//                 .trim()
//                 .split(/[\s\n]+/)
//                 .map((item) => Number(item));
//             const [staticAvg, dynamicAvg] = [
//                 _static / loopTimes / TopLevelFieldCount,
//                 _dynamic / loopTimes / TopLevelFieldCount,
//             ];

//             result[command].push({
//                 name,
//                 datas: [
//                     _static,
//                     _dynamic,
//                     staticAvg,
//                     dynamicAvg,
//                     _dynamic / _static,
//                 ],
//             });

//             // table.push([
//             //     command,
//             //     name,
//             //     // output.trim().split(/[\s\n]+/).map((item) => Number(item).toFixed(2)).join(' '),
//             //     ...[
//             //         _static,
//             //         _dynamic,
//             //         staticAvg,
//             //         dynamicAvg,
//             //         _dynamic / _static,
//             //     ].map((item) => (item ?? 0).toFixed(10)),
//             // ]);
//         });
//     });
// }

// Object.entries(result)
//     .sort(([a, _1], [b, _2]) => (a >= b ? 1 : -1))
//     .forEach(([command, data]) => {
//         const [staticTotal, dynamicTotal, staticAvg, dynamicAvg, diff] = data
//             .reduce(
//                 (r, item) => {
//                     item.datas.forEach((v, i) => (r[i] = (r[i] ?? 0) + v));
//                     return r;
//                 },
//                 [0, 0, 0, 0, 0]
//             )
//             .map((item) => item / data.length);

//         table.push([
//             command,
//             "",
//             // data[0].name,
//             staticTotal.toFixed(2),
//             dynamicTotal.toFixed(2),
//             staticAvg.toFixed(10),
//             dynamicAvg.toFixed(10),
//             diff.toFixed(10),
//         ]);
//     });

// console.log("\n", table.toString());

// writeFileSync(
//     join(outDir, "result.md"),
//     `
// # Bench Mark

// ${table.toString()}
// `,
//     "utf-8"
// );

const generator1: IGenerator[] = [
    {
        name: "static-dynamic",
        generateFactor: [
            {
                name: files[1],
                handler: (context) => {
                    return `
                    {${generateBenchCode(context, { isDynamic: false })}};
                    \n
                    {${generateBenchCode(context)}};
                    `;
                },
            },
        ],
        benchmarkFormatter: (output: string) => {
            const [_static, _dynamic] = output
                .trim()
                .split(/[\s\n]/)
                .map(Number);

            return {
                staticTotalTime: _static,
                dynamicTotalTime: _dynamic,
                diff: _dynamic / _static,
            };
        },
    },
    {
        name: "dynamic-static",
        generateFactor: [
            {
                name: files[1],
                handler: (context) => {
                    return `
                {${generateBenchCode(context)}};
                \n
                {${generateBenchCode(context, { isDynamic: true })}};
                `;
                },
            },
        ],
        benchmarkFormatter(output) {
            const [_dynamic, _static] = output
                .trim()
                .split(/[\s\n]/)
                .map(Number);

            return {
                staticTotalTime: _static,
                dynamicTotalTime: _dynamic,
                diff: _dynamic / _static,
            };
        },
    },
];

const generator2: IGenerator[] = [
    {
        name: "_dynamic",
        generateFactor: [
            {
                name: files[0],
                handler: (context) => {
                    return generateBenchCode(context);
                },
            },
        ],
        benchmarkFormatter(output) {
            const [_dynamic] = output.split(/\s\n/).map(Number);

            return {
                dynamicTotalTime: _dynamic,
            };
        },
    },
    {
        name: "_static",
        generateFactor: [
            {
                name: files[1],
                handler: (context) => {
                    return generateBenchCode(context, { isDynamic: false });
                },
            },
        ],
        benchmarkFormatter(output) {
            const [_static] = output.split(/\s\n/).map(Number);

            return {
                staticTotalTime: _static,
            };
        },
    },
    {
        name: "hybrid",
        generateFactor: [
            {
                name: files[1],
                handler: (context) => {
                    return generateBenchCode(context, {
                        isDynamic: false,
                        isHybrid: true,
                    });
                },
            },
        ],
        benchmarkFormatter(output) {
            const [_static] = output.split(/\s\n/).map(Number);

            return {
                staticTotalTime: _static,
            };
        },
    },
    {
        name: "dynamic-nested-visit",
        generateFactor: [
            {
                name: files[1],
                handler: (context) => {
                    return generateBenchCode(context, {
                        isDynamic: true,
                        isHybrid: false,
                        isNestedVisit: true,
                    });
                },
            },
        ],
        benchmarkFormatter(output) {
            const [_static] = output.split(/\s\n/).map(Number);

            return {
                dynamicTotalTime: _static,
            };
        },
    },
    {
        name: "static-nested-visit",
        generateFactor: [
            {
                name: files[1],
                handler: (context) => {
                    return generateBenchCode(context, {
                        isDynamic: false,
                        isHybrid: false,
                        isNestedVisit: true,
                    });
                },
            },
        ],
        benchmarkFormatter(output) {
            const [_static] = output.split(/\s\n/).map(Number);

            return {
                staticTotalTime: _static,
            };
        },
    },
];

const avgNum =
    context.shouldVisitLevelObject.size *
    context.options.topLevelFieldCount *
    loopTimes;

// const benchmarkDatas: { staticTime: number; dynamicTime: number } = [];

const combineRunDataList = generator1
    .map((item) => {
        const generator = new GenerateCode(context, {
            generateBenchCode: item.generateFactor,
        });
        const outputFiles = generator.generate(item.name);
        return {
            name: item.name,
            outputFile: outputFiles,
            generator: item,
        };
    })
    .map((item) => {
        const perfData = {
            staticTotalTime: 0,
            dynamicTotalTime: 0,
            diff: 0,
        };

        for (let i = 0; i < runTimes; i++) {
            console.log(`${item.name} Loop ${i + 1}...`);
            const output = runSync("node", item.outputFile);
            const data = item.generator.benchmarkFormatter(output);
            perfData.staticTotalTime += data.staticTotalTime ?? 0;
            perfData.dynamicTotalTime += data.dynamicTotalTime ?? 0;
        }

        perfData.diff = perfData.dynamicTotalTime / perfData.staticTotalTime;

        table.push([
            "node",
            item.name,
            perfData.staticTotalTime.toFixed(2),
            perfData.dynamicTotalTime.toFixed(2),
            perfData.diff.toFixed(10),
        ]);
        return perfData;
    });
const [_dynamic, _static, _hybrid, dynamicNestedVisit, staticNestedVisit] =
    generator2.map((item, index) => {
        const generator = new GenerateCode(context, {
            generateBenchCode: item.generateFactor,
        });
        const outputFiles = generator.generate(item.name);

        const perfData = {
            staticTotalTime: 0,
            dynamicTotalTime: 0,
        };

        for (let i = 0; i < runTimes; i++) {
            console.log(`${item.name} Loop ${i + 1}...`);
            const output = runSync("node", outputFiles);
            const data = item.benchmarkFormatter(output);
            perfData.staticTotalTime += data.staticTotalTime || 0;
            perfData.dynamicTotalTime += data.dynamicTotalTime || 0;
        }
        return perfData;
    });

interface PerfData {
    staticTotalTime: number;
    dynamicTotalTime: number;
    diff?: number;
}

function mergeData(d1: PerfData, d2: PerfData): PerfData {
    const d: PerfData = {
        staticTotalTime: d1.staticTotalTime || d2.staticTotalTime,
        dynamicTotalTime: d2.dynamicTotalTime || d1.dynamicTotalTime,
    };
    d.diff = d.dynamicTotalTime / d.staticTotalTime;
    return d;
}

const combinePerfData = mergeData(_static, _dynamic);
const nestedVisit = mergeData(staticNestedVisit, dynamicNestedVisit);

const logHeader = `
read times: ${avgNum}
loop times: ${loopTimes}
nest level: ${context.options.nestedLevel}
top level field count: ${context.options.topLevelFieldCount}
visit nested count: ${context.shouldVisitLevelObject.size}
level size: ${Object.entries(context.levelObjectLitIdentList).reduce(
    (r, [level, set]) => r + set.size,
    0
)}
`;
console.log(logHeader);
table.push([
    "node",
    "normal",
    combinePerfData.staticTotalTime.toFixed(2),
    combinePerfData.dynamicTotalTime.toFixed(2),
    combinePerfData.diff?.toFixed(10) ?? 0,
]);
table.push([
    "node",
    "hybrid",
    _hybrid.staticTotalTime.toFixed(2),
    _hybrid.dynamicTotalTime.toFixed(2),
    0,
]);

table.push([
    "node",
    "nested-visit",
    nestedVisit.staticTotalTime.toFixed(2),
    nestedVisit.dynamicTotalTime.toFixed(2),
    0,
]);

console.log(table.toString());

writeFileSync(path.join(outDir, "result.md"), `

# result

## metadata

${logHeader}

## result

\`\`\`json
${JSON.stringify(table, null, 4)}
\`\`\`

`);
