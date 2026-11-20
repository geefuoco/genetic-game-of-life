import { RuleTester } from 'eslint';
import svelteParser from 'svelte-eslint-parser';
import tseslint from 'typescript-eslint';
import { afterAll, describe, it } from 'vitest';
import noComments from './no-comments.ts';

Object.assign(RuleTester, { afterAll, describe, it });

const typescript = new RuleTester({
  languageOptions: { parser: tseslint.parser },
  linterOptions: { noInlineConfig: true },
});
const svelte = new RuleTester({
  languageOptions: { parser: svelteParser, parserOptions: { parser: tseslint.parser } },
});

const comment = { messageId: 'ordinary' };

typescript.run('no-comments in TypeScript', noComments, {
  valid: [
    'const a = 1;',
    '/** Doc comment on a constant. */\nconst a = 1;',
    '/**\n * Multi-line doc comment.\n */\nfunction f(): void {}',
    'const url = "http://example.com";',
    'const s = "/* not a comment */ // nor this";',
    'const r = /\\/\\//;',
  ],
  invalid: [
    { code: '// line comment\nconst a = 1;', errors: [comment] },
    { code: 'const a = 1; // trailing', errors: [comment] },
    { code: '/* block comment */\nconst a = 1;', errors: [comment] },
    {
      code: '/* eslint-disable */\nconst a = 1;',
      errors: [{ message: /has no effect because you have 'noInlineConfig'/ }, comment],
    },
    { code: '/// <reference types="vite/client" />\nconst a = 1;', errors: [comment] },
    { code: '/**/\nconst a = 1;', errors: [comment] },
    { code: '/*** banner ***/\nconst a = 1;', errors: [comment] },
    { code: '// one\n// two\nconst a = 1;', errors: [comment, comment] },
  ],
});

svelte.run('no-comments in Svelte', noComments, {
  valid: [
    { filename: 'A.svelte', code: '<script lang="ts">\n/** Doc. */\nconst a = 1;\n</script>\n<p>{a}</p>' },
    { filename: 'A.svelte', code: '<p>&lt;!-- shown as text --&gt;</p>' },
  ],
  invalid: [
    { filename: 'A.svelte', code: '<!-- html comment -->\n<p>hi</p>', errors: [comment] },
    { filename: 'A.svelte', code: '<div>\n  <!-- nested -->\n</div>', errors: [comment] },
    { filename: 'A.svelte', code: '<script lang="ts">\n// script comment\nconst a = 1;\n</script>', errors: [comment] },
    { filename: 'A.svelte', code: '<p>{/* expression comment */ 1}</p>', errors: [comment] },
  ],
});
