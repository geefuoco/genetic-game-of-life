export default {
  ignoreFiles: ['dist/**', 'wasm-pkg/**', 'test-results/**', 'playwright-report/**'],
  overrides: [{ files: ['**/*.svelte'], customSyntax: 'postcss-html' }],
  rules: {
    'comment-pattern': ['^(?!)', { message: 'Comments are not allowed in CSS' }],
  },
};
