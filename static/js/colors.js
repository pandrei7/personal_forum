/**
 * @file Provides code to interact with the page which allows
 * color customization.
 */

/**
 * Creates a color input for the given parameters.
 * @param {string} name The name of the property.
 * @param {string} color The initial value of the input.
 * @return {HTMLElement} An HTML element ready to be placed on the page.
 */
const makeColorParam = (name, color) => {
    const input = document.createElement('input');
    input.type = 'color'
    input.value = color;
    input.addEventListener('input', async () => {
        const colors = await loadStoredColors();
        colors[name] = input.value;
        storeColors(colors);
        document.documentElement.style.setProperty(name, input.value);
    });

    const description = document.createElement('span');
    description.textContent = name;

    const element = document.createElement('li');
    element.appendChild(input);
    element.appendChild(description);
    return element;
};

/**
 * Creates inputs for the given properties and places them on the page.
 * @param {object} colors An object containing color properties.
 */
const populateColorsList = (colors) => {
    const colorsList = document.getElementById('colors-list');
    for (const [name, color] of Object.entries(colors)) {
        colorsList.appendChild(makeColorParam(name, color));
    }
};

// Populate the inputs on the page with the customizable parameters.
window.addEventListener('load', async () => {
    const colors = await loadStoredColors();
    populateColorsList(colors);
});
