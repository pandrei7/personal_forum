/** @file Provides code needed by all pages. */

/**
 * Loads the chosen colors from storage and returns them.
 *
 * If there are no colors stored, the default values are returned.
 *
 * @return {Promise<object>} The object containing the color properties.
 */
const loadStoredColors = async () => {
    const style = getComputedStyle(document.body);

    const storedColors = JSON.parse(localStorage.getItem('colors')) ?? {
        '--background': style.getPropertyValue('--background'),
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
