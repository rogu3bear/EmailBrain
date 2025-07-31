import { next } from 'eslint-config-next';
import js from '@eslint/js';

export default [
  js.configs.recommended,
  ...next,
  {
    rules: {
      '@next/next/no-html-link-for-pages': 'off',
    },
  },
];
