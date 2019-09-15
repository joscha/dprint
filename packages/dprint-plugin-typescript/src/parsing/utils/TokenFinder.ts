import * as babel from "@babel/types";
import { BabelToken } from "../BabelToken";

type TokenTexts = "(" | "[" | "<" | "{" | ")" | "]" | ">" | "}" | "else" | "catch" | "finally" | ";" | "&&" | "||" | "??" | "?" | "+" | "-" | "/" | "%" | "*"
    | "**" | "&" | "|" | ">>" | ">>>" | "<<" | "^" | "==" | "===" | "!=" | "!==" | "in" | "instanceof" | ">" | "<" | ">=" | "<=";
type IsMatchFunction = (token: BabelToken) => boolean;

// todo: unit test this class for the edge cases

/**
 * Helps improve the speed of finding tokens in a file by searching
 * from the last found token position.
 */
export class TokenFinder {
    private tokenIndex = 0;

    constructor(private readonly tokens: BabelToken[]) {
    }

    private get currentToken() {
        return this.tokens[this.tokenIndex];
    }

    isFirstTokenInNodeMatch(node: BabelToken | babel.Node, tokenOrIsMatch: TokenTexts | IsMatchFunction) {
        this.moveToNodeStart(node);

        const isMatch = getTokenIsMatchFunction(tokenOrIsMatch);
        return isMatch(this.currentToken);
    }

    getFirstTokenWithin(node: BabelToken | babel.Node, tokenOrIsMatch: TokenTexts | IsMatchFunction): BabelToken | undefined {
        this.moveToNodeStart(node);

        const isMatch = getTokenIsMatchFunction(tokenOrIsMatch);
        while (!isMatch(this.currentToken) && this.currentToken.end <= node.end!) {
            if (this.tokenIndex === this.tokens.length - 1)
                return undefined;
            else
                this.tokenIndex++;
        }

        return isMatch(this.currentToken) ? this.currentToken : undefined;
    }

    getFirstTokenBefore(node: BabelToken | babel.Node, tokenOrIsMatch: TokenTexts | IsMatchFunction) {
        this.moveToNodeStart(node);

        const isMatch = getTokenIsMatchFunction(tokenOrIsMatch);
        do {
            if (this.tokenIndex === 0)
                return undefined;
            this.tokenIndex--;
        } while (!isMatch(this.currentToken));

        return this.currentToken;
    }

    getFirstTokenAfter(node: BabelToken | babel.Node, tokenOrIsMatch: TokenTexts | IsMatchFunction) {
        this.moveToNodeEnd(node);

        const isMatch = getTokenIsMatchFunction(tokenOrIsMatch);
        do {
            if (this.tokenIndex === this.tokens.length - 1)
                return undefined;
            this.tokenIndex++;
        } while (!isMatch(this.currentToken));

        return this.currentToken;
    }

    private moveToNodeStart(node: BabelToken | babel.Node) {
        const nodeStart = node.start!;

        while (this.currentToken.start < nodeStart)
            this.tokenIndex++;
        while (this.currentToken.start > nodeStart)
            this.tokenIndex--;
    }

    private moveToNodeEnd(node: BabelToken | babel.Node) {
        const nodeEnd = node.end!;

        while (this.currentToken.end < nodeEnd)
            this.tokenIndex++;
        while (this.currentToken.end > nodeEnd)
            this.tokenIndex--;
    }
}

function getTokenIsMatchFunction(tokenOrIsMatch: TokenTexts | IsMatchFunction) {
    if (tokenOrIsMatch instanceof Function)
        return tokenOrIsMatch;
    const tokenText = tokenOrIsMatch;
    return (token: BabelToken) => getTokenText(token) === tokenText;
}

function getTokenText(token: BabelToken) {
    if (token.value)
        return token.value;
    if (token.type && typeof token.type !== "string" && token.type.label)
        return token.type.label;
    return undefined;
}