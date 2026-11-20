import type { AST, Rule } from 'eslint';

/**
 * Bans ordinary comments, and with them lint suppression comments, in
 * TypeScript and Svelte, including HTML comments in Svelte markup. Doc
 * comments (`/** ... *\/`) are allowed, matching the Rust comment check.
 */
const rule: Rule.RuleModule = {
  meta: {
    type: 'problem',
    docs: { description: 'Ban ordinary comments; doc comments are allowed' },
    schema: [],
    messages: { ordinary: 'Ordinary comments are not allowed; use a doc comment or remove it.' },
  },
  create(context) {
    const report = (loc: AST.SourceLocation | null | undefined): void => {
      if (loc) context.report({ loc, messageId: 'ordinary' });
    };
    return {
      Program() {
        for (const comment of context.sourceCode.getAllComments()) {
          if (isDoc(comment.type, comment.value)) continue;
          report(comment.loc);
        }
      },
      SvelteHTMLComment(node: { loc: AST.SourceLocation }) {
        report(node.loc);
      },
    } as Rule.RuleListener;
  },
};

/** Whether a comment is a `/** ... *\/` doc comment. */
function isDoc(type: string, value: string): boolean {
  return type === 'Block' && value.startsWith('*') && !value.startsWith('**') && value !== '*';
}

export default rule;
