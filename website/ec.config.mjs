// @ts-check
// Expressive Code for the docs: two quiet themes in the site's colours,
// crisp white and deep obsidian, with cobalt for functions. The values are
// the landing page's code tokens (--mt-code-* in src/styles/tokens.css).
import { defineEcConfig, ExpressiveCodeTheme } from '@astrojs/starlight/expressive-code';

/** @param {'light' | 'dark'} type */
function codeTheme(type) {
	const c =
		type === 'light'
			? { bg: '#fafafa', fg: '#1b1b20', muted: '#8a8a93', comment: '#a3a3ab', keyword: '#8a2be2', string: '#0f7a45', number: '#b3470f', func: '#2338f0', sel: '#2f45ff24' }
			: { bg: '#0d0d10', fg: '#e2e2e6', muted: '#6b6b75', comment: '#5e5e68', keyword: '#c9a8ff', string: '#7ee2a8', number: '#ffc46b', func: '#95a1ff', sel: '#5b6cff38' };
	return new ExpressiveCodeTheme({
		name: `matenadar-${type}`,
		type,
		colors: {
			'editor.background': c.bg,
			'editor.foreground': c.fg,
			'editor.selectionBackground': c.sel,
			'editorLineNumber.foreground': c.comment,
			'terminal.background': c.bg,
			'terminal.foreground': c.fg,
		},
		tokenColors: [
			{ settings: { foreground: c.fg } },
			{ scope: ['comment', 'punctuation.definition.comment'], settings: { foreground: c.comment, fontStyle: 'italic' } },
			{ scope: ['string', 'string.quoted', 'punctuation.definition.string', 'string.template'], settings: { foreground: c.string } },
			{ scope: ['constant.numeric', 'constant.language', 'constant.character', 'keyword.other.unit'], settings: { foreground: c.number } },
			{
				scope: ['keyword', 'storage', 'storage.type', 'storage.modifier', 'keyword.control', 'keyword.operator.new', 'variable.language'],
				settings: { foreground: c.keyword },
			},
			{
				scope: ['entity.name.function', 'support.function', 'meta.function-call entity.name.function', 'entity.name.command', 'support.function.builtin'],
				settings: { foreground: c.func },
			},
			{ scope: ['entity.name.type', 'entity.name.class', 'support.class', 'support.type', 'entity.name.namespace'], settings: { foreground: c.func } },
			{
				scope: ['support.type.property-name', 'support.type.property-name.json', 'meta.object-literal.key', 'variable.other.property', 'entity.name.tag'],
				settings: { foreground: c.fg },
			},
			{ scope: ['constant.other.option', 'variable.parameter', 'variable.other.normal.shell'], settings: { foreground: c.keyword } },
			{ scope: ['punctuation', 'meta.brace', 'keyword.operator', 'punctuation.separator'], settings: { foreground: c.muted } },
			{ scope: ['markup.inserted'], settings: { foreground: c.string } },
			{ scope: ['markup.deleted'], settings: { foreground: '#ef4444' } },
			{ scope: ['markup.italic'], settings: { fontStyle: 'italic' } },
			{ scope: ['markup.bold'], settings: { fontStyle: 'bold' } },
		],
	});
}

export default defineEcConfig({
	themes: [codeTheme("dark"), codeTheme("light")],
	useStarlightDarkModeSwitch: true,
	useStarlightUiThemeColors: false,
	styleOverrides: {
		borderRadius: '10px',
		borderWidth: '1px',
		borderColor: 'var(--mt-border)',
		codeBackground: 'var(--mt-code-bg)',
		codeFontFamily: 'var(--mt-mono)',
		codeFontSize: '0.8125rem',
		codeLineHeight: '1.75',
		uiFontFamily: 'var(--mt-sans)',
		uiFontSize: '0.8125rem',
		focusBorder: 'var(--mt-accent)',
		frames: {
			shadowColor: 'transparent',
			frameBoxShadowCssValue: 'none',
			editorBackground: 'var(--mt-code-bg)',
			terminalBackground: 'var(--mt-code-bg)',
			editorTabBarBackground: 'var(--mt-code-bar)',
			terminalTitlebarBackground: 'var(--mt-code-bar)',
			editorActiveTabBackground: 'var(--mt-code-bg)',
			editorActiveTabIndicatorTopColor: 'transparent',
			editorActiveTabIndicatorBottomColor: 'transparent',
			editorTabBarBorderBottomColor: 'var(--mt-border)',
			terminalTitlebarBorderBottomColor: 'var(--mt-border)',
			terminalTitlebarDotsForeground: 'var(--mt-fg-faint)',
			terminalTitlebarDotsOpacity: '0.6',
			inlineButtonBackground: 'var(--mt-fg)',
			inlineButtonForeground: 'var(--mt-fg)',
			inlineButtonBorder: 'var(--mt-border-strong)',
			tooltipSuccessBackground: 'var(--mt-ok)',
			tooltipSuccessForeground: '#ffffff',
		},
	},
});
