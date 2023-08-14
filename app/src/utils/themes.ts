export const initialize = () => {
	const localHasTheme = localStorage.getItem("theme");

	let themeName: string;

	if (localHasTheme) {
		themeName = localHasTheme;
	} else {
		themeName = window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "dark";
	}

	document.documentElement.classList.add(themeName);
};
