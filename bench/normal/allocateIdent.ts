const declIdents = "abcdefghijklmnopqrstuvwxyz0123456789".split("");

const allocateIdentChar = () =>
    declIdents[Math.floor(Math.random() * declIdents.length)];

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
    "as",
]);

interface AllocateIdentOptions {
    minLength?: number;
    excludeSets?: Set<string>[];
}

export function allocateIdent(options: AllocateIdentOptions = {}): string {
    const { minLength = 1, excludeSets = [] } = options;
    let newIdent = allocateIdentChar();

    while (
        newIdent.length < minLength ||
        excludeSets.some((i) => i.has(newIdent)) ||
        excludeIdents.has(newIdent)
    ) {
        newIdent += allocateIdentChar();
    }

    if (newIdent[0] >= "0" && newIdent[0] <= "9") {
        newIdent = "_" + newIdent;
    }

    return newIdent;
}
