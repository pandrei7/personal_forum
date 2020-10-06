/** @file Provides code needed by all pages. */

/**
 * Loads the chosen colors from storage and returns them.
 *
 * If there are no colors stored, the default values are returned.
 *
 * @return {Promise<object>} The object containing the color properties.
 */
const loadStoredColors = async () => {
    const style = getComputedStyle(document.documentElement);

    const storedColors = JSON.parse(localStorage.getItem('colors')) ?? {
        '--background1': style.getPropertyValue('--background1'),
        '--background2': style.getPropertyValue('--background2'),
        '--primary1': style.getPropertyValue('--primary1'),
        '--primary2': style.getPropertyValue('--primary2'),
        '--secondary1': style.getPropertyValue('--secondary1'),
        '--secondary2': style.getPropertyValue('--secondary2'),
        '--extra1': style.getPropertyValue('--extra1'),
        '--extra2': style.getPropertyValue('--extra2'),
        '--text-color1': style.getPropertyValue('--text-color1'),
        '--text-color2': style.getPropertyValue('--text-color2'),
        '--text-color3': style.getPropertyValue('--text-color3'),
        '--text-color4': style.getPropertyValue('--text-color4'),
        '--text-faded1': style.getPropertyValue('--text-faded1'),
        '--mark-background': style.getPropertyValue('--mark-background'),
        '--mark-text-color': style.getPropertyValue('--mark-text-color'),
    };
    return storedColors;
};

/**
 * Stores the given colors to make them "persistent".
 * @param {object} colors An object containing the color properties.
 */
const storeColors = (colors) => {
    localStorage.setItem('colors', JSON.stringify(colors));
};

/**
 * Applies the given colors on the current page, making them visible.
 * @param {object} colors An object containing the color properties.
 */
const applyColors = (colors) => {
    for (const [name, value] of Object.entries(colors)) {
        document.documentElement.style.setProperty(name, value);
    }
};

/**
 * Changes the color theme of the site, by storing the given colors and applying them.
 * @param {object} newColors An object containing the color properties.
 */
const changeColors = (newColors) => {
    storeColors(newColors);
    applyColors(newColors);
};

// Make sure the chosen colors are applied to every page, when loaded.
window.addEventListener('load', async () => {
    const colors = await loadStoredColors();
    storeColors(colors); // Store the defaults if they are not stored yet.
    applyColors(colors);
});
