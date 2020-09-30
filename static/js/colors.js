let storedColors = {};

const loadStoredColors = () => {
    const style = getComputedStyle(document.body);

    storedColors = JSON.parse(localStorage.getItem('colors')) ?? {
        '--background': style.getPropertyValue('--background'),
    };
};

const storeColors = () => {
    localStorage.setItem('colors', JSON.stringify(storedColors));
};

const applyColors = () => {
    for (const [name, value] of Object.entries(storedColors)) {
        document.documentElement.style.setProperty(name, value);
    }
};

const makeColorParam = (name, color) => {
    const input = document.createElement('input');
    input.type = 'color'
    input.value = color;
    input.addEventListener('input', () => {
        storedColors[name] = input.value;
        document.documentElement.style.setProperty(name, input.value);
        storeColors();
    });

    const element = document.createElement('li');
    element.textContent = name;
    element.appendChild(input);
    return element;
};

const populateColorsList = () => {
    const colorsList = document.getElementById('colors-list');
    colorsList.appendChild(makeColorParam('--background', storedColors['--background']));
};

window.addEventListener('load', async () => {
    await loadStoredColors();
    await applyColors();
    populateColorsList();
});
