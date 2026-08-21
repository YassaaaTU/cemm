import betterTailwindcss from 'eslint-plugin-better-tailwindcss'
import { getDefaultSelectors } from 'eslint-plugin-better-tailwindcss/defaults'
import withNuxt from './.nuxt/eslint.config.mjs'

export default withNuxt(
	// Vue-specific rules
	{
		files: ['**/*.vue'],
		rules: {
			'vue/multi-word-component-names': 'error',
			'vue/component-name-in-template-casing': ['error', 'kebab-case'],
			'vue/attributes-order': 'error',
			'vue/require-v-for-key': 'error',
			'vue/no-use-v-if-with-v-for': 'error',
			'vue/html-self-closing': ['error', {
				html: { void: 'always', normal: 'always', component: 'always' },
				svg: 'always',
				math: 'always'
			}]
		}
	},	{
		files: ['app/app.vue', 'app/error.vue', 'app/pages/**/*.vue', 'app/layouts/**/*.vue'],
		rules: {
			'vue/multi-word-component-names': 'off'
		}
	},
	// General JavaScript/TypeScript rules
	{
		rules: {
			// Stylistic rules
			'@stylistic/indent': ['error', 'tab'],
			'@stylistic/no-tabs': ['off', { allowIndentationTabs: true }],
			'@stylistic/semi': ['error', 'never'],
			'@stylistic/quotes': ['error', 'single'],
			'@stylistic/brace-style': ['error', 'allman'],
			'@stylistic/comma-dangle': ['error', 'never'],
			'@stylistic/block-spacing': ['error', 'always'],
			'@stylistic/arrow-parens': ['error', 'always'],

			// General JavaScript/TypeScript rules from instructions
			'curly': ['error', 'multi-line', 'consistent'],
			'eqeqeq': ['error', 'always', { null: 'ignore' }],
			'prefer-arrow-callback': 'error',
			'object-shorthand': 'error',
			// Use custom pino logger rule instead of no-console
			// 'pino-logger/no-console-to-pino-logger': 'warn',
			'no-console': 'off',
			'no-var': 'error',
			'prefer-const': 'error'
		},
		plugins: {
			'pino-logger': (await import('eslint-plugin-pino-logger/index.js')).default || (await import('eslint-plugin-pino-logger/index.js'))
		}
	},
	// Import/Export rules configuration
	{
		files: ['**/*.{js,ts,vue}'],
		plugins: {
			'simple-import-sort': (await import('eslint-plugin-simple-import-sort')).default
		}, rules: {
			// Import/Export rules from instructions
			'simple-import-sort/imports': 'error',
			'simple-import-sort/exports': 'error',
			// Use Nuxt's built-in import rules
			'import/newline-after-import': 'error',
			'import/no-duplicates': 'error',
			'import/no-empty-named-blocks': 'error',
			'import/first': 'error'
		} },
	// Tailwind CSS rules (mirrors nk-glanz; catches what tailwindcss-intellisense hints at)
	{
		name: 'better-tailwindcss',
		plugins: {
			'better-tailwindcss': betterTailwindcss
		},
		settings: {
			'better-tailwindcss': {
				entryPoint: 'app/assets/css/main.css',
				// `anim(...)` from useMotion returns its argument as class list —
				// lint string arguments inside those calls too (not a default callee)
				selectors: [
					...getDefaultSelectors(),
					{
						kind: 'callee',
						name: '^anim$',
						match: [{ type: 'strings' }]
					}
				]
			}
		},
		rules: {
			'better-tailwindcss/enforce-shorthand-classes': 'off',
			'better-tailwindcss/enforce-consistent-important-position': 'off',
			'better-tailwindcss/enforce-consistent-variable-syntax': 'off',
			'better-tailwindcss/enforce-consistent-class-order': 'warn',
			'better-tailwindcss/enforce-canonical-classes': 'warn'
		}
	},
	// Ignore patterns
	{
		// app/types/generated is written by ts-rs from the Rust structs; its
		// formatting is the generator's business, not this config's.
		ignores: ['.nuxt/**', 'node_modules/**', '.output/**', 'src-tauri/target/**', 'dist/**', 'app/types/generated/**']
	}
).override('nuxt/typescript/rules', {
	files: ['**/*.ts', '**/*.tsx', '**/*.vue'],
	languageOptions: {
		parserOptions: {
			project: './tsconfig.json',
			tsconfigRootDir: import.meta.dirname
		}
	},
	rules: {
		'@typescript-eslint/no-unnecessary-condition': 'error',
		'@typescript-eslint/only-throw-error': 'off',
		'@typescript-eslint/no-non-null-assertion': 'error',
		'@typescript-eslint/no-explicit-any': 'warn',
		'@typescript-eslint/no-unused-vars': [
			'warn',
			{
				varsIgnorePattern: '^_',
				argsIgnorePattern: '^_',
				caughtErrorsIgnorePattern: '^_'
			}
		],
		'@typescript-eslint/no-unused-expressions': [
			'error',
			{ allowShortCircuit: true }
		],
		'@typescript-eslint/strict-boolean-expressions': [
			'error',
			{
				allowNullableString: false,
				allowNullableNumber: false,
				allowNullableObject: false
			}
		],
		// Additional TypeScript rules from instructions
		'@typescript-eslint/no-unnecessary-boolean-literal-compare': 'error',
		'@typescript-eslint/array-type': ['error', { default: 'array-simple' }],
		'@typescript-eslint/consistent-type-imports': [
			'error',
			{ prefer: 'type-imports', fixStyle: 'inline-type-imports' }
		],
		'@typescript-eslint/no-import-type-side-effects': 'error'
	}
})
