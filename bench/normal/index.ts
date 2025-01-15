import { writeFileSync } from "node:fs";
import { join } from "node:path";

const declIdents = "abcdefghijklmnopqrstuvwxyz".split("");
const excludeIdents = new Set([
    "null",
    "undefined",
    "true",
    "false",
    "NaN",
    "Infinity",
    "assert",
    "if",
    "do",
    "break",
    "continue",
    "while",
    "in",
    "of",
    "ch",
    "var",
    "let",
    "const",
]);
function randomVal() {
    let v = Math.random();
    if (v <= 0.2) {
        return Math.random() + 0.1;
    }
    if (v <= 0.4) {
        return (Math.random() + "0.1").toString();
    }
    if (v <= 0.6) {
        return Math.random() > 0.5 + 0.1;
    }

    if (v <= 0.8) {
        return Math.random() > 0.5 ? { a: "1" } : ["1"];
    }

    return Math.random() > 0.5 ? 1 : 2;
}

function generateRandomDecl() {
    const times = Math.floor(Math.random() * 100);

    if (!times) return "";

    const prefix = "var ";

    const obj = new Set();
    const delcs: string[] = [];

    for (let i = 0; i < times; i++) {
        let ident = declIdents[Math.floor(Math.random() * declIdents.length)];

        while (obj.has(ident) || excludeIdents.has(ident)) {
            ident += declIdents[Math.floor(Math.random() * declIdents.length)];
        }

        obj.add(ident);

        delcs.push(ident);
    }

    return (
        prefix +
        delcs
            .map((ident) => `${ident} = ${JSON.stringify(randomVal())}`)
            .join(", ") +
        ";" +
        delcs.map((ident) => `assert(!!${ident})`).join(";")
    );
}

function generateNested(nested: number, content: () => string) {
    if (!nested) return content();

    return `function nested${nested}() {
        ${generateRandomDecl()}
        ${generateNested(nested - 1, content)}
    }
    assert(Math.random());
    nested${nested}();
    `;
}

function generateWithBenchmarkCode(loopTimes = 1000, content: () => string) {
    return `
const start = performance.now();

for(let i = 0; i < ${loopTimes}; i++) {
    ${content}
}

const end = performance.now();

console.log(end - start);
`;
}

function e1() {
    return `
    assert(!!obj.a);
    `;
}

function e2() {
    return `
    assert(!!obj[field]);
    `;
}

function generateContext(code: string) {
    return `
/* eslint-disable */
const obj = { a: 1 };
const field = 'a';
function assert(v) {
    if (typeof v === 'undefined') throw new Error('assertion failed');
}

${code}
    `;
}

interface RunOption {
    name?: string;
    content: () => string;
    disk?: boolean;
}

function run(options: RunOption) {
    const c1 = generateContext(
        generateNested(
            100,
            generateWithBenchmarkCode.bind(null, 1000000000, options.content)
        )
    );

    if (options.disk) {
        writeFileSync(join(process.cwd(), `${options.name}.js`), c1);
    }else {
        eval(c1);
    }
}

function runTimes(times: number, options: RunOption) {
    for (let i = 0; i < times; i++) {
        run(options);
        if (options.disk) {
            break;
        }
    }
}

const disk = process.argv[2] === 'disk';
console.log("start e1");
runTimes(5, { name: 'e1', content: e1, disk });
console.log("start e2");
runTimes(5, { name: 'e2', content: e2, disk });
