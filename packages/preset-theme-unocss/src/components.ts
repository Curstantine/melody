export const buttonShortcuts: Record<string, string> = {
	"button-layout": [
		"inline-flex items-center justify-center",
		"px-4 font-orbiter-deck rounded h-8 transition-colors duration-standard ease-standard",
		"disabled:(opacity-50 pointer-events-none)",
	].join(" "),
	"icon-button-layout": [
		"inline-flex items-center justify-center",
		"rounded transition-colors duration-standard ease-standard",
	].join(" "),
	"button-template-primary": createButtonStyleShortcut("primary"),
	"button-template-error": createButtonStyleShortcut("error"),
	"button-template-text": createButtonStyleShortcut("text"),
};

function createButtonStyleShortcut(name: string) {
	const styles = [
		`bg-button-${name}-background-idle text-button-${name}-text-idle`,
		`hover:(bg-button-${name}-background-hover text-button-${name}-text-hover)`,
		`active:(bg-button-${name}-background-active text-button-${name}-text-active)`,
	];

	return styles.join(" ");
}
